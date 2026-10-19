import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate, useParams } from 'react-router'

import { usePaymentAccounts, usePaymentMethods } from '../../api/money'
import { type PurchaseDetail, purchasingApi, purchasingKeys, usePurchase } from '../../api/purchasing'
import { newKey } from '../../api/query'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { PageHeader } from '../../components/ui/PageHeader'
import { Select } from '../../components/ui/Select'
import { Spinner } from '../../components/ui/Spinner'
import { TextField } from '../../components/ui/TextField'
import { ZERO, dec, sum, toFixed, toPlain, tryParse } from '../../lib/decimal'
import { formatMoney, formatQuantity } from '../../lib/format'
import { optional } from '../../lib/form'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { estimateCredit } from './costing'
import { LINK, useRefreshAfterMoney } from './helpers'

/** `/purchases/:id/return`: send items back to the supplier. */
export function PurchaseReturnPage() {
  const { t } = useTranslation()
  const { id = '' } = useParams()
  const { me } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const purchase = usePurchase(tenantId, id)
  if (purchase.isPending) return <Spinner />
  if (purchase.isError) return <Alert tone="danger">{problemMessage(purchase.error, t)}</Alert>
  if (purchase.data.status !== 'received') {
    return (
      <div className="flex max-w-3xl flex-col gap-4">
        <Alert tone="warning">{t('purchases.return.notReceived')}</Alert>
        <Link to={`/purchases/${id}`} className={LINK}>
          ← {purchase.data.purchase_no ?? t('purchases.draft')}
        </Link>
      </div>
    )
  }
  return <ReturnForm tenantId={tenantId} purchase={purchase.data} />
}

