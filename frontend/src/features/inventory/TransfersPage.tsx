import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { TRANSFER_STATUSES, type TransferFilter, type TransferSummary, useTransfers } from '../../api/inventory'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { DateRangeFilter } from '../../components/ui/DateRangeFilter'
import { PageHeader } from '../../components/ui/PageHeader'
import { Select } from '../../components/ui/Select'
import { dec } from '../../lib/decimal'
import { formatDate } from '../../lib/dates'
import { formatMoney } from '../../lib/format'
import { LinkButton, LoadMore, QueryStatus, TextLink } from '../money/shared'
import { useTenant } from '../money/utils'
import { InventoryNav } from './InventoryNav'
import { TRANSFER_TONES } from './tones'

/** Stock transfers between branches and locations (`/inventory/transfers`). */
export function TransfersPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { tenantId, can, hasFeature } = useTenant()
  const [direction, setDirection] = useState('')
  const [status, setStatus] = useState('')
  const [range, setRange] = useState({ from: '', to: '' })
  const filter: TransferFilter = {
    direction: (direction || undefined) as TransferFilter['direction'],
    status: status || undefined,
    from: range.from || undefined,
    to: range.to || undefined,
  }
  const list = useTransfers(tenantId, filter)
  const rows = list.data?.pages.flatMap((p) => p.data) ?? []

  if (!can('inventory.view')) return <Alert tone="warning">{t('inventory.noPermission')}</Alert>

  const columns: Column<TransferSummary>[] = [
    {
      key: 'no',
      header: t('inventory.transferNo'),
      render: (x) => <TextLink to={`/inventory/transfers/${x.id}`}>{x.transfer_no ?? t('common.view')}</TextLink>,
    },
    { key: 'date', header: t('common.date'), render: (x) => <span className="whitespace-nowrap">{formatDate(x.business_date, lang)}</span> },
    { key: 'from', header: t('inventory.fromLocation'), render: (x) => `${x.from_branch_name} — ${x.from_location_name}` },
    { key: 'to', header: t('inventory.toLocation'), render: (x) => `${x.to_branch_name} — ${x.to_location_name}` },
    { key: 'items', header: t('inventory.itemCount'), align: 'right', render: (x) => x.item_count },
    { key: 'value', header: t('inventory.value'), align: 'right', render: (x) => (x.total_value === null ? '—' : formatMoney(x.total_value, lang)) },
    {
      key: 'loss',
      header: t('inventory.transitLoss'),
      align: 'right',
      render: (x) => (x.loss_value !== null && dec(x.loss_value) > 0n ? <span className="text-danger">{formatMoney(x.loss_value, lang)}</span> : ''),
    },
    {
      key: 'status',
      header: t('common.status'),
      render: (x) => <Badge tone={TRANSFER_TONES[x.status]}>{t(`inventory.transferStatuses.${x.status}`, { defaultValue: x.status })}</Badge>,
    },
    { key: 'by', header: t('inventory.createdBy'), render: (x) => x.created_by_name ?? '—' },
  ]

  return (
    <div className="flex flex-col gap-6">
      <PageHeader
        title={t('inventory.transfers')}
        subtitle={t('inventory.transfersSubtitle')}
        actions={
          can('inventory.transfer') &&
          hasFeature('inventory.transfers') && (
            <LinkButton to="/inventory/transfers/new" variant="primary">
              {t('inventory.newTransfer')}
            </LinkButton>
          )
        }
      />
      <InventoryNav />
      <div className="flex flex-wrap items-end gap-3 print:hidden">
        <Select
          label={t('inventory.transferDirection')}
          value={direction}
          onChange={(e) => setDirection(e.target.value)}
          options={[
            { value: '', label: t('inventory.directionAll') },
            { value: 'out', label: t('inventory.directionOut') },
            { value: 'in', label: t('inventory.directionIn') },
          ]}
        />
        <Select
          label={t('common.status')}
          value={status}
          onChange={(e) => setStatus(e.target.value)}
          options={[{ value: '', label: t('common.all') }, ...TRANSFER_STATUSES.map((s) => ({ value: s, label: t(`inventory.transferStatuses.${s}`) }))]}
        />
        <DateRangeFilter from={range.from} to={range.to} onChange={setRange} />
      </div>
      {list.isSuccess ? (
        <>
          <DataTable caption={t('inventory.transfers')} columns={columns} rows={rows} rowKey={(x) => x.id} empty={t('inventory.noTransfers')} />
          <LoadMore query={list} />
        </>
      ) : (
        <QueryStatus query={list} />
      )}
    </div>
  )
}
