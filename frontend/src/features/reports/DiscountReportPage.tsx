import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { useReport } from '../../api/reports'
import { DataTable } from '../../components/ui/DataTable'
import { DateRangeFilter } from '../../components/ui/DateRangeFilter'
import { StatCard } from '../../components/ui/StatCard'
import { downloadCsv } from '../../lib/csv'
import { formatDate } from '../../lib/dates'
import { useSession } from '../auth/session'
import { reportAccess } from './access'
import { money, pct, usePeriodLabel, useRange } from './format'
import { type ExportAction, Loaded, ReportFrame, Section } from './shared'

/** Discount report (REP-12): how much was given away, by whom, on which invoices. */
export function DiscountReportPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const allowed = reportAccess(can).discounts
  const [range, setRange] = useRange()
  const periodLabel = usePeriodLabel()
  const report = useReport(tenantId, 'discounts', range, allowed)
  const cashier = (name: string | null) => name ?? t('reports.discounts.unknownCashier')

  const exports: ExportAction[] = []
  const d = report.data
  if (d) {
    const file = (base: string) => `${base}-${range.from}-${range.to}`
    exports.push(
      {
        label: t('reports.discounts.byCashier'),
        run: () =>
          downloadCsv(file('discounts-by-cashier'), d.by_cashier, [
            { header: t('reports.discounts.cashier'), value: (r) => cashier(r.cashier_name) },
            { header: t('reports.discounts.invoices'), value: (r) => r.invoices },
            { header: t('reports.discounts.discountedInvoices'), value: (r) => r.discounted_invoices },
            { header: t('reports.discounts.gross'), value: (r) => r.gross },
            { header: t('reports.discounts.itemDiscount'), value: (r) => r.item_discount },
            { header: t('reports.discounts.invoiceDiscount'), value: (r) => r.invoice_discount },
            { header: t('reports.discounts.totalDiscount'), value: (r) => r.total_discount },
          ]),
      },
      {
        label: t('reports.discounts.topInvoices'),
        run: () =>
          downloadCsv(file('discounted-invoices'), d.top_invoices, [
            { header: t('reports.discounts.invoice'), value: (r) => r.invoice_no },
            { header: t('common.date'), value: (r) => r.business_date },
            { header: t('reports.discounts.customer'), value: (r) => r.customer_name },
            { header: t('reports.discounts.cashier'), value: (r) => cashier(r.cashier_name) },
            { header: t('reports.discounts.subtotal'), value: (r) => r.subtotal },
            { header: t('reports.discounts.discount'), value: (r) => r.discount },
            { header: t('reports.discounts.grandTotal'), value: (r) => r.grand_total },
          ]),
      },
    )
  }

  return (
    <ReportFrame
      title={t('reports.discounts.title')}
      allowed={allowed}
      period={periodLabel(range.from, range.to)}
      exports={exports}
      updatedAt={report.dataUpdatedAt}
      filters={<DateRangeFilter from={range.from} to={range.to} onChange={setRange} />}
    >
      <Loaded query={report}>
        {(r) => (
          <>
            <div className="grid gap-3 sm:grid-cols-3 lg:grid-cols-5">
              <StatCard label={t('reports.discounts.gross')} value={money(r.gross, lang)} />
              <StatCard label={t('reports.discounts.itemDiscount')} value={money(r.item_discount, lang)} />
              <StatCard label={t('reports.discounts.invoiceDiscount')} value={money(r.invoice_discount, lang)} />
              <StatCard label={t('reports.discounts.totalDiscount')} value={money(r.total_discount, lang)} tone="bad" />
              <StatCard label={t('reports.discounts.discountPct')} value={pct(r.discount_pct)} hint={t('reports.discounts.discountPctHint')} />
            </div>
            <Section title={t('reports.discounts.byCashier')}>
              <DataTable
                caption={t('reports.discounts.byCashier')}
                rows={r.by_cashier}
                rowKey={(c) => c.cashier_id}
                empty={t('reports.noData')}
                columns={[
                  { key: 'cashier', header: t('reports.discounts.cashier'), render: (c) => cashier(c.cashier_name) },
                  { key: 'invoices', header: t('reports.discounts.invoices'), align: 'right', render: (c) => c.invoices },
                  { key: 'discounted', header: t('reports.discounts.discountedInvoices'), align: 'right', render: (c) => c.discounted_invoices },
                  { key: 'gross', header: t('reports.discounts.gross'), align: 'right', render: (c) => money(c.gross, lang) },
                  { key: 'item', header: t('reports.discounts.itemDiscount'), align: 'right', render: (c) => money(c.item_discount, lang) },
                  { key: 'invoice', header: t('reports.discounts.invoiceDiscount'), align: 'right', render: (c) => money(c.invoice_discount, lang) },
                  { key: 'total', header: t('reports.discounts.totalDiscount'), align: 'right', render: (c) => <span className="font-semibold">{money(c.total_discount, lang)}</span> },
                ]}
                footer={[t('common.totals'), null, null, money(r.gross, lang), money(r.item_discount, lang), money(r.invoice_discount, lang), money(r.total_discount, lang)]}
              />
            </Section>
            <Section title={t('reports.discounts.topInvoices')}>
              <DataTable
                caption={t('reports.discounts.topInvoices')}
                rows={r.top_invoices}
                rowKey={(s) => s.sale_id}
                empty={t('reports.noData')}
                columns={[
                  {
                    key: 'invoice',
                    header: t('reports.discounts.invoice'),
                    render: (s) => (
                      <Link to={`/sales/${s.sale_id}`} className="font-mono text-xs font-semibold text-brand-700 hover:underline">
                        {s.invoice_no}
                      </Link>
                    ),
                  },
                  { key: 'date', header: t('common.date'), render: (s) => <span className="whitespace-nowrap">{formatDate(s.business_date, lang)}</span> },
                  { key: 'customer', header: t('reports.discounts.customer'), render: (s) => s.customer_name },
                  { key: 'cashier', header: t('reports.discounts.cashier'), render: (s) => cashier(s.cashier_name) },
                  { key: 'subtotal', header: t('reports.discounts.subtotal'), align: 'right', render: (s) => money(s.subtotal, lang) },
                  { key: 'discount', header: t('reports.discounts.discount'), align: 'right', render: (s) => <span className="font-semibold">{money(s.discount, lang)}</span> },
                  { key: 'total', header: t('reports.discounts.grandTotal'), align: 'right', render: (s) => money(s.grand_total, lang) },
                ]}
              />
            </Section>
          </>
        )}
      </Loaded>
    </ReportFrame>
  )
}
