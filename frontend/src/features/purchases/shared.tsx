import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { moneyApi, usePaymentAccounts, usePaymentMethods } from '../../api/money'
import { type Purchase, type PurchaseDetail, type PurchaseStatus, purchasingApi, purchasingKeys } from '../../api/purchasing'
import { newKey } from '../../api/query'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { Checkbox } from '../../components/ui/Checkbox'
import { Dialog } from '../../components/ui/Dialog'
import { Select } from '../../components/ui/Select'
import { TextField } from '../../components/ui/TextField'
import { formatMoney } from '../../lib/format'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { type PaymentValues, checkPayment, toPayInput, trimDecimal, useRefreshAfterMoney } from './helpers'

export function StatusBadge({ status }: { status: PurchaseStatus }) {
  const { t } = useTranslation()
  const tone =
    status === 'received' ? 'good' : status === 'cancelled' ? 'bad' : status === 'partially_received' ? 'warn' : status === 'draft' ? 'neutral' : 'info'
  return <Badge tone={tone}>{t(`purchases.statuses.${status}`)}</Badge>
}

export function PaymentBadge({ status }: { status: Purchase['payment_status'] }) {
  const { t } = useTranslation()
  const tone = status === 'paid' ? 'good' : status === 'partial' ? 'warn' : status === 'refunded' ? 'info' : 'bad'
  return <Badge tone={tone}>{t(`purchases.paymentStatuses.${status}`)}</Badge>
}

/** Method, amount, account and reference (used when posting a bill and paying a supplier). */
export function PaymentFields({
  tenantId,
  value,
  onChange,
  errors,
}: {
  tenantId: string
  value: PaymentValues
  onChange: (value: PaymentValues) => void
  errors: Record<string, string | undefined>
}) {
  const { t } = useTranslation()
  const methods = (usePaymentMethods(tenantId).data ?? []).filter((m) => m.is_active && m.method_type !== 'customer_credit')
  const accounts = (usePaymentAccounts(tenantId).data ?? []).filter((a) => a.status === 'active')
  const method = methods.find((m) => m.id === value.method_id)
  return (
    <div className="grid gap-4 sm:grid-cols-2">
      <Select
        label={t('purchases.pay.method')}
        value={value.method_id}
        onChange={(e) => {
          const next = methods.find((m) => m.id === e.target.value)
          onChange({ ...value, method_id: e.target.value, account_id: next?.default_account_id ?? '' })
        }}
        options={[{ value: '', label: t('purchases.pay.chooseMethod') }, ...methods.map((m) => ({ value: m.id, label: m.name }))]}
        error={errors.method_id}
      />
      <TextField
        label={t('common.amount')}
        inputMode="decimal"
        value={value.amount}
        onChange={(e) => onChange({ ...value, amount: e.target.value })}
        error={errors.amount}
      />
      <Select
        label={t('purchases.pay.account')}
        value={value.account_id}
        onChange={(e) => onChange({ ...value, account_id: e.target.value })}
        options={[{ value: '', label: t('purchases.pay.defaultAccount') }, ...accounts.map((a) => ({ value: a.id, label: a.name }))]}
        error={errors.account_id}
      />
      <TextField
        label={method?.requires_reference ? t('purchases.pay.reference') : `${t('purchases.pay.reference')} (${t('common.optional')})`}
        value={value.reference}
        onChange={(e) => onChange({ ...value, reference: e.target.value })}
        error={errors.reference}
        maxLength={100}
      />
    </div>
  )
}

/**
 * Pay a supplier. With `purchaseId` the whole amount goes to that bill;
 * otherwise the server allocates oldest bill first and keeps the rest as an advance.
 * Mount it only while open so each opening gets a fresh Idempotency-Key.
 */
