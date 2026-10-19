import { useTranslation } from 'react-i18next'

import { useReport } from '../../api/reports'
import { DataTable } from '../../components/ui/DataTable'
import { DateRangeFilter } from '../../components/ui/DateRangeFilter'
import { downloadCsv } from '../../lib/csv'
import { useSession } from '../auth/session'
import { reportAccess } from './access'
import { formatPeriod, money, pct, share, total, usePeriodLabel, useRange } from './format'
import { type ExportAction, Loaded, ReportFrame, Section } from './shared'

/** Expense report (REP-07): by category with share, and by month. */
export function ExpenseReportPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const allowed = reportAccess(can).expenses
  const [range, setRange] = useRange()
  const periodLabel = usePeriodLabel()
  const report = useReport(tenantId, 'expenses', range, allowed)

  const exports: ExportAction[] = []
  const d = report.data
  if (d) {
    const file = (base: string) => `${base}-${range.from}-${range.to}`
    exports.push(
      {
        label: t('reports.expenses.byCategory'),
        run: () =>
          downloadCsv(file('expenses-by-category'), d.by_category, [
            { header: t('reports.expenses.category'), value: (r) => r.category_name },
            { header: t('reports.count'), value: (r) => r.count },
            { header: t('common.amount'), value: (r) => r.amount },
            { header: t('reports.expenses.vat'), value: (r) => r.tax_amount },
            { header: t('reports.expenses.share'), value: (r) => share(r.amount, d.total) },
          ]),
      },
      {
        label: t('reports.expenses.byMonth'),
        run: () =>
          downloadCsv(file('expenses-by-month'), d.by_month, [
            { header: t('reports.expenses.month'), value: (r) => r.period },
            { header: t('reports.count'), value: (r) => r.count },
            { header: t('common.amount'), value: (r) => r.amount },
          ]),
      },
    )
  }

  return (
    <ReportFrame
      title={t('reports.expenses.title')}
      allowed={allowed}
      period={periodLabel(range.from, range.to)}
      exports={exports}
      updatedAt={report.dataUpdatedAt}
      filters={<DateRangeFilter from={range.from} to={range.to} onChange={setRange} />}
    >
      <Loaded query={report}>
        {(r) => (
          <>
            <Section title={t('reports.expenses.byCategory')}>
              <DataTable
                caption={t('reports.expenses.byCategory')}
                rows={r.by_category}
                rowKey={(c) => c.category_id}
                empty={t('reports.noData')}
                columns={[
                  { key: 'category', header: t('reports.expenses.category'), render: (c) => c.category_name },
                  { key: 'count', header: t('reports.count'), align: 'right', render: (c) => c.count },
                  { key: 'amount', header: t('common.amount'), align: 'right', render: (c) => money(c.amount, lang) },
                  { key: 'tax', header: t('reports.expenses.vat'), align: 'right', render: (c) => money(c.tax_amount, lang) },
                  {
                    key: 'share',
                    header: t('reports.expenses.share'),
                    align: 'right',
                    render: (c) => {
                      const s = share(c.amount, r.total)
                      return (
                        <span className="inline-flex items-center justify-end gap-2">
                          <span aria-hidden className="hidden h-2 w-16 overflow-hidden rounded bg-canvas sm:inline-block">
                            <span className="block h-full bg-brand-500" style={{ width: `${s ?? 0}%` }} />
                          </span>
                          {pct(s)}
                        </span>
                      )
                    },
                  },
                ]}
                footer={[t('common.totals'), r.count, money(r.total, lang), money(total(r.by_category.map((c) => c.tax_amount)), lang), r.by_category.length ? '100.00%' : null]}
              />
            </Section>
            <Section title={t('reports.expenses.byMonth')}>
              <DataTable
                caption={t('reports.expenses.byMonth')}
                rows={r.by_month}
                rowKey={(m) => m.period}
                empty={t('reports.noData')}
                columns={[
                  { key: 'month', header: t('reports.expenses.month'), render: (m) => formatPeriod(m.period, lang) },
                  { key: 'count', header: t('reports.count'), align: 'right', render: (m) => m.count },
                  { key: 'amount', header: t('common.amount'), align: 'right', render: (m) => money(m.amount, lang) },
                ]}
                footer={[t('common.totals'), r.count, money(r.total, lang)]}
              />
            </Section>
          </>
        )}
      </Loaded>
    </ReportFrame>
  )
}
