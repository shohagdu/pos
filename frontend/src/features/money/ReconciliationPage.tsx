import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useParams } from 'react-router'

import { type Reconciliation, type ReconciliationLine, moneyApi, moneyKeys, useReconciliation } from '../../api/money'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { ConfirmButton } from '../../components/ui/ConfirmButton'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { PageHeader } from '../../components/ui/PageHeader'
import { StatCard } from '../../components/ui/StatCard'
import { dec } from '../../lib/decimal'
import { formatDate, formatDateTime } from '../../lib/dates'
import { formatMoney } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { ReasonDialog } from './ReasonDialog'
import { QueryStatus, TextLink } from './shared'
import { RECONCILIATION_TONE, referencePath, useTenant } from './utils'

/** One reconciliation (`/accounts/reconciliations/:id`): tick the statement's lines until the difference is 0. */
export function ReconciliationPage() {
  const { id = '' } = useParams()
  const { tenantId } = useTenant()
  const rec = useReconciliation(tenantId, id)
  if (!rec.data) return <QueryStatus query={rec} />
  return <Worksheet rec={rec.data} tenantId={tenantId} key={rec.data.id} />
}

function Worksheet({ rec, tenantId }: { rec: Reconciliation; tenantId: string }) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const queryClient = useQueryClient()
  const { can } = useTenant()
  const [reopening, setReopening] = useState(false)
  const editable = rec.status === 'in_progress' && can('accounts.manage')
  const money = (v: string) => formatMoney(v, lang)
  const saved = async (next: Reconciliation) => {
    queryClient.setQueryData(moneyKeys.reconciliation(tenantId, next.id), next)
    await queryClient.invalidateQueries({ queryKey: ['money', tenantId, 'reconciliations'] })
  }
  // Every tick is saved at once; the server returns the new figures.
  const tick = useMutation({
    mutationFn: (change: { clear?: string[]; unclear?: string[] }) => moneyApi.setCleared(rec.id, change),
    onSuccess: saved,
  })
  const action = useMutation({
    mutationFn: (kind: 'complete' | 'cancel') =>
      kind === 'complete' ? moneyApi.completeReconciliation(rec.id) : moneyApi.cancelReconciliation(rec.id),
    onSuccess: saved,
  })
  const reopen = useMutation({
    mutationFn: (reason: string) => moneyApi.reopenReconciliation(rec.id, reason),
    onSuccess: async (next) => {
      await saved(next)
      setReopening(false)
    },
  })

  const label = (l: ReconciliationLine) =>
    `${formatDate(l.business_date, lang)} ${t(`money.txnTypes.${l.txn_type}`, { defaultValue: l.txn_type })} ${l.reference_no ?? ''} ${money(l.amount)}`
  const difference = dec(rec.difference)
  const uncleared = rec.lines.filter((l) => !l.cleared).map((l) => l.id)
  const cleared = rec.lines.filter((l) => l.cleared).map((l) => l.id)

  const columns: Column<ReconciliationLine>[] = [
    ...(editable
      ? ([
          {
            key: 'tick',
            header: <span className="sr-only">{t('money.reconcile.cleared')}</span>,
            render: (l) => (
              <input
                type="checkbox"
                className="size-5 accent-brand-600"
                aria-label={t('money.reconcile.clearLine', { label: label(l) })}
                checked={l.cleared}
                disabled={tick.isPending}
                onChange={(e) => tick.mutate(e.target.checked ? { clear: [l.id] } : { unclear: [l.id] })}
              />
            ),
          },
        ] satisfies Column<ReconciliationLine>[])
      : []),
    { key: 'date', header: t('common.date'), render: (l) => <span className="whitespace-nowrap">{formatDate(l.business_date, lang)}</span> },
    { key: 'type', header: t('money.txnType'), render: (l) => t(`money.txnTypes.${l.txn_type}`, { defaultValue: l.txn_type }) },
    {
      key: 'ref',
      header: t('money.referenceNo'),
      render: (l) => {
        const to = referencePath(l.reference_type, l.reference_id)
        return to && l.reference_no ? <TextLink to={to}>{l.reference_no}</TextLink> : (l.reference_no ?? '—')
      },
    },
    { key: 'note', header: t('common.note'), render: (l) => <span className="line-clamp-1 max-w-72">{l.note ?? ''}</span> },
    { key: 'in', header: t('money.moneyIn'), align: 'right', render: (l) => (dec(l.amount) > 0n ? money(l.amount) : '') },
    { key: 'out', header: t('money.moneyOut'), align: 'right', render: (l) => (dec(l.amount) < 0n ? money(l.amount.replace('-', '')) : '') },
  ]

  return (
    <div className="flex max-w-6xl flex-col gap-6">
      <Link to={`/accounts/reconciliations?account_id=${rec.account_id}`} className="text-sm font-medium text-brand-700 hover:underline print:hidden">
        ← {t('money.reconcile.title')}
      </Link>
      <PageHeader
        title={`${rec.account_name} · ${formatDate(rec.statement_date, lang)}`}
        subtitle={
          <span className="inline-flex flex-wrap items-center gap-2">
            <Badge tone={RECONCILIATION_TONE[rec.status]}>{t(`money.reconcile.statuses.${rec.status}`)}</Badge>
            {t('money.reconcile.startedBy')}: {rec.created_by_name ?? '—'} · {formatDateTime(rec.created_at, lang)}
            {rec.completed_at && ` · ${t('money.reconcile.completedBy')}: ${rec.completed_by_name ?? '—'} · ${formatDateTime(rec.completed_at, lang)}`}
          </span>
        }
        actions={
          <div className="flex flex-wrap gap-2 print:hidden">
            {editable && (
              <ConfirmButton disabled={action.isPending} onConfirm={() => action.mutate('cancel')}>
                {t('money.reconcile.cancel')}
              </ConfirmButton>
            )}
            {editable && (
              <Button disabled={difference !== 0n || action.isPending} title={t('money.reconcile.completeHint')} onClick={() => action.mutate('complete')}>
                {t('money.reconcile.complete')}
              </Button>
            )}
            {rec.status === 'completed' && can('accounts.manage') && (
              <Button variant="secondary" onClick={() => setReopening(true)}>
                {t('money.reconcile.reopen')}
              </Button>
            )}
            <Button variant="secondary" onClick={() => window.print()}>
              {t('common.print')}
            </Button>
          </div>
        }
      />
      {(tick.isError || action.isError) && <Alert tone="danger">{problemMessage(tick.error ?? action.error, t)}</Alert>}
      {rec.note && <p className="text-sm whitespace-pre-line">{rec.note}</p>}

      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        <StatCard label={t('money.reconcile.statementBalance')} value={money(rec.statement_balance)} />
        <StatCard label={t('money.reconcile.openingBalance')} value={money(rec.opening_balance)} />
        <StatCard
          label={t('money.reconcile.clearedBalance')}
          value={money(rec.cleared_balance)}
          hint={`+ ${money(rec.cleared_in)} / − ${money(rec.cleared_out)}`}
        />
        <div aria-live="polite">
          <StatCard
            label={t('money.reconcile.difference')}
            value={money(rec.difference)}
            hint={difference === 0n ? '✓' : t('money.reconcile.completeHint')}
          />
        </div>
      </div>
      <p className="text-sm text-ink-muted">
        {t('money.reconcile.bookBalance')}: <span className="font-medium text-ink tabular-nums">{money(rec.book_balance)}</span>
      </p>

      <section className="flex flex-col gap-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <h2 className="text-lg font-semibold">{t('money.reconcile.lines')}</h2>
          {editable && rec.lines.length > 0 && (
            <div className="flex gap-2 print:hidden">
              <Button variant="secondary" disabled={tick.isPending || uncleared.length === 0} onClick={() => tick.mutate({ clear: uncleared })}>
                {t('money.reconcile.tickAll')}
              </Button>
              <Button variant="ghost" disabled={tick.isPending || cleared.length === 0} onClick={() => tick.mutate({ unclear: cleared })}>
                {t('money.reconcile.untickAll')}
              </Button>
            </div>
          )}
        </div>
        {editable && <p className="text-sm text-ink-muted print:hidden">{t('money.reconcile.linesHint')}</p>}
        <DataTable caption={t('money.reconcile.lines')} columns={columns} rows={rec.lines} rowKey={(l) => l.id} empty={t('money.reconcile.noLines')} />
      </section>

      {reopening && (
        <ReasonDialog
          title={t('money.reconcile.reopenTitle')}
          description={t('money.reconcile.reopenHint')}
          confirmLabel={t('money.reconcile.reopen')}
          pending={reopen.isPending}
          error={reopen.error}
          onConfirm={(reason) => reopen.mutate(reason)}
          onClose={() => setReopening(false)}
        />
      )}
    </div>
  )
}
