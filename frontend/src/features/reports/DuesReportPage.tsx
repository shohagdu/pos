import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { type DueRecord, useDuesReport } from '../../api/reports'
import { DataTable } from '../../components/ui/DataTable'
import { DateRangeFilter } from '../../components/ui/DateRangeFilter'
import { TextField } from '../../components/ui/TextField'
import { downloadCsv } from '../../lib/csv'
import { formatDate } from '../../lib/dates'
import { useSession } from '../auth/session'
import { reportAccess } from './access'
import { money } from './format'
import { type ExportAction, Loaded, ReportFrame } from './shared'

type Party = 'customers' | 'suppliers'

/** Ages that deserve attention: over a month is late, over three months very late. */
function ageTone(days: number) {
  if (days > 90) return 'font-semibold text-danger'
  if (days > 30) return 'font-medium text-warning'
  return ''
}

/**
 * Due customer records (sales still owed) or suppliers waiting for payment
 * (received purchases still to pay): one row per unpaid document, oldest first.
 */
export function DuesReportPage({ party }: { party: Party }) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const allowed = reportAccess(can).parties
  const [search, setSearch] = useState('')
  const [q, setQ] = useState('')
  const [range, setRange] = useState({ from: '', to: '' })
  // Wait for a pause in typing before asking the server.
  useEffect(() => {
    const id = setTimeout(() => setQ(search.trim()), 300)
    return () => clearTimeout(id)
  }, [search])
  const report = useDuesReport(tenantId, party, { q, ...range }, allowed)
  const k = `reports.dues.${party}`
  const partyBase = party === 'customers' ? '/customers' : '/suppliers'
  const docBase = party === 'customers' ? '/sales' : '/purchases'

  const exports: ExportAction[] = []
  const d = report.data
  if (d) {
    exports.push({
      run: () =>
        downloadCsv(`${party === 'customers' ? 'customer-dues' : 'supplier-payables'}-${d.as_of}`, d.rows, [
          { header: t(`${k}.party`), value: (r) => r.party_name },
          { header: t('reports.dues.phone'), value: (r) => r.party_phone },
          { header: t(`${k}.doc`), value: (r) => r.doc_no },
          ...(party === 'suppliers' ? [{ header: t('reports.dues.reference'), value: (r: DueRecord) => r.reference }] : []),
          { header: t('reports.dues.date'), value: (r) => r.doc_date },
          { header: t('reports.dues.daysOpen'), value: (r) => String(r.days_open) },
          { header: t('reports.dues.total'), value: (r) => r.grand_total },
          { header: t('reports.dues.paid'), value: (r) => r.paid_total },
          { header: t('reports.dues.returned'), value: (r) => r.returned_total },
          { header: t(`${k}.due`), value: (r) => r.due_total },
        ]),
    })
  }

  return (
    <ReportFrame
      title={t(`${k}.title`)}
      allowed={allowed}
      period={
        range.from || range.to
          ? `${range.from ? formatDate(range.from, lang) : '…'} – ${range.to ? formatDate(range.to, lang) : '…'}`
          : t('reports.dues.allOpen')
      }
      exports={exports}
      updatedAt={report.dataUpdatedAt}
      links={
        <Link to="/reports/aging" className="text-sm font-medium text-brand-700 hover:underline">
          {t('reports.dues.seeAging')}
        </Link>
      }
      filters={
        <>
          <TextField label={t(`${k}.search`)} type="search" value={search} onChange={(e) => setSearch(e.target.value)} />
          <DateRangeFilter from={range.from} to={range.to} onChange={setRange} />
        </>
      }
    >
      <Loaded query={report}>
        {(r) => (
          <>
            <dl className="grid gap-3 sm:grid-cols-3">
              <div className="rounded-xl border border-border bg-surface p-4">
                <dt className="text-sm text-ink-muted">{t(`${k}.due`)}</dt>
                <dd className="text-xl font-bold text-danger">{money(r.totals.due_total, lang)}</dd>
              </div>
              <div className="rounded-xl border border-border bg-surface p-4">
                <dt className="text-sm text-ink-muted">{t(`${k}.parties`)}</dt>
                <dd className="text-xl font-bold">{r.totals.parties}</dd>
              </div>
              <div className="rounded-xl border border-border bg-surface p-4">
                <dt className="text-sm text-ink-muted">{t(`${k}.documents`)}</dt>
                <dd className="text-xl font-bold">{r.rows.length}</dd>
              </div>
            </dl>
            <DataTable<DueRecord>
              caption={t(`${k}.title`)}
              rows={r.rows}
              rowKey={(row) => row.id}
              empty={t(`${k}.none`)}
              columns={[
                {
                  key: 'party',
                  header: t(`${k}.party`),
                  render: (row) => (
                    <div className="flex flex-col">
                      <Link to={`${partyBase}/${row.party_id}`} className="font-medium text-brand-700 hover:underline">
                        {row.party_name}
                      </Link>
                      <span className="text-xs text-ink-muted">{[row.party_code, row.party_phone].filter(Boolean).join(' · ')}</span>
                    </div>
                  ),
                },
                {
                  key: 'doc',
                  header: t(`${k}.doc`),
                  render: (row) => (
                    <div className="flex flex-col">
                      <Link to={`${docBase}/${row.id}`} className="font-medium text-brand-700 hover:underline">
                        {row.doc_no ?? '—'}
                      </Link>
                      {row.reference && <span className="text-xs text-ink-muted">{t('reports.dues.billNo', { no: row.reference })}</span>}
                    </div>
                  ),
                },
                { key: 'date', header: t('reports.dues.date'), render: (row) => formatDate(row.doc_date, lang) },
                {
                  key: 'days',
                  header: t('reports.dues.daysOpen'),
                  align: 'right',
                  render: (row) => <span className={ageTone(row.days_open)}>{t('reports.dues.days', { count: row.days_open })}</span>,
                },
                { key: 'total', header: t('reports.dues.total'), align: 'right', render: (row) => money(row.grand_total, lang) },
                { key: 'paid', header: t('reports.dues.paid'), align: 'right', render: (row) => money(row.paid_total, lang) },
                { key: 'returned', header: t('reports.dues.returned'), align: 'right', render: (row) => money(row.returned_total, lang) },
                { key: 'due', header: t(`${k}.due`), align: 'right', render: (row) => <span className="font-semibold">{money(row.due_total, lang)}</span> },
              ]}
              footer={[
                t('common.totals'),
                null,
                null,
                null,
                money(r.totals.grand_total, lang),
                money(r.totals.paid_total, lang),
                money(r.totals.returned_total, lang),
                money(r.totals.due_total, lang),
              ]}
            />
          </>
        )}
      </Loaded>
    </ReportFrame>
  )
}
