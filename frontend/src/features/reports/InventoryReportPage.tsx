import { useQuery } from '@tanstack/react-query'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { type StockFilter, type StockRow, inventoryApi, useStockLocations, useValuation } from '../../api/inventory'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { Select } from '../../components/ui/Select'
import { type CsvColumn, downloadCsv } from '../../lib/csv'
import { formatDate, today } from '../../lib/dates'
import { formatQuantity } from '../../lib/format'
import { useSession } from '../auth/session'
import { reportAccess } from './access'
import { money } from './format'
import { type ExportAction, Loaded, ReportFrame, Section } from './shared'

const LIST_LIMIT = 100

function useStockList(tenantId: string, filter: StockFilter, enabled: boolean) {
  return useQuery({
    queryKey: ['inventory', tenantId, 'stock-report', filter],
    queryFn: () => inventoryApi.stock(filter, null, LIST_LIMIT),
    enabled,
  })
}

/** Inventory report (REP-06): valuation by category, low and out-of-stock lists. */
export function InventoryReportPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const allowed = reportAccess(can).inventory
  const canCost = can('products.view_cost')
  const [locationId, setLocationId] = useState('')
  const location = locationId || undefined
  const locations = useStockLocations(tenantId)
  const valuation = useValuation(tenantId, location, allowed && canCost)
  const low = useStockList(tenantId, { status: 'low', location_id: location }, allowed)
  const out = useStockList(tenantId, { status: 'out', location_id: location }, allowed)

  const stockColumns: Column<StockRow>[] = [
    {
      key: 'product',
      header: t('reports.sales.product'),
      render: (r) => (
        <>
          <span className="font-medium">{r.name}</span>
          <span className="block font-mono text-xs text-ink-muted">{r.sku}</span>
        </>
      ),
    },
    { key: 'category', header: t('reports.sales.category'), render: (r) => r.category_name ?? t('reports.uncategorised') },
    { key: 'qty', header: t('reports.inventory.onHand'), align: 'right', render: (r) => `${formatQuantity(r.quantity, lang, r.allow_fraction)} ${r.unit_short_name}` },
    { key: 'reorder', header: t('reports.inventory.reorderLevel'), align: 'right', render: (r) => (r.reorder_level ? formatQuantity(r.reorder_level, lang, r.allow_fraction) : '—') },
  ]

  const exports: ExportAction[] = []
  if (valuation.data) {
    const v = valuation.data
    exports.push({
      label: t('reports.inventory.valuation'),
      run: () =>
        downloadCsv(`inventory-valuation-${today()}`, v.rows, [
          { header: t('reports.sales.category'), value: (r) => r.category_name ?? t('reports.uncategorised') },
          { header: t('reports.inventory.items'), value: (r) => r.items },
          { header: t('common.quantity'), value: (r) => r.quantity },
          { header: t('reports.inventory.costValue'), value: (r) => r.value },
          { header: t('reports.inventory.retailValue'), value: (r) => r.retail_value },
        ]),
    })
  }
  const stockCsv: CsvColumn<StockRow>[] = [
    { header: t('reports.sales.product'), value: (r: StockRow) => r.name },
    { header: t('reports.sku'), value: (r: StockRow) => r.sku },
    { header: t('reports.sales.category'), value: (r: StockRow) => r.category_name },
    { header: t('reports.inventory.onHand'), value: (r: StockRow) => r.quantity },
    { header: t('reports.unit'), value: (r: StockRow) => r.unit_short_name },
    { header: t('reports.inventory.reorderLevel'), value: (r: StockRow) => r.reorder_level },
  ]
  if (low.data) {
    const rows = low.data.data
    exports.push({ label: t('reports.inventory.lowStock'), run: () => downloadCsv(`low-stock-${today()}`, rows, stockCsv) })
  }
  if (out.data) {
    const rows = out.data.data
    exports.push({ label: t('reports.inventory.outOfStock'), run: () => downloadCsv(`out-of-stock-${today()}`, rows, stockCsv) })
  }

  const locationName = locations.data?.find((l) => l.id === locationId)?.name
  return (
    <ReportFrame
      title={t('reports.inventory.title')}
      allowed={allowed}
      period={`${t('reports.asOf', { date: formatDate(today(), lang) })}${locationName ? ` · ${locationName}` : ''}`}
      exports={exports}
      updatedAt={valuation.dataUpdatedAt || low.dataUpdatedAt}
      links={
        <>
          <Link to="/inventory" className="text-sm font-semibold text-brand-700 hover:underline">
            {t('reports.inventory.stockLink')}
          </Link>
          <Link to="/inventory/movements" className="text-sm font-semibold text-brand-700 hover:underline">
            {t('reports.inventory.movementsLink')}
          </Link>
        </>
      }
      filters={
        <Select
          label={t('reports.inventory.location')}
          value={locationId}
          onChange={(e) => setLocationId(e.target.value)}
          options={[{ value: '', label: t('reports.inventory.allLocations') }, ...(locations.data ?? []).map((l) => ({ value: l.id, label: l.name }))]}
        />
      }
    >
      <Section title={t('reports.inventory.valuation')}>
        {!canCost ? (
          <p className="text-sm text-ink-muted">{t('reports.inventory.noCost')}</p>
        ) : (
          <Loaded query={valuation}>
            {(v) => (
              <DataTable
                caption={t('reports.inventory.valuation')}
                rows={v.rows}
                rowKey={(r) => r.category_id ?? 'none'}
                empty={t('reports.noData')}
                columns={[
                  { key: 'category', header: t('reports.sales.category'), render: (r) => r.category_name ?? t('reports.uncategorised') },
                  { key: 'items', header: t('reports.inventory.items'), align: 'right', render: (r) => r.items },
                  { key: 'qty', header: t('common.quantity'), align: 'right', render: (r) => formatQuantity(r.quantity, lang, true) },
                  { key: 'value', header: t('reports.inventory.costValue'), align: 'right', render: (r) => money(r.value, lang) },
                  { key: 'retail', header: t('reports.inventory.retailValue'), align: 'right', render: (r) => money(r.retail_value, lang) },
                ]}
                footer={[
                  t('common.totals'),
                  v.rows.reduce((s, r) => s + r.items, 0),
                  formatQuantity(v.total_quantity, lang, true),
                  money(v.total_value, lang),
                  money(v.total_retail_value, lang),
                ]}
              />
            )}
          </Loaded>
        )}
      </Section>
      <Section title={t('reports.inventory.lowStock')}>
        <Loaded query={low}>
          {(p) => (
            <>
              <DataTable caption={t('reports.inventory.lowStock')} columns={stockColumns} rows={p.data} rowKey={(r) => r.variant_id} empty={t('reports.inventory.noneLow')} />
              {p.next_cursor && <p className="text-xs text-ink-muted">{t('reports.inventory.firstRows', { count: LIST_LIMIT })}</p>}
            </>
          )}
        </Loaded>
      </Section>
      <Section title={t('reports.inventory.outOfStock')}>
        <Loaded query={out}>
          {(p) => (
            <>
              <DataTable caption={t('reports.inventory.outOfStock')} columns={stockColumns} rows={p.data} rowKey={(r) => r.variant_id} empty={t('reports.inventory.noneOut')} />
              {p.next_cursor && <p className="text-xs text-ink-muted">{t('reports.inventory.firstRows', { count: LIST_LIMIT })}</p>}
            </>
          )}
        </Loaded>
      </Section>
    </ReportFrame>
  )
}
