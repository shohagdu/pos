// Audit log filter helpers.

import type { AuditFilter } from '../../api/audit'

/** Common entity types offered as suggestions in the filter. */
export const ENTITY_TYPES = [
  'product',
  'sale',
  'sale_return',
  'purchase',
  'purchase_return',
  'customer',
  'supplier',
  'payment',
  'expense',
  'stock_adjustment',
  'stock_transfer',
  'member',
  'role',
  'invitation',
  'branch',
  'tenant',
  'subscription_invoice',
  'subscription_payment_claim',
  'journal_entry',
]

/** The day after a `YYYY-MM-DD` business date (calendar arithmetic, no timezone drift). */
export function nextDay(date: string): string {
  const d = new Date(`${date}T00:00:00Z`)
  d.setUTCDate(d.getUTCDate() + 1)
  return d.toISOString().slice(0, 10)
}

/** Business dates (Asia/Dhaka) → the API's RFC 3339 range; `to` is exclusive. */
export function dateRange(from: string, to: string): Pick<AuditFilter, 'from' | 'to'> {
  return {
    from: from ? `${from}T00:00:00+06:00` : undefined,
    to: to ? `${nextDay(to)}T00:00:00+06:00` : undefined,
  }
}
