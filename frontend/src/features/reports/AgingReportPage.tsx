import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { type AgingRow, useAgingReport } from '../../api/reports'
import { DataTable } from '../../components/ui/DataTable'
import { StateTabs } from '../../components/ui/Tabs'
import { TextField } from '../../components/ui/TextField'
import { downloadCsv } from '../../lib/csv'
import { dec } from '../../lib/decimal'
import { formatDate, today } from '../../lib/dates'
import { useSession } from '../auth/session'
import { reportAccess } from './access'
import { money } from './format'
import { type ExportAction, Loaded, ReportFrame } from './shared'

type Party = 'customers' | 'suppliers'

/** Receivable / payable aging (REP-10): dues by days open, as of a date. */
export function AgingReportPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const allowed = reportAccess(can).parties
  const [party, setParty] = useState<Party>('customers')
  const [asOf, setAsOf] = useState(today())
  const report = useAgingReport(tenantId, party, asOf, allowed)

  const exports: ExportAction[] = []
  const d = report.data
  if (d) {
    exports.push({
      run: () =>
        downloadCsv(`aging-${d.party}-${d.as_of}`, d.rows, [
          { header: t('reports.code'), value: (r) => r.code },
          { header: t('reports.aging.party'), value: (r) => r.name },
          { header: t('reports.aging.phone'), value: (r) => r.phone },
          { header: t('reports.aging.current'), value: (r) => r.current },
          { header: t('reports.aging.d31'), value: (r) => r.days_31_60 },
          { header: t('reports.aging.d61'), value: (r) => r.days_61_90 },
          { header: t('reports.aging.d90'), value: (r) => r.over_90 },
          { header: t('reports.aging.other'), value: (r) => r.other },
          { header: t('reports.aging.total'), value: (r) => r.total },
          { header: t('reports.aging.oldest'), value: (r) => r.oldest_date },
        ]),
    })
  }
  const base = party === 'customers' ? '/customers' : '/suppliers'

  return (
    <ReportFrame
      title={t('reports.aging.title')}
      allowed={allowed}
      period={`${t(`reports.aging.${party}`)} · ${t('reports.asOf', { date: formatDate(d?.as_of ?? asOf, lang) })}`}
      exports={exports}
      updatedAt={report.dataUpdatedAt}
      filters={
        <>
          <StateTabs<Party>
            label={t('reports.aging.parties')}
            value={party}
            onChange={setParty}
            items={[
              { value: 'customers', label: t('reports.aging.customers') },
              { value: 'suppliers', label: t('reports.aging.suppliers') },
            ]}
          />
          <TextField label={t('reports.asOfLabel')} type="date" value={asOf} max={today()} onChange={(e) => setAsOf(e.target.value)} />
        </>
      }
    >
      <Loaded query={report}>
        {(r) => (
          <>
            <DataTable<AgingRow>
              caption={t(`reports.aging.${r.party}`)}
              rows={r.rows}
              rowKey={(row) => row.party_id}
              empty={t('reports.noData')}
              columns={[
                {
                  key: 'name',
                  header: t('reports.aging.party'),
                  render: (row) => (
                    <div className="flex flex-col">
                      <Link to={`${base}/${row.party_id}`} className="font-medium text-brand-700 hover:underline">
                        {row.name}
                      </Link>
                      <span className="text-xs text-ink-muted">{[row.code, row.phone].filter(Boolean).join(' · ')}</span>
                    </div>
                  ),
                },
                { key: 'current', header: t('reports.aging.current'), align: 'right', render: (row) => money(row.current, lang) },
                { key: 'd31', header: t('reports.aging.d31'), align: 'right', render: (row) => money(row.days_31_60, lang) },
                { key: 'd61', header: t('reports.aging.d61'), align: 'right', render: (row) => money(row.days_61_90, lang) },
                {
                  key: 'd90',
                  header: t('reports.aging.d90'),
                  align: 'right',
                  render: (row) => <span className={dec(row.over_90) > 0n ? 'font-semibold text-danger' : ''}>{money(row.over_90, lang)}</span>,
                },
                { key: 'other', header: t('reports.aging.other'), align: 'right', render: (row) => money(row.other, lang) },
                { key: 'total', header: t('reports.aging.total'), align: 'right', render: (row) => <span className="font-semibold">{money(row.total, lang)}</span> },
                { key: 'oldest', header: t('reports.aging.oldest'), render: (row) => (row.oldest_date ? formatDate(row.oldest_date, lang) : '—') },
              ]}
              footer={[
                t('common.totals'),
                money(r.totals.current, lang),
                money(r.totals.days_31_60, lang),
                money(r.totals.days_61_90, lang),
                money(r.totals.over_90, lang),
                money(r.totals.other, lang),
                money(r.totals.total, lang),
                null,
              ]}
            />
            <p className="text-sm text-ink-muted">{t('reports.aging.otherHint')}</p>
          </>
        )}
      </Loaded>
    </ReportFrame>
  )
}
