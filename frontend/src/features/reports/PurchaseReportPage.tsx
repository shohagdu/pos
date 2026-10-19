import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { useSuppliers } from '../../api/parties'
import { useReport } from '../../api/reports'
import { DataTable } from '../../components/ui/DataTable'
import { DateRangeFilter } from '../../components/ui/DateRangeFilter'
import { Select } from '../../components/ui/Select'
import { StatCard } from '../../components/ui/StatCard'
import { downloadCsv } from '../../lib/csv'
import { dec } from '../../lib/decimal'
import { formatQuantity } from '../../lib/format'
import { useSession } from '../auth/session'
import { reportAccess } from './access'
import { money, total, usePeriodLabel, useRange } from './format'
import { type ExportAction, Loaded, ReportFrame, Section } from './shared'

/** Purchase report (REP-05): totals, by supplier and by product. */
export function PurchaseReportPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const allowed = reportAccess(can).purchases
  const [range, setRange] = useRange()
  const [supplierId, setSupplierId] = useState('')
  const periodLabel = usePeriodLabel()
  const report = useReport(tenantId, 'purchases', { ...range, supplier_id: supplierId || undefined }, allowed)
  const suppliers = useSuppliers(tenantId, { status: 'all' })
  const supplierOptions = suppliers.data?.pages.flatMap((p) => p.data) ?? []

  const exports: ExportAction[] = []
  const d = report.data
  if (d) {
    const file = (base: string) => `${base}-${range.from}-${range.to}`
    exports.push(
      {
        label: t('reports.purchases.bySupplier'),
        run: () =>
          downloadCsv(file('purchases-by-supplier'), d.by_supplier, [
            { header: t('reports.purchases.supplier'), value: (r) => r.supplier_name },
            { header: t('reports.purchases.bills'), value: (r) => r.bills },
            { header: t('reports.purchases.total'), value: (r) => r.grand_total },
            { header: t('reports.purchases.returned'), value: (r) => r.returned_total },
            { header: t('reports.purchases.paid'), value: (r) => r.paid_total },
            { header: t('reports.purchases.due'), value: (r) => r.due_total },
          ]),
      },
      {
        label: t('reports.purchases.byProduct'),
        run: () =>
          downloadCsv(file('purchases-by-product'), d.by_product, [
            { header: t('reports.sales.product'), value: (r) => r.product_name },
            { header: t('reports.sku'), value: (r) => r.sku },
            { header: t('common.quantity'), value: (r) => r.quantity },
            { header: t('reports.unit'), value: (r) => r.unit_short_name },
            { header: t('reports.purchases.netCost'), value: (r) => r.net_cost },
            { header: t('reports.purchases.avgCost'), value: (r) => r.avg_unit_cost },
            { header: t('reports.purchases.lastCost'), value: (r) => r.last_unit_cost },
          ]),
      },
    )
  }

  const supplierName = supplierOptions.find((s) => s.id === supplierId)?.name
  return (
    <ReportFrame
      title={t('reports.purchases.title')}
      allowed={allowed}
      period={supplierName ? `${periodLabel(range.from, range.to)} · ${supplierName}` : periodLabel(range.from, range.to)}
      exports={exports}
      updatedAt={report.dataUpdatedAt}
      filters={
        <>
          <DateRangeFilter from={range.from} to={range.to} onChange={setRange} />
          <Select
            label={t('reports.purchases.supplier')}
            value={supplierId}
            onChange={(e) => setSupplierId(e.target.value)}
            options={[{ value: '', label: t('reports.allSuppliers') }, ...supplierOptions.map((s) => ({ value: s.id, label: s.name }))]}
          />
        </>
      }
    >
      <Loaded query={report}>
        {(r) => (
          <>
            <div className="grid grid-cols-2 gap-3 md:grid-cols-3 lg:grid-cols-6">
              <StatCard label={t('reports.purchases.bills')} value={r.bills} />
              <StatCard label={t('reports.purchases.total')} value={money(r.grand_total, lang)} />
              <StatCard label={t('reports.purchases.returned')} value={money(r.returned_total, lang)} />
              <StatCard label={t('reports.purchases.paid')} value={money(r.paid_total, lang)} />
              <StatCard label={t('reports.purchases.due')} value={money(r.due_total, lang)} tone={dec(r.due_total) > 0n ? 'bad' : 'default'} />
              <StatCard label={t('reports.purchases.vat')} value={money(r.tax_total, lang)} />
            </div>
            <Section title={t('reports.purchases.bySupplier')}>
              <DataTable
                caption={t('reports.purchases.bySupplier')}
                rows={r.by_supplier}
                rowKey={(s) => s.supplier_id}
                empty={t('reports.noData')}
                columns={[
                  { key: 'supplier', header: t('reports.purchases.supplier'), render: (s) => s.supplier_name },
                  { key: 'bills', header: t('reports.purchases.bills'), align: 'right', render: (s) => s.bills },
                  { key: 'total', header: t('reports.purchases.total'), align: 'right', render: (s) => money(s.grand_total, lang) },
                  { key: 'returned', header: t('reports.purchases.returned'), align: 'right', render: (s) => money(s.returned_total, lang) },
                  { key: 'paid', header: t('reports.purchases.paid'), align: 'right', render: (s) => money(s.paid_total, lang) },
                  { key: 'due', header: t('reports.purchases.due'), align: 'right', render: (s) => money(s.due_total, lang) },
                ]}
                footer={[
                  t('common.totals'),
                  r.bills,
                  money(r.grand_total, lang),
                  money(r.returned_total, lang),
                  money(r.paid_total, lang),
                  money(r.due_total, lang),
                ]}
              />
            </Section>
            <Section title={t('reports.purchases.byProduct')}>
              <DataTable
                caption={t('reports.purchases.byProduct')}
                rows={r.by_product}
                rowKey={(p) => p.variant_id}
                empty={t('reports.noData')}
                columns={[
                  {
                    key: 'product',
                    header: t('reports.sales.product'),
                    render: (p) => (
                      <>
                        <span className="font-medium">{p.product_name}</span>
                        <span className="block font-mono text-xs text-ink-muted">{p.sku}</span>
                      </>
                    ),
                  },
                  { key: 'qty', header: t('common.quantity'), align: 'right', render: (p) => `${formatQuantity(p.quantity, lang, true)} ${p.unit_short_name}` },
                  { key: 'cost', header: t('reports.purchases.netCost'), align: 'right', render: (p) => money(p.net_cost, lang) },
                  { key: 'avg', header: t('reports.purchases.avgCost'), align: 'right', render: (p) => money(p.avg_unit_cost, lang) },
                  { key: 'last', header: t('reports.purchases.lastCost'), align: 'right', render: (p) => money(p.last_unit_cost, lang) },
                ]}
                footer={[t('common.totals'), null, money(total(r.by_product.map((p) => p.net_cost)), lang)]}
              />
            </Section>
          </>
        )}
      </Loaded>
    </ReportFrame>
  )
}
