import type { TFunction } from 'i18next'

import { formatDate } from '../../lib/dates'

/** `B-12 · exp. 31 Mar 2027`; a batch named by its date shows the date once. */
export function batchLabel(b: { batch_no: string; expiry_date: string | null }, t: TFunction, lang = 'en'): string {
  if (!b.expiry_date) return b.batch_no
  const date = formatDate(b.expiry_date, lang)
  return b.batch_no === b.expiry_date ? t('inventory.expires', { date }) : `${b.batch_no} · ${t('inventory.expires', { date })}`
}
