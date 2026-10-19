import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { type JournalEntry, useJournals } from '../../api/accounting'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { DataTable } from '../../components/ui/DataTable'
import { DateRangeFilter } from '../../components/ui/DateRangeFilter'
import { PageHeader } from '../../components/ui/PageHeader'
import { Select } from '../../components/ui/Select'
import { Spinner } from '../../components/ui/Spinner'
import { StateTabs } from '../../components/ui/Tabs'
import { startOfMonth, today, formatDate } from '../../lib/dates'
import { formatMoney } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { AccountingNav } from './AccountingNav'
import { ChartOfAccounts } from './ChartOfAccounts'
import { SOURCE_TYPES } from './ledger'
import { NewJournalDialog } from './NewJournalDialog'

type Tab = 'journals' | 'chart'

/** Journal list (ACC-R02) and chart of accounts. */
export function JournalsPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const allowed = can('accounting.view')
  const [tab, setTab] = useState<Tab>('journals')
  const [range, setRange] = useState({ from: startOfMonth(), to: today() })
  const [source, setSource] = useState('')
  const [creating, setCreating] = useState(false)
  const filter = { from: range.from || undefined, to: range.to || undefined, source_type: source || undefined }
  const journals = useJournals(allowed ? tenantId : '', filter)
  const rows = journals.data?.pages.flatMap((p) => p.data) ?? []

  if (!allowed) {
    return (
      <div className="flex max-w-6xl flex-col gap-6">
        <PageHeader title={t('accounting.title')} />
        <Alert tone="warning">{t('reports.noAccess')}</Alert>
      </div>
    )
  }

  return (
    <div className="flex max-w-6xl flex-col gap-6">
      <PageHeader
        title={t('accounting.title')}
        subtitle={t('accounting.subtitle')}
        actions={
          can('accounting.journal_create') && (
            <Button onClick={() => setCreating(true)} className="print:hidden">
              {t('accounting.journal.new')}
            </Button>
          )
        }
      />
      <AccountingNav />
      <StateTabs
        label={t('accounting.title')}
        value={tab}
        onChange={setTab}
        items={[
          { value: 'journals', label: t('accounting.journals') },
          { value: 'chart', label: t('accounting.chart.title') },
        ]}
      />

      {tab === 'chart' ? (
        <ChartOfAccounts tenantId={tenantId} />
      ) : (
        <>
          <div className="flex flex-wrap items-end gap-3 print:hidden">
            <DateRangeFilter from={range.from} to={range.to} onChange={setRange} />
            <Select
              label={t('accounting.sourceType')}
              value={source}
              onChange={(e) => setSource(e.target.value)}
              options={[{ value: '', label: t('common.all') }, ...SOURCE_TYPES.map((s) => ({ value: s, label: t(`accounting.sources.${s}`) }))]}
            />
          </div>
          {journals.isPending ? (
            <Spinner />
          ) : journals.isError ? (
            <Alert tone="danger">{problemMessage(journals.error, t)}</Alert>
          ) : (
            <DataTable<JournalEntry>
              caption={t('accounting.journals')}
              rows={rows}
              rowKey={(j) => j.id}
              empty={t('accounting.noJournals')}
              columns={[
                {
                  key: 'no',
                  header: t('accounting.entryNo'),
                  render: (j) => (
                    <Link to={`/accounting/journals/${j.id}`} className="font-mono text-xs font-semibold text-brand-700 hover:underline">
                      {j.entry_no}
                    </Link>
                  ),
                },
                { key: 'date', header: t('common.date'), render: (j) => <span className="whitespace-nowrap">{formatDate(j.entry_date, lang)}</span> },
                { key: 'source', header: t('accounting.sourceType'), render: (j) => <Badge tone={j.source_type === 'manual' ? 'info' : 'neutral'}>{t(`accounting.sources.${j.source_type}`, { defaultValue: j.source_type })}</Badge> },
                { key: 'sourceNo', header: t('accounting.sourceNo'), render: (j) => j.source_no ?? '—' },
                { key: 'memo', header: t('accounting.journal.memo'), render: (j) => j.memo ?? '' },
                { key: 'amount', header: t('common.amount'), align: 'right', render: (j) => formatMoney(j.total_debit, lang) },
                {
                  key: 'status',
                  header: t('common.status'),
                  render: (j) => <Badge tone={j.status === 'posted' ? 'good' : 'warn'}>{t(`accounting.statuses.${j.status}`)}</Badge>,
                },
              ]}
            />
          )}
          {journals.hasNextPage && (
            <Button variant="secondary" className="self-center" disabled={journals.isFetchingNextPage} onClick={() => void journals.fetchNextPage()}>
              {journals.isFetchingNextPage ? t('common.loading') : t('common.loadMore')}
            </Button>
          )}
        </>
      )}
      {creating && <NewJournalDialog tenantId={tenantId} open onClose={() => setCreating(false)} />}
    </div>
  )
}
