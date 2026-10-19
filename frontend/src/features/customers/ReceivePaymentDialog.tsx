import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { PaymentDetail } from '../../api/money'
import { moneyKeys, usePaymentAccounts, usePaymentMethods } from '../../api/money'
import type { Customer } from '../../api/parties'
import { newKey } from '../../api/query'
import { type CollectionInput, salesApi, useSales } from '../../api/sales'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Checkbox } from '../../components/ui/Checkbox'
import { Dialog } from '../../components/ui/Dialog'
import { Select } from '../../components/ui/Select'
import { Spinner } from '../../components/ui/Spinner'
import { TextField } from '../../components/ui/TextField'
import { dec, sum, toFixed, tryParse } from '../../lib/decimal'
import { formatDate } from '../../lib/dates'
import { formatMoney } from '../../lib/format'
import { optional } from '../../lib/form'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { isPositive, trimDecimal } from '../money/utils'

const INPUT =
  'min-h-10 w-full max-w-36 rounded-lg border border-border bg-surface px-2 text-right text-sm outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100 aria-invalid:border-danger'

/** "Receive payment" from a customer (POST /collections). Mount only while open. */
export function ReceivePaymentDialog({ customer, tenantId, onClose }: { customer: Customer; tenantId: string; onClose: () => void }) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const queryClient = useQueryClient()
  const methods = usePaymentMethods(tenantId)
  const accounts = usePaymentAccounts(tenantId)
  const usable = (methods.data ?? []).filter((m) => m.is_active && m.method_type !== 'customer_credit')
  const [amount, setAmount] = useState(isPositive(customer.current_balance) ? trimDecimal(customer.current_balance) : '')
  const [methodId, setMethodId] = useState('')
  const [accountId, setAccountId] = useState('')
  const [reference, setReference] = useState('')
  const [note, setNote] = useState('')
  const [allocate, setAllocate] = useState(false)
  const [shares, setShares] = useState<Record<string, string>>({})
  const [localErrors, setLocalErrors] = useState<Record<string, string>>({})
  const [key] = useState(newKey)
  const [result, setResult] = useState<PaymentDetail | null>(null)

  const method = usable.find((m) => m.id === (methodId || usable[0]?.id))
  const accountOptions = (accounts.data ?? []).filter((a) => a.status === 'active')

  const collect = useMutation({
    mutationFn: () => {
      const allocations = allocate
        ? Object.entries(shares)
            .filter(([, v]) => (tryParse(v) ?? 0n) > 0n)
            .map(([sale_id, v]) => ({ sale_id, amount: v.trim() }))
        : []
      const input: CollectionInput = {
        customer_id: customer.id,
        amount: amount.trim(),
        method_id: method?.id ?? '',
        account_id: accountId || undefined,
        reference: optional(reference),
        note: optional(note),
        allocations: allocations.length > 0 ? allocations : undefined,
      }
      return salesApi.collect(input, key)
    },
    onSuccess: async (payment) => {
      setResult(payment)
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['parties', tenantId] }),
        queryClient.invalidateQueries({ queryKey: ['sales', tenantId] }),
        queryClient.invalidateQueries({ queryKey: moneyKeys.all(tenantId) }),
        queryClient.invalidateQueries({ queryKey: ['reports', tenantId] }),
      ])
    },
  })

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    const errs: Record<string, string> = {}
    const value = tryParse(amount)
    if (value === null || value <= 0n) errs.amount = t('customers.errors.amount')
    if (!method) errs.method_id = t('customers.errors.method')
    if (method?.requires_reference && !reference.trim()) errs.reference = t('customers.errors.reference')
    if (allocate && value !== null) {
      const total = sum(Object.values(shares).map((v) => dec(v)))
      if (total > value) errs.allocations = t('customers.errors.allocationsTooLarge')
    }
    setLocalErrors(errs)
    if (Object.keys(errs).length === 0) collect.mutate()
  }

  const errors = { ...fieldErrors(collect.error, t), ...localErrors }

  if (result) {
    return (
      <Dialog
        open
        onClose={onClose}
        title={t('customers.receivePayment')}
        footer={<Button onClick={onClose}>{t('common.close')}</Button>}
      >
        <div className="flex flex-col gap-2">
          <Alert tone="success">{t('customers.paymentReceived', { no: result.payment_no, amount: formatMoney(result.amount, lang) })}</Alert>
          {isPositive(result.unallocated_amount) && (
            <p className="text-sm">{t('customers.leftAsAdvance', { amount: formatMoney(result.unallocated_amount, lang) })}</p>
          )}
        </div>
      </Dialog>
    )
  }

  return (
    <Dialog open onClose={onClose} title={t('customers.receivePayment')} wide>
      {methods.isPending || accounts.isPending ? (
        <Spinner />
      ) : (
        <form onSubmit={onSubmit} noValidate className="flex flex-col gap-4">
          <p className="text-sm text-ink-muted">
            {customer.name} · {t('customers.currentBalance')}: {formatMoney(customer.current_balance, lang)}
          </p>
          {collect.isError && hasFormError(collect.error) && <Alert tone="danger">{problemMessage(collect.error, t)}</Alert>}
          {methods.isError && <Alert tone="danger">{problemMessage(methods.error, t)}</Alert>}
          <div className="grid gap-4 sm:grid-cols-2">
            <TextField label={t('common.amount')} value={amount} onChange={(e) => setAmount(e.target.value)} inputMode="decimal" required error={errors.amount} />
            <Select
              label={t('customers.method')}
              value={method?.id ?? ''}
              onChange={(e) => {
                setMethodId(e.target.value)
                setAccountId('')
              }}
              error={errors.method_id}
              options={usable.map((m) => ({ value: m.id, label: m.name }))}
            />
            <Select
              label={t('customers.account')}
              value={accountId}
              onChange={(e) => setAccountId(e.target.value)}
              error={errors.account_id}
              options={[
                { value: '', label: method?.default_account_name ? t('customers.methodAccount', { name: method.default_account_name }) : t('customers.defaultAccount') },
                ...accountOptions.map((a) => ({ value: a.id, label: a.name })),
              ]}
            />
            <TextField
              label={method?.requires_reference ? t('customers.reference') : `${t('customers.reference')} (${t('common.optional')})`}
              value={reference}
              onChange={(e) => setReference(e.target.value)}
              maxLength={100}
              hint={method?.requires_reference ? t('customers.referenceHint') : undefined}
              error={errors.reference}
            />
          </div>
          <TextField label={`${t('common.note')} (${t('common.optional')})`} value={note} onChange={(e) => setNote(e.target.value)} maxLength={500} error={errors.note} />
          <Checkbox
            label={t('customers.chooseInvoices')}
            hint={t('customers.chooseInvoicesHint')}
            checked={allocate}
            onChange={(e) => setAllocate(e.target.checked)}
          />
          {allocate && <OpenInvoices tenantId={tenantId} customerId={customer.id} shares={shares} setShares={setShares} errors={errors} />}
          {errors.allocations && <p className="text-sm text-danger">{errors.allocations}</p>}
          <div className="flex flex-wrap justify-end gap-2">
            <Button variant="ghost" onClick={onClose}>
              {t('common.cancel')}
            </Button>
            <Button type="submit" disabled={collect.isPending || usable.length === 0}>
              {collect.isPending ? t('common.saving') : t('customers.receive')}
            </Button>
          </div>
        </form>
      )}
    </Dialog>
  )
}

