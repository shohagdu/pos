import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate, useParams } from 'react-router'

import { newKey } from '../../api/query'
import { type RefundMode, type ReturnInput, type SaleDetail, salesApi, useSale } from '../../api/sales'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { PageHeader } from '../../components/ui/PageHeader'
import { Select } from '../../components/ui/Select'
import { Spinner } from '../../components/ui/Spinner'
import { TextField } from '../../components/ui/TextField'
import { ZERO, dec, max, min, sum, toFixed, toPlain } from '../../lib/decimal'
import { formatMoney, formatQuantity } from '../../lib/format'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { REFUND_MODES } from './constants'
import { checkQty, estimatedRefund, returnable } from './returnMath'

type Condition = 'restock' | 'damaged'
const inputClass = 'min-h-10 rounded-lg border border-border bg-surface px-2 text-sm outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100 aria-invalid:border-danger'

/** Return items of a sale (E5.4): quantities, condition, reason and refund mode. */
export function SaleReturnPage() {
  const { id = '' } = useParams()
  const { t } = useTranslation()
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const sale = useSale(tenantId, id)

  if (!can('sales.return')) return <Alert tone="danger">{t('problems.PERMISSION_DENIED')}</Alert>
  if (sale.isPending) return <Spinner />
  if (sale.isError) return <Alert tone="danger">{problemMessage(sale.error, t)}</Alert>
  return <ReturnForm sale={sale.data} tenantId={tenantId} />
}

