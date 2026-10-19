import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { ApiError } from '../../api/client'
import { ACCOUNT_TYPES, type AccountType, type PaymentAccount, type PaymentAccountInput, moneyApi, moneyKeys } from '../../api/money'
import { newKey } from '../../api/query'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Checkbox } from '../../components/ui/Checkbox'
import { Dialog } from '../../components/ui/Dialog'
import { Select } from '../../components/ui/Select'
import { TextField } from '../../components/ui/TextField'
import { tryParse } from '../../lib/decimal'
import { formatMoney } from '../../lib/format'
import { optional } from '../../lib/form'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'

function useRefreshMoney(tenantId: string) {
  const queryClient = useQueryClient()
  return () =>
    Promise.all([
      queryClient.invalidateQueries({ queryKey: moneyKeys.all(tenantId) }),
      queryClient.invalidateQueries({ queryKey: ['reports', tenantId] }),
    ])
}

/** Create (no `account`) or edit a payment account. Mount only while open. */
export function AccountDialog({ account, tenantId, onClose }: { account?: PaymentAccount; tenantId: string; onClose: () => void }) {
  const { t } = useTranslation()
  const refresh = useRefreshMoney(tenantId)
  const [name, setName] = useState(account?.name ?? '')
  const [type, setType] = useState<AccountType>(account?.account_type ?? 'bank')
  const [number, setNumber] = useState(account?.account_number ?? '')
  const [bank, setBank] = useState(account?.bank_name ?? '')
  const [opening, setOpening] = useState('')
  const [active, setActive] = useState(account ? account.status === 'active' : true)

  const save = useMutation({
    mutationFn: () => {
      const input: PaymentAccountInput = {
        name: name.trim(),
        account_number: optional(number),
        bank_name: optional(bank),
      }
      if (account) return moneyApi.updateAccount(account.id, account.version, { ...input, status: active ? 'active' : 'inactive' })
      return moneyApi.createAccount({ ...input, account_type: type, opening_balance: optional(opening) })
    },
    onSuccess: async () => {
      await refresh()
      onClose()
    },
    onError: (err) => {
      if (err instanceof ApiError && err.status === 412) void refresh()
    },
  })
  const errors = fieldErrors(save.error, t)

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    save.mutate()
  }

  const showBank = type === 'bank' || type === 'card'
  return (
    <Dialog open onClose={onClose} title={account ? t('money.editAccount') : t('money.newAccount')}>
      <form onSubmit={onSubmit} noValidate className="flex flex-col gap-4">
        {save.isError && hasFormError(save.error) && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}
        <TextField label={t('money.accountName')} value={name} onChange={(e) => setName(e.target.value)} required maxLength={100} error={errors.name} />
        {!account && (
          <Select
            label={t('money.accountType')}
            value={type}
            onChange={(e) => setType(e.target.value as AccountType)}
            error={errors.account_type}
            options={ACCOUNT_TYPES.map((a) => ({ value: a, label: t(`money.types.${a}`) }))}
          />
        )}
        <TextField
          label={type === 'mobile_wallet' ? t('money.walletNumber') : t('money.accountNumber')}
          value={number}
          onChange={(e) => setNumber(e.target.value)}
          maxLength={50}
          error={errors.account_number}
        />
        {showBank && <TextField label={t('money.bankName')} value={bank} onChange={(e) => setBank(e.target.value)} maxLength={100} error={errors.bank_name} />}
        {!account && (
          <TextField
            label={t('money.openingBalance')}
            value={opening}
            onChange={(e) => setOpening(e.target.value)}
            inputMode="decimal"
            hint={t('money.openingBalanceHint')}
            error={errors.opening_balance}
          />
        )}
        {account && (
          <Checkbox
            label={t('money.active')}
            hint={account.is_default ? t('money.defaultCannotDeactivate') : undefined}
            checked={active}
            disabled={account.is_default}
            onChange={(e) => setActive(e.target.checked)}
          />
        )}
        <div className="flex flex-wrap justify-end gap-2">
          <Button variant="ghost" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button type="submit" disabled={save.isPending}>
            {save.isPending ? t('common.saving') : t('common.save')}
          </Button>
        </div>
      </form>
    </Dialog>
  )
}