function OpenInvoices({
  tenantId,
  customerId,
  shares,
  setShares,
  errors,
}: {
  tenantId: string
  customerId: string
  shares: Record<string, string>
  setShares: (fn: (s: Record<string, string>) => Record<string, string>) => void
  errors: Record<string, string>
}) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const sales = useSales(tenantId, { customer_id: customerId, due_only: true })
  const rows = (sales.data?.pages.flatMap((p) => p.data) ?? []).filter((s) => isPositive(s.due_total))
  if (sales.isPending) return <Spinner />
  if (sales.isError) return <Alert tone="danger">{problemMessage(sales.error, t)}</Alert>
  if (rows.length === 0) return <p className="text-sm text-ink-muted">{t('customers.noOpenInvoices')}</p>
  const allocated = sum(Object.values(shares).map((v) => dec(v)))
  // Server errors index the allocations sent: shares above 0 in entry order.
  const sent = Object.entries(shares)
    .filter(([, v]) => (tryParse(v) ?? 0n) > 0n)
    .map(([id]) => id)
  return (
    <div className="overflow-x-auto rounded-lg border border-border">
      <table className="w-full text-sm">
        <caption className="sr-only">{t('customers.openInvoices')}</caption>
        <thead className="bg-canvas text-xs text-ink-muted uppercase">
          <tr>
            <th scope="col" className="px-3 py-2 text-left">{t('customers.invoiceNo')}</th>
            <th scope="col" className="px-3 py-2 text-left">{t('common.date')}</th>
            <th scope="col" className="px-3 py-2 text-right">{t('customers.due')}</th>
            <th scope="col" className="px-3 py-2 text-right">{t('customers.apply')}</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((s) => {
            const err = errors[`allocations[${sent.indexOf(s.id)}].amount`] ?? errors[`allocations[${sent.indexOf(s.id)}].sale_id`]
            return (
              <tr key={s.id} className="border-t border-border">
                <td className="px-3 py-2 font-medium">{s.invoice_no}</td>
                <td className="px-3 py-2">{formatDate(s.business_date, lang)}</td>
                <td className="px-3 py-2 text-right tabular-nums">{formatMoney(s.due_total, lang)}</td>
                <td className="px-3 py-2 text-right">
                  <div className="flex items-center justify-end gap-2">
                    <Button
                      variant="ghost"
                      className="min-h-9 px-2 text-xs"
                      onClick={() => setShares((sh) => ({ ...sh, [s.id]: trimDecimal(s.due_total) }))}
                    >
                      {t('customers.full')}
                    </Button>
                    <input
                      aria-label={t('customers.applyTo', { no: s.invoice_no })}
                      aria-invalid={err ? true : undefined}
                      className={INPUT}
                      inputMode="decimal"
                      value={shares[s.id] ?? ''}
                      onChange={(e) => setShares((sh) => ({ ...sh, [s.id]: e.target.value }))}
                    />
                  </div>
                  {err && <p className="text-sm text-danger">{err}</p>}
                </td>
              </tr>
            )
          })}
        </tbody>
        <tfoot className="border-t-2 border-border bg-canvas font-semibold">
          <tr>
            <td className="px-3 py-2" colSpan={3}>
              {t('customers.allocated')}
            </td>
            <td className="px-3 py-2 text-right tabular-nums">{formatMoney(toFixed(allocated), lang)}</td>
          </tr>
        </tfoot>
      </table>
    </div>
  )
}
