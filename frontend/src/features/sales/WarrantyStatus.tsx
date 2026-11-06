import { useTranslation } from 'react-i18next'

import type { Sale } from '../../api/sales'
import { formatDate, today } from '../../lib/dates'
import { warrantyExpired } from './warranty'

/** An invoice's warranty in a list: the latest expiry, red once it has passed. */
export function WarrantyStatus({ sale }: { sale: Pick<Sale, 'warranty_items' | 'warranty_until'> }) {
  const { t, i18n } = useTranslation()
  if (!sale.warranty_until || !sale.warranty_items) return <span className="text-ink-muted">—</span>
  const expired = warrantyExpired(sale.warranty_until, today())
  return (
    <span className="whitespace-nowrap">
      <span className={expired ? 'text-danger' : 'text-brand-700'}>
        {t(expired ? 'sales.warrantyListExpired' : 'sales.warrantyListUntil', { date: formatDate(sale.warranty_until, i18n.language) })}
      </span>
      {sale.warranty_items > 1 && (
        <span className="block text-xs text-ink-muted">{t('sales.warrantyItems', { count: sale.warranty_items })}</span>
      )}
    </span>
  )
}
