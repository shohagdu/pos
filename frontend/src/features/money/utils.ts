// Non-component helpers shared by the inventory, customers, money and expenses pages.

import type { ReconciliationStatus } from '../../api/money'
import { dec } from '../../lib/decimal'
import { useSession } from '../auth/session'

export const RECONCILIATION_TONE: Record<ReconciliationStatus, 'good' | 'warn' | 'neutral'> = {
  in_progress: 'warn',
  completed: 'good',
  cancelled: 'neutral',
}

/** Session with the active tenant id. */
export function useTenant() {
  const session = useSession()
  return { ...session, tenantId: session.me?.tenant?.tenant_id ?? '' }
}

/** Fetches every page of a cursor-paged list (CSV export), at most `maxPages`. */
export async function fetchAll<T>(fetchPage: (cursor: string | null) => Promise<{ data: T[]; next_cursor: string | null }>, maxPages = 100) {
  const out: T[] = []
  let cursor: string | null = null
  for (let i = 0; i < maxPages; i++) {
    const page = await fetchPage(cursor)
    out.push(...page.data)
    cursor = page.next_cursor
    if (!cursor) break
  }
  return out
}

export const isPositive = (value: string | null | undefined) => dec(value) > 0n
export const isNegative = (value: string | null | undefined) => dec(value) < 0n

/** Absolute value of a decimal string, keeping its digits. */
export function absText(value: string): string {
  return value.trim().startsWith('-') ? value.trim().slice(1) : value.trim()
}

/** "450.5000" → "450.5" for form inputs (no float conversion). */
export function trimDecimal(value: string | null | undefined): string {
  if (!value) return ''
  return value.includes('.') ? value.replace(/0+$/, '').replace(/\.$/, '') : value
}

/** A `YYYY-MM-DD` slice of a timestamp in local time, for file names. */
export function stamp(): string {
  const d = new Date()
  return `${d.getFullYear()}${String(d.getMonth() + 1).padStart(2, '0')}${String(d.getDate()).padStart(2, '0')}`
}

/** Where a document referenced by a ledger line lives in the app, if anywhere. */
export function referencePath(type: string, id: string): string | null {
  switch (type) {
    case 'sale':
    case 'sale_cancel':
      return `/sales/${id}`
    case 'sale_return':
      return `/sales/returns/${id}`
    case 'purchase':
      return `/purchases/${id}`
    case 'purchase_return':
      return `/purchases/returns/${id}`
    case 'stock_adjustment':
      return `/inventory/adjustments/${id}`
    case 'stock_transfer':
      return `/inventory/transfers/${id}`
    case 'stock_count':
      return `/inventory/counts/${id}`
    default:
      return null
  }
}
