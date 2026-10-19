import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { categoryTree, useCategories } from '../../api/catalog'
import { type CashierSalesRow, type CategorySalesRow, type ProductSalesRow, type SalesPeriodRow, useReport } from '../../api/reports'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { DateRangeFilter } from '../../components/ui/DateRangeFilter'
import { Select } from '../../components/ui/Select'
import { StateTabs } from '../../components/ui/Tabs'
import { downloadCsv } from '../../lib/csv'
import { formatQuantity } from '../../lib/format'
import { useSession } from '../auth/session'
import { reportAccess } from './access'
import { formatPeriod, money, pct, total, usePeriodLabel, useRange } from './format'
import { type ExportAction, Loaded, ReportFrame, Section } from './shared'

type Tab = 'summary' | 'product' | 'category' | 'cashier'

/** Sales reports (REP-01…04): summary by day/month, by product, category and cashier. */
export function SalesReportPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const allowed = reportAccess(can).sales
  const [range, setRange] = useRange()
  const [tab, setTab] = useState<Tab>('summary')
  const [group, setGroup] = useState<'day' | 'month'>('day')
  const [categoryId, setCategoryId] = useState('')
  const periodLabel = usePeriodLabel()

  const summary = useReport(tenantId, 'salesSummary', { ...range, group }, allowed && tab === 'summary')
  const byProduct = useReport(tenantId, 'salesByProduct', { ...range, category_id: categoryId || undefined }, allowed && tab === 'product')
  const byCategory = useReport(tenantId, 'salesByCategory', range, allowed && tab === 'category')
  const byCashier = useReport(tenantId, 'salesByCashier', range, allowed && tab === 'cashier')
  const categories = useCategories(tenantId)

  const name = (base: string) => `${base}-${range.from}-${range.to}`

  // Summary columns; cost columns only when the server returns them.
  const summaryColumns = (showCost: boolean): Column<SalesPeriodRow>[] =>
    [
      { key: 'period', header: t('reports.sales.period'), render: (r: SalesPeriodRow) => formatPeriod(r.period, lang) },
      { key: 'invoices', header: t('reports.sales.invoices'), align: 'right' as const, render: (r: SalesPeriodRow) => r.invoices },
      { key: 'gross', header: t('reports.sales.gross'), align: 'right' as const, render: (r: SalesPeriodRow) => money(r.gross, lang) },
      { key: 'discount', header: t('reports.sales.discount'), align: 'right' as const, render: (r: SalesPeriodRow) => money(r.discount, lang) },
      { key: 'tax', header: t('reports.sales.vat'), align: 'right' as const, render: (r: SalesPeriodRow) => money(r.tax, lang) },
      { key: 'returns', header: t('reports.sales.returns'), align: 'right' as const, render: (r: SalesPeriodRow) => money(r.returns, lang) },
      { key: 'net', header: t('reports.sales.netSales'), align: 'right' as const, render: (r: SalesPeriodRow) => money(r.net_sales, lang) },
      ...(showCost
        ? [
            { key: 'cogs', header: t('reports.sales.cogs'), align: 'right' as const, render: (r: SalesPeriodRow) => money(r.cogs, lang) },
            { key: 'gp', header: t('reports.sales.grossProfit'), align: 'right' as const, render: (r: SalesPeriodRow) => money(r.gross_profit, lang) },
          ]
        : []),
      { key: 'collected', header: t('reports.sales.collected'), align: 'right' as const, render: (r: SalesPeriodRow) => money(r.collected, lang) },
      { key: 'due', header: t('reports.sales.due'), align: 'right' as const, render: (r: SalesPeriodRow) => money(r.due, lang) },
    ] satisfies Column<SalesPeriodRow>[]

  const productColumns = (showCost: boolean): Column<ProductSalesRow>[] => [
    {
      key: 'product',
      header: t('reports.sales.product'),
      render: (r) => (
        <>
          <span className="font-medium">{r.product_name}</span>
          <span className="block font-mono text-xs text-ink-muted">{r.sku}</span>
        </>
      ),
    },
    { key: 'category', header: t('reports.sales.category'), render: (r) => r.category_name ?? t('reports.uncategorised') },
    { key: 'qty', header: t('common.quantity'), align: 'right', render: (r) => `${formatQuantity(r.quantity, lang, true)} ${r.unit_short_name}` },
    { key: 'net', header: t('reports.sales.netSales'), align: 'right', render: (r) => money(r.net_sales, lang) },
    ...(showCost
      ? ([
          { key: 'cogs', header: t('reports.sales.cogs'), align: 'right', render: (r) => money(r.cogs, lang) },
          { key: 'gp', header: t('reports.sales.grossProfit'), align: 'right', render: (r) => money(r.gross_profit, lang) },
          { key: 'margin', header: t('reports.sales.margin'), align: 'right', render: (r) => pct(r.margin_pct) },
        ] satisfies Column<ProductSalesRow>[])
      : []),
  ]

  const categoryColumns = (showCost: boolean): Column<CategorySalesRow>[] => [
    { key: 'category', header: t('reports.sales.category'), render: (r) => r.category_name ?? t('reports.uncategorised') },
    { key: 'qty', header: t('common.quantity'), align: 'right', render: (r) => formatQuantity(r.quantity, lang, true) },
    { key: 'net', header: t('reports.sales.netSales'), align: 'right', render: (r) => money(r.net_sales, lang) },
    ...(showCost
      ? ([
          { key: 'cogs', header: t('reports.sales.cogs'), align: 'right', render: (r) => money(r.cogs, lang) },
          { key: 'gp', header: t('reports.sales.grossProfit'), align: 'right', render: (r) => money(r.gross_profit, lang) },
          { key: 'margin', header: t('reports.sales.margin'), align: 'right', render: (r) => pct(r.margin_pct) },
        ] satisfies Column<CategorySalesRow>[])
      : []),
  ]

  const cashierColumns: Column<CashierSalesRow>[] = [
    { key: 'cashier', header: t('reports.sales.cashier'), render: (r) => r.cashier_name ?? '—' },
    { key: 'invoices', header: t('reports.sales.invoices'), align: 'right', render: (r) => r.invoices },
    { key: 'total', header: t('reports.sales.grandTotal'), align: 'right', render: (r) => money(r.grand_total, lang) },
    { key: 'discounts', header: t('reports.sales.discount'), align: 'right', render: (r) => money(r.discounts, lang) },
    { key: 'collected', header: t('reports.sales.collected'), align: 'right', render: (r) => money(r.collected, lang) },
    { key: 'cancelled', header: t('reports.sales.cancelledCol'), align: 'right', render: (r) => r.cancelled },
  ]

  const exports: ExportAction[] = []
  let updatedAt: number | undefined
  if (tab === 'summary' && summary.data) {
    const d = summary.data
    updatedAt = summary.dataUpdatedAt
    exports.push({
      run: () =>
        downloadCsv(name('sales-summary'), [...d.rows, { ...d.totals, period: t('common.totals') }], [
          { header: t('reports.sales.period'), value: (r) => r.period },
          { header: t('reports.sales.invoices'), value: (r) => r.invoices },
          { header: t('reports.sales.gross'), value: (r) => r.gross },
          { header: t('reports.sales.discount'), value: (r) => r.discount },
          { header: t('reports.sales.vat'), value: (r) => r.tax },
          { header: t('reports.sales.returns'), value: (r) => r.returns },
          { header: t('reports.sales.netSales'), value: (r) => r.net_sales },
          { header: t('reports.sales.cogs'), value: (r) => r.cogs },
          { header: t('reports.sales.grossProfit'), value: (r) => r.gross_profit },
          { header: t('reports.sales.collected'), value: (r) => r.collected },
          { header: t('reports.sales.due'), value: (r) => r.due },
        ]),
    })
  } else if (tab === 'product' && byProduct.data) {
    const d = byProduct.data
    updatedAt = byProduct.dataUpdatedAt
    exports.push({
      run: () =>
        downloadCsv(name('sales-by-product'), d.rows, [
          { header: t('reports.sales.product'), value: (r) => r.product_name },
          { header: t('reports.sku'), value: (r) => r.sku },
          { header: t('reports.sales.category'), value: (r) => r.category_name },
          { header: t('common.quantity'), value: (r) => r.quantity },
          { header: t('reports.unit'), value: (r) => r.unit_short_name },
          { header: t('reports.sales.netSales'), value: (r) => r.net_sales },
          { header: t('reports.sales.cogs'), value: (r) => r.cogs },
          { header: t('reports.sales.grossProfit'), value: (r) => r.gross_profit },
          { header: t('reports.sales.margin'), value: (r) => r.margin_pct },
        ]),
    })
  } else if (tab === 'category' && byCategory.data) {
    const d = byCategory.data
    updatedAt = byCategory.dataUpdatedAt
    exports.push({
      run: () =>
        downloadCsv(name('sales-by-category'), d, [
          { header: t('reports.sales.category'), value: (r) => r.category_name ?? t('reports.uncategorised') },
          { header: t('common.quantity'), value: (r) => r.quantity },
          { header: t('reports.sales.netSales'), value: (r) => r.net_sales },
          { header: t('reports.sales.cogs'), value: (r) => r.cogs },
          { header: t('reports.sales.grossProfit'), value: (r) => r.gross_profit },
          { header: t('reports.sales.margin'), value: (r) => r.margin_pct },
        ]),
    })
  } else if (tab === 'cashier' && byCashier.data) {
    const d = byCashier.data
    updatedAt = byCashier.dataUpdatedAt
    exports.push({
      run: () =>
        downloadCsv(name('sales-by-cashier'), d, [
          { header: t('reports.sales.cashier'), value: (r) => r.cashier_name },
          { header: t('reports.sales.invoices'), value: (r) => r.invoices },
          { header: t('reports.sales.grandTotal'), value: (r) => r.grand_total },
          { header: t('reports.sales.discount'), value: (r) => r.discounts },
          { header: t('reports.sales.collected'), value: (r) => r.collected },
          { header: t('reports.sales.cancelledCol'), value: (r) => r.cancelled },
        ]),
    })
  }

  return (
    <ReportFrame
      title={t('reports.sales.title')}
      allowed={allowed}
      period={`${t(`reports.sales.tabs.${tab}`)} · ${periodLabel(range.from, range.to)}`}
      exports={exports}
      updatedAt={updatedAt}
      filters={
        <>
          <DateRangeFilter from={range.from} to={range.to} onChange={setRange} />
          {tab === 'summary' && (
            <Select
              label={t('reports.sales.groupBy')}
              value={group}
              onChange={(e) => setGroup(e.target.value as 'day' | 'month')}
              options={[
                { value: 'day', label: t('reports.sales.byDay') },
                { value: 'month', label: t('reports.sales.byMonth') },
              ]}
            />
          )}
          {tab === 'product' && (
            <Select
              label={t('reports.sales.category')}
              value={categoryId}
              onChange={(e) => setCategoryId(e.target.value)}
              options={[
                { value: '', label: t('reports.allCategories') },
                ...categoryTree(categories.data ?? []).map((c) => ({ value: c.id, label: `${'— '.repeat(c.depth)}${c.name}` })),
              ]}
            />
          )}
        </>
      }
    >
      <div className="print:hidden">
        <StateTabs
          label={t('reports.sales.title')}
          value={tab}
          onChange={setTab}
          items={(['summary', 'product', 'category', 'cashier'] as const).map((v) => ({ value: v, label: t(`reports.sales.tabs.${v}`) }))}
        />
      </div>

      {tab === 'summary' && (
        <Loaded query={summary}>
          {(d) => {
            const cols = summaryColumns(d.totals.cogs !== null)
            return (
              <>
                <DataTable
                  caption={t('reports.sales.tabs.summary')}
                  columns={cols}
                  rows={d.rows}
                  rowKey={(r) => r.period}
                  empty={t('reports.noData')}
                  footer={cols.map((c) => (c.key === 'period' ? t('common.totals') : c.render(d.totals, -1)))}
                />
                {d.cancelled_count > 0 && <p className="text-sm text-ink-muted">{t('reports.sales.cancelledNote', { count: d.cancelled_count })}</p>}
                <Section title={t('reports.sales.paymentsByMethod')}>
                  <DataTable
                    caption={t('reports.sales.paymentsByMethod')}
                    rows={d.payments}
                    rowKey={(r) => r.method_name}
                    empty={t('reports.noData')}
                    columns={[
                      { key: 'method', header: t('reports.method'), render: (r) => r.method_name },
                      { key: 'in', header: t('reports.moneyIn'), align: 'right', render: (r) => money(r.money_in, lang) },
                      { key: 'out', header: t('reports.moneyOut'), align: 'right', render: (r) => money(r.money_out, lang) },
                      { key: 'count', header: t('reports.count'), align: 'right', render: (r) => r.count },
                    ]}
                    footer={[
                      t('common.totals'),
                      money(total(d.payments.map((p) => p.money_in)), lang),
                      money(total(d.payments.map((p) => p.money_out)), lang),
                      d.payments.reduce((s, p) => s + p.count, 0),
                    ]}
                  />
                </Section>
              </>
            )
          }}
        </Loaded>
      )}

      {tab === 'product' && (
        <Loaded query={byProduct}>
          {(d) => {
            const cols = productColumns(d.total_cogs !== null)
            const totals: Record<string, string> = {
              net: money(d.total_net_sales, lang),
              cogs: money(d.total_cogs, lang),
              gp: money(d.total_gross_profit, lang),
            }
            return (
              <DataTable
                caption={t('reports.sales.tabs.product')}
                columns={cols}
                rows={d.rows}
                rowKey={(r) => r.variant_id}
                empty={t('reports.noData')}
                footer={cols.map((c) => (c.key === 'product' ? t('common.totals') : (totals[c.key] ?? null)))}
              />
            )
          }}
        </Loaded>
      )}

      {tab === 'category' && (
        <Loaded query={byCategory}>
          {(rows) => {
            const showCost = rows.every((r) => r.cogs !== null)
            const cols = categoryColumns(showCost)
            const totals: Record<string, string> = {
              net: money(total(rows.map((r) => r.net_sales)), lang),
              cogs: money(total(rows.map((r) => r.cogs)), lang),
              gp: money(total(rows.map((r) => r.gross_profit)), lang),
            }
            return (
              <DataTable
                caption={t('reports.sales.tabs.category')}
                columns={cols}
                rows={rows}
                rowKey={(r) => r.category_id ?? 'none'}
                empty={t('reports.noData')}
                footer={cols.map((c) => (c.key === 'category' ? t('common.totals') : (totals[c.key] ?? null)))}
              />
            )
          }}
        </Loaded>
      )}

      {tab === 'cashier' && (
        <Loaded query={byCashier}>
          {(rows) => (
            <DataTable
              caption={t('reports.sales.tabs.cashier')}
              columns={cashierColumns}
              rows={rows}
              rowKey={(r) => r.cashier_id}
              empty={t('reports.noData')}
              footer={[
                t('common.totals'),
                rows.reduce((s, r) => s + r.invoices, 0),
                money(total(rows.map((r) => r.grand_total)), lang),
                money(total(rows.map((r) => r.discounts)), lang),
                money(total(rows.map((r) => r.collected)), lang),
                rows.reduce((s, r) => s + r.cancelled, 0),
              ]}
            />
          )}
        </Loaded>
      )}
    </ReportFrame>
  )
}