function ReturnForm({ tenantId, purchase }: { tenantId: string; purchase: PurchaseDetail }) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [key] = useState(newKey)
  const methods = (usePaymentMethods(tenantId).data ?? []).filter((m) => m.is_active && m.method_type !== 'customer_credit')
  const accounts = (usePaymentAccounts(tenantId).data ?? []).filter((a) => a.status === 'active')
  const [qty, setQty] = useState<Record<string, string>>({})
  const [reason, setReason] = useState('')
  const [mode, setMode] = useState<'reduce_payable' | 'refund'>('reduce_payable')
  const [methodId, setMethodId] = useState('')
  const [accountId, setAccountId] = useState('')
  const [clientErrors, setClientErrors] = useState<Record<string, string>>({})
  const refresh = useRefreshAfterMoney(tenantId)

  const items = purchase.items.map((i) => {
    const left = dec(i.received_quantity) - dec(i.returned_quantity)
    const entered = tryParse(qty[i.id] ?? '') ?? ZERO
    return { item: i, left: left > ZERO ? left : ZERO, entered, credit: estimateCredit(i.line_total, i.quantity, entered > left ? left : entered) }
  })
  const total = sum(items.map((x) => x.credit))

  const submit = useMutation({
    mutationFn: () =>
      purchasingApi.createReturn(
        {
          purchase_id: purchase.id,
          reason: reason.trim(),
          settlement_mode: mode,
          refund_method_id: mode === 'refund' ? methodId : undefined,
          refund_account_id: mode === 'refund' ? optional(accountId) : undefined,
          lines: items.filter((x) => x.entered > ZERO).map((x) => ({ purchase_item_id: x.item.id, quantity: toPlain(x.entered) })),
        },
        key,
      ),
    onSuccess: async (saved) => {
      queryClient.setQueryData(purchasingKeys.purchaseReturn(tenantId, saved.id), saved)
      await refresh()
      navigate(`/purchases/returns/${saved.id}`, { replace: true })
    },
  })
  const errors: Record<string, string | undefined> = { ...fieldErrors(submit.error, t), ...clientErrors }

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    const found: Record<string, string> = {}
    items.forEach((x, i) => {
      const text = (qty[x.item.id] ?? '').trim()
      if (!text) return
      const v = tryParse(text)
      if (v === null || v < ZERO) found[`lines[${i}].quantity`] = t('purchases.errors.quantityInvalid')
      else if (v > x.left) found[`lines[${i}].quantity`] = t('purchases.return.tooMany', { max: toPlain(x.left) })
    })
    if (!items.some((x) => x.entered > ZERO)) found.lines = t('purchases.return.noLines')
    if (!reason.trim()) found.reason = t('purchases.errors.reasonRequired')
    if (mode === 'refund' && !methodId) found.refund_method_id = t('purchases.errors.methodRequired')
    setClientErrors(found)
    if (Object.keys(found).length === 0) submit.mutate()
  }

  return (
    <div className="flex max-w-5xl flex-col gap-6">
      <Link to={`/purchases/${purchase.id}`} className={`text-sm ${LINK}`}>
        ← {purchase.purchase_no}
      </Link>
      <PageHeader title={t('purchases.return.title')} subtitle={`${purchase.purchase_no ?? ''} · ${purchase.supplier_name}`} />
      <form onSubmit={onSubmit} noValidate className="flex flex-col gap-6">
        <div className="overflow-x-auto rounded-xl border border-border bg-surface">
          <table className="w-full text-sm">
            <caption className="sr-only">{t('purchases.return.lines')}</caption>
            <thead className="border-b border-border bg-canvas text-xs text-ink-muted uppercase">
              <tr>
                <th scope="col" className="px-3 py-2.5 text-left">{t('purchases.product')}</th>
                <th scope="col" className="px-3 py-2.5 text-right">{t('purchases.received')}</th>
                <th scope="col" className="px-3 py-2.5 text-right">{t('purchases.returned')}</th>
                <th scope="col" className="px-3 py-2.5 text-right">{t('purchases.return.returnable')}</th>
                <th scope="col" className="px-3 py-2.5 text-right">{t('purchases.lineTotal')}</th>
                <th scope="col" className="w-32 px-3 py-2.5 text-left">{t('purchases.return.quantity')}</th>
                <th scope="col" className="px-3 py-2.5 text-right">{t('purchases.return.credit')}</th>
              </tr>
            </thead>
            <tbody>
              {items.map((x, i) => {
                const error = errors[`lines[${i}].quantity`]
                return (
                  <tr key={x.item.id} className="border-b border-border align-top last:border-b-0">
                    <td className="px-3 py-2.5">
                      <span className="font-medium">{x.item.product_name}</span>
                      <span className="block font-mono text-xs text-ink-muted">{x.item.sku}</span>
                    </td>
                    <td className="px-3 py-2.5 text-right tabular-nums">{formatQuantity(x.item.received_quantity, lang, true)}</td>
                    <td className="px-3 py-2.5 text-right tabular-nums">{formatQuantity(x.item.returned_quantity, lang, true)}</td>
                    <td className="px-3 py-2.5 text-right tabular-nums">{`${formatQuantity(toPlain(x.left), lang, true)} ${x.item.unit_short_name}`}</td>
                    <td className="px-3 py-2.5 text-right tabular-nums">{formatMoney(x.item.line_total, lang)}</td>
                    <td className="px-3 py-2.5">
                      <input
                        aria-label={`${t('purchases.return.quantity')} ${x.item.product_name}`}
                        inputMode="decimal"
                        disabled={x.left === ZERO}
                        value={qty[x.item.id] ?? ''}
                        onChange={(e) => setQty((q) => ({ ...q, [x.item.id]: e.target.value }))}
                        aria-invalid={error ? true : undefined}
                        className="min-h-11 w-full rounded-lg border border-border bg-surface px-2 text-sm outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100 disabled:bg-canvas aria-invalid:border-danger"
                      />
                      {error && <p className="mt-1 text-xs text-danger">{error}</p>}
                    </td>
                    <td className="px-3 py-2.5 text-right tabular-nums">{formatMoney(toFixed(x.credit, 2), lang)}</td>
                  </tr>
                )
              })}
            </tbody>
            <tfoot className="border-t-2 border-border bg-canvas font-semibold">
              <tr>
                <td colSpan={6} className="px-3 py-2.5 text-right">
                  {t('purchases.return.estimatedCredit')}
                </td>
                <td className="px-3 py-2.5 text-right tabular-nums">{formatMoney(toFixed(total, 2), lang)}</td>
              </tr>
            </tfoot>
          </table>
        </div>
        {errors.lines && <p className="text-sm text-danger">{errors.lines}</p>}
        <Card className="flex flex-col gap-4">
          <TextField label={t('common.reason')} value={reason} onChange={(e) => setReason(e.target.value)} maxLength={255} error={errors.reason} />
          <Select
            label={t('purchases.return.settlement')}
            value={mode}
            onChange={(e) => setMode(e.target.value as typeof mode)}
            options={[
              { value: 'reduce_payable', label: t('purchases.return.modes.reduce_payable') },
              { value: 'refund', label: t('purchases.return.modes.refund') },
            ]}
            error={errors.settlement_mode}
          />
          <p className="text-sm text-ink-muted">{t(`purchases.return.modeHints.${mode}`, { due: formatMoney(purchase.due_total, lang) })}</p>
          {mode === 'refund' && (
            <div className="grid gap-4 sm:grid-cols-2">
              <Select
                label={t('purchases.return.refundMethod')}
                value={methodId}
                onChange={(e) => {
                  setMethodId(e.target.value)
                  setAccountId(methods.find((m) => m.id === e.target.value)?.default_account_id ?? '')
                }}
                options={[{ value: '', label: t('purchases.pay.chooseMethod') }, ...methods.map((m) => ({ value: m.id, label: m.name }))]}
                error={errors.refund_method_id}
              />
              <Select
                label={t('purchases.pay.account')}
                value={accountId}
                onChange={(e) => setAccountId(e.target.value)}
                options={[{ value: '', label: t('purchases.pay.defaultAccount') }, ...accounts.map((a) => ({ value: a.id, label: a.name }))]}
                error={errors.refund_account_id}
              />
            </div>
          )}
        </Card>
        {submit.isError && hasFormError(submit.error) && <Alert tone="danger">{problemMessage(submit.error, t)}</Alert>}
        <div className="flex flex-wrap justify-end gap-2">
          <Link to={`/purchases/${purchase.id}`} className="inline-flex min-h-11 items-center rounded-lg px-4 text-sm font-semibold text-ink-muted hover:bg-canvas">
            {t('common.cancel')}
          </Link>
          <Button type="submit" disabled={submit.isPending}>
            {submit.isPending ? t('common.saving') : t('purchases.return.submit')}
          </Button>
        </div>
      </form>
    </div>
  )
}