function ReturnForm({ sale, tenantId }: { sale: SaleDetail; tenantId: string }) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const money = (v: string) => formatMoney(v, lang)
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  // One key per opened form: a retry after a network error is not posted twice.
  const [idempotencyKey] = useState(newKey)
  const [qty, setQty] = useState<Record<string, string>>({})
  const [condition, setCondition] = useState<Record<string, Condition>>({})
  const [reason, setReason] = useState('')
  const [refundMode, setRefundMode] = useState<RefundMode>('cash')
  const [touched, setTouched] = useState(false)
  const [submitted, setSubmitted] = useState<string[]>([])

  const items = sale.items.filter((i) => returnable(i) > ZERO)
  const checks = items.map((item) => ({ item, check: checkQty(qty[item.id] ?? '', item) }))
  const chosen = checks.flatMap(({ item, check }) => (check.ok && check.qty > ZERO ? [{ item, qty: check.qty }] : []))
  const invalid = checks.some(({ check }) => !check.ok)
  const total = sum(chosen.map((c) => estimatedRefund(c.item, c.qty)))
  const toDue = min(total, max(dec(sale.due_total), ZERO))
  const rest = total - toDue
  const modes = REFUND_MODES.filter((m) => m !== 'customer_credit' || !sale.is_walk_in)
  const returnableStatus = sale.status === 'completed' || sale.status === 'partially_returned'

  const create = useMutation({
    mutationFn: (input: ReturnInput) => salesApi.createReturn(input, idempotencyKey),
    onSuccess: (ret) => {
      for (const key of ['sales', 'inventory', 'parties', 'money', 'reports']) void queryClient.invalidateQueries({ queryKey: [key, tenantId] })
      void navigate(`/sales/returns/${ret.id}`)
    },
  })
  const serverErrors = fieldErrors(create.error, t)
  const lineError = (itemId: string) => {
    const index = submitted.indexOf(itemId)
    return index >= 0 ? (serverErrors[`lines[${index}].quantity`] ?? serverErrors[`lines[${index}].sale_item_id`]) : undefined
  }

  function submit(e: FormEvent) {
    e.preventDefault()
    setTouched(true)
    if (invalid || chosen.length === 0 || !reason.trim()) return
    const lines = chosen.map((c) => ({ sale_item_id: c.item.id, quantity: toPlain(c.qty), condition: condition[c.item.id] ?? 'restock' }))
    setSubmitted(lines.map((l) => l.sale_item_id))
    create.mutate({ sale_id: sale.id, reason: reason.trim(), refund_mode: refundMode, lines })
  }

  return (
    <form onSubmit={submit} className="flex max-w-5xl flex-col gap-6" noValidate>
      <Link to={`/sales/${sale.id}`} className="text-sm font-medium text-brand-700 hover:underline">
        ← {t('sales.invoiceTitle', { no: sale.invoice_no })}
      </Link>
      <PageHeader
        title={t('sales.returnTitle')}
        subtitle={`${sale.invoice_no} · ${sale.is_walk_in ? t('sales.walkIn') : sale.customer_name}`}
      />
      {!returnableStatus || items.length === 0 ? (
        <Alert tone="warning">{t('sales.nothingToReturn')}</Alert>
      ) : (
        <>
          <div className="flex justify-end">
            <Button
              variant="secondary"
              onClick={() => setQty(Object.fromEntries(items.map((i) => [i.id, toPlain(returnable(i))])))}
            >
              {t('sales.returnAll')}
            </Button>
          </div>
          <div className="overflow-x-auto rounded-xl border border-border bg-surface">
            <table className="w-full text-sm">
              <caption className="sr-only">{t('sales.items')}</caption>
              <thead className="border-b border-border bg-canvas text-xs text-ink-muted uppercase">
                <tr>
                  <th scope="col" className="px-3 py-2.5 text-left">{t('sales.product')}</th>
                  <th scope="col" className="px-3 py-2.5 text-right">{t('sales.sold')}</th>
                  <th scope="col" className="px-3 py-2.5 text-right">{t('sales.returnable')}</th>
                  <th scope="col" className="px-3 py-2.5 text-left">{t('sales.returnQty')}</th>
                  <th scope="col" className="px-3 py-2.5 text-left">{t('sales.condition')}</th>
                  <th scope="col" className="px-3 py-2.5 text-right">{t('sales.estimatedRefund')}</th>
                </tr>
              </thead>
              <tbody>
                {checks.map(({ item, check }) => {
                  const left = returnable(item)
                  const error = !check.ok ? t(`sales.qtyErrors.${check.error}`, { max: toPlain(left) }) : lineError(item.id)
                  return (
                    <tr key={item.id} className="border-b border-border last:border-b-0">
                      <td className="px-3 py-2.5">
                        <span className="font-medium">{item.product_name}</span>
                        <span className="block text-xs text-ink-muted">
                          {money(item.line_total)} / {formatQuantity(item.quantity, lang, true)} {item.unit_short_name}
                        </span>
                      </td>
                      <td className="px-3 py-2.5 text-right tabular-nums">{formatQuantity(item.quantity, lang, true)}</td>
                      <td className="px-3 py-2.5 text-right tabular-nums">{formatQuantity(toPlain(left), lang, true)}</td>
                      <td className="px-3 py-2.5">
                        <input
                          className={`${inputClass} w-24`}
                          inputMode="decimal"
                          aria-label={t('sales.returnQtyFor', { name: item.product_name })}
                          aria-invalid={error ? true : undefined}
                          max={toPlain(left)}
                          min="0"
                          value={qty[item.id] ?? ''}
                          placeholder="0"
                          onChange={(e) => setQty((q) => ({ ...q, [item.id]: e.target.value }))}
                        />
                        {error && <p className="mt-1 text-xs text-danger">{error}</p>}
                      </td>
                      <td className="px-3 py-2.5">
                        <select
                          className={inputClass}
                          aria-label={t('sales.conditionFor', { name: item.product_name })}
                          value={condition[item.id] ?? 'restock'}
                          onChange={(e) => setCondition((c) => ({ ...c, [item.id]: e.target.value as Condition }))}
                        >
                          <option value="restock">{t('sales.conditions.restock')}</option>
                          <option value="damaged">{t('sales.conditions.damaged')}</option>
                        </select>
                      </td>
                      <td className="px-3 py-2.5 text-right tabular-nums">
                        {money(toFixed(check.ok ? estimatedRefund(item, check.qty) : ZERO, 2))}
                      </td>
                    </tr>
                  )
                })}
              </tbody>
            </table>
          </div>
          {touched && chosen.length === 0 && !invalid && <Alert tone="warning">{t('sales.chooseQty')}</Alert>}

          <div className="grid gap-6 md:grid-cols-[1fr_20rem]">
            <div className="flex flex-col gap-4">
              <TextField
                label={t('common.reason')}
                value={reason}
                maxLength={500}
                onChange={(e) => setReason(e.target.value)}
                error={touched && !reason.trim() ? t('sales.reasonRequired') : serverErrors.reason}
              />
              <Select
                label={t('sales.refundMode')}
                value={refundMode}
                onChange={(e) => setRefundMode(e.target.value as RefundMode)}
                options={modes.map((m) => ({ value: m, label: t(`sales.refundModes.${m}`) }))}
                error={serverErrors.refund_mode}
              />
            </div>
            <Card className="flex flex-col gap-1 self-start p-4 text-sm">
              <div className="flex justify-between gap-3 text-base font-bold">
                <span>{t('sales.estimatedTotal')}</span>
                <span className="tabular-nums">{money(toFixed(total, 2))}</span>
              </div>
              {toDue > ZERO && (
                <div className="flex justify-between gap-3">
                  <span>{t('sales.receipt.appliedToDue')}</span>
                  <span className="tabular-nums">{money(toFixed(toDue, 2))}</span>
                </div>
              )}
              {rest > ZERO && (
                <div className="flex justify-between gap-3">
                  <span>{refundMode === 'customer_credit' ? t('sales.receipt.credited') : t('sales.receipt.refunded')}</span>
                  <span className="tabular-nums">{money(toFixed(rest, 2))}</span>
                </div>
              )}
              <p className="mt-1 text-xs text-ink-muted">{t('sales.estimateHint')}</p>
            </Card>
          </div>
          {create.isError && hasFormError(create.error) && <Alert tone="danger">{problemMessage(create.error, t)}</Alert>}
          <div className="flex flex-wrap gap-2">
            <Button type="submit" disabled={create.isPending}>
              {create.isPending ? t('common.saving') : t('sales.submitReturn')}
            </Button>
            <Link to={`/sales/${sale.id}`} className="inline-flex min-h-11 items-center rounded-lg px-4 text-sm font-semibold text-ink-muted hover:bg-canvas">
              {t('common.cancel')}
            </Link>
          </div>
        </>
      )}
    </form>
  )
}
