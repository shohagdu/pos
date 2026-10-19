import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useSearchParams } from 'react-router'

import { categoryTree, useCategories } from '../../api/catalog'
import { type StockFilter, type StockRow, type StockStatusFilter, inventoryApi, useStock, useStockLocations } from '../../api/inventory'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { PageHeader } from '../../components/ui/PageHeader'
import { Select } from '../../components/ui/Select'
import { StatCard } from '../../components/ui/StatCard'
import { TextField } from '../../components/ui/TextField'
import { downloadCsv } from '../../lib/csv'
import { dec } from '../../lib/decimal'
import { formatMoney, formatQuantity } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { LinkButton, LoadMore, QueryStatus, TextLink } from '../money/shared'
import { fetchAll, stamp, useTenant } from '../money/utils'
import { InventoryNav } from './InventoryNav'

const STATUSES: StockStatusFilter[] = ['all', 'in', 'low', 'out']

function isStatus(value: string | null): value is StockStatusFilter {
  return STATUSES.includes(value as StockStatusFilter)
}

/** Low / out-of-stock badge for a stock row (`null` when fine). */
export function StockBadge({ row }: { row: Pick<StockRow, 'quantity' | 'is_low'> }) {
  const { t } = useTranslation()
  if (dec(row.quantity) <= 0n) return <Badge tone="bad">{t('inventory.out')}</Badge>
  if (row.is_low) return <Badge tone="warn">{t('inventory.low')}</Badge>
  return null
}

