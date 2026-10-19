import { useTranslation } from 'react-i18next'
import { Link, useParams } from 'react-router'

import { type AdjustmentItem, useAdjustment } from '../../api/inventory'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { PageHeader } from '../../components/ui/PageHeader'
import { dec, mul, toFixed } from '../../lib/decimal'
import { formatDate, formatDateTime } from '../../lib/dates'
import { formatMoney, formatQuantity } from '../../lib/format'
import { QueryStatus } from '../money/shared'
import { useTenant } from '../money/utils'

/** One posted adjustment or opening stock entry (`/inventory/adjustments/:id`). */
export function AdjustmentDetailPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { id = '' } = useParams()
  const { tenantId } = useTenant()
  const adjustment = useAdjustment(tenantId, id)
  const a = adjustment.data
  if (!a) return <QueryStatus query={adjustment} />

  const reason = t(`inventory.reasons.${a.reason}`, { defaultValue: a.reason })
  const hasCost = a.items.some((i) => i.unit_cost !== null)
  const columns: Column<AdjustmentItem>[] = [
    {
      key: 'product',
      header: t('inventory.product'),
      render: (i) => (
        <div className="flex flex-col">
          <span className="font-medium">{i.product_name}</span>
          <span className="font-mono text-xs text-ink-muted">{i.sku}</span>
        </div>
      ),
    },
    {
      key: 'qty',
      header: t('inventory.quantity'),
      align: 'right',
      render: (i) => (
        <span className={dec(i.quantity) < 0n ? 'text-danger' : 'text-brand-700'}>
          {dec(i.quantity) > 0n ? '+' : ''}
          {formatQuantity(i.quantity, lang, true)} {i.unit_short_name}
        </span>
      ),
    },
    ...(hasCost
      ? ([
          { key: 'cost', header: t('inventory.unitCost'), align: 'right', render: (i) => (i.unit_cost === null ? '—' : formatMoney(i.unit_cost, lang)) },
          {
            key: 'value',
            header: t('inventory.value'),
            align: 'right',
            render: (i) => (i.unit_cost === null ? '—' : formatMoney(toFixed(mul(dec(i.quantity), dec(i.unit_cost))), lang)),
          },
        ] satisfies Column<AdjustmentItem>[])
      : []),
    { key: 'note', header: t('common.note'), render: (i) => i.note ?? '' },
  ]

  return (
    <div className="flex max-w-4xl flex-col gap-6">
      <Link to="/inventory/adjustments" className="text-sm font-medium text-brand-700 hover:underline print:hidden">
        ← {t('inventory.adjustments')}
      </Link>
      <PageHeader
        title={`${a.reason === 'opening' ? t('inventory.openingStock') : t('inventory.adjustment')} ${a.adjustment_no ?? ''}`.trim()}
        subtitle={formatDate(a.business_date, lang)}
        actions={
          <Button variant="secondary" className="print:hidden" onClick={() => window.print()}>
            {t('common.print')}
          </Button>
        }
      />
      <Card>
        <dl className="grid gap-4 text-sm sm:grid-cols-3">
          <div>
            <dt className="text-ink-muted">{t('common.reason')}</dt>
            <dd>
              <Badge tone={a.reason === 'opening' ? 'info' : 'warn'}>{reason}</Badge>
            </dd>
          </div>
          <div>
            <dt className="text-ink-muted">{t('inventory.location')}</dt>
            <dd className="font-medium">{a.location_name}</dd>
          </div>
          <div>
            <dt className="text-ink-muted">{t('common.status')}</dt>
            <dd className="font-medium">{t(`inventory.adjustmentStatuses.${a.status}`, { defaultValue: a.status })}</dd>
          </div>
          <div>
            <dt className="text-ink-muted">{t('inventory.itemCount')}</dt>
            <dd className="font-medium">{a.item_count}</dd>
          </div>
          {a.total_value !== null && (
            <div>
              <dt className="text-ink-muted">{t('inventory.value')}</dt>
              <dd className="font-medium">{formatMoney(a.total_value, lang)}</dd>
            </div>
          )}
          <div>
            <dt className="text-ink-muted">{t('inventory.createdBy')}</dt>
            <dd className="font-medium">
              {a.created_by_name ?? '—'} · {formatDateTime(a.created_at, lang)}
            </dd>
          </div>
          {a.note && (
            <div className="sm:col-span-3">
              <dt className="text-ink-muted">{t('common.note')}</dt>
              <dd className="whitespace-pre-line">{a.note}</dd>
            </div>
          )}
        </dl>
      </Card>
      <DataTable
        caption={t('inventory.lines')}
        columns={columns}
        rows={a.items}
        rowKey={(i) => i.id}
        footer={hasCost && a.total_value !== null ? [t('common.total'), '', '', formatMoney(a.total_value, lang), ''] : undefined}
      />
    </div>
  )
}
