import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { useReport } from '../../api/reports'
import { DateRangeFilter } from '../../components/ui/DateRangeFilter'
import { StatCard } from '../../components/ui/StatCard'
import { downloadCsv } from '../../lib/csv'
import { dec } from '../../lib/decimal'
import { useSession } from '../auth/session'
import { reportAccess } from './access'
import { money, pct, usePeriodLabel, useRange } from './format'
import { type ExportAction, Loaded, ReportFrame } from './shared'

/** Profit estimate (REP-09): net sales − COGS − expenses. */
export function ProfitReportPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const allowed = reportAccess(can).profit
  const [range, setRange] = useRange()
  const periodLabel = usePeriodLabel()
  const report = useReport(tenantId, 'profit', range, allowed)

  const lines = (r: NonNullable<typeof report.data>) => [
    { key: 'netSales', label: t('reports.profit.netSales'), amount: r.net_sales, strong: false },
    { key: 'cogs', label: t('reports.profit.lessCogs'), amount: r.cogs, strong: false },
    { key: 'grossProfit', label: t('reports.profit.grossProfit'), amount: r.gross_profit, strong: true },
    { key: 'expenses', label: t('reports.profit.lessExpenses'), amount: r.expenses, strong: false },
    { key: 'netProfit', label: t('reports.profit.netProfit'), amount: r.net_profit, strong: true },
  ]

  const exports: ExportAction[] = []
  if (report.data) {
    const rows = [...lines(report.data), { key: 'margin', label: t('reports.profit.netMargin'), amount: report.data.net_margin_pct ?? '', strong: false }]
    exports.push({
      run: () =>
        downloadCsv(`profit-${range.from}-${range.to}`, rows, [
          { header: t('reports.item'), value: (l) => l.label },
          { header: t('common.amount'), value: (l) => l.amount },
        ]),
    })
  }

  return (
    <ReportFrame
      title={t('reports.profit.title')}
      allowed={allowed}
      period={periodLabel(range.from, range.to)}
      exports={exports}
      updatedAt={report.dataUpdatedAt}
      links={
        <Link to="/reports/profit-and-loss" className="text-sm font-semibold text-brand-700 hover:underline">
          {t('reports.profit.plLink')}
        </Link>
      }
      filters={<DateRangeFilter from={range.from} to={range.to} onChange={setRange} />}
    >
      <Loaded query={report}>
        {(r) => (
          <>
            <div className="grid grid-cols-2 gap-3 md:grid-cols-4">
              <StatCard label={t('reports.profit.netSales')} value={money(r.net_sales, lang)} />
              <StatCard label={t('reports.profit.grossProfit')} value={money(r.gross_profit, lang)} tone={dec(r.gross_profit) < 0n ? 'bad' : 'default'} />
              <StatCard label={t('reports.profit.netProfit')} value={money(r.net_profit, lang)} tone={dec(r.net_profit) < 0n ? 'bad' : 'good'} />
              <StatCard label={t('reports.profit.netMargin')} value={pct(r.net_margin_pct)} />
            </div>
            <div className="max-w-xl overflow-x-auto rounded-xl border border-border bg-surface">
              <table className="w-full text-sm">
                <caption className="sr-only">{t('reports.profit.title')}</caption>
                <thead className="sr-only">
                  <tr>
                    <th scope="col">{t('reports.item')}</th>
                    <th scope="col">{t('common.amount')}</th>
                  </tr>
                </thead>
                <tbody>
                  {lines(r).map((l) => (
                    <tr key={l.key} className={`border-b border-border last:border-b-0 ${l.strong ? 'bg-canvas font-semibold' : ''}`}>
                      <th scope="row" className="px-4 py-2.5 text-left font-[inherit]">
                        {l.label}
                      </th>
                      <td className="px-4 py-2.5 text-right tabular-nums">{money(l.amount, lang)}</td>
                    </tr>
                  ))}
                  <tr>
                    <th scope="row" className="px-4 py-2.5 text-left font-normal text-ink-muted">
                      {t('reports.profit.netMargin')}
                    </th>
                    <td className="px-4 py-2.5 text-right tabular-nums">{pct(r.net_margin_pct)}</td>
                  </tr>
                </tbody>
              </table>
            </div>
            <p className="text-xs text-ink-muted">{t('reports.profit.note')}</p>
          </>
        )}
      </Loaded>
    </ReportFrame>
  )
}
