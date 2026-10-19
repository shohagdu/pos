import { useTranslation } from 'react-i18next'

import { type ProfitAndLoss, useProfitAndLoss } from '../../api/accounting'
import { DateRangeFilter } from '../../components/ui/DateRangeFilter'
import { downloadCsv } from '../../lib/csv'
import { useSession } from '../auth/session'
import { reportAccess } from './access'
import { usePeriodLabel, useRange } from './format'
import { type ExportAction, Loaded, ReportFrame } from './shared'
import { StatementTable } from './Statement'
import { type StatementRow, sectionRows, statementCsv, useAccountDepths } from './statementRows'

/** Profit & loss account from the ledger (ACC-R05). */
export function ProfitLossPage() {
  const { t } = useTranslation()
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const allowed = reportAccess(can).statements
  const [range, setRange] = useRange()
  const periodLabel = usePeriodLabel()
  const pl = useProfitAndLoss(tenantId, range.from || undefined, range.to || undefined)
  const depths = useAccountDepths(tenantId)

  const rows = (d: ProfitAndLoss): StatementRow[] => [
    ...sectionRows('rev', t('reports.pl.revenue'), d.revenue, t('reports.pl.totalRevenue')),
    ...sectionRows('cos', t('reports.pl.costOfSales'), d.cost_of_sales, t('reports.pl.totalCostOfSales')),
    { key: 'gp', kind: 'grand', label: t('reports.pl.grossProfit'), amount: d.gross_profit },
    ...sectionRows('oi', t('reports.pl.otherIncome'), d.other_income, t('reports.pl.totalOtherIncome')),
    ...sectionRows('exp', t('reports.pl.expenses'), d.expenses, t('reports.pl.totalExpenses')),
    { key: 'np', kind: 'grand', label: t('reports.pl.netProfit'), amount: d.net_profit },
  ]

  const exports: ExportAction[] = []
  if (pl.data) {
    const data = rows(pl.data)
    exports.push({ run: () => downloadCsv(`profit-and-loss-${range.from}-${range.to}`, data, statementCsv(t)) })
  }

  return (
    <ReportFrame
      title={t('reports.pl.title')}
      allowed={allowed}
      period={periodLabel(pl.data?.from ?? range.from, pl.data?.to ?? range.to)}
      exports={exports}
      updatedAt={pl.dataUpdatedAt}
      filters={<DateRangeFilter from={range.from} to={range.to} onChange={setRange} />}
    >
      <Loaded query={pl}>
        {(d) => <StatementTable caption={t('reports.pl.title')} rows={rows(d)} depths={depths} ledgerQuery={`&from=${range.from}&to=${range.to}`} />}
      </Loaded>
    </ReportFrame>
  )
}
