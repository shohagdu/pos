import { type FormEvent, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { PaymentMethod } from '../../api/money'
import type { SalePaymentInput } from '../../api/sales'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Dialog } from '../../components/ui/Dialog'
import { TextField } from '../../components/ui/TextField'
import { type Dec, ZERO, dec, max, sum, toFixed, toPlain, tryParse } from '../../lib/decimal'
import { formatMoney } from '../../lib/format'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { type CartCustomer, ceilTo } from './cart'

interface PayLine {
  id: number
  methodId: string
  amount: string
  reference: string
}

const NOTE_STEPS = ['10', '50', '100', '500', '1000']
const select =
  'min-h-11 rounded-lg border border-border bg-surface px-2 text-sm outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100'

/**
 * Pay (F8): split tenders, change, due; confirm posts the sale. Enter in a
 * field, F8 again or Ctrl+Enter confirms from anywhere in the dialog. Tab
 * order is amount(s) → Confirm, then the rest of the dialog.
 */
export function PaymentDialog({
  methods,
  grandTotal,
  customer,
  pending,
  error,
  onClose,
  onConfirm,
}: {
  methods: PaymentMethod[]
  grandTotal: Dec
  customer: CartCustomer | null
  pending: boolean
  error: unknown
  onClose: () => void
  onConfirm: (payments: SalePaymentInput[], note: string) => void
}) {
  const { t, i18n } = useTranslation()
  const money = (v: Dec) => formatMoney(toFixed(v, 2), i18n.language)
  const cashMethod = methods.find((m) => m.method_type === 'cash') ?? methods[0]
  const [lines, setLines] = useState<PayLine[]>([{ id: 1, methodId: cashMethod?.id ?? '', amount: toFixed(grandTotal, 2), reference: '' }])
  const [note, setNote] = useState('')
  const [submitted, setSubmitted] = useState<number[]>([])
  const formRef = useRef<HTMLFormElement>(null)
  const methodOf = (id: string) => methods.find((m) => m.id === id)
  const isCash = (l: PayLine) => methodOf(l.methodId)?.method_type === 'cash'

  const amounts = lines.map((l) => (l.amount.trim() ? tryParse(l.amount) : ZERO))
  const invalid = amounts.some((a) => a === null || a < ZERO)
  const values = amounts.map((a) => (a === null || a < ZERO ? ZERO : a))
  const tendered = sum(values)
  const cashTendered = sum(values.filter((_, i) => isCash(lines[i] as PayLine)))
  const change = max(tendered - grandTotal, ZERO)
  const overpayNotCash = change > cashTendered
  const paid = tendered - change
  const due = max(grandTotal - paid, ZERO)
  const dueWalkIn = due > ZERO && !customer
  const overCredit =
    due > ZERO && customer !== null && customer.creditLimit !== null && dec(customer.balance) + due > dec(customer.creditLimit)
  const missingRef = lines.some((l) => methodOf(l.methodId)?.requires_reference && !l.reference.trim() && dec(l.amount) > ZERO)

  const set = (id: number, patch: Partial<PayLine>) => setLines((ls) => ls.map((l) => (l.id === id ? { ...l, ...patch } : l)))
  const remainingFor = (id: number) =>
    max(grandTotal - sum(lines.filter((l) => l.id !== id).map((l) => dec(l.amount))), ZERO)

  function exact() {
    const last = lines[lines.length - 1]
    if (last) set(last.id, { amount: toFixed(remainingFor(last.id), 2) })
  }

  function cashNote(value: Dec) {
    const target = [...lines].reverse().find(isCash)
    if (target) set(target.id, { amount: toPlain(value) })
    else if (cashMethod) setLines((ls) => [...ls, { id: Date.now(), methodId: cashMethod.id, amount: toPlain(value), reference: '' }])
  }

  function addLine() {
    const other = methods.find((m) => m.method_type !== 'cash') ?? cashMethod
    if (!other) return
    const rest = max(grandTotal - tendered, ZERO)
    setLines((ls) => [...ls, { id: Date.now(), methodId: other.id, amount: toFixed(rest, 2), reference: '' }])
  }

  const notes = [...new Set(NOTE_STEPS.map((s) => toPlain(ceilTo(grandTotal, dec(s)))))]
    .map((s) => dec(s))
    .filter((v) => v > ZERO)
    .slice(0, 4)

  function submit(e: FormEvent) {
    e.preventDefault()
    if (invalid || overpayNotCash || dueWalkIn || missingRef || pending) return
    const chosen = lines.filter((l) => dec(l.amount) > ZERO)
    setSubmitted(chosen.map((l) => l.id))
    onConfirm(
      chosen.map((l) => {
        const p: SalePaymentInput = { method_id: l.methodId, amount: toPlain(dec(l.amount)) }
        if (l.reference.trim()) p.reference = l.reference.trim()
        return p
      }),
      note,
    )
  }

  // A held-down F8 auto-repeats; ignore repeats so the key that opened the
  // dialog cannot also post the sale.
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

  const errors = fieldErrors(error, t)
  const lineError = (id: number, field: string) => {
    const i = submitted.indexOf(id)
    return i >= 0 ? errors[`payments[${i}].${field}`] : undefined
  }

  return (
    <Dialog
      open
      onClose={onClose}
      title={t('pos.payTitle')}
      wide
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button
            type="submit"
            form="pos-pay"
            tabIndex={2}
            aria-keyshortcuts="F8 Control+Enter"
            disabled={pending || invalid || overpayNotCash || dueWalkIn || missingRef}
          >
            {pending ? t('pos.posting') : t('pos.confirmSale')}{' '}
            <kbd aria-hidden="true" className="text-sm opacity-80">
              F8
            </kbd>
          </Button>
        </>
      }
    >
      <form ref={formRef} id="pos-pay" onSubmit={submit} className="flex flex-col gap-4">
        <div className="flex items-baseline justify-between gap-3 rounded-lg bg-brand-50 px-4 py-3">
          <span className="font-semibold">{t('pos.grandTotal')}</span>
          <span className="text-2xl font-bold text-brand-700 tabular-nums">{money(grandTotal)}</span>
        </div>

        <div className="flex flex-col gap-3">
          {lines.map((l, i) => {
            const method = methodOf(l.methodId)
            return (
              <div key={l.id} className="flex flex-wrap items-end gap-2">
                <label className="flex flex-col gap-1.5 text-sm font-medium">
                  {t('pos.method')}
                  <select className={select} value={l.methodId} onChange={(e) => set(l.id, { methodId: e.target.value })}>
                    {methods.map((m) => (
                      <option key={m.id} value={m.id}>
                        {m.name}
                      </option>
                    ))}
                  </select>
                </label>
                <TextField
                  className="w-36"
                  label={t('pos.tendered', { n: i + 1 })}
                  inputMode="decimal"
                  value={l.amount}
                  autoFocus={i === 0}
                  data-autofocus={i === 0 ? true : undefined}
                  tabIndex={1}
                  onFocus={(e) => e.target.select()}
                  onChange={(e) => set(l.id, { amount: e.target.value })}
                  error={amounts[i] === null || (amounts[i] ?? ZERO) < ZERO ? t('pos.errors.amount') : lineError(l.id, 'amount')}
                />
                {method?.requires_reference && (
                  <TextField
                    className="w-44"
                    label={t('pos.reference')}
                    value={l.reference}
                    onChange={(e) => set(l.id, { reference: e.target.value })}
                    error={lineError(l.id, 'reference') ?? (!l.reference.trim() && dec(l.amount) > ZERO ? t('pos.referenceNeeded') : undefined)}
                  />
                )}
                {lines.length > 1 && (
                  <Button variant="ghost" onClick={() => setLines((ls) => ls.filter((x) => x.id !== l.id))} aria-label={t('pos.removePayment', { n: i + 1 })}>
                    ✕
                  </Button>
                )}
              </div>
            )
          })}
        </div>

        <div className="flex flex-wrap gap-2">
          <Button variant="secondary" onClick={exact}>
            {t('pos.exactAmount')}
          </Button>
          {notes.map((n) => (
            <Button key={n.toString()} variant="secondary" onClick={() => cashNote(n)}>
              {money(n)}
            </Button>
          ))}
          {methods.length > 0 && lines.length < 10 && (
            <Button variant="ghost" onClick={addLine}>
              {t('pos.splitPayment')}
            </Button>
          )}
        </div>

        <dl className="grid grid-cols-3 gap-3 text-center">
          <div className="rounded-lg border border-border p-2">
            <dt className="text-xs text-ink-muted uppercase">{t('pos.paid')}</dt>
            <dd className="text-lg font-bold tabular-nums">{money(paid)}</dd>
          </div>
          <div className="rounded-lg border border-border p-2">
            <dt className="text-xs text-ink-muted uppercase">{t('pos.change')}</dt>
            <dd data-testid="change" className="text-lg font-bold text-brand-700 tabular-nums">{money(change)}</dd>
          </div>
          <div className="rounded-lg border border-border p-2">
            <dt className="text-xs text-ink-muted uppercase">{t('pos.due')}</dt>
            <dd className={`text-lg font-bold tabular-nums ${due > ZERO ? 'text-danger' : ''}`}>{money(due)}</dd>
          </div>
        </dl>

        {overpayNotCash && <Alert tone="danger">{t('problems.OVERPAYMENT_NOT_CASH')}</Alert>}
        {dueWalkIn && <Alert tone="warning">{t('pos.dueNeedsCustomer')}</Alert>}
        {overCredit && <Alert tone="warning">{t('pos.overCreditLimit')}</Alert>}

        <TextField label={`${t('common.note')} (${t('common.optional')})`} value={note} maxLength={500} onChange={(e) => setNote(e.target.value)} />

        {error !== null && error !== undefined && hasFormError(error) && <Alert tone="danger">{problemMessage(error, t)}</Alert>}
      </form>
    </Dialog>
  )
}
