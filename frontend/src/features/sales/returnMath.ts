// Refund estimates for the return form. The server keeps the last units'
// rounding exact; the screen shows the pro-rata share (RET-R04).

import type { SaleDetail, SaleItem, SaleReturnDetail } from '../../api/sales'
import { type Dec, ZERO, dec, div, mul, round, tryParse } from '../../lib/decimal'

export function returnable(item: SaleItem): Dec {
  const left = dec(item.quantity) - dec(item.returned_quantity)
  return left > ZERO ? left : ZERO
}

/**
 * Buy X Get Y on the same product (server: `returns::bogo_refund`): the units
 * kept are charged as if bought alone, so returning the paid unit of a "buy 1
 * get 1" refunds nothing and returning both refunds the line.
 */
function bogoRefund(quantity: Dec, free: Dec, group: Dec, get: Dec, lineTotal: Dec, paidLeft: Dec, left: Dec, qty: Dec): Dec {
  if (qty >= left) return paidLeft
  const paidUnits = quantity - free
  if (paidUnits <= ZERO || lineTotal === ZERO || group <= ZERO) return ZERO
  const kept = left - qty
  const sets = (kept / group) * dec(1)
  const entitled = free < mul(sets, get) ? free : mul(sets, get)
  const charge = round(div(mul(kept - entitled, lineTotal), paidUnits), 2)
  const refund = paidLeft - charge
  return refund < ZERO ? ZERO : refund > paidLeft ? paidLeft : refund
}

/** line_total × qty / quantity, to 2 decimals (the Buy X Get Y rule for lines with free units). */
export function estimatedRefund(item: SaleItem, qty: Dec): Dec {
  const quantity = dec(item.quantity)
  if (quantity === ZERO || qty <= ZERO) return ZERO
  const rule = (item.discounts ?? []).find((d) => d.promotion_type === 'bxgy' && d.bxgy_get_target_type === 'same_product')
  const free = dec(item.free_quantity ?? '0')
  if (rule && free > ZERO && rule.bxgy_buy_quantity && rule.bxgy_get_quantity) {
    const group = dec(rule.bxgy_buy_quantity) + dec(rule.bxgy_get_quantity)
    const get = dec(rule.bxgy_get_quantity)
    const lineTotal = dec(item.line_total)
    const returned = dec(item.returned_quantity)
    // What earlier returns took, by the same rule (the server uses the stored amounts).
    const before = returned > ZERO ? bogoRefund(quantity, free, group, get, lineTotal, lineTotal, quantity, returned) : ZERO
    return bogoRefund(quantity, free, group, get, lineTotal, lineTotal - before, quantity - returned, qty)
  }
  return round(div(mul(dec(item.line_total), qty), quantity), 2)
}

export type QtyCheck = { ok: true; qty: Dec } | { ok: false; error: 'invalid' | 'tooMany' | 'decimals' }

/** Validates the typed quantity (blank = 0). */
export function checkQty(text: string, item: SaleItem): QtyCheck {
  if (!text.trim()) return { ok: true, qty: ZERO }
  const qty = tryParse(text)
  if (qty === null || qty < ZERO) return { ok: false, error: 'invalid' }
  if (round(qty, 3) !== qty) return { ok: false, error: 'decimals' }
  if (qty > returnable(item)) return { ok: false, error: 'tooMany' }
  return { ok: true, qty }
}

/** One line of an invoice printed without what was returned. */
export interface NetLine {
  item: SaleItem
  quantity: Dec
  lineTotal: Dec
  lineDiscount: Dec
}

/** An invoice as it stands after its returns (printing only; the sale is unchanged). */
export interface NetInvoice {
  /** Lines with something left; fully returned lines are dropped. */
  lines: NetLine[]
  subtotal: Dec
  discount: Dec
  tax: Dec
  grandTotal: Dec
  refunded: Dec
  credited: Dec
  returnNos: string[]
}

/**
 * The invoice without its returned items. Line totals and VAT use the exact
 * amounts each return took off (the server rounds the last units exactly), so
 * the total is always `grand_total − returned_total`. Discounts are the
 * kept share of each line's discount, and the subtotal is what makes the
 * printed rows add up.
 */
export function netOfReturns(sale: SaleDetail, returns: SaleReturnDetail[]): NetInvoice {
  const taken = new Map<string, { value: Dec; tax: Dec }>()
  for (const r of returns)
    for (const ri of r.items) {
      const t = taken.get(ri.sale_item_id) ?? { value: ZERO, tax: ZERO }
      taken.set(ri.sale_item_id, { value: t.value + dec(ri.line_total), tax: t.tax + dec(ri.tax_amount) })
    }

  const lines: NetLine[] = []
  let tax = ZERO
  let discount = ZERO
  for (const item of sale.items) {
    const back = taken.get(item.id) ?? { value: ZERO, tax: ZERO }
    const quantity = dec(item.quantity) - dec(item.returned_quantity)
    if (quantity <= ZERO) continue
    tax += dec(item.tax_amount) - back.tax
    const share = (v: string) => (quantity === dec(item.quantity) ? dec(v) : round(div(mul(dec(v), quantity), dec(item.quantity)), 2))
    const lineDiscount = share(item.line_discount)
    // Promotions are discounts too: the kept share of them keeps the subtotal row adding up.
    discount += lineDiscount + share(item.invoice_discount_alloc) + share(item.promotion_discount ?? '0')
    lines.push({ item, quantity, lineTotal: dec(item.line_total) - back.value, lineDiscount })
  }

  const grandTotal = dec(sale.grand_total) - dec(sale.returned_total)
  const charges = dec(sale.shipping_charge) + dec(sale.other_charge) + dec(sale.rounding_adjustment)
  const subtotal = grandTotal - charges - (sale.prices_include_tax ? ZERO : tax) + discount
  return {
    lines,
    subtotal,
    discount,
    tax,
    grandTotal,
    refunded: returns.reduce((s, r) => s + dec(r.refunded_amount), ZERO),
    credited: returns.reduce((s, r) => s + dec(r.credited_amount), ZERO),
    returnNos: returns.map((r) => r.return_no),
  }
}
