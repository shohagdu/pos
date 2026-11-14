import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate, useSearchParams } from 'react-router'

import { BATCH_STATUSES, type BatchFilter, type BatchStock, inventoryApi, useBatches, useStockLocations } from '../../api/inventory'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { PageHeader } from '../../components/ui/PageHeader'
import { Select } from '../../components/ui/Select'
import { StatCard } from '../../components/ui/StatCard'
import { TextField } from '../../components/ui/TextField'
import { downloadCsv } from '../../lib/csv'
import { formatDate } from '../../lib/dates'
import { formatMoney, formatQuantity } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { LoadMore, QueryStatus } from '../money/shared'
import { fetchAll, stamp, useTenant } from '../money/utils'
import { InventoryNav } from './InventoryNav'

type Status = (typeof BATCH_STATUSES)[number]
const WINDOWS = [30, 60, 90, 180, 365]

/** A batch to write off, handed to the adjustment form. */
export interface WriteOff {
  variant_id: string
  name: string
  sku: string
  unit: string
  batch_id: string
  quantity: string
}

const writeOff = (b: BatchStock): WriteOff => ({
  variant_id: b.variant_id,
  name: b.name,
  sku: b.sku,
  unit: b.unit_short_name,
  batch_id: b.batch_id,
  quantity: b.quantity,
})

/** When a batch expires: expired, days left, or nothing without a date. */
export function ExpiryBadge({ days, window }: { days: number | null; window: number }) {
  const { t } = useTranslation()
  if (days === null) return <span className="text-ink-muted">—</span>
  if (days < 0) return <Badge tone="bad">{t('inventory.expiryReport.expiredAgo', { count: -days })}</Badge>
  if (days === 0) return <Badge tone="bad">{t('inventory.expiryReport.today')}</Badge>
  return <Badge tone={days <= window ? 'warn' : 'neutral'}>{t('inventory.expiryReport.daysLeft', { count: days })}</Badge>
}

