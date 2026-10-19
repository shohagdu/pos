import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, type ReactNode, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useParams } from 'react-router'

import { usePaymentMethods } from '../../api/money'
import { type PurchaseDetail, type PurchaseItem, type PurchasePayment, purchasingApi, purchasingKeys, usePurchase } from '../../api/purchasing'
import { newKey } from '../../api/query'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { Checkbox } from '../../components/ui/Checkbox'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { Dialog } from '../../components/ui/Dialog'
import { PageHeader } from '../../components/ui/PageHeader'
import { Spinner } from '../../components/ui/Spinner'
import { dec, toFixed } from '../../lib/decimal'
import { formatDate, formatDateTime } from '../../lib/dates'
import { formatMoney, formatQuantity } from '../../lib/format'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { LINK, type PaymentValues, SECONDARY_LINK, checkPayment, toPayInput, trimDecimal, useRefreshAfterMoney } from './helpers'
import { PaySupplierDialog, PaymentBadge, PaymentFields, ReasonDialog, StatusBadge, VoidPaymentDialog } from './shared'

type Open = 'post' | 'cancel' | 'pay' | { void: string } | null

/** `/purchases/:id`: bill header, items, payments, returns and actions. */
export function PurchaseDetailPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { id = '' } = useParams()
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const purchase = usePurchase(tenantId, id)
  const [open, setOpen] = useState<Open>(null)
  const refresh = useRefreshAfterMoney(tenantId)
  const cancel = useMutation({
    mutationFn: (reason: string) => purchasingApi.cancelPurchase(id, reason),
    onSuccess: async () => {
      await refresh()
      setOpen(null)
    },
  })

  if (purchase.isPending) return <Spinner />
  if (purchase.isError) return <Alert tone="danger">{problemMessage(purchase.error, t)}</Alert>
  const p = purchase.data
  const money = (v: string) => formatMoney(v, lang)
  const isDraft = p.status === 'draft'
  const isReceived = p.status === 'received'
  const due = dec(p.due_total)

  const itemColumns: Column<PurchaseItem>[] = [
    {
      key: 'product',
      header: t('purchases.product'),
      render: (i) => (
        <>
          <span className="font-medium">{i.product_name}</span>
          <span className="block font-mono text-xs text-ink-muted">{i.sku}</span>
        </>
      ),
    },
    { key: 'qty', header: t('common.quantity'), align: 'right', render: (i) => `${formatQuantity(i.quantity, lang, true)} ${i.unit_short_name}` },
    { key: 'cost', header: t('purchases.unitCost'), align: 'right', render: (i) => money(i.unit_cost) },
    {
      key: 'disc',
      header: t('purchases.discount'),
      align: 'right',
      render: (i) => {
        const total = dec(i.line_discount) + dec(i.bill_discount_alloc)
        return total === 0n ? '—' : money(toFixed(total, 2))
      },
    },
    { key: 'vat', header: t('purchases.vat'), align: 'right', render: (i) => (dec(i.tax_amount) === 0n ? '—' : `${money(i.tax_amount)} (${trimDecimal(i.tax_rate_pct)}%)`) },
    { key: 'total', header: t('purchases.lineTotal'), align: 'right', render: (i) => money(i.line_total) },
    { key: 'landed', header: t('purchases.landedCost'), align: 'right', render: (i) => formatMoney(i.landed_unit_cost, lang) },
    ...(isDraft
      ? []
      : [
          { key: 'received', header: t('purchases.received'), align: 'right' as const, render: (i: PurchaseItem) => formatQuantity(i.received_quantity, lang, true) },
          { key: 'returned', header: t('purchases.returned'), align: 'right' as const, render: (i: PurchaseItem) => formatQuantity(i.returned_quantity, lang, true) },
        ]),
  ]

  const paymentColumns: Column<PurchasePayment>[] = [
    { key: 'no', header: t('purchases.paymentNo'), render: (x) => x.payment_no },
    { key: 'date', header: t('common.date'), render: (x) => formatDateTime(x.created_at, lang) },
    { key: 'method', header: t('purchases.pay.method'), render: (x) => `${x.method_name} · ${x.account_name}` },
    { key: 'ref', header: t('purchases.pay.reference'), render: (x) => x.reference ?? '—' },
    { key: 'amount', header: t('common.amount'), align: 'right', render: (x) => money(x.amount) },
    {
      key: 'status',
      header: t('common.status'),
      render: (x) => <Badge tone={x.status === 'posted' ? 'good' : 'bad'}>{t(`purchases.paymentRowStatuses.${x.status}`)}</Badge>,
    },
    {
      key: 'actions',
      header: <span className="sr-only">{t('common.actions')}</span>,
      align: 'right',
      render: (x) =>
        x.status === 'posted' && can('payments.void') ? (
          <Button variant="ghost" className="min-h-9 print:hidden" onClick={() => setOpen({ void: x.payment_id })}>
            {t('purchases.voidPayment.button')}
          </Button>
        ) : null,
    },
  ]

  return (
    <div className="flex max-w-6xl flex-col gap-6">
      <Link to="/purchases" className={`text-sm print:hidden ${LINK}`}>
        ← {t('purchases.title')}
      </Link>
      <PageHeader
        title={p.purchase_no ?? t('purchases.draftTitle')}
        subtitle={
          <span className="flex flex-wrap items-center gap-2">
            <StatusBadge status={p.status} />
            {isReceived && <PaymentBadge status={p.payment_status} />}
          </span>
        }
        actions={
          <div className="flex flex-wrap gap-2 print:hidden">
            {isDraft && can('purchases.create') && (
              <Link to={`/purchases/${p.id}/edit`} className={SECONDARY_LINK}>
                {t('common.edit')}
              </Link>
            )}
            {isDraft && (can('purchases.cancel') || can('purchases.create')) && (
              <Button variant="secondary" onClick={() => setOpen('cancel')}>
                {t('purchases.cancelDraft.button')}
              </Button>
            )}
            {isDraft && can('purchases.post') && <Button onClick={() => setOpen('post')}>{t('purchases.post.button')}</Button>}
            {isReceived && due > 0n && can('suppliers.pay') && <Button onClick={() => setOpen('pay')}>{t('purchases.pay.button')}</Button>}
            {isReceived && can('purchases.return') && (
              <Link to={`/purchases/${p.id}/return`} className={SECONDARY_LINK}>
                {t('purchases.returnItems')}
              </Link>
            )}
            <Button variant="secondary" onClick={() => window.print()}>
              {t('common.print')}
            </Button>
          </div>
        }
      />
      {p.status === 'cancelled' && <Alert tone="warning">{t('purchases.cancelledNote', { reason: p.cancel_reason ?? '—', date: formatDateTime(p.cancelled_at, lang) })}</Alert>}
      {isDraft && <Alert>{t('purchases.draftNote')}</Alert>}

      <div className="grid gap-6 lg:grid-cols-[2fr_1fr]">
        <Card>
          <dl className="grid gap-x-6 gap-y-3 text-sm sm:grid-cols-2">
            <Info label={t('purchases.supplier')}>
              <Link to={`/suppliers/${p.supplier_id}`} className={LINK}>
                {p.supplier_name}
              </Link>
            </Info>
            <Info label={t('purchases.supplierInvoiceNo')}>{p.supplier_invoice_no ?? '—'}</Info>
            <Info label={t('purchases.purchaseDate')}>{formatDate(p.purchase_date, lang)}</Info>
            <Info label={t('purchases.location')}>{p.location_name}</Info>
            <Info label={t('purchases.createdBy')}>{`${p.created_by_name ?? '—'} · ${formatDateTime(p.created_at, lang)}`}</Info>
            <Info label={t('purchases.postedAt')}>{formatDateTime(p.posted_at, lang)}</Info>
            {p.note && <Info label={t('common.note')}>{p.note}</Info>}
          </dl>
        </Card>
        <Card>
          <dl className="flex flex-col gap-1 text-sm" aria-label={t('common.totals')}>
            <TotalRow label={t('purchases.subtotal')} value={money(p.subtotal)} />
            <TotalRow label={t('purchases.itemDiscounts')} value={`− ${money(p.item_discount_total)}`} />
            <TotalRow label={t('purchases.billDiscount')} value={`− ${money(p.bill_discount_amount)}`} />
            <TotalRow label={t('purchases.vat')} value={money(p.tax_total)} />
            <TotalRow label={t('purchases.shipping')} value={money(p.shipping_cost)} />
            <TotalRow label={t('purchases.otherCost')} value={money(p.other_cost)} />
            <TotalRow label={t('purchases.grandTotal')} value={money(p.grand_total)} strong />
            <TotalRow label={t('purchases.paid')} value={money(p.paid_total)} />
            <TotalRow label={t('purchases.returned')} value={money(p.returned_total)} />
            <TotalRow label={t('purchases.due')} value={money(p.due_total)} strong />
          </dl>
        </Card>
      </div>

      <section className="flex flex-col gap-3">
        <h2 className="text-lg font-semibold">{t('purchases.form.items')}</h2>
        <DataTable caption={t('purchases.form.items')} columns={itemColumns} rows={p.items} rowKey={(i) => i.id} empty={t('purchases.form.noItems')} />
      </section>

      {!isDraft && (
        <section className="flex flex-col gap-3">
          <h2 className="text-lg font-semibold">{t('purchases.payments')}</h2>
          <DataTable caption={t('purchases.payments')} columns={paymentColumns} rows={p.payments} rowKey={(x) => x.payment_id} empty={t('purchases.noPayments')} />
        </section>
      )}

      {p.returns.length > 0 && (
        <section className="flex flex-col gap-3">
          <h2 className="text-lg font-semibold">{t('purchases.returns.title')}</h2>
          <ul className="flex flex-col divide-y divide-border rounded-xl border border-border bg-surface text-sm">
            {p.returns.map((r) => (
              <li key={r.id} className="flex flex-wrap justify-between gap-2 px-3 py-2.5">
                <Link to={`/purchases/returns/${r.id}`} className={LINK}>
                  {r.return_no}
                </Link>
                <span className="text-ink-muted">{formatDate(r.business_date, lang)}</span>
                <span className="tabular-nums">{money(r.total)}</span>
              </li>
            ))}
          </ul>
        </section>
      )}

      {open === 'post' && <PostDialog tenantId={tenantId} purchase={p} onClose={() => setOpen(null)} />}
      {open === 'pay' && (
        <PaySupplierDialog
          tenantId={tenantId}
          supplierId={p.supplier_id}
          purchaseId={p.id}
          defaultAmount={p.due_total}
          maxAmount={p.due_total}
          onClose={() => setOpen(null)}
        />
      )}
      {open === 'cancel' && (
        <ReasonDialog
          title={t('purchases.cancelDraft.title')}
          hint={t('purchases.cancelDraft.hint')}
          submitLabel={t('purchases.cancelDraft.submit')}
          pending={cancel.isPending}
          error={cancel.error}
          onConfirm={(reason) => cancel.mutate(reason)}
          onClose={() => setOpen(null)}
        />
      )}
      {open !== null && typeof open === 'object' && <VoidPaymentDialog tenantId={tenantId} paymentId={open.void} onClose={() => setOpen(null)} />}
    </div>
  )
}

