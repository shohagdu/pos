import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { usePaymentMethods } from '../../api/money'
import { type GoodsReceiptInput, type PurchaseDetail, purchasingApi, purchasingKeys } from '../../api/purchasing'
import { newKey } from '../../api/query'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Checkbox } from '../../components/ui/Checkbox'
import { Dialog } from '../../components/ui/Dialog'
import { TextField } from '../../components/ui/TextField'
import { today } from '../../lib/dates'
import { dec, div, mul, round, sum, toFixed, tryParse } from '../../lib/decimal'
import { formatMoney, formatQuantity } from '../../lib/format'
import { optional } from '../../lib/form'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { type PaymentValues, checkPayment, toPayInput, trimDecimal, useRefreshAfterMoney } from './helpers'
import { PaymentFields } from './shared'

const INPUT =
  'min-h-10 w-full max-w-28 rounded-lg border border-border bg-surface px-2 text-right text-sm outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100 aria-invalid:border-danger'

const remaining = (i: PurchaseDetail['items'][number]) => dec(i.quantity) - dec(i.received_quantity)

/**
 * One delivery against an approved order (PUR-R06): each line starts at what
 * is still to come. Only lines with something arriving are sent. Mount only
 * while open.
 */
export function ReceiveGoodsDialog({ tenantId, purchase, onClose }: { tenantId: string; purchase: PurchaseDetail; onClose: () => void }) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { can } = useSession()
  const queryClient = useQueryClient()
  const refresh = useRefreshAfterMoney(tenantId)
  const [key] = useState(newKey)
  const open = purchase.items.filter((i) => remaining(i) > 0n)
  const [qty, setQty] = useState<Record<string, string>>(() =>
    Object.fromEntries(open.map((i) => [i.id, trimDecimal(toFixed(remaining(i), 3))])),
  )
  const [date, setDate] = useState(today())
  const [note, setNote] = useState('')
  const [withPayment, setWithPayment] = useState(false)
  const methods = usePaymentMethods(tenantId).data ?? []
  const [payment, setPayment] = useState<PaymentValues>({ method_id: '', amount: '', account_id: '', reference: '' })
  const [clientErrors, setClientErrors] = useState<Record<string, string>>({})

  // The server books the share of each line arriving; this preview matches it to the paisa except for remainders.
  const value = sum(
    open.map((i) => {
      const q = tryParse(qty[i.id] ?? '') ?? 0n
      if (q <= 0n) return 0n
      return round(div(mul(dec(i.line_total) + dec(i.charges_alloc), q), dec(i.quantity)), 2)
    }),
  )

  const receive = useMutation({
    mutationFn: () => {
      const all = open.every((i) => tryParse(qty[i.id] ?? '') === remaining(i))
      const input: GoodsReceiptInput = {
        // Everything still to come: let the server take the exact remainders.
        lines: all
          ? undefined
          : open.filter((i) => (tryParse(qty[i.id] ?? '') ?? 0n) > 0n).map((i) => ({ purchase_item_id: i.id, quantity: (qty[i.id] ?? '').trim() })),
        business_date: date && date !== today() ? date : undefined,
        note: optional(note),
        payment: withPayment ? toPayInput(payment) : undefined,
      }
      return purchasingApi.receivePurchase(purchase.id, input, key)
    },
    onSuccess: async (saved) => {
      queryClient.setQueryData(purchasingKeys.purchase(tenantId, saved.id), saved)
      await refresh()
      onClose()
    },
  })

  const server = fieldErrors(receive.error, t)
  // Line errors from the server index the lines sent, not the rows shown.
  const lineError = Object.entries(server).find(([field]) => field.startsWith('lines'))?.[1]
  const errors: Record<string, string | undefined> = {
    lines: lineError,
    method_id: server['payment.method_id'],
    amount: server['payment.amount'],
    account_id: server['payment.account_id'],
    reference: server['payment.reference'],
    ...clientErrors,
  }

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    const found: Record<string, string> = {}
    for (const i of open) {
      const q = tryParse(qty[i.id] ?? '')
      if (q === null || q < 0n || q > remaining(i)) found[`qty.${i.id}`] = t('purchases.errors.receiveQty')
    }
    if (Object.keys(found).length === 0 && value === 0n && open.every((i) => (tryParse(qty[i.id] ?? '') ?? 0n) === 0n)) {
      found.lines = t('purchases.errors.nothingArriving')
    }
    if (withPayment) {
      const method = methods.find((m) => m.id === payment.method_id)
      // Payable now: what is already due plus this delivery.
      Object.assign(found, checkPayment(payment, toFixed(dec(purchase.due_total) + value, 2), t, Boolean(method?.requires_reference)))
    }
    setClientErrors(found)
    if (Object.keys(found).length === 0) receive.mutate()
  }

  return (
    <Dialog
      open
      wide
      onClose={onClose}
      title={t('purchases.receive.title')}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button type="submit" form="receive-goods" disabled={receive.isPending}>
            {receive.isPending ? t('common.saving') : t('purchases.receive.submit')}
          </Button>
        </>
      }
    >
      <form id="receive-goods" onSubmit={onSubmit} noValidate className="flex flex-col gap-4">
        <p className="text-sm text-ink-muted">{t('purchases.receive.hint')}</p>
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <caption className="sr-only">{t('purchases.receive.title')}</caption>
            <thead className="border-y border-border bg-canvas text-xs text-ink-muted uppercase">
              <tr>
                <th scope="col" className="px-3 py-2 text-left">{t('purchases.product')}</th>
                <th scope="col" className="px-3 py-2 text-right">{t('purchases.receive.ordered')}</th>
                <th scope="col" className="px-3 py-2 text-right">{t('purchases.receive.remaining')}</th>
                <th scope="col" className="px-3 py-2 text-left">{t('purchases.receive.arriving')}</th>
              </tr>
            </thead>
            <tbody>
              {open.map((i) => {
                return (
                  <tr key={i.id} className="border-b border-border align-top last:border-b-0">
                    <td className="px-3 py-2">
                      <div className="font-medium">{i.product_name}</div>
                      <div className="font-mono text-xs text-ink-muted">{i.sku}</div>
                    </td>
                    <td className="px-3 py-2 text-right tabular-nums">{formatQuantity(i.quantity, lang, true)}</td>
                    <td className="px-3 py-2 text-right tabular-nums">
                      {formatQuantity(toFixed(remaining(i), 3), lang, true)} {i.unit_short_name}
                    </td>
                    <td className="px-3 py-2">
                      <input
                        aria-label={t('purchases.receive.arrivingFor', { name: i.product_name })}
                        aria-invalid={errors[`qty.${i.id}`] ? true : undefined}
                        className={INPUT}
                        inputMode="decimal"
                        value={qty[i.id] ?? ''}
                        onChange={(e) => setQty((q) => ({ ...q, [i.id]: e.target.value }))}
                      />
                      {errors[`qty.${i.id}`] && <p className="text-sm text-danger">{errors[`qty.${i.id}`]}</p>}
                    </td>
                  </tr>
                )
              })}
            </tbody>
          </table>
        </div>
        {errors.lines && <p className="text-sm text-danger">{errors.lines}</p>}
        <p className="text-sm font-medium">{t('purchases.receive.value', { value: formatMoney(toFixed(value, 2), lang) })}</p>
        <div className="grid gap-4 sm:grid-cols-2">
          <TextField
            label={t('purchases.receive.date')}
            type="date"
            value={date}
            min={purchase.purchase_date}
            max={today()}
            onChange={(e) => setDate(e.target.value)}
            error={server.business_date}
          />
          <TextField label={`${t('purchases.receive.note')} (${t('common.optional')})`} value={note} maxLength={500} onChange={(e) => setNote(e.target.value)} />
        </div>
        {can('suppliers.pay') && <Checkbox label={t('purchases.receive.withPayment')} checked={withPayment} onChange={(e) => setWithPayment(e.target.checked)} />}
        {withPayment && <PaymentFields tenantId={tenantId} value={payment} onChange={setPayment} errors={errors} />}
        {receive.isError && hasFormError(receive.error) && <Alert tone="danger">{problemMessage(receive.error, t)}</Alert>}
      </form>
    </Dialog>
  )
}