/** Current stock (`/inventory`): filters, summary and the stock list. */
export function StockPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { tenantId, can } = useTenant()
  const [params, setParams] = useSearchParams()
  const status = params.get('status')
  const filter: StockFilter = {
    q: params.get('q') || undefined,
    status: isStatus(status) ? status : 'all',
    category_id: params.get('category_id') || undefined,
    location_id: params.get('location_id') || undefined,
  }
  const [text, setText] = useState(filter.q ?? '')
  const [exportError, setExportError] = useState<unknown>(null)
  const [exporting, setExporting] = useState(false)
  const categories = useCategories(tenantId)
  const locations = useStockLocations(tenantId)
  const stock = useStock(tenantId, filter)
  const canCost = can('products.view_cost')

  const set = (patch: Partial<Record<keyof StockFilter, string | undefined>>) =>
    setParams(
      (prev) => {
        const next = new URLSearchParams(prev)
        for (const [k, v] of Object.entries(patch)) {
          if (v && !(k === 'status' && v === 'all')) next.set(k, v)
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

  const rows = stock.data?.pages.flatMap((p) => p.data) ?? []
  const summary = stock.data?.pages[0]?.summary
  const qty = (r: StockRow) => `${formatQuantity(r.quantity, lang, r.allow_fraction)} ${r.unit_short_name}`

  const columns: Column<StockRow>[] = [
    {
      key: 'name',
      header: t('inventory.product'),
      render: (r) => (
        <div className="flex flex-col">
          <span className="font-medium">{r.name}</span>
          {r.name_bn && (
            <span lang="bn" className="text-xs text-ink-muted">
              {r.name_bn}
            </span>
          )}
        </div>
      ),
    },
    { key: 'sku', header: t('inventory.sku'), render: (r) => <span className="font-mono text-xs">{r.sku}</span> },
    { key: 'category', header: t('inventory.category'), render: (r) => r.category_name ?? '—' },
    { key: 'qty', header: t('inventory.quantity'), align: 'right', render: qty },
    ...(canCost
      ? ([
          { key: 'avg', header: t('inventory.avgCost'), align: 'right', render: (r) => (r.avg_cost === null ? '—' : formatMoney(r.avg_cost, lang)) },
          { key: 'value', header: t('inventory.value'), align: 'right', render: (r) => (r.value === null ? '—' : formatMoney(r.value, lang)) },
        ] satisfies Column<StockRow>[])
      : []),
    { key: 'price', header: t('inventory.sellingPrice'), align: 'right', render: (r) => formatMoney(r.selling_price, lang) },
    {
      key: 'reorder',
      header: t('inventory.reorderLevel'),
      align: 'right',
      render: (r) => (r.reorder_level === null ? '—' : formatQuantity(r.reorder_level, lang, r.allow_fraction)),
    },
    { key: 'badge', header: t('common.status'), render: (r) => <StockBadge row={r} /> },
    {
      key: 'actions',
      header: <span className="sr-only">{t('common.actions')}</span>,
      render: (r) => <TextLink to={`/inventory/movements?variant_id=${r.variant_id}`}>{t('inventory.movements')}</TextLink>,
    },
  ]

  async function exportCsv() {
    setExporting(true)
    setExportError(null)
    try {
      const all = await fetchAll((cursor) => inventoryApi.stock(filter, cursor))
      downloadCsv(`stock-${stamp()}`, all, [
        { header: t('inventory.product'), value: (r) => r.name },
        { header: t('inventory.nameBn'), value: (r) => r.name_bn },
        { header: t('inventory.sku'), value: (r) => r.sku },
        { header: t('inventory.category'), value: (r) => r.category_name },
        { header: t('inventory.quantity'), value: (r) => r.quantity },
        { header: t('inventory.unit'), value: (r) => r.unit_short_name },
        ...(canCost
          ? [
              { header: t('inventory.avgCost'), value: (r: StockRow) => r.avg_cost },
              { header: t('inventory.value'), value: (r: StockRow) => r.value },
            ]
          : []),
        { header: t('inventory.sellingPrice'), value: (r) => r.selling_price },
        { header: t('inventory.reorderLevel'), value: (r) => r.reorder_level },
        {
          header: t('common.status'),
          value: (r) => (dec(r.quantity) <= 0n ? t('inventory.out') : r.is_low ? t('inventory.low') : t('inventory.inStock')),
        },
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
      <PageHeader
        title={t('inventory.title')}
        subtitle={t('inventory.subtitle')}
        actions={
          <>
            {can('inventory.opening_stock') && <LinkButton to="/inventory/adjustments/new?reason=opening">{t('inventory.openingStock')}</LinkButton>}
            {can('inventory.adjust') && (
              <LinkButton to="/inventory/adjustments/new" variant="primary">
                {t('inventory.adjustStock')}
              </LinkButton>
            )}
          </>
        }
      />
      <InventoryNav />
      <div className="flex flex-col gap-3 print:hidden">
        <form onSubmit={onSearch} className="flex flex-wrap items-end gap-2" role="search">
          <TextField
            className="min-w-60 flex-1"
            label={t('common.search')}
            type="search"
            value={text}
            onChange={(e) => setText(e.target.value)}
            placeholder={t('inventory.searchHint')}
          />
          <Button type="submit">{t('common.search')}</Button>
        </form>
        <div className="grid gap-3 sm:grid-cols-3">
          <Select
            label={t('inventory.category')}
            value={filter.category_id ?? ''}
            onChange={(e) => set({ category_id: e.target.value || undefined })}
            options={[
              { value: '', label: t('inventory.allCategories') },
              ...categoryTree(categories.data ?? []).map((c) => ({ value: c.id, label: `${'— '.repeat(c.depth)}${c.name}` })),
            ]}
          />
          <Select
            label={t('inventory.stockStatus')}
            value={filter.status ?? 'all'}
            onChange={(e) => set({ status: e.target.value })}
            options={STATUSES.map((s) => ({ value: s, label: t(`inventory.statuses.${s}`) }))}
          />
          <Select
            label={t('inventory.location')}
            value={filter.location_id ?? ''}
            onChange={(e) => set({ location_id: e.target.value || undefined })}
            options={[{ value: '', label: t('inventory.allLocations') }, ...activeLocations.map((l) => ({ value: l.id, label: l.name }))]}
          />
        </div>
      </div>
      {summary && (
        <div className="grid grid-cols-2 gap-3 md:grid-cols-5">
          <StatCard label={t('inventory.items')} value={formatQuantity(String(summary.items), lang)} />
          <StatCard label={t('inventory.totalQuantity')} value={formatQuantity(summary.quantity, lang, true)} />
          {canCost && summary.value !== null && <StatCard label={t('inventory.stockValue')} value={formatMoney(summary.value, lang)} />}
          <StatCard label={t('inventory.lowCount')} value={formatQuantity(String(summary.low_count), lang)} tone={summary.low_count > 0 ? 'bad' : 'default'} />
          <StatCard label={t('inventory.outCount')} value={formatQuantity(String(summary.out_count), lang)} tone={summary.out_count > 0 ? 'bad' : 'default'} />
        </div>
      )}
      {stock.isSuccess ? (
        <>
          <div className="flex justify-end print:hidden">
            <Button variant="secondary" disabled={exporting || rows.length === 0} onClick={() => void exportCsv()}>
              {t('common.exportCsv')}
            </Button>
          </div>
          {exportError !== null && <Alert tone="danger">{problemMessage(exportError, t)}</Alert>}
          <DataTable
            caption={t('inventory.title')}
            columns={columns}
            rows={rows}
            rowKey={(r) => r.variant_id}
            empty={filter.q || filter.status !== 'all' || filter.category_id ? t('common.noResults') : t('inventory.noStock')}
          />
          <LoadMore query={stock} />
        </>
      ) : (
        <QueryStatus query={stock} />
      )}
    </div>
  )
}
