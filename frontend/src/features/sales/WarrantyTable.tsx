import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import type { WarrantyItem } from '../../api/sales'
import { Badge } from '../../components/ui/Badge'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { ZERO, dec } from '../../lib/decimal'
import { formatDate } from '../../lib/dates'
import { formatQuantity } from '../../lib/format'
import { warrantyTenure } from './warranty'

const link = 'font-medium text-brand-700 hover:underline'

/** Whole days from `from` to `to` (both `YYYY-MM-DD`). */
export function daysBetween(from: string, to: string): number {
  return Math.round((Date.parse(`${to}T00:00:00Z`) - Date.parse(`${from}T00:00:00Z`)) / 86_400_000)
}

/** Covered, running out, expired or returned, counted from the business's `today`. */
export function WarrantyBadge({ item, today }: { item: WarrantyItem; today: string }) {
  const { t } = useTranslation()
  if (dec(item.quantity) > ZERO && dec(item.returned_quantity) >= dec(item.quantity)) {
    return <Badge tone="neutral">{t('sales.warrantyTable.returned')}</Badge>
  }
  const left = daysBetween(today, item.warranty_expires_on)
  if (left < 0) return <Badge tone="bad">{t('sales.warrantyTable.expired')}</Badge>
  return (
    <Badge tone={left <= 30 ? 'warn' : 'good'}>
      {left === 0 ? t('sales.warrantyTable.lastDay') : t('sales.warrantyTable.daysLeft', { count: left })}
    </Badge>
  )
}

/** Items sold with a warranty: the customer's Warranties tab and the warranty lookup page. */
export function WarrantyTable({
  rows,
  today,
  caption,
  empty,
  showCustomer = false,
}: {
  rows: WarrantyItem[]
  today: string
  caption: string
  empty: string
  showCustomer?: boolean
}) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language

  const columns: Column<WarrantyItem>[] = [
    {
      key: 'product',
      header: t('sales.warrantyTable.product'),
      render: (w) => (
        <>
          <span className="font-medium">{w.product_name}</span>
          <span className="block font-mono text-xs text-ink-muted">{w.sku}</span>
        </>
      ),
    },
    {
      key: 'invoice',
      header: t('sales.invoiceNo'),
      render: (w) => (
        <Link to={`/sales/${w.sale_id}`} className={link}>
          {w.invoice_no}
        </Link>
      ),
    },
    ...(showCustomer
      ? [
          {
            key: 'customer',
            header: t('sales.customer'),
            render: (w: WarrantyItem) =>
              w.is_walk_in ? (
                t('sales.walkIn')
              ) : (
                <>
                  <Link to={`/customers/${w.customer_id}`} className={link}>
                    {w.customer_name}
                  </Link>
                  {w.customer_phone && <span className="block text-xs text-ink-muted">{w.customer_phone}</span>}
                </>
              ),
          },
        ]
      : []),
    { key: 'sold', header: t('sales.warrantyTable.soldOn'), render: (w) => <span className="whitespace-nowrap">{formatDate(w.business_date, lang)}</span> },
    {
      key: 'qty',
      header: t('common.quantity'),
      align: 'right',
      render: (w) => (
        <>
          {`${formatQuantity(w.quantity, lang, true)} ${w.unit_short_name}`}
          {dec(w.returned_quantity) > ZERO && (
            <span className="block text-xs text-ink-muted">
              {t('sales.warrantyTable.returnedQty', { qty: formatQuantity(w.returned_quantity, lang, true) })}
            </span>
          )}
        </>
      ),
    },
    { key: 'tenure', header: t('sales.warrantyTable.tenure'), render: (w) => warrantyTenure(w, t) },
    {
      key: 'expires',
      header: t('sales.warrantyTable.expiresOn'),
      render: (w) => <span className="whitespace-nowrap">{formatDate(w.warranty_expires_on, lang)}</span>,
    },
    { key: 'state', header: t('common.status'), render: (w) => <WarrantyBadge item={w} today={today} /> },
  ]

  return <DataTable caption={caption} columns={columns} rows={rows} rowKey={(w) => w.id} empty={empty} />
}
