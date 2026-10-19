import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { type TrialBalanceRow, useTrialBalance } from '../../api/accounting'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { DateRangeFilter } from '../../components/ui/DateRangeFilter'
import { downloadCsv } from '../../lib/csv'
import { startOfMonth, today } from '../../lib/dates'
import { useSession } from '../auth/session'
import { reportAccess } from './access'
import { money, usePeriodLabel } from './format'
import { BalanceBadge, type ExportAction, Loaded, ReportFrame } from './shared'
import { useAccountDepths, useDrCr } from './statementRows'

/** Trial balance (ACC-R04): opening, period movement and closing per account. */
export function TrialBalancePage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const allowed = reportAccess(can).statements
  const [range, setRange] = useState({ from: startOfMonth(), to: today() })
  const periodLabel = usePeriodLabel()
  const tb = useTrialBalance(tenantId, range.from || undefined, range.to || undefined)
  const depths = useAccountDepths(tenantId)
  const drCr = useDrCr()
  const withOpening = Boolean(range.from)
  const glQuery = `${range.from ? `&from=${range.from}` : ''}${range.to ? `&to=${range.to}` : ''}`

  const columns: Column<TrialBalanceRow>[] = [
    {
      key: 'code',
      header: t('reports.code'),
      render: (r) => (
        <Link to={`/reports/general-ledger?account_id=${r.account_id}${glQuery}`} className="font-mono text-xs text-brand-700 hover:underline">
          {r.code}
        </Link>
      ),
    },
    { key: 'name', header: t('reports.account'), render: (r) => <span style={{ paddingLeft: `${depths.get(r.account_id) ?? 0}rem` }}>{r.name}</span> },
    ...(withOpening ? [{ key: 'opening', header: t('reports.tb.opening'), align: 'right' as const, render: (r: TrialBalanceRow) => drCr(r.opening) }] : []),
    { key: 'debit', header: t('reports.tb.debit'), align: 'right', render: (r) => money(r.debit, lang) },
    { key: 'credit', header: t('reports.tb.credit'), align: 'right', render: (r) => money(r.credit, lang) },
    { key: 'cdebit', header: t('reports.tb.closingDebit'), align: 'right', render: (r) => money(r.closing_debit, lang) },
    { key: 'ccredit', header: t('reports.tb.closingCredit'), align: 'right', render: (r) => money(r.closing_credit, lang) },
  ]

  const exports: ExportAction[] = []
  if (tb.data) {
    const d = tb.data
    exports.push({
      run: () =>
        downloadCsv(`trial-balance-${range.from || 'start'}-${d.to}`, [...d.rows, { account_id: '', code: '', name: t('common.totals'), account_type: 'asset', opening: '', debit: d.total_debit, credit: d.total_credit, closing_debit: d.total_closing_debit, closing_credit: d.total_closing_credit }], [
          { header: t('reports.code'), value: (r) => r.code },
          { header: t('reports.account'), value: (r) => r.name },
          { header: t('reports.tb.opening'), value: (r) => r.opening },
          { header: t('reports.tb.debit'), value: (r) => r.debit },
          { header: t('reports.tb.credit'), value: (r) => r.credit },
          { header: t('reports.tb.closingDebit'), value: (r) => r.closing_debit },
          { header: t('reports.tb.closingCredit'), value: (r) => r.closing_credit },
        ]),
    })
  }

  const totals: Record<string, string> = tb.data
    ? {
        debit: money(tb.data.total_debit, lang),
        credit: money(tb.data.total_credit, lang),
        cdebit: money(tb.data.total_closing_debit, lang),
        ccredit: money(tb.data.total_closing_credit, lang),
      }
    : {}

  return (
    <ReportFrame
      title={t('reports.tb.title')}
      allowed={allowed}
      period={periodLabel(range.from, tb.data?.to ?? range.to)}
      exports={exports}
      updatedAt={tb.dataUpdatedAt}
      filters={
        <>
          <DateRangeFilter from={range.from} to={range.to} onChange={setRange} />
          <p className="text-xs text-ink-muted">{t('reports.tb.fromHint')}</p>
        </>
      }
    >
      <Loaded query={tb}>
        {(d) => (
          <>
            <div>
              <BalanceBadge balanced={d.balanced} />
            </div>
            <DataTable
              caption={t('reports.tb.title')}
              columns={columns}
              rows={d.rows}
              rowKey={(r) => r.account_id}
              empty={t('reports.noData')}
              footer={columns.map((c) => (c.key === 'name' ? t('common.totals') : (totals[c.key] ?? null)))}
            />
          </>
        )}
      </Loaded>
    </ReportFrame>
  )
}