function Info({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div>
      <dt className="text-xs font-medium text-ink-muted uppercase">{label}</dt>
      <dd className="mt-0.5">{children}</dd>
    </div>
  )
}

function TotalRow({ label, value, strong = false }: { label: string; value: string; strong?: boolean }) {
  return (
    <div className={`flex justify-between gap-4 ${strong ? 'border-t border-border pt-1.5 font-bold' : ''}`}>
      <dt className={strong ? '' : 'text-ink-muted'}>{label}</dt>
      <dd className="tabular-nums">{value}</dd>
    </div>
  )
}

/** Post a draft: goods receipt, stock, ledger and an optional payment. */
function PostDialog({ tenantId, purchase, onClose }: { tenantId: string; purchase: PurchaseDetail; onClose: () => void }) {
  const { t, i18n } = useTranslation()
  const { can } = useSession()
  const queryClient = useQueryClient()
  const [key] = useState(newKey)
  const canPay = can('suppliers.pay')
  const methods = usePaymentMethods(tenantId).data ?? []
  const [withPayment, setWithPayment] = useState(false)
  const [values, setValues] = useState<PaymentValues>({ method_id: '', amount: trimDecimal(purchase.grand_total), account_id: '', reference: '' })
  const [clientErrors, setClientErrors] = useState<Record<string, string>>({})
  const refresh = useRefreshAfterMoney(tenantId)
  const post = useMutation({
    mutationFn: () => purchasingApi.postPurchase(purchase.id, withPayment ? toPayInput(values) : undefined, key),
    onSuccess: async (saved) => {
      queryClient.setQueryData(purchasingKeys.purchase(tenantId, saved.id), saved)
      await refresh()
      onClose()
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

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    if (withPayment) {
      const method = methods.find((m) => m.id === values.method_id)
      const found = checkPayment(values, purchase.grand_total, t, Boolean(method?.requires_reference))
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
          <Button type="submit" form="post-purchase" disabled={post.isPending}>
            {post.isPending ? t('common.saving') : t('purchases.post.submit')}
          </Button>
        </>
      }
    >
      <form id="post-purchase" onSubmit={onSubmit} noValidate className="flex flex-col gap-4">
        <p className="text-sm text-ink-muted">{t('purchases.post.hint', { total: formatMoney(purchase.grand_total, i18n.language) })}</p>
        {canPay && <Checkbox label={t('purchases.post.withPayment')} checked={withPayment} onChange={(e) => setWithPayment(e.target.checked)} />}
        {withPayment && <PaymentFields tenantId={tenantId} value={values} onChange={setValues} errors={errors} />}
        {post.isError && hasFormError(post.error) && <Alert tone="danger">{problemMessage(post.error, t)}</Alert>}
      </form>
    </Dialog>
  )
}
