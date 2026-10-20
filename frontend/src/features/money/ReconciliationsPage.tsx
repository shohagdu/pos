import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate, useSearchParams } from 'react-router'

import { type PaymentAccount, type ReconciliationSummary, moneyApi, moneyKeys, usePaymentAccounts, useReconciliations } from '../../api/money'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { Dialog } from '../../components/ui/Dialog'
import { PageHeader } from '../../components/ui/PageHeader'
import { Select } from '../../components/ui/Select'
import { TextField } from '../../components/ui/TextField'
import { formatDate, today } from '../../lib/dates'
import { formatMoney } from '../../lib/format'
import { optional } from '../../lib/form'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { QueryStatus, TextLink } from './shared'
import { RECONCILIATION_TONE, useTenant } from './utils'

/** Bank, wallet and card accounts can be reconciled; cash is counted with shifts. */
const reconcilable = (a: PaymentAccount) => a.account_type !== 'cash' && a.status === 'active'

/** Bank reconciliations (`/accounts/reconciliations`, PAY-R06). */
export function ReconciliationsPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const [params, setParams] = useSearchParams()
  const accountId = params.get('account_id') ?? ''
  const { tenantId, can } = useTenant()
  const accounts = usePaymentAccounts(tenantId)
  const list = useReconciliations(tenantId, accountId || undefined)
  const [starting, setStarting] = useState(false)
  const choices = (accounts.data ?? []).filter(reconcilable)

  const columns: Column<ReconciliationSummary>[] = [
    {
      key: 'date',
      header: t('money.reconcile.statementDate'),
      render: (r) => <TextLink to={`/accounts/reconciliations/${r.id}`}>{formatDate(r.statement_date, lang)}</TextLink>,
    },
    { key: 'account', header: t('money.reconcile.account'), render: (r) => r.account_name },
    { key: 'statement', header: t('money.reconcile.statementBalance'), align: 'right', render: (r) => formatMoney(r.statement_balance, lang) },
    { key: 'cleared', header: t('money.reconcile.clearedBalance'), align: 'right', render: (r) => formatMoney(r.cleared_balance, lang) },
    { key: 'lines', header: t('money.reconcile.clearedCount'), align: 'right', render: (r) => r.cleared_count },
    {
      key: 'status',
      header: t('common.status'),
      render: (r) => <Badge tone={RECONCILIATION_TONE[r.status]}>{t(`money.reconcile.statuses.${r.status}`)}</Badge>,
    },
    {
      key: 'by',
      header: t('money.reconcile.completedBy'),
      render: (r) => (r.completed_at ? `${r.completed_by_name ?? '—'} · ${formatDate(r.completed_at.slice(0, 10), lang)}` : '—'),
    },
  ]

  return (
    <div className="flex flex-col gap-6">
      <Link to="/accounts" className="text-sm font-medium text-brand-700 hover:underline">
        ← {t('money.title')}
      </Link>
      <PageHeader
        title={t('money.reconcile.title')}
        subtitle={t('money.reconcile.subtitle')}
        actions={can('accounts.manage') && <Button onClick={() => setStarting(true)}>{t('money.reconcile.start')}</Button>}
      />
      {choices.length > 1 && (
        <div className="flex flex-wrap items-end gap-3">
          <Select
            label={t('money.reconcile.account')}
            value={accountId}
            onChange={(e) => setParams(e.target.value ? { account_id: e.target.value } : {}, { replace: true })}
            options={[{ value: '', label: t('common.all') }, ...choices.map((a) => ({ value: a.id, label: a.name }))]}
          />
        </div>
      )}
      {list.isSuccess ? (
        <DataTable caption={t('money.reconcile.title')} columns={columns} rows={list.data} rowKey={(r) => r.id} empty={t('money.reconcile.none')} />
      ) : (
        <QueryStatus query={list} />
      )}
      {starting && <StartDialog tenantId={tenantId} accounts={choices} defaultAccountId={accountId} onClose={() => setStarting(false)} />}
    </div>
  )
}

function StartDialog({
  tenantId,
  accounts,
  defaultAccountId,
  onClose,
}: {
  tenantId: string
  accounts: PaymentAccount[]
  defaultAccountId: string
  onClose: () => void
}) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [accountId, setAccountId] = useState(defaultAccountId || (accounts.length === 1 ? accounts[0]!.id : ''))
  const [date, setDate] = useState(today())
  const [balance, setBalance] = useState('')
  const [note, setNote] = useState('')
  const [localErrors, setLocalErrors] = useState<Record<string, string>>({})
  const start = useMutation({
    mutationFn: () => moneyApi.startReconciliation({ account_id: accountId, statement_date: date, statement_balance: balance.trim(), note: optional(note) }),
    onSuccess: async (rec) => {
      queryClient.setQueryData(moneyKeys.reconciliation(tenantId, rec.id), rec)
      await queryClient.invalidateQueries({ queryKey: ['money', tenantId, 'reconciliations'] })
      navigate(`/accounts/reconciliations/${rec.id}`)
    },
  })

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    const errs: Record<string, string> = {}
    if (!accountId) errs.account_id = t('money.reconcile.errors.account')
    if (!date || date > today()) errs.statement_date = t('money.reconcile.errors.date')
    if (!/^-?\d+(\.\d{1,2})?$/.test(balance.trim())) errs.statement_balance = t('money.reconcile.errors.balance')
    setLocalErrors(errs)
    if (Object.keys(errs).length === 0) start.mutate()
  }

  const errors = { ...fieldErrors(start.error, t), ...localErrors }
  return (
    <Dialog open onClose={onClose} title={t('money.reconcile.start')}>
      <form onSubmit={onSubmit} noValidate className="flex flex-col gap-4">
        {start.isError && hasFormError(start.error) && <Alert tone="danger">{problemMessage(start.error, t)}</Alert>}
        <Select
          label={t('money.reconcile.account')}
          value={accountId}
          onChange={(e) => setAccountId(e.target.value)}
          error={errors.account_id}
          options={[{ value: '', label: t('money.reconcile.chooseAccount') }, ...accounts.map((a) => ({ value: a.id, label: a.name }))]}
        />
        <TextField label={t('money.reconcile.statementDate')} type="date" max={today()} value={date} onChange={(e) => setDate(e.target.value)} error={errors.statement_date} />
        <TextField
          label={t('money.reconcile.statementBalance')}
          inputMode="decimal"
          value={balance}
          onChange={(e) => setBalance(e.target.value)}
          hint={t('money.reconcile.statementBalanceHint')}
          error={errors.statement_balance}
        />
        <TextField label={`${t('common.note')} (${t('common.optional')})`} value={note} maxLength={500} onChange={(e) => setNote(e.target.value)} />
        <div className="flex flex-wrap justify-end gap-2">
          <Button variant="ghost" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button type="submit" disabled={start.isPending}>
            {start.isPending ? t('common.saving') : t('money.reconcile.start')}
          </Button>
        </div>
      </form>
    </Dialog>
  )
}
