// Refund estimates for the return form. The server keeps the last units'
// rounding exact; the screen shows the pro-rata share (RET-R04).

import type { SaleDetail, SaleItem, SaleReturnDetail } from '../../api/sales'
import { type Dec, ZERO, dec, div, mul, round, tryParse } from '../../lib/decimal'

export function returnable(item: SaleItem): Dec {
  const left = dec(item.quantity) - dec(item.returned_quantity)
  return left > ZERO ? left : ZERO
}

/** line_total × qty / quantity, to 2 decimals. */
export function estimatedRefund(item: SaleItem, qty: Dec): Dec {
  const quantity = dec(item.quantity)
  if (quantity === ZERO || qty <= ZERO) return ZERO
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
    discount += lineDiscount + share(item.invoice_discount_alloc)
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
