import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { type ExpenseCategory, type ExpenseInput, type PaymentAccount, moneyApi, moneyKeys } from '../../api/money'
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

/** Records a paid expense (POST /expenses). Mount only while open. */
export function ExpenseDialog({
  tenantId,
  categories,
  accounts,
  onClose,
}: {
  tenantId: string
  categories: ExpenseCategory[]
  accounts: PaymentAccount[]
  onClose: () => void
}) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const active = categories.filter((c) => c.status === 'active')
  const activeAccounts = accounts.filter((a) => a.status === 'active')
  const [categoryId, setCategoryId] = useState('')
  const [amount, setAmount] = useState('')
  const [tax, setTax] = useState('')
  const [accountId, setAccountId] = useState('')
  const [date, setDate] = useState(today())
  const [payee, setPayee] = useState('')
  const [reference, setReference] = useState('')
  const [note, setNote] = useState('')
  const [localErrors, setLocalErrors] = useState<Record<string, string>>({})
  const [key, setKey] = useState(newKey)

  const save = useMutation({
    mutationFn: () => {
      const input: ExpenseInput = {
        category_id: categoryId,
        amount: amount.trim(),
        tax_amount: optional(tax),
        account_id: accountId || undefined,
        business_date: date && date !== today() ? date : undefined,
        payee: optional(payee),
        reference: optional(reference),
        note: optional(note),
      }
      return moneyApi.createExpense(input, key)
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
    if (!categoryId) errs.category_id = t('expenses.errors.category')
    if (value === null || value <= 0n) errs.amount = t('expenses.errors.amount')
    const vat = tax.trim() ? tryParse(tax) : 0n
    if (vat === null || vat < 0n) errs.tax_amount = t('expenses.errors.tax')
    else if (value !== null && vat > value) errs.tax_amount = t('expenses.errors.taxTooLarge')
    if (date > today()) errs.business_date = t('expenses.errors.futureDate')
    setLocalErrors(errs)
    if (Object.keys(errs).length === 0) save.mutate()
  }

  const errors = { ...fieldErrors(save.error, t), ...localErrors }
  const defaultAccount = activeAccounts.find((a) => a.is_default && a.account_type === 'cash') ?? activeAccounts.find((a) => a.account_type === 'cash')
  return (
    <Dialog open onClose={onClose} title={t('expenses.new')} wide>
      <form onSubmit={onSubmit} noValidate className="flex flex-col gap-4">
        {save.isError && hasFormError(save.error) && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}
        <div className="grid gap-4 sm:grid-cols-2">
          <Select
            label={t('expenses.category')}
            value={categoryId}
            onChange={(e) => setCategoryId(e.target.value)}
            error={errors.category_id}
            options={[{ value: '', label: t('expenses.chooseCategory') }, ...active.map((c) => ({ value: c.id, label: c.name }))]}
          />
          <TextField label={t('common.date')} type="date" value={date} max={today()} onChange={(e) => setDate(e.target.value)} error={errors.business_date} />
          <TextField
            label={t('expenses.amount')}
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
            inputMode="decimal"
            required
            hint={t('expenses.amountHint')}
            error={errors.amount}
          />
          <TextField label={`${t('expenses.vat')} (${t('common.optional')})`} value={tax} onChange={(e) => setTax(e.target.value)} inputMode="decimal" error={errors.tax_amount} />
          <Select
            label={t('expenses.paidFrom')}
            value={accountId}
            onChange={(e) => setAccountId(e.target.value)}
            error={errors.account_id}
            options={[
              { value: '', label: defaultAccount ? t('expenses.defaultCashNamed', { name: defaultAccount.name }) : t('expenses.defaultCash') },
              ...activeAccounts.map((a) => ({ value: a.id, label: a.name })),
            ]}
          />
          <TextField label={`${t('expenses.payee')} (${t('common.optional')})`} value={payee} onChange={(e) => setPayee(e.target.value)} maxLength={200} error={errors.payee} />
          <TextField
            label={`${t('expenses.reference')} (${t('common.optional')})`}
            value={reference}
            onChange={(e) => setReference(e.target.value)}
            maxLength={100}
            error={errors.reference}
          />
          <TextField label={`${t('common.note')} (${t('common.optional')})`} value={note} onChange={(e) => setNote(e.target.value)} maxLength={500} error={errors.note} />
        </div>
        <div className="flex flex-wrap justify-end gap-2">
          <Button variant="ghost" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button type="submit" disabled={save.isPending}>
            {save.isPending ? t('common.saving') : t('expenses.save')}
          </Button>
        </div>
      </form>
    </Dialog>
  )
}
