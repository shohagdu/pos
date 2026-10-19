import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { type IncomeCategory, type IncomeInput, type PaymentAccount, moneyApi, moneyKeys } from '../../api/money'
import { newKey } from '../../api/query'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Dialog } from '../../components/ui/Dialog'
import { Select } from '../../components/ui/Select'
import { TextField } from '../../components/ui/TextField'
import { today } from '../../lib/dates'
import { tryParse } from '../../lib/decimal'
import { optional } from '../../lib/form'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'

/** Records other income received (POST /incomes). Mount only while open. */
export function IncomeDialog({
  tenantId,
  categories,
  accounts,
  onClose,
}: {
  tenantId: string
  categories: IncomeCategory[]
  accounts: PaymentAccount[]
  onClose: () => void
}) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const active = categories.filter((c) => c.status === 'active')
  const activeAccounts = accounts.filter((a) => a.status === 'active')
  const [categoryId, setCategoryId] = useState('')
  const [amount, setAmount] = useState('')
  const [accountId, setAccountId] = useState('')
  const [date, setDate] = useState(today())
  const [source, setSource] = useState('')
  const [reference, setReference] = useState('')
  const [note, setNote] = useState('')
  const [localErrors, setLocalErrors] = useState<Record<string, string>>({})
  const [key, setKey] = useState(newKey)

  const save = useMutation({
    mutationFn: () => {
      const input: IncomeInput = {
        category_id: categoryId,
        amount: amount.trim(),
        account_id: accountId || undefined,
        business_date: date && date !== today() ? date : undefined,
        source: optional(source),
        reference: optional(reference),
        note: optional(note),
      }
      return moneyApi.createIncome(input, key)
    },
    onSuccess: async () => {
      setKey(newKey())
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: moneyKeys.all(tenantId) }),
        queryClient.invalidateQueries({ queryKey: ['reports', tenantId] }),
      ])
      onClose()
    },
  })

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    const errs: Record<string, string> = {}
    const value = tryParse(amount)
    if (!categoryId) errs.category_id = t('income.errors.category')
    if (value === null || value <= 0n) errs.amount = t('income.errors.amount')
    if (date > today()) errs.business_date = t('income.errors.futureDate')
    setLocalErrors(errs)
    if (Object.keys(errs).length === 0) save.mutate()
  }

  const errors = { ...fieldErrors(save.error, t), ...localErrors }
  const defaultAccount = activeAccounts.find((a) => a.is_default && a.account_type === 'cash') ?? activeAccounts.find((a) => a.account_type === 'cash')
  return (
    <Dialog open onClose={onClose} title={t('income.new')} wide>
      <form onSubmit={onSubmit} noValidate className="flex flex-col gap-4">
        {save.isError && hasFormError(save.error) && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}
        <div className="grid gap-4 sm:grid-cols-2">
          <Select
            label={t('income.category')}
            value={categoryId}
            onChange={(e) => setCategoryId(e.target.value)}
            error={errors.category_id}
            options={[{ value: '', label: t('income.chooseCategory') }, ...active.map((c) => ({ value: c.id, label: c.name }))]}
          />
          <TextField label={t('common.date')} type="date" value={date} max={today()} onChange={(e) => setDate(e.target.value)} error={errors.business_date} />
          <TextField label={t('income.amount')} value={amount} onChange={(e) => setAmount(e.target.value)} inputMode="decimal" required error={errors.amount} />
          <Select
            label={t('income.receivedInto')}
            value={accountId}
            onChange={(e) => setAccountId(e.target.value)}
            error={errors.account_id}
            options={[
              { value: '', label: defaultAccount ? t('income.defaultCashNamed', { name: defaultAccount.name }) : t('income.defaultCash') },
              ...activeAccounts.map((a) => ({ value: a.id, label: a.name })),
            ]}
          />
          <TextField label={`${t('income.source')} (${t('common.optional')})`} value={source} onChange={(e) => setSource(e.target.value)} maxLength={200} error={errors.source} />
          <TextField
            label={`${t('income.reference')} (${t('common.optional')})`}
            value={reference}
            onChange={(e) => setReference(e.target.value)}
            maxLength={100}
            error={errors.reference}
          />
          <TextField
            className="sm:col-span-2"
            label={`${t('common.note')} (${t('common.optional')})`}
            value={note}
            onChange={(e) => setNote(e.target.value)}
            maxLength={500}
            error={errors.note}
          />
        </div>
        <div className="flex flex-wrap justify-end gap-2">
          <Button variant="ghost" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button type="submit" disabled={save.isPending}>
            {save.isPending ? t('common.saving') : t('income.save')}
          </Button>
        </div>
      </form>
    </Dialog>
  )
}
