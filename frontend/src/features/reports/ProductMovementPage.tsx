import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { type DateRange, type MovementClass, type ProductMovementRow, useReport } from '../../api/reports'
import { Badge } from '../../components/ui/Badge'
import { DataTable } from '../../components/ui/DataTable'
import { DateRangeFilter } from '../../components/ui/DateRangeFilter'
import { Select } from '../../components/ui/Select'
import { StatCard } from '../../components/ui/StatCard'
import { downloadCsv } from '../../lib/csv'
import { formatDate } from '../../lib/dates'
import { formatQuantity } from '../../lib/format'
import { useSession } from '../auth/session'
import { reportAccess } from './access'
import { money, usePeriodLabel, useRange } from './format'
import { type ExportAction, Loaded, ReportFrame } from './shared'

const CLASSES = ['fast', 'slow', 'non_moving'] as const
const TONES = { fast: 'good', slow: 'warn', non_moving: 'bad' } as const

/** Fast / slow / non-moving products (REP-13) with days of stock left. */
export function ProductMovementPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const allowed = reportAccess(can).movement
  const [range, setRange] = useRange()
  const [cls, setCls] = useState<MovementClass | ''>('')
  const periodLabel = usePeriodLabel()
  // The class filter rides along with the range (and so in the query key).
  const params = (cls ? { ...range, class: cls } : range) as DateRange
  const report = useReport(tenantId, 'productMovement', params, allowed)
  const qty = (value: string) => formatQuantity(value, lang, true)
  const className = (c: string) => t(`reports.movement.classes.${c}`, { defaultValue: c })

  const exports: ExportAction[] = []
  const d = report.data
  if (d) {
    exports.push({
      run: () =>
        downloadCsv(`product-movement-${range.from}-${range.to}`, d.rows, [
          { header: t('reports.movement.product'), value: (r) => r.product_name },
          { header: t('reports.sku'), value: (r) => r.sku },
          { header: t('reports.movement.category'), value: (r) => r.category_name },
          { header: t('reports.movement.class'), value: (r) => className(r.class) },
          { header: t('reports.movement.sold'), value: (r) => r.quantity_sold },
          { header: t('reports.unit'), value: (r) => r.unit_short_name },
          { header: t('reports.movement.netSales'), value: (r) => r.net_sales },
          { header: t('reports.movement.dailyRate'), value: (r) => r.daily_rate },
          { header: t('reports.movement.stock'), value: (r) => r.stock_qty },
          { header: t('reports.movement.stockValue'), value: (r) => r.stock_value },
          { header: t('reports.movement.daysOfStock'), value: (r) => r.days_of_stock },
          { header: t('reports.movement.lastSold'), value: (r) => r.last_sold_on },
        ]),
    })
  }

  return (
    <ReportFrame
      title={t('reports.movement.title')}
      allowed={allowed}
      period={periodLabel(range.from, range.to)}
      exports={exports}
      updatedAt={report.dataUpdatedAt}
      filters={
        <>
          <DateRangeFilter from={range.from} to={range.to} onChange={setRange} />
          <Select
            label={t('reports.movement.class')}
            value={cls}
            onChange={(e) => setCls(e.target.value as MovementClass | '')}
            options={[{ value: '', label: t('reports.movement.classes.all') }, ...CLASSES.map((c) => ({ value: c, label: className(c) }))]}
          />
        </>
      }
    >
      <Loaded query={report}>
        {(r) => (
          <>
            <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
              <StatCard label={t('reports.movement.fastCount')} value={r.fast_count} tone="good" />
              <StatCard label={t('reports.movement.slowCount')} value={r.slow_count} />
              <StatCard label={t('reports.movement.nonMovingCount')} value={r.non_moving_count} tone={r.non_moving_count > 0 ? 'bad' : 'default'} />
              {r.non_moving_value !== null && <StatCard label={t('reports.movement.nonMovingValue')} value={money(r.non_moving_value, lang)} />}
            </div>
            <DataTable<ProductMovementRow>
              caption={t('reports.movement.title')}
              rows={r.rows}
              rowKey={(row) => row.variant_id}
              empty={t('reports.noData')}
              columns={[
                {
                  key: 'product',
                  header: t('reports.movement.product'),
                  render: (row) => (
                    <div className="flex flex-col">
                      <span className="font-medium">{row.product_name}</span>
                      <span className="font-mono text-xs text-ink-muted">{row.sku}</span>
                    </div>
                  ),
                },
                { key: 'category', header: t('reports.movement.category'), render: (row) => row.category_name ?? t('reports.uncategorised') },
                { key: 'class', header: t('reports.movement.class'), render: (row) => <Badge tone={TONES[row.class] ?? 'neutral'}>{className(row.class)}</Badge> },
                { key: 'sold', header: t('reports.movement.sold'), align: 'right', render: (row) => `${qty(row.quantity_sold)} ${row.unit_short_name}` },
                { key: 'sales', header: t('reports.movement.netSales'), align: 'right', render: (row) => money(row.net_sales, lang) },
                { key: 'rate', header: t('reports.movement.dailyRate'), align: 'right', render: (row) => qty(row.daily_rate) },
                { key: 'stock', header: t('reports.movement.stock'), align: 'right', render: (row) => `${qty(row.stock_qty)} ${row.unit_short_name}` },
                { key: 'value', header: t('reports.movement.stockValue'), align: 'right', render: (row) => money(row.stock_value, lang) },
                { key: 'days', header: t('reports.movement.daysOfStock'), align: 'right', render: (row) => (row.days_of_stock === null ? '—' : formatQuantity(row.days_of_stock, lang)) },
                {
                  key: 'last',
                  header: t('reports.movement.lastSold'),
                  render: (row) => <span className="whitespace-nowrap">{row.last_sold_on ? formatDate(row.last_sold_on, lang) : t('reports.movement.never')}</span>,
                },
              ]}
            />
          </>
        )}
      </Loaded>
    </ReportFrame>
  )
}