export function PaySupplierDialog({
  tenantId,
  supplierId,
  purchaseId,
  defaultAmount,
  maxAmount,
  onClose,
}: {
  tenantId: string
  supplierId: string
  purchaseId?: string
  defaultAmount: string
  maxAmount?: string
  onClose: () => void
}) {
  const { t, i18n } = useTranslation()
  const [key] = useState(newKey)
  const methods = usePaymentMethods(tenantId).data ?? []
  const [values, setValues] = useState<PaymentValues>({ method_id: '', amount: trimDecimal(defaultAmount), account_id: '', reference: '' })
  const [clientErrors, setClientErrors] = useState<Record<string, string>>({})
  const refresh = useRefreshAfterMoney(tenantId)
  const pay = useMutation({
    mutationFn: () => {
      const input = toPayInput(values)
      return purchasingApi.paySupplier(
        {
          supplier_id: supplierId,
          ...input,
          allocations: purchaseId ? [{ purchase_id: purchaseId, amount: input.amount }] : undefined,
        },
        key,
      )
    },
    onSuccess: async () => {
      await refresh()
      onClose()
    },
  })
  const server = fieldErrors(pay.error, t)
  const errors = { ...server, amount: server.amount ?? server['allocations[0].amount'] ?? server.allocations, ...clientErrors }

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    const method = methods.find((m) => m.id === values.method_id)
    const found = checkPayment(values, maxAmount ?? null, t, Boolean(method?.requires_reference))
    setClientErrors(found)
    if (Object.keys(found).length === 0) pay.mutate()
  }

  return (
    <Dialog
      open
      onClose={onClose}
      title={t('purchases.pay.title')}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button type="submit" form="pay-supplier" disabled={pay.isPending}>
            {pay.isPending ? t('common.saving') : t('purchases.pay.submit')}
          </Button>
        </>
      }
    >
      <form id="pay-supplier" onSubmit={onSubmit} noValidate className="flex flex-col gap-4">
        <p className="text-sm text-ink-muted">
          {purchaseId ? t('purchases.pay.toBill') : t('purchases.pay.fifo')}
          {maxAmount && ` ${t('purchases.pay.due', { amount: formatMoney(maxAmount, i18n.language) })}`}
        </p>
        <PaymentFields tenantId={tenantId} value={values} onChange={setValues} errors={errors} />
        {pay.isError && hasFormError(pay.error) && <Alert tone="danger">{problemMessage(pay.error, t)}</Alert>}
      </form>
    </Dialog>
  )
}

/** Asks for a reason, then runs `onConfirm(reason)` (cancel a draft, void a payment). */
export function ReasonDialog({
  title,
  hint,
  submitLabel,
  pending,
  error,
  onConfirm,
  onClose,
}: {
  title: string
  hint?: string
  submitLabel: string
  pending: boolean
  error: unknown
  onConfirm: (reason: string) => void
  onClose: () => void
}) {
  const { t } = useTranslation()
  const [reason, setReason] = useState('')
  const [missing, setMissing] = useState(false)
  function onSubmit(e: FormEvent) {
    e.preventDefault()
    if (!reason.trim()) {
      setMissing(true)
      return
    }
    onConfirm(reason.trim())
  }
  const errors = fieldErrors(error, t)
  return (
    <Dialog
      open
      onClose={onClose}
      title={title}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button type="submit" form="reason-dialog" className="bg-danger hover:bg-danger/90" disabled={pending}>
            {pending ? t('common.saving') : submitLabel}
          </Button>
        </>
      }
    >
      <form id="reason-dialog" onSubmit={onSubmit} noValidate className="flex flex-col gap-4">
        {hint && <p className="text-sm text-ink-muted">{hint}</p>}
        <TextField
          label={t('common.reason')}
          value={reason}
          onChange={(e) => setReason(e.target.value)}
          maxLength={255}
          error={missing ? t('purchases.errors.reasonRequired') : errors.reason}
        />
        {Boolean(error) && hasFormError(error) && <Alert tone="danger">{problemMessage(error, t)}</Alert>}
      </form>
    </Dialog>
  )
}

/** Void a posted supplier payment (payments.void). */
export function VoidPaymentDialog({ tenantId, paymentId, onClose }: { tenantId: string; paymentId: string; onClose: () => void }) {
  const { t } = useTranslation()
  const refresh = useRefreshAfterMoney(tenantId)
  const voidIt = useMutation({
    mutationFn: (reason: string) => moneyApi.voidPayment(paymentId, reason),
    onSuccess: async () => {
      await refresh()
      onClose()
    },
  })
  return (
    <ReasonDialog
      title={t('purchases.voidPayment.title')}
      hint={t('purchases.voidPayment.hint')}
      submitLabel={t('purchases.voidPayment.submit')}
      pending={voidIt.isPending}
      error={voidIt.error}
      onConfirm={(reason) => voidIt.mutate(reason)}
      onClose={onClose}
    />
  )
}