/** Batches in stock by expiry date (`/inventory/expiry`): what to sell first or write off. */
export function ExpiryPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const navigate = useNavigate()
  const { tenantId, can } = useTenant()
  const [params, setParams] = useSearchParams()
  const rawStatus = params.get('status')
  const status: Status = BATCH_STATUSES.includes(rawStatus as Status) ? (rawStatus as Status) : 'expiring'
  const days = Number(params.get('days')) || 30
  const filter: BatchFilter = {
    status,
    days,
    q: params.get('q') || undefined,
    location_id: params.get('location_id') || undefined,
  }
  const [text, setText] = useState(filter.q ?? '')
  const [exportError, setExportError] = useState<unknown>(null)
  const [exporting, setExporting] = useState(false)
  const locations = useStockLocations(tenantId)
  const batches = useBatches(tenantId, filter)
  const canCost = can('products.view_cost')
  const canWriteOff = can('inventory.adjust')

  const set = (patch: Record<string, string | undefined>) =>
    setParams(
      (prev) => {
        const next = new URLSearchParams(prev)
        for (const [k, v] of Object.entries(patch)) {
          if (v) next.set(k, v)
          else next.delete(k)
        }
        return next
      },
      { replace: true },
    )

  function onSearch(e: FormEvent) {
    e.preventDefault()
    set({ q: text.trim() || undefined })
  }

  const rows = batches.data?.pages.flatMap((p) => p.data) ?? []
  const summary = batches.data?.pages[0]?.summary
  const expired = rows.filter((r) => r.days_left !== null && r.days_left < 0)

  function startWriteOff(list: BatchStock[]) {
    navigate('/inventory/adjustments/new', { state: { writeOff: list.slice(0, 500).map(writeOff) } })
  }

  const columns: Column<BatchStock>[] = [
    {
      key: 'name',
      header: t('inventory.product'),
      render: (r) => (
        <div className="flex flex-col">
          <span className="font-medium">{r.name}</span>
          <span className="font-mono text-xs text-ink-muted">{r.sku}</span>
        </div>
      ),
    },
    { key: 'batch', header: t('inventory.batchNo'), render: (r) => <span className="font-mono text-xs">{r.batch_no}</span> },
    { key: 'expiry', header: t('inventory.expiry'), render: (r) => formatDate(r.expiry_date, lang) },
    { key: 'left', header: t('common.status'), render: (r) => <ExpiryBadge days={r.days_left} window={days} /> },
    { key: 'location', header: t('inventory.location'), render: (r) => r.location_name },
    { key: 'qty', header: t('inventory.quantity'), align: 'right', render: (r) => `${formatQuantity(r.quantity, lang, true)} ${r.unit_short_name}` },
    ...(canCost
      ? ([
          { key: 'value', header: t('inventory.value'), align: 'right', render: (r) => (r.value === null ? '—' : formatMoney(r.value, lang)) },
        ] satisfies Column<BatchStock>[])
      : []),
    ...(canWriteOff
      ? ([
          {
            key: 'actions',
            header: <span className="sr-only">{t('common.actions')}</span>,
            render: (r) => (
              <Button
                variant="ghost"
                className="min-h-9"
                onClick={() => startWriteOff([r])}
                aria-label={t('inventory.expiryReport.writeOffOne', { name: r.name, batch: r.batch_no })}
              >
                {t('inventory.expiryReport.writeOff')}
              </Button>
            ),
          },
        ] satisfies Column<BatchStock>[])
      : []),
  ]

  async function exportCsv() {
    setExporting(true)
    setExportError(null)
    try {
      const all = await fetchAll((cursor) => inventoryApi.batches(filter, cursor))
      downloadCsv(`expiry-${stamp()}`, all, [
        { header: t('inventory.product'), value: (r) => r.name },
        { header: t('inventory.sku'), value: (r) => r.sku },
        { header: t('inventory.batchNo'), value: (r) => r.batch_no },
        { header: t('inventory.expiry'), value: (r) => r.expiry_date },
        { header: t('inventory.expiryReport.daysLeftColumn'), value: (r) => (r.days_left === null ? '' : String(r.days_left)) },
        { header: t('inventory.location'), value: (r) => r.location_name },
        { header: t('inventory.quantity'), value: (r) => r.quantity },
        { header: t('inventory.unit'), value: (r) => r.unit_short_name },
        ...(canCost ? [{ header: t('inventory.value'), value: (r: BatchStock) => r.value }] : []),
      ])
    } catch (err) {
      setExportError(err)
    } finally {
      setExporting(false)
    }
  }

  const activeLocations = (locations.data ?? []).filter((l) => l.status === 'active')

  return (
    <div className="flex flex-col gap-6">
      <PageHeader title={t('inventory.expiryReport.title')} subtitle={t('inventory.expiryReport.subtitle')} />
      <InventoryNav />
      {summary && (
        <div className="grid grid-cols-2 gap-3 md:grid-cols-3">
          <StatCard
            label={t('inventory.expiryReport.expiredCount')}
            value={formatQuantity(String(summary.expired), lang)}
            tone={summary.expired > 0 ? 'bad' : 'default'}
          />
          <StatCard
            label={t('inventory.expiryReport.expiringCount', { days })}
            value={formatQuantity(String(summary.expiring), lang)}
            tone={summary.expiring > 0 ? 'bad' : 'default'}
          />
          {canCost && summary.expired_value !== null && (
            <StatCard label={t('inventory.expiryReport.expiredValue')} value={formatMoney(summary.expired_value, lang)} />
          )}
        </div>
      )}
      <div className="flex flex-col gap-3 print:hidden">
        <form onSubmit={onSearch} className="flex flex-wrap items-end gap-2" role="search">
          <TextField
            className="min-w-60 flex-1"
            label={t('common.search')}
            type="search"
            value={text}
            onChange={(e) => setText(e.target.value)}
            placeholder={t('inventory.expiryReport.searchHint')}
          />
          <Button type="submit">{t('common.search')}</Button>
        </form>
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
          <Select
            label={t('common.status')}
            value={status}
            onChange={(e) => set({ status: e.target.value })}
            options={BATCH_STATUSES.map((s) => ({ value: s, label: t(`inventory.expiryReport.statuses.${s}`, { days }) }))}
          />
          <Select
            label={t('inventory.expiryReport.window')}
            value={String(days)}
            onChange={(e) => set({ days: e.target.value === '30' ? undefined : e.target.value })}
            options={WINDOWS.map((d) => ({ value: String(d), label: t('inventory.expiryReport.days', { count: d }) }))}
          />
          {activeLocations.length > 1 && (
            <Select
              label={t('inventory.location')}
              value={filter.location_id ?? ''}
              onChange={(e) => set({ location_id: e.target.value || undefined })}
              options={[{ value: '', label: t('inventory.allLocations') }, ...activeLocations.map((l) => ({ value: l.id, label: l.name }))]}
            />
          )}
        </div>
      </div>
      {batches.isSuccess ? (
        <>
          <div className="flex flex-wrap justify-end gap-2 print:hidden">
            {canWriteOff && expired.length > 0 && (
              <Button variant="secondary" onClick={() => startWriteOff(expired)}>
                {t('inventory.expiryReport.writeOffExpired', { count: expired.length })}
              </Button>
            )}
            <Button variant="secondary" disabled={exporting || rows.length === 0} onClick={() => void exportCsv()}>
              {t('common.exportCsv')}
            </Button>
          </div>
          {exportError !== null && <Alert tone="danger">{problemMessage(exportError, t)}</Alert>}
          <DataTable
            caption={t('inventory.expiryReport.title')}
            columns={columns}
            rows={rows}
            rowKey={(r) => `${r.batch_id}:${r.location_id}`}
            empty={t('inventory.expiryReport.empty')}
          />
          <LoadMore query={batches} />
        </>
      ) : (
        <QueryStatus query={batches} />
      )}
    </div>
  )
}
