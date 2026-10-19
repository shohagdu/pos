import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { type BalanceSheet, useBalanceSheet } from '../../api/accounting'
import { TextField } from '../../components/ui/TextField'
import { downloadCsv } from '../../lib/csv'
import { formatDate, today } from '../../lib/dates'
import { dec, toFixed } from '../../lib/decimal'
import { useSession } from '../auth/session'
import { reportAccess } from './access'
import { BalanceBadge, type ExportAction, Loaded, ReportFrame } from './shared'
import { StatementTable } from './Statement'
import { type StatementRow, sectionRows, statementCsv, useAccountDepths } from './statementRows'

/** Balance sheet as of a date (ACC-R06). */
export function BalanceSheetPage() {
  const { t, i18n } = useTranslation()
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const allowed = reportAccess(can).statements
  const [asOf, setAsOf] = useState(today())
  const bs = useBalanceSheet(tenantId, asOf || undefined)
  const depths = useAccountDepths(tenantId)

  const rows = (d: BalanceSheet): StatementRow[] => {
    const lines = (key: string, section: BalanceSheet['assets']) => sectionRows(key, '', section, '').filter((r) => r.kind === 'line')
    return [
      { key: 'as-h', kind: 'heading', label: t('reports.bs.assets') },
      ...lines('as', d.assets),
      { key: 'as-g', kind: 'grand', label: t('reports.bs.totalAssets'), amount: d.total_assets },
      ...sectionRows('li', t('reports.bs.liabilities'), d.liabilities, t('reports.bs.totalLiabilities')),
      { key: 'eq-h', kind: 'heading', label: t('reports.bs.equity') },
      ...lines('eq', d.equity),
      { key: 'eq-ce', kind: 'line', label: t('reports.bs.currentEarnings'), amount: d.current_earnings },
      { key: 'eq-t', kind: 'total', label: t('reports.bs.totalEquity'), amount: toFixed(dec(d.equity.total) + dec(d.current_earnings), 2) },
      { key: 'le-g', kind: 'grand', label: t('reports.bs.totalLiabilitiesEquity'), amount: d.total_liabilities_and_equity },
    ]
  }

  const exports: ExportAction[] = []
  if (bs.data) {
    const data = rows(bs.data)
    exports.push({ run: () => downloadCsv(`balance-sheet-${asOf}`, data, statementCsv(t)) })
  }

  return (
    <ReportFrame
      title={t('reports.bs.title')}
      allowed={allowed}
      period={t('reports.asOf', { date: formatDate(bs.data?.as_of ?? asOf, i18n.language) })}
      exports={exports}
      updatedAt={bs.dataUpdatedAt}
      filters={<TextField label={t('reports.asOfLabel')} type="date" value={asOf} onChange={(e) => setAsOf(e.target.value)} />}
    >
      <Loaded query={bs}>
        {(d) => (
          <>
            <div>
              <BalanceBadge balanced={d.balanced} />
            </div>
            <StatementTable caption={t('reports.bs.title')} rows={rows(d)} depths={depths} ledgerQuery={`&to=${d.as_of}`} />
            <p className="text-xs text-ink-muted">{t('reports.bs.earningsNote')}</p>
          </>
        )}
      </Loaded>
    </ReportFrame>
  )
}