/** Move money between two payment accounts. Mount only while open. */
export function TransferDialog({
  accounts,
  tenantId,
  showBalances,
  onClose,
}: {
  accounts: PaymentAccount[]
  tenantId: string
  showBalances: boolean
  onClose: () => void
}) {
  const { t, i18n } = useTranslation()
  const refresh = useRefreshMoney(tenantId)
  const active = accounts.filter((a) => a.status === 'active')
  const [from, setFrom] = useState(active[0]?.id ?? '')
  const [to, setTo] = useState(active.find((a) => a.id !== active[0]?.id)?.id ?? '')
  const [amount, setAmount] = useState('')
  const [fee, setFee] = useState('')
  const [reference, setReference] = useState('')
  const [note, setNote] = useState('')
  const [localErrors, setLocalErrors] = useState<Record<string, string>>({})
  const [key] = useState(newKey)

  const transfer = useMutation({
    mutationFn: () =>
      moneyApi.createTransfer(
        {
          from_account_id: from,
          to_account_id: to,
          amount: amount.trim(),
          fee_amount: optional(fee),
          reference: optional(reference),
          note: optional(note),
        },
        key,
      ),
    onSuccess: async () => {
      await refresh()
      onClose()
    },
  })

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    const errs: Record<string, string> = {}
    const value = tryParse(amount)
    if (value === null || value <= 0n) errs.amount = t('money.errors.amount')
    if (fee.trim() && (tryParse(fee) ?? -1n) < 0n) errs.fee_amount = t('money.errors.fee')
    if (from && from === to) errs.to_account_id = t('money.errors.sameAccount')
    setLocalErrors(errs)
    if (Object.keys(errs).length === 0) transfer.mutate()
  }

  const errors = { ...fieldErrors(transfer.error, t), ...localErrors }
  const option = (a: PaymentAccount) => ({
    value: a.id,
    label: showBalances ? `${a.name} (${formatMoney(a.current_balance, i18n.language)})` : a.name,
  })
  return (
    <Dialog open onClose={onClose} title={t('money.transfer')}>
      <form onSubmit={onSubmit} noValidate className="flex flex-col gap-4">
        {transfer.isError && hasFormError(transfer.error) && <Alert tone="danger">{problemMessage(transfer.error, t)}</Alert>}
        <div className="grid gap-4 sm:grid-cols-2">
          <Select label={t('money.fromAccount')} value={from} onChange={(e) => setFrom(e.target.value)} error={errors.from_account_id} options={active.map(option)} />
          <Select label={t('money.toAccount')} value={to} onChange={(e) => setTo(e.target.value)} error={errors.to_account_id} options={active.map(option)} />
          <TextField label={t('common.amount')} value={amount} onChange={(e) => setAmount(e.target.value)} inputMode="decimal" required error={errors.amount} />
          <TextField
            label={`${t('money.fee')} (${t('common.optional')})`}
            value={fee}
            onChange={(e) => setFee(e.target.value)}
            inputMode="decimal"
            hint={t('money.feeHint')}
            error={errors.fee_amount}
          />
        </div>
        <TextField label={`${t('money.reference')} (${t('common.optional')})`} value={reference} onChange={(e) => setReference(e.target.value)} maxLength={100} error={errors.reference} />
        <TextField label={`${t('common.note')} (${t('common.optional')})`} value={note} onChange={(e) => setNote(e.target.value)} maxLength={500} error={errors.note} />
        <div className="flex flex-wrap justify-end gap-2">
          <Button variant="ghost" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button type="submit" disabled={transfer.isPending || active.length < 2}>
            {transfer.isPending ? t('common.saving') : t('money.transfer')}
          </Button>
        </div>
      </form>
    </Dialog>
  )
}
