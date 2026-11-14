// POS cart state. Every amount and quantity is kept as the text the cashier
// typed and turned into exact decimals (lib/decimal) only for pricing; the
// cart is plain JSON so it can be held on the server and resumed.

import type { Customer } from '../../api/parties'
import type { DiscountType, PosContext, PosItem, SaleInput, SalePaymentInput } from '../../api/sales'
import { type Dec, ZERO, dec, mul, round, toFixed, toPlain, tryParse } from '../../lib/decimal'
import { type Discount, PricingError, type Totals, calculate } from '../../lib/pricing'
import { type EngineLine, type Output as PromoOutput, type Promotion, applyPromotions, scheduleOpen } from '../../lib/promotions'
import type { LivePromotion } from '../../api/promotions'

export interface CartLine {
  item: PosItem
  quantity: string
  unitPrice: string
  discountType: DiscountType
  discountValue: string
  /** Automatic promotions removed from this line by the cashier (`pos.remove_promotion`). */
  promotionsOff?: boolean
}

export interface CartCustomer {
  id: string
  name: string
  phone: string | null
  /** `null` = no limit. */
  creditLimit: string | null
  balance: string
}

export function toCartCustomer(c: Customer): CartCustomer {
  return { id: c.id, name: c.name, phone: c.phone, creditLimit: c.credit_limit, balance: c.current_balance }
}

export interface Cart {
  lines: CartLine[]
  /** `null` = walk-in customer. */
  customer: CartCustomer | null
  invoiceDiscountType: DiscountType
  invoiceDiscountValue: string
  /** A coupon the server validated for this cart (`POST /pos/coupons/validate`). */
  coupon?: Promotion | null
}

export const emptyCart: Cart = { lines: [], customer: null, invoiceDiscountType: 'amount', invoiceDiscountValue: '', coupon: null }

export type CartAction =
  | { type: 'add'; item: PosItem }
  | { type: 'quantity'; variantId: string; quantity: string }
  | { type: 'step'; variantId: string; delta: 1 | -1 }
  | { type: 'price'; variantId: string; price: string }
  | { type: 'discount'; variantId: string; discountType: DiscountType; value: string }
  | { type: 'remove'; variantId: string }
  | { type: 'customer'; customer: CartCustomer | null }
  | { type: 'invoiceDiscount'; discountType: DiscountType; value: string }
  | { type: 'promotionsOff'; variantId: string; off: boolean }
  | { type: 'coupon'; coupon: Promotion | null }
  | { type: 'replace'; cart: Cart }
  | { type: 'clear' }

const ONE = dec(1)

function update(cart: Cart, variantId: string, patch: (line: CartLine) => CartLine): Cart {
  return { ...cart, lines: cart.lines.map((l) => (l.item.variant_id === variantId ? patch(l) : l)) }
}

export function cartReducer(cart: Cart, action: CartAction): Cart {
  switch (action.type) {
    case 'add': {
      const existing = cart.lines.find((l) => l.item.variant_id === action.item.variant_id)
      if (existing) return update(cart, action.item.variant_id, (l) => ({ ...l, quantity: toPlain(dec(l.quantity) + ONE) }))
      const line: CartLine = {
        item: action.item,
        quantity: '1',
        unitPrice: toPlain(dec(action.item.selling_price)),
        discountType: 'amount',
        discountValue: '',
      }
      return { ...cart, lines: [...cart.lines, line] }
    }
    case 'quantity':
      return update(cart, action.variantId, (l) => ({ ...l, quantity: action.quantity }))
    case 'step':
      return update(cart, action.variantId, (l) => {
        const next = dec(l.quantity) + (action.delta === 1 ? ONE : -ONE)
        return next > ZERO ? { ...l, quantity: toPlain(next) } : l
      })
    case 'price':
      return update(cart, action.variantId, (l) => ({ ...l, unitPrice: action.price }))
    case 'discount':
      return update(cart, action.variantId, (l) => ({ ...l, discountType: action.discountType, discountValue: action.value }))
    case 'remove':
      return { ...cart, lines: cart.lines.filter((l) => l.item.variant_id !== action.variantId) }
    case 'customer':
      return { ...cart, customer: action.customer }
    case 'invoiceDiscount':
      return { ...cart, invoiceDiscountType: action.discountType, invoiceDiscountValue: action.value }
    case 'promotionsOff':
      return update(cart, action.variantId, (l) => ({ ...l, promotionsOff: action.off }))
    case 'coupon':
      return { ...cart, coupon: action.coupon }
    case 'replace':
      return action.cart
    case 'clear':
      return emptyCart
  }
}

/** What is wrong with a line's inputs (translation key under `pos.errors`). */
export type LineProblem = 'quantity' | 'wholeNumber' | 'price' | 'discount' | 'discountTooLarge' | 'discountOverLimit' | 'invoiceWithPromotions'