/**
 * Post a bill: goods receipt, stock, ledger and an optional payment. `draft`
 * hands over the saved draft (the form saves it first); F8 or Ctrl+Enter
 * confirms, like the POS payment.
 */
export function PostDialog({
  tenantId,
  grandTotal,
  draft,
  onClose,
  onPosted,
}: {
  tenantId: string
  grandTotal: string
  draft: () => Promise<PurchaseDetail>
  onClose: () => void
  onPosted: (posted: PurchaseDetail) => void
}) {
  const { t, i18n } = useTranslation()
  const { can } = useSession()
  const queryClient = useQueryClient()
  const [key] = useState(newKey)
  const canPay = can('suppliers.pay')
  const methods = usePaymentMethods(tenantId).data ?? []
  const [withPayment, setWithPayment] = useState(false)
  const [values, setValues] = useState<PaymentValues>({ method_id: '', amount: trimDecimal(grandTotal), account_id: '', reference: '' })
  const [clientErrors, setClientErrors] = useState<Record<string, string>>({})
  const formRef = useRef<HTMLFormElement>(null)
  const refresh = useRefreshAfterMoney(tenantId)
  const post = useMutation({
    mutationFn: async () => {
      const saved = await draft()
      return purchasingApi.postPurchase(saved.id, withPayment ? toPayInput(values) : undefined, key)
    },
    onSuccess: async (saved) => {
      queryClient.setQueryData(purchasingKeys.purchase(tenantId, saved.id), saved)
      await refresh()
      onPosted(saved)
    },
  })
  const server = fieldErrors(post.error, t)
  const errors: Record<string, string | undefined> = {
    method_id: server['payment.method_id'],
    amount: server['payment.amount'],
    account_id: server['payment.account_id'],
    reference: server['payment.reference'],
    ...clientErrors,
  }

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.repeat) return
      if (e.key === 'F8' || (e.key === 'Enter' && (e.ctrlKey || e.metaKey))) {
        e.preventDefault()
        formRef.current?.requestSubmit()
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [])

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    if (post.isPending) return
    if (withPayment) {
      const method = methods.find((m) => m.id === values.method_id)
      const found = checkPayment(values, grandTotal, t, Boolean(method?.requires_reference))
      setClientErrors(found)
      if (Object.keys(found).length > 0) return
    }
    post.mutate()
  }

  return (
    <Dialog
      open
      onClose={onClose}
      title={t('purchases.post.title')}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button type="submit" form="post-purchase" disabled={post.isPending} aria-keyshortcuts="F8 Control+Enter">
            {post.isPending ? t('common.saving') : t('purchases.post.submit')}{' '}
            <kbd aria-hidden="true" className="text-sm opacity-80">
              F8
            </kbd>
          </Button>
        </>
      }
    >
      <form ref={formRef} id="post-purchase" onSubmit={onSubmit} noValidate className="flex flex-col gap-4">
        <div className="flex items-baseline justify-between gap-3 rounded-lg bg-brand-50 px-4 py-3">
          <span className="font-semibold">{t('purchases.grandTotal')}</span>
          <span className="text-2xl font-bold text-brand-700 tabular-nums">{formatMoney(grandTotal, i18n.language)}</span>
        </div>
        <p className="text-sm text-ink-muted">{t('purchases.post.hint', { total: formatMoney(grandTotal, i18n.language) })}</p>
        {canPay && (
          <Checkbox
            label={t('purchases.post.withPayment')}
            checked={withPayment}
            onChange={(e) => setWithPayment(e.target.checked)}
            autoFocus
            data-autofocus
          />
        )}
        {withPayment && <PaymentFields tenantId={tenantId} value={values} onChange={setValues} errors={errors} />}
        {post.isError && hasFormError(post.error) && <Alert tone="danger">{problemMessage(post.error, t)}</Alert>}
      </form>
    </Dialog>
  )
}
