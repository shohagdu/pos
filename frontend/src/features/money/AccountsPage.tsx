import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { type PaymentAccount, type Transfer, usePaymentAccounts, useTransfers } from '../../api/money'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { PageHeader } from '../../components/ui/PageHeader'
import { StatCard } from '../../components/ui/StatCard'
import { sum, toFixed, dec } from '../../lib/decimal'
import { formatDate } from '../../lib/dates'
import { formatMoney } from '../../lib/format'
import { AccountDialog, TransferDialog } from './AccountDialogs'
import { PaymentMethodsSection } from './PaymentMethodsSection'
import { LinkButton, LoadMore, QueryStatus, TextLink } from './shared'
import { useTenant } from './utils'

const TYPE_TONE = { cash: 'good', bank: 'info', mobile_wallet: 'warn', card: 'neutral' } as const

/** Payment accounts (`/accounts`): balances, transfers and payment methods. */
export function AccountsPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { tenantId, can } = useTenant()
  const canManage = can('accounts.manage')
  const showBalances = can('accounts.view')
  const canTransfer = can('transfers.create')
  const accounts = usePaymentAccounts(tenantId, canManage)
  const [dialog, setDialog] = useState<{ kind: 'new' } | { kind: 'edit'; account: PaymentAccount } | { kind: 'transfer' } | null>(null)
  const list = accounts.data ?? []
  const activeList = list.filter((a) => a.status === 'active')

  const columns: Column<PaymentAccount>[] = [
    {
      key: 'name',
      header: t('money.accountName'),
      render: (a) => (
        <span className="inline-flex flex-wrap items-center gap-2">
          <TextLink to={`/accounts/${a.id}`}>{a.name}</TextLink>
          {a.is_default && <Badge tone="info">{t('money.default')}</Badge>}
        </span>
      ),
    },
    { key: 'type', header: t('money.accountType'), render: (a) => <Badge tone={TYPE_TONE[a.account_type]}>{t(`money.types.${a.account_type}`)}</Badge> },
    {
      key: 'number',
      header: t('money.numberBank'),
      render: (a) => [a.account_number, a.bank_name].filter(Boolean).join(' · ') || '—',
    },
    ...(showBalances
      ? ([{ key: 'balance', header: t('money.balance'), align: 'right', render: (a) => formatMoney(a.current_balance, lang) }] satisfies Column<PaymentAccount>[])
      : []),
    {
      key: 'status',
      header: t('common.status'),
      render: (a) => <Badge tone={a.status === 'active' ? 'good' : 'neutral'}>{t(`money.statuses.${a.status}`)}</Badge>,
    },
    ...(canManage
      ? ([
          {
            key: 'actions',
            header: <span className="sr-only">{t('common.actions')}</span>,
            render: (a) => (
              <Button variant="ghost" className="min-h-9" onClick={() => setDialog({ kind: 'edit', account: a })} aria-label={t('money.editAccountName', { name: a.name })}>
                {t('common.edit')}
              </Button>
            ),
          },
        ] satisfies Column<PaymentAccount>[])
      : []),
  ]

  const total = toFixed(sum(activeList.map((a) => dec(a.current_balance))))
  return (
    <div className="flex flex-col gap-8">
      <PageHeader
        title={t('money.title')}
        subtitle={t('money.subtitle')}
        actions={
          <>
            {showBalances && list.some((a) => a.account_type !== 'cash') && (
              <LinkButton to="/accounts/reconciliations">{t('money.reconcile.reconcile')}</LinkButton>
            )}
            {canTransfer && (
              <Button variant="secondary" disabled={activeList.length < 2} onClick={() => setDialog({ kind: 'transfer' })}>
                {t('money.transfer')}
              </Button>
            )}
            {canManage && <Button onClick={() => setDialog({ kind: 'new' })}>{t('money.newAccount')}</Button>}
          </>
        }
      />
      {showBalances && accounts.isSuccess && (
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
          <StatCard label={t('money.totalBalance')} value={formatMoney(total, lang)} />
          {(['cash', 'bank', 'mobile_wallet'] as const).map((type) => {
            const ofType = activeList.filter((a) => a.account_type === type)
            if (ofType.length === 0) return null
            return <StatCard key={type} label={t(`money.types.${type}`)} value={formatMoney(toFixed(sum(ofType.map((a) => dec(a.current_balance)))), lang)} />
          })}
        </div>
      )}
      {accounts.isSuccess ? (
        <DataTable caption={t('money.title')} columns={columns} rows={list} rowKey={(a) => a.id} empty={t('money.noAccounts')} />
      ) : (
        <QueryStatus query={accounts} />
      )}

      {showBalances && <TransfersSection tenantId={tenantId} />}
      {canManage && accounts.isSuccess && <PaymentMethodsSection tenantId={tenantId} accounts={list} />}

      {dialog?.kind === 'new' && <AccountDialog tenantId={tenantId} onClose={() => setDialog(null)} />}
      {dialog?.kind === 'edit' && <AccountDialog key={dialog.account.id} account={dialog.account} tenantId={tenantId} onClose={() => setDialog(null)} />}
      {dialog?.kind === 'transfer' && <TransferDialog accounts={list} tenantId={tenantId} showBalances={showBalances} onClose={() => setDialog(null)} />}
    </div>
  )
}

function TransfersSection({ tenantId }: { tenantId: string }) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const transfers = useTransfers(tenantId)
  const rows = transfers.data?.pages.flatMap((p) => p.data) ?? []
  const columns: Column<Transfer>[] = [
    { key: 'no', header: t('money.transferNo'), render: (x) => <span className="font-medium">{x.transfer_no}</span> },
    { key: 'date', header: t('common.date'), render: (x) => formatDate(x.business_date, lang) },
    { key: 'from', header: t('money.fromAccount'), render: (x) => x.from_account_name },
    { key: 'to', header: t('money.toAccount'), render: (x) => x.to_account_name },
    { key: 'amount', header: t('common.amount'), align: 'right', render: (x) => formatMoney(x.amount, lang) },
    { key: 'fee', header: t('money.fee'), align: 'right', render: (x) => (dec(x.fee_amount) === 0n ? '' : formatMoney(x.fee_amount, lang)) },
    { key: 'ref', header: t('money.reference'), render: (x) => x.reference ?? '' },
    { key: 'note', header: t('common.note'), render: (x) => x.note ?? '' },
  ]
  return (
    <section className="flex flex-col gap-3" aria-labelledby="transfers-heading">
      <h2 id="transfers-heading" className="text-lg font-semibold">
        {t('money.recentTransfers')}
      </h2>
      {transfers.isSuccess ? (
        <>
          <DataTable caption={t('money.recentTransfers')} columns={columns} rows={rows} rowKey={(x) => x.id} empty={t('money.noTransfers')} />
          <LoadMore query={transfers} />
        </>
      ) : (
        <QueryStatus query={transfers} />
      )}
    </section>
  )
}