export interface CartTotals {
  totals: Totals | null
  /** Per variant id. */
  lineProblems: Record<string, LineProblem>
  invoiceProblem: LineProblem | null
  /** Promotions and coupon on the cart (engine result, lines in cart order). */
  promotions: PromoOutput | null
}

/** Promotions the cart is priced with. */
export interface PromoInput {
  live: LivePromotion[]
  /** The business's local time now (`YYYY-MM-DDTHH:MM:SS`). */
  now: string | null
  promotionsStack: boolean
  couponStacks: boolean
  /** The invoice discount (F7) is allowed together with promotions. */
  manualWithPromotions: boolean
}

export const noPromotions: PromoInput = { live: [], now: null, promotionsStack: false, couponStacks: false, manualWithPromotions: true }

/** The engine's view of a cart line. */
export function engineLine(l: CartLine, quantity: Dec, unitPrice: Dec): EngineLine {
  const i = l.item
  return {
    variantId: i.variant_id,
    productId: i.product_id,
    categoryIds: [i.category_id, i.category_parent_id, i.category_grandparent_id].filter((c): c is string => !!c),
    brandId: i.brand_id ?? null,
    quantity,
    unitPrice,
    minUnitPrice: i.min_selling_price === null ? null : dec(i.min_selling_price),
    promotionsOff: !!l.promotionsOff,
  }
}

function discountOf(type: DiscountType, value: string): Discount | null | 'invalid' {
  if (!value.trim()) return null
  const v = tryParse(value)
  if (v === null || v < ZERO) return 'invalid'
  return v === ZERO ? null : { type, value: v }
}

/** Prices the cart with the shared engine (same as the server, POS-R01). */
export function priceCart(
  cart: Cart,
  ctx: Pick<PosContext, 'prices_include_tax' | 'cash_rounding' | 'cashier_max_discount_pct'>,
  canDiscountOverLimit: boolean,
  promo: PromoInput = noPromotions,
): CartTotals {
  const lineProblems: Record<string, LineProblem> = {}
  const inputs = cart.lines.map((l) => {
    const qty = tryParse(l.quantity)
    const price = tryParse(l.unitPrice)
    const discount = discountOf(l.discountType, l.discountValue)
    const id = l.item.variant_id
    if (qty === null || qty <= ZERO || round(qty, 3) !== qty) lineProblems[id] = 'quantity'
    else if (!l.item.allow_fraction && round(qty, 0) !== qty) lineProblems[id] = 'wholeNumber'
    else if (price === null || price < ZERO || round(price, 4) !== price) lineProblems[id] = 'price'
    else if (discount === 'invalid' || (discount?.type === 'percent' && discount.value > dec(100))) lineProblems[id] = 'discount'
    return {
      quantity: qty ?? ZERO,
      unitPrice: price ?? ZERO,
      discount: discount === 'invalid' ? null : discount,
      taxRatePct: dec(l.item.tax_rate_pct),
    }
  })
  const invoice = discountOf(cart.invoiceDiscountType, cart.invoiceDiscountValue)
  let invoiceProblem: LineProblem | null = invoice === 'invalid' || (invoice?.type === 'percent' && invoice.value > dec(100)) ? 'discount' : null
  if (Object.keys(lineProblems).length > 0 || invoiceProblem) return { totals: null, lineProblems, invoiceProblem, promotions: null }

  // Promotions live now (the server's clock), then the coupon (§5 steps 2–4).
  const live = promo.now === null ? promo.live : promo.live.filter((p) => scheduleOpen(p, promo.now!))
  const promotions = applyPromotions(
    cart.lines.map((l, i) => engineLine(l, inputs[i]!.quantity, inputs[i]!.unitPrice)),
    live,
    cart.coupon ?? null,
    { promotionsStack: promo.promotionsStack, couponStacks: promo.couponStacks },
  )
  if (!promo.manualWithPromotions && invoice && invoice !== 'invalid' && promotions.lines.some((l) => l.amount > ZERO)) {
    invoiceProblem = 'invoiceWithPromotions'
    return { totals: null, lineProblems, invoiceProblem, promotions }
  }

  let totals: Totals
  try {
    totals = calculate({
      lines: inputs.map((l, i) => ({ ...l, promoDiscount: promotions.lines[i]!.amount })),
      invoiceDiscount: invoice === 'invalid' ? null : invoice,
      pricesIncludeTax: ctx.prices_include_tax,
      cashRounding: dec(ctx.cash_rounding),
    })
  } catch (err) {
    if (!(err instanceof PricingError)) throw err
    const m = /^lines\[(\d+)\]/.exec(err.field)
    const line = m ? cart.lines[Number(m[1])] : undefined
    const problem: LineProblem = err.code === 'DISCOUNT_TOO_LARGE' ? 'discountTooLarge' : 'discount'
    if (line) lineProblems[line.item.variant_id] = problem
    else invoiceProblem = problem
    return { totals: null, lineProblems, invoiceProblem, promotions }
  }

  // Discount limit (POS-R03), as the server checks it: discount / base × 100,
  // on the price after promotions (promotions are the business's own offers).
  if (!canDiscountOverLimit) {
    const limit = dec(ctx.cashier_max_discount_pct)
    const over = (disc: Dec, base: Dec) => base > ZERO && disc * 100n > mul(limit, base)
    totals.lines.forEach((r, i) => {
      const line = cart.lines[i]
      if (line && over(r.lineDiscount, r.gross - r.promoDiscount)) lineProblems[line.item.variant_id] = 'discountOverLimit'
    })
    const net = totals.lines.reduce((s, r) => s + r.netBeforeInvoice, ZERO)
    if (over(totals.invoiceDiscount, net)) invoiceProblem = 'discountOverLimit'
  }
  return { totals, lineProblems, invoiceProblem, promotions }
}

