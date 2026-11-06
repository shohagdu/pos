import type { PurchaseDetail } from '../../api/purchasing'
import { dec } from '../../lib/decimal'

export const MAX_COPIES = 999

const ONE = dec('1')

/** Whole labels for a quantity: 2.5 kg → 3 labels; never negative. */
const labelCount = (qty: bigint) => {
  if (qty <= 0n) return 0
  const whole = qty / ONE
  return Math.min(MAX_COPIES, Number(qty % ONE === 0n ? whole : whole + 1n))
}

/**
 * Copies per variant for a purchase: what is in the shop now (received less
 * returned) once goods have arrived, the ordered quantity before that.
 */
export function purchaseCopies(p: PurchaseDetail): Map<string, number> {
  const arrived = p.status === 'received' || p.status === 'partially_received'
  const copies = new Map<string, bigint>()
  for (const item of p.items) {
    const qty = arrived ? dec(item.received_quantity) - dec(item.returned_quantity) : dec(item.quantity)
    copies.set(item.variant_id, (copies.get(item.variant_id) ?? 0n) + qty)
  }
  return new Map([...copies].map(([id, q]) => [id, labelCount(q)]))
}
