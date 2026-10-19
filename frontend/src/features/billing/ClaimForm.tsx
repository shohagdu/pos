import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { CLAIM_METHODS, type ClaimMethod, type SubscriptionInvoice, billingApi, billingKeys, billingOverviewKey } from '../../api/billing'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Select } from '../../components/ui/Select'
import { TextField } from '../../components/ui/TextField'
import { formatDate } from '../../lib/dates'
import { dec, toFixed, tryParse } from '../../lib/decimal'
import { formatMoney } from '../../lib/format'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'

/**
 * The invoice to pay, how to pay it (platform payment instructions) and the
 * "I have paid" form (SUB-R05). The platform verifies the claim (ADM-R05).
 */
export function ClaimForm({
  tenantId,
  invoice,
  instructions,
  onSubmitted,
  onCancel,
}: {
  tenantId: string
  invoice: SubscriptionInvoice
  instructions: Record<string, string>
  onSubmitted: () => void
  onCancel: () => void
}) {
  const { t, i18n } = useTranslation()
  const queryClient = useQueryClient()
  const [method, setMethod] = useState<ClaimMethod>('bkash')
  const [amount, setAmount] = useState(toFixed(dec(invoice.total), 2))
  const [reference, setReference] = useState('')
  const [payer, setPayer] = useState('')
  const [local, setLocal] = useState<Record<string, string>>({})

  const submit = useMutation({
    mutationFn: () =>
      billingApi.submitClaim(invoice.id, {
        method,
        amount: amount.trim(),
        transaction_ref: reference.trim(),
        ...(payer.trim() ? { payer_account: payer.trim() } : {}),
      }),
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: billingOverviewKey(tenantId) }),
        queryClient.invalidateQueries({ queryKey: billingKeys.subscription(tenantId) }),
      ])
      onSubmitted()
    },
  })

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    const errors: Record<string, string> = {}
    const ref = reference.trim()
    if (ref.length < 4 || ref.length > 60) errors.transaction_ref = t('problems.INVALID_REFERENCE')
    const value = tryParse(amount)
    if (value === null || value <= 0n) errors.amount = t('billing.claim.amountInvalid')
    setLocal(errors)
    if (Object.keys(errors).length === 0) submit.mutate()
  }

  const server = submit.isError ? fieldErrors(submit.error, t) : {}
  const errors = { ...server, ...local }
  const instruction = instructions[method]
  const note = instructions.note

  return (
    <div className="flex flex-col gap-4">
      <dl className="grid grid-cols-[auto_1fr] gap-x-6 gap-y-1 rounded-lg bg-canvas p-3 text-sm">
        <dt className="text-ink-muted">{t('billing.invoice.number')}</dt>
        <dd className="font-mono">{invoice.invoice_no}</dd>
        <dt className="text-ink-muted">{t('billing.invoice.plan')}</dt>
        <dd>{t(`plan.names.${invoice.plan_code}`, { defaultValue: invoice.plan_name })}</dd>
        <dt className="text-ink-muted">{t('billing.invoice.cycle')}</dt>
        <dd>{t(`billing.cycles.${invoice.billing_cycle}`)}</dd>
        <dt className="text-ink-muted">{t('billing.invoice.period')}</dt>
        <dd>
          {formatDate(invoice.period_start, i18n.language)} → {formatDate(invoice.period_end, i18n.language)}
        </dd>
        <dt className="text-ink-muted">{t('billing.invoice.total')}</dt>
        <dd className="font-semibold tabular-nums">{formatMoney(invoice.total, i18n.language)}</dd>
      </dl>

      <form onSubmit={onSubmit} noValidate className="flex flex-col gap-4">
        <Select
          label={t('billing.claim.method')}
          value={method}
          onChange={(e) => setMethod(e.target.value as ClaimMethod)}
          options={CLAIM_METHODS.map((m) => ({ value: m, label: t(`billing.methods.${m}`) }))}
          error={errors.method}
        />
        {(instruction || note) && (
          <div className="flex flex-col gap-1 rounded-lg border border-border p-3 text-sm">
            <h3 className="font-semibold">{t('billing.claim.howToPay', { method: t(`billing.methods.${method}`) })}</h3>
            {instruction && <p className="whitespace-pre-line">{instruction}</p>}
            {note && <p className="whitespace-pre-line text-ink-muted">{note}</p>}
          </div>
        )}
        <div className="grid gap-4 sm:grid-cols-2">
          <TextField
            label={t('billing.claim.amount')}
            inputMode="decimal"
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
            error={errors.amount}
            required
          />
          <TextField
            label={t('billing.claim.reference')}
            value={reference}
            onChange={(e) => setReference(e.target.value)}
            error={errors.transaction_ref}
            hint={t('billing.claim.referenceHint')}
            maxLength={60}
            autoComplete="off"
            required
          />
          <TextField
            label={`${t('billing.claim.payer')} (${t('common.optional')})`}
            value={payer}
            onChange={(e) => setPayer(e.target.value)}
            hint={t('billing.claim.payerHint')}
            maxLength={60}
            error={errors.payer_account}
          />
        </div>
        {submit.isError && hasFormError(submit.error) && <Alert tone="danger">{problemMessage(submit.error, t)}</Alert>}
        <div className="flex flex-wrap gap-2">
          <Button type="submit" disabled={submit.isPending}>
            {submit.isPending ? t('common.saving') : t('billing.claim.submit')}
          </Button>
          <Button variant="ghost" onClick={onCancel}>
            {t('common.cancel')}
          </Button>
        </div>
      </form>
    </div>
  )
}
