// Refund estimates for the return form. The server keeps the last units'
// rounding exact; the screen shows the pro-rata share (RET-R04).

import type { SaleItem } from '../../api/sales'
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
