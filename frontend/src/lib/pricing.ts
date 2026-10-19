// The POS pricing engine (Requirement Analysis §8.2) — the same algorithm as
// the server's `pos_sales::pricing`, locked by the shared test vectors in
// `pricing.vectors.json`. The server stays authoritative (POS-R01); this
// copy shows the totals while the cashier works.

import { type Dec, ZERO, dec, div, max, min, mul, round, sub, sum } from './decimal'

export type DiscountType = 'amount' | 'percent'

export interface Discount {
  type: DiscountType
  value: Dec
}

export interface LineInput {
  quantity: Dec
  unitPrice: Dec
  discount?: Discount | null
  /** Percent, e.g. 15. */
  taxRatePct: Dec
}

export interface CartInput {
  lines: LineInput[]
  invoiceDiscount?: Discount | null
  shipping?: Dec
  other?: Dec
  pricesIncludeTax: boolean
  /** 0 = none, else the step (0.50, 1.00). */
  cashRounding: Dec
}

export interface LineResult {
  gross: Dec
  lineDiscount: Dec
  netBeforeInvoice: Dec
  invoiceDiscount: Dec
  taxable: Dec
  tax: Dec
  lineTotal: Dec
}

export interface Totals {
  lines: LineResult[]
  subtotal: Dec
  itemDiscountTotal: Dec
  invoiceDiscount: Dec
  taxTotal: Dec
  shipping: Dec
  other: Dec
  preRoundTotal: Dec
  roundingAdjustment: Dec
  grandTotal: Dec
}

export class PricingError extends Error {
  readonly code: string
  readonly field: string

  constructor(code: string, field: string) {
    super(code)
    this.name = 'PricingError'
    this.code = code
    this.field = field
  }
}

const HUNDRED = dec(100)
const r2 = (v: Dec) => round(v, 2)

function discountAmount(base: Dec, d: Discount | null | undefined, field: string): Dec {
  if (!d) return ZERO
  if (d.value < ZERO) throw new PricingError('INVALID_DISCOUNT', field)
  let value: Dec
  if (d.type === 'percent') {
    if (d.value > HUNDRED) throw new PricingError('INVALID_DISCOUNT', field)
    value = r2(div(mul(base, d.value), HUNDRED))
  } else {
    value = r2(d.value)
  }
  if (value > base) throw new PricingError('DISCOUNT_TOO_LARGE', field)
  return value
}

/** Rounds to the nearest multiple of `step` (cash rounding); 0 = none. */
export function roundToStep(value: Dec, step: Dec): Dec {
  if (step <= ZERO) return value
  return r2(mul(round(div(value, step), 0), step))
}

export function calculate(cart: CartInput): Totals {
  const lines: LineResult[] = cart.lines.map((line, i) => {
    const gross = r2(mul(line.quantity, line.unitPrice))
    const lineDiscount = discountAmount(gross, line.discount, `lines[${i}].discount_value`)
    return { gross, lineDiscount, netBeforeInvoice: gross - lineDiscount, invoiceDiscount: ZERO, taxable: ZERO, tax: ZERO, lineTotal: ZERO }
  })
  const netSum = sum(lines.map((l) => l.netBeforeInvoice))
  const invoiceDiscount = discountAmount(netSum, cart.invoiceDiscount, 'invoice_discount_value')

  if (invoiceDiscount !== ZERO) {
    let last = 0
    lines.forEach((l, i) => {
      if (l.netBeforeInvoice !== ZERO) last = i
    })
    let allocated = ZERO
    lines.forEach((l, i) => {
      const share = i === last ? invoiceDiscount - allocated : r2(div(mul(invoiceDiscount, l.netBeforeInvoice), netSum))
      l.invoiceDiscount = max(min(share, l.netBeforeInvoice), ZERO)
      allocated += l.invoiceDiscount
    })
    if (allocated !== invoiceDiscount) throw new PricingError('DISCOUNT_TOO_LARGE', 'invoice_discount_value')
  }

  lines.forEach((l, i) => {
    const rate = cart.lines[i]?.taxRatePct ?? ZERO
    l.taxable = l.netBeforeInvoice - l.invoiceDiscount
    if (cart.pricesIncludeTax) {
      l.tax = r2(div(mul(l.taxable, rate), HUNDRED + rate))
      l.lineTotal = l.taxable
    } else {
      l.tax = r2(div(mul(l.taxable, rate), HUNDRED))
      l.lineTotal = l.taxable + l.tax
    }
  })

  const shipping = r2(cart.shipping ?? ZERO)
  const other = r2(cart.other ?? ZERO)
  const preRoundTotal = sum(lines.map((l) => l.lineTotal)) + shipping + other
  const grandTotal = roundToStep(preRoundTotal, cart.cashRounding)
  return {
    lines,
    subtotal: sum(lines.map((l) => l.gross)),
    itemDiscountTotal: sum(lines.map((l) => l.lineDiscount)),
    invoiceDiscount,
    taxTotal: sum(lines.map((l) => l.tax)),
    shipping,
    other,
    preRoundTotal,
    roundingAdjustment: sub(grandTotal, preRoundTotal),
    grandTotal,
  }
}