/** The `POST /sales` body for this cart and tendered payments. */
export function saleInput(cart: Cart, totals: Totals, payments: SalePaymentInput[], note: string, couponApplied = false): SaleInput {
  const input: SaleInput = {
    lines: cart.lines.map((l) => {
      const line: SaleInput['lines'][number] = { variant_id: l.item.variant_id, quantity: toPlain(dec(l.quantity)) }
      if (dec(l.unitPrice) !== dec(l.item.selling_price)) line.unit_price = toPlain(dec(l.unitPrice))
      if (dec(l.discountValue) > ZERO) {
        line.discount_type = l.discountType
        line.discount_value = toPlain(dec(l.discountValue))
      }
      if (l.promotionsOff) line.no_promotions = true
      return line
    }),
    payments,
    expected_total: toFixed(totals.grandTotal, 2),
  }
  if (cart.customer) input.customer_id = cart.customer.id
  // Only a coupon the engine applied is sent (the server checks it again).
  if (cart.coupon?.coupon_code && couponApplied) input.coupon_code = cart.coupon.coupon_code
  if (dec(cart.invoiceDiscountValue) > ZERO) {
    input.invoice_discount_type = cart.invoiceDiscountType
    input.invoice_discount_value = toPlain(dec(cart.invoiceDiscountValue))
  }
  if (note.trim()) input.note = note.trim()
  return input
}

/** Held-cart JSON (decimal strings only). */
export function serializeCart(cart: Cart): Record<string, unknown> {
  return { version: 1, ...cart }
}

function isObject(v: unknown): v is Record<string, unknown> {
  return typeof v === 'object' && v !== null
}

/** Restores a held cart; unknown shapes give an empty cart. */
export function deserializeCart(raw: Record<string, unknown>): Cart {
  const lines = Array.isArray(raw.lines) ? raw.lines : []
  const str = (v: unknown, fallback = '') => (typeof v === 'string' ? v : fallback)
  const type = (v: unknown): DiscountType => (v === 'percent' ? 'percent' : 'amount')
  return {
    lines: lines
      .filter((l): l is Record<string, unknown> => isObject(l) && isObject(l.item))
      .map((l) => {
        const item = l.item as unknown as PosItem
        return {
          item,
          quantity: str(l.quantity, '1'),
          unitPrice: str(l.unitPrice, item.selling_price),
          discountType: type(l.discountType),
          discountValue: str(l.discountValue),
          promotionsOff: l.promotionsOff === true,
        }
      }),
    customer: isObject(raw.customer) ? (raw.customer as unknown as CartCustomer) : null,
    invoiceDiscountType: type(raw.invoiceDiscountType),
    invoiceDiscountValue: str(raw.invoiceDiscountValue),
    // The coupon is checked again when the cart is resumed.
    coupon: isObject(raw.coupon) ? (raw.coupon as unknown as Promotion) : null,
  }
}

/**
 * Re-reads product data after resuming a held cart (prices may have
 * changed): list prices follow the current price, overridden ones stay.
 */
export function refreshLine(line: CartLine, fresh: PosItem): CartLine {
  const overridden = dec(line.unitPrice) !== dec(line.item.selling_price)
  return { ...line, item: fresh, unitPrice: overridden ? line.unitPrice : toPlain(dec(fresh.selling_price)) }
}

/** Smallest multiple of `step` that is ≥ `value` (quick cash notes). */
export function ceilTo(value: Dec, step: Dec): Dec {
  if (step <= ZERO) return value
  return ((value + step - 1n) / step) * step
}
