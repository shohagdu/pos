import type { TFunction } from 'i18next'

import type { WarrantyUnit } from '../../api/catalog'

interface Tenure {
  warranty_period?: number | null
  warranty_unit?: WarrantyUnit | null
}

/** "3 months", from a warranty tenure; `null` without one. */
export function warrantyTenure(item: Tenure, t: TFunction): string | null {
  if (item.warranty_period == null || !item.warranty_unit) return null
  return t(`sales.warrantyTenure.${item.warranty_unit}`, { count: item.warranty_period })
}

/** The warranty has run out: its last day is before `today` (both `YYYY-MM-DD`). */
export const warrantyExpired = (expiresOn: string, today: string) => expiresOn < today

/**
 * The warranty's last day for a sale on `from` (`YYYY-MM-DD`), as the server
 * stores it: months and years keep the day of the month, or end on the
 * month's last day when it is shorter (31 Jan + 1 month = 28 Feb).
 */
export function warrantyExpiresOn(from: string, item: Tenure): string | null {
  const n = item.warranty_period
  if (n == null || n <= 0 || !item.warranty_unit) return null
  const [y, m, d] = from.split('-').map(Number) as [number, number, number]
  let date: Date
  if (item.warranty_unit === 'day') {
    date = new Date(Date.UTC(y, m - 1, d + n))
  } else {
    const months = item.warranty_unit === 'year' ? n * 12 : n
    const lastDay = new Date(Date.UTC(y, m - 1 + months + 1, 0)).getUTCDate()
    date = new Date(Date.UTC(y, m - 1 + months, Math.min(d, lastDay)))
  }
  return date.toISOString().slice(0, 10)
}
