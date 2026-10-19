import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useSearchParams } from 'react-router'

import { type GeneralLedger, useGeneralLedger, useLedgerAccounts } from '../../api/accounting'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { DataTable } from '../../components/ui/DataTable'
import { DateRangeFilter } from '../../components/ui/DateRangeFilter'
import { Select } from '../../components/ui/Select'
import { downloadCsv } from '../../lib/csv'
import { formatDate, startOfMonth, today } from '../../lib/dates'
import { useSession } from '../auth/session'
import { accountLabel, chartOrder } from '../accounting/ledger'
import { reportAccess } from './access'
import { money, total, usePeriodLabel } from './format'
import { type ExportAction, Loaded, ReportFrame } from './shared'
import { useDrCr } from './statementRows'

type GlRow = GeneralLedger['rows'][number]

/** General ledger of one account with a running balance (ACC-R03). */
export function GeneralLedgerPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const allowed = reportAccess(can).statements
  const [params, setParams] = useSearchParams()
  const accountId = params.get('account_id') ?? ''
  const from = params.get('from') ?? startOfMonth() // blank = from the beginning
  const to = params.get('to') ?? today()
  const periodLabel = usePeriodLabel()
  const drCr = useDrCr()
  const accounts = useLedgerAccounts(tenantId)
  const postable = useMemo(() => chartOrder(accounts.data ?? []).filter((a) => a.is_postable), [accounts.data])
  const gl = useGeneralLedger(allowed ? tenantId : '', allowed ? accountId : '', from || undefined, to || undefined)

  const update = (patch: Record<string, string>) => {
    const next = new URLSearchParams(params)
    for (const [k, v] of Object.entries({ account_id: accountId, from, to, ...patch })) {
      if (v || k !== 'account_id') next.set(k, v)
      else next.delete(k)
    }
    setParams(next, { replace: true })
  }

  const exports: ExportAction[] = []
  if (gl.data && accountId) {
    const d = gl.data
    exports.push({
      run: () =>
        downloadCsv(`general-ledger-${d.account.code}-${from || 'start'}-${d.to}`, d.rows, [
          { header: t('common.date'), value: (r) => r.entry_date },
          { header: t('reports.gl.entryNo'), value: (r) => r.entry_no },
          { header: t('reports.gl.source'), value: (r) => t(`accounting.sources.${r.source_type}`, { defaultValue: r.source_type }) },
          { header: t('reports.gl.sourceNo'), value: (r) => r.source_no },
          { header: t('reports.gl.memo'), value: (r) => r.memo },
          { header: t('reports.tb.debit'), value: (r) => r.debit },
          { header: t('reports.tb.credit'), value: (r) => r.credit },
          { header: t('reports.gl.balance'), value: (r) => r.balance },
        ]),
    })
  }

  const account = postable.find((a) => a.id === accountId) ?? gl.data?.account
  return (
    <ReportFrame
      title={t('reports.gl.title')}
      allowed={allowed}
      period={`${account ? `${accountLabel(account)} · ` : ''}${periodLabel(from, gl.data?.to ?? to)}`}
      exports={exports}
      updatedAt={gl.dataUpdatedAt}
      filters={
        <>
          <Select
            label={t('reports.account')}
            value={accountId}
            onChange={(e) => update({ account_id: e.target.value })}
            options={[{ value: '', label: t('reports.gl.chooseAccount') }, ...postable.map((a) => ({ value: a.id, label: accountLabel(a) }))]}
          />
          <DateRangeFilter from={from} to={to} onChange={(r) => update(r)} />
        </>
      }
    >
      {!accountId ? (
        <p className="text-sm text-ink-muted">{t('reports.gl.pickAccount')}</p>
      ) : (
        <Loaded query={gl}>
          {(d) => (
            <>
              {d.truncated && <Alert tone="warning">{t('reports.gl.truncated')}</Alert>}
              <dl className="grid grid-cols-2 gap-3 text-sm sm:max-w-md">
                <dt className="text-ink-muted">{t('reports.gl.opening')}</dt>
                <dd className="text-right font-semibold tabular-nums">{drCr(d.opening)}</dd>
                <dt className="text-ink-muted">{t('reports.gl.closing')}</dt>
                <dd className="text-right font-semibold tabular-nums">{drCr(d.closing)}</dd>
              </dl>
              <DataTable<GlRow>
                caption={t('reports.gl.title')}
                rows={d.rows}
                rowKey={(r, i) => `${r.entry_id}-${i}`}
                empty={t('reports.gl.noEntries')}
                columns={[
                  { key: 'date', header: t('common.date'), render: (r) => <span className="whitespace-nowrap">{formatDate(r.entry_date, lang)}</span> },
                  {
                    key: 'entry',
                    header: t('reports.gl.entryNo'),
                    render: (r) => (
                      <Link to={`/accounting/journals/${r.entry_id}`} className="font-mono text-xs font-semibold text-brand-700 hover:underline">
                        {r.entry_no}
                      </Link>
                    ),
                  },
                  { key: 'source', header: t('reports.gl.source'), render: (r) => <Badge>{t(`accounting.sources.${r.source_type}`, { defaultValue: r.source_type })}</Badge> },
                  { key: 'sourceNo', header: t('reports.gl.sourceNo'), render: (r) => r.source_no ?? '—' },
                  { key: 'memo', header: t('reports.gl.memo'), render: (r) => r.memo ?? '' },
                  { key: 'debit', header: t('reports.tb.debit'), align: 'right', render: (r) => money(r.debit, lang) },
                  { key: 'credit', header: t('reports.tb.credit'), align: 'right', render: (r) => money(r.credit, lang) },
                  { key: 'balance', header: t('reports.gl.balance'), align: 'right', render: (r) => drCr(r.balance) },
                ]}
                footer={[
                  t('common.totals'),
                  null,
                  null,
                  null,
                  null,
                  money(total(d.rows.map((r) => r.debit)), lang),
                  money(total(d.rows.map((r) => r.credit)), lang),
                  drCr(d.closing),
                ]}
              />
            </>
          )}
        </Loaded>
      )}
    </ReportFrame>
  )
}
