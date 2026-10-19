import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { ADJUSTMENT_REASONS, type AdjustmentSummary, useAdjustments } from '../../api/inventory'
import { Badge } from '../../components/ui/Badge'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { DateRangeFilter } from '../../components/ui/DateRangeFilter'
import { PageHeader } from '../../components/ui/PageHeader'
import { Select } from '../../components/ui/Select'
import { formatDate } from '../../lib/dates'
import { formatMoney } from '../../lib/format'
import { LinkButton, LoadMore, QueryStatus, TextLink } from '../money/shared'
import { useTenant } from '../money/utils'
import { InventoryNav } from './InventoryNav'

const ALL_REASONS = ['opening', ...ADJUSTMENT_REASONS] as const

/** Stock adjustments and opening stock entries (`/inventory/adjustments`). */
export function AdjustmentsPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { tenantId, can } = useTenant()
  const [reason, setReason] = useState('')
  const [range, setRange] = useState({ from: '', to: '' })
  const list = useAdjustments(tenantId, { reason: reason || undefined, from: range.from || undefined, to: range.to || undefined })
  const rows = list.data?.pages.flatMap((p) => p.data) ?? []

  const columns: Column<AdjustmentSummary>[] = [
    {
      key: 'no',
      header: t('inventory.adjustmentNo'),
      render: (a) => <TextLink to={`/inventory/adjustments/${a.id}`}>{a.adjustment_no ?? t('common.view')}</TextLink>,
    },
    { key: 'date', header: t('common.date'), render: (a) => formatDate(a.business_date, lang) },
    {
      key: 'reason',
      header: t('common.reason'),
      render: (a) => <Badge tone={a.reason === 'opening' ? 'info' : 'warn'}>{t(`inventory.reasons.${a.reason}`, { defaultValue: a.reason })}</Badge>,
    },
    { key: 'location', header: t('inventory.location'), render: (a) => a.location_name },
    { key: 'items', header: t('inventory.itemCount'), align: 'right', render: (a) => a.item_count },
    { key: 'value', header: t('inventory.value'), align: 'right', render: (a) => (a.total_value === null ? '—' : formatMoney(a.total_value, lang)) },
    { key: 'by', header: t('inventory.createdBy'), render: (a) => a.created_by_name ?? '—' },
    { key: 'note', header: t('common.note'), render: (a) => <span className="line-clamp-1 max-w-60">{a.note ?? ''}</span> },
  ]

  return (
    <div className="flex flex-col gap-6">
      <PageHeader
        title={t('inventory.adjustments')}
        subtitle={t('inventory.adjustmentsSubtitle')}
        actions={
          <>
            {can('inventory.opening_stock') && <LinkButton to="/inventory/adjustments/new?reason=opening">{t('inventory.openingStock')}</LinkButton>}
            {can('inventory.adjust') && (
              <LinkButton to="/inventory/adjustments/new" variant="primary">
                {t('inventory.newAdjustment')}
              </LinkButton>
            )}
          </>
        }
      />
      <InventoryNav />
      <div className="flex flex-wrap items-end gap-3 print:hidden">
        <Select
          label={t('common.reason')}
          value={reason}
          onChange={(e) => setReason(e.target.value)}
          options={[{ value: '', label: t('common.all') }, ...ALL_REASONS.map((r) => ({ value: r, label: t(`inventory.reasons.${r}`) }))]}
        />
        <DateRangeFilter from={range.from} to={range.to} onChange={setRange} />
      </div>
      {list.isSuccess ? (
        <>
          <DataTable caption={t('inventory.adjustments')} columns={columns} rows={rows} rowKey={(a) => a.id} empty={t('inventory.noAdjustments')} />
          <LoadMore query={list} />
        </>
      ) : (
        <QueryStatus query={list} />
      )}
    </div>
  )
}
