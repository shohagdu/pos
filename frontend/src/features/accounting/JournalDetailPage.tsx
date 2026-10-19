import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate, useParams } from 'react-router'

import { type JournalLine, accountingApi, accountingKeys, useJournal } from '../../api/accounting'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { ConfirmButton } from '../../components/ui/ConfirmButton'
import { DataTable } from '../../components/ui/DataTable'
import { PageHeader } from '../../components/ui/PageHeader'
import { Spinner } from '../../components/ui/Spinner'
import { formatDate, formatDateTime } from '../../lib/dates'
import { dec, isZero } from '../../lib/decimal'
import { formatMoney } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { AccountingNav } from './AccountingNav'

/** One journal entry with its lines; manual journals can be reversed. */
export function JournalDetailPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { id = '' } = useParams()
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const journal = useJournal(tenantId, id)

  const reverse = useMutation({
    mutationFn: () => accountingApi.reverseJournal(id),
    onSuccess: (reversal) => {
      void queryClient.invalidateQueries({ queryKey: accountingKeys.all(tenantId) })
      void queryClient.invalidateQueries({ queryKey: ['reports', tenantId] })
      void navigate(`/accounting/journals/${reversal.id}`)
    },
  })

  const j = journal.data
  const canReverse = j && j.source_type === 'manual' && j.status === 'posted' && !j.reversal_of_id && can('accounting.journal_create')

  return (
    <div className="flex max-w-5xl flex-col gap-6">
      <PageHeader
        title={j ? t('accounting.detail.title', { no: j.entry_no }) : t('accounting.detail.loading')}
        actions={
          <div className="flex flex-wrap gap-2 print:hidden">
            <Link to="/accounting/journals" className="inline-flex min-h-11 items-center rounded-lg px-3 text-sm font-semibold text-ink-muted hover:bg-canvas">
              {t('common.back')}
            </Link>
            {j && (
              <Button variant="secondary" onClick={() => window.print()}>
                {t('common.print')}
              </Button>
            )}
            {canReverse && (
              <ConfirmButton onConfirm={() => reverse.mutate()} disabled={reverse.isPending}>
                {t('accounting.detail.reverse')}
              </ConfirmButton>
            )}
          </div>
        }
      />
      <AccountingNav />
      {reverse.isError && <Alert tone="danger">{problemMessage(reverse.error, t)}</Alert>}
      {journal.isPending ? (
        <Spinner />
      ) : journal.isError ? (
        <Alert tone="danger">{problemMessage(journal.error, t)}</Alert>
      ) : (
        j && (
          <>
            <Card className="p-4">
              <dl className="grid gap-x-6 gap-y-2 text-sm sm:grid-cols-[auto_1fr_auto_1fr]">
                <dt className="text-ink-muted">{t('common.date')}</dt>
                <dd>{formatDate(j.entry_date, lang)}</dd>
                <dt className="text-ink-muted">{t('common.status')}</dt>
                <dd>
                  <Badge tone={j.status === 'posted' ? 'good' : 'warn'}>{t(`accounting.statuses.${j.status}`)}</Badge>
                </dd>
                <dt className="text-ink-muted">{t('accounting.sourceType')}</dt>
                <dd>{t(`accounting.sources.${j.source_type}`, { defaultValue: j.source_type })}</dd>
                <dt className="text-ink-muted">{t('accounting.sourceNo')}</dt>
                <dd>{j.source_no ?? '—'}</dd>
                <dt className="text-ink-muted">{t('accounting.journal.memo')}</dt>
                <dd className="sm:col-span-3">{j.memo ?? '—'}</dd>
                <dt className="text-ink-muted">{t('accounting.detail.createdAt')}</dt>
                <dd>{formatDateTime(j.created_at, lang)}</dd>
                {j.reversal_of_id && (
                  <>
                    <dt className="text-ink-muted">{t('accounting.detail.reverses')}</dt>
                    <dd>
                      <Link to={`/accounting/journals/${j.reversal_of_id}`} className="font-semibold text-brand-700 hover:underline">
                        {t('accounting.detail.original')}
                      </Link>
                    </dd>
                  </>
                )}
              </dl>
            </Card>
            <DataTable<JournalLine>
              caption={t('accounting.journal.lines')}
              rows={j.lines}
              rowKey={(l, i) => `${l.account_id}-${i}`}
              columns={[
                {
                  key: 'account',
                  header: t('accounting.journal.account'),
                  render: (l) => (
                    <Link to={`/reports/general-ledger?account_id=${l.account_id}`} className="hover:underline">
                      <span className="mr-2 font-mono text-xs text-ink-muted">{l.account_code}</span>
                      {l.account_name}
                    </Link>
                  ),
                },
                { key: 'memo', header: t('accounting.journal.lineMemo'), render: (l) => l.memo ?? '' },
                { key: 'debit', header: t('accounting.debit'), align: 'right', render: (l) => (isZero(dec(l.debit)) ? '' : formatMoney(l.debit, lang)) },
                { key: 'credit', header: t('accounting.credit'), align: 'right', render: (l) => (isZero(dec(l.credit)) ? '' : formatMoney(l.credit, lang)) },
              ]}
              footer={[t('common.totals'), null, formatMoney(j.total_debit, lang), formatMoney(j.total_credit, lang)]}
            />
          </>
        )
      )}
    </div>
  )
}
