import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type ReactNode, useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useParams } from 'react-router'

import { type SaleItem, type SalePayment, salesApi, salesKeys, usePosContext, useSale } from '../../api/sales'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { PageHeader } from '../../components/ui/PageHeader'
import { Spinner } from '../../components/ui/Spinner'
import { ZERO, dec, toFixed } from '../../lib/decimal'
import { formatDate, formatDateTime, today } from '../../lib/dates'
import { formatMoney, formatQuantity } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { CancelSaleDialog } from './CancelSaleDialog'
import { PaymentStatusBadge, SaleStatusBadge } from './labels'
import { Info } from './Info'
import { PrintOnly, ScreenOnly } from './print'
import { SaleReceipt } from './Receipt'
import { type NetInvoice, netOfReturns } from './returnMath'
import { SaleTotals } from './SaleTotals'
import { usePrint } from './usePrint'
import { warrantyExpired, warrantyTenure } from './warranty'

const linkButton = 'inline-flex min-h-11 items-center rounded-lg px-4 text-sm font-semibold'

/** One sale: lines, totals, payments, returns; reprint, cancel and return. */
export function SaleDetailPage() {
  const { id = '' } = useParams()
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const queryClient = useQueryClient()
  const sale = useSale(tenantId, id)
  const ctx = usePosContext(tenantId)
  const print = usePrint()
  const [cancelling, setCancelling] = useState(false)
  // Set while printing the invoice without its returned items.
  const [net, setNet] = useState<NetInvoice | null>(null)

  // A reprint is counted either way; "without returns" also loads each
  // return's lines so the printed totals use the exact amounts taken off.
  const reprint = useMutation({
    mutationFn: async (withoutReturns: boolean) => {
      const detail = await salesApi.reprint(id)
      const returns = withoutReturns
        ? await Promise.all(
            detail.returns.map((r) => queryClient.fetchQuery({ queryKey: salesKeys.saleReturn(tenantId, r.id), queryFn: () => salesApi.saleReturn(r.id) })),
          )
        : null
      return { detail, returns }
    },
    onSuccess: ({ detail, returns }) => {
      queryClient.setQueryData(salesKeys.sale(tenantId, id), detail)
      setNet(returns ? netOfReturns(detail, returns) : null)
      print()
    },
  })

  // Back to the full invoice once the print dialog closes (Ctrl+P prints that).
  useEffect(() => {
    const reset = () => setNet(null)
    window.addEventListener('afterprint', reset)
    return () => window.removeEventListener('afterprint', reset)
  }, [])

  if (sale.isPending) return <Spinner />
  if (sale.isError) return <Alert tone="danger">{problemMessage(sale.error, t)}</Alert>
  const s = sale.data
  const money = (v: string) => formatMoney(v, lang)
  const showCost = s.items.some((i) => i.unit_cost !== null)
  const businessDate = ctx.data?.business_date ?? today()
  const canCancel = can('sales.cancel') && s.status === 'completed' && dec(s.returned_total) === ZERO && s.business_date === businessDate
  // Something was returned and something is left to print.
  const canPrintNet = s.status !== 'cancelled' && dec(s.returned_total) > ZERO && s.items.some((i) => dec(i.returned_quantity) < dec(i.quantity))
  const canReturn = can('sales.return') && (s.status === 'completed' || s.status === 'partially_returned')

  // A line with anything returned: name in red and a "returned item" note.
  const returnedNote = (i: SaleItem) => {
    if (dec(i.returned_quantity) === ZERO) return null
    const qty = `${formatQuantity(i.returned_quantity, lang, true)} ${i.unit_short_name}`
    const all = dec(i.returned_quantity) >= dec(i.quantity)
    return (
      <span className="mt-0.5 inline-flex items-center gap-1 rounded bg-danger/10 px-1.5 py-0.5 text-xs font-medium text-danger">
        <span aria-hidden="true">↩</span>
        {all
          ? t('sales.itemReturnedAll', { qty })
          : t('sales.itemReturnedPart', { qty, total: `${formatQuantity(i.quantity, lang, true)} ${i.unit_short_name}` })}
      </span>
    )
  }
  const productCell = (i: SaleItem) => (
    <>
      <span className={`font-medium ${dec(i.returned_quantity) === ZERO ? '' : 'text-danger'}`}>{i.product_name}</span>
      <span className="block font-mono text-xs text-ink-muted">{i.sku}</span>
      {returnedNote(i) && <span className="block">{returnedNote(i)}</span>}
      {(i.discounts ?? []).length > 0 && (
        <span className="mt-0.5 flex flex-wrap gap-1">
          {(i.discounts ?? []).map((d) => (
            <span
              key={`${d.promotion_id}-${d.coupon_code ?? ''}`}
              className="inline-flex items-center gap-1 rounded bg-brand-50 px-1.5 py-0.5 text-xs font-medium text-brand-700"
            >
              {d.coupon_code ? t('sales.couponWithCode', { code: d.coupon_code }) : lang === 'bn' && d.name_bn ? d.name_bn : d.name}
              {dec(d.free_quantity) > ZERO && (
                <span className="rounded bg-brand-600 px-1 text-[0.625rem] font-bold text-white uppercase">
                  {t('sales.freeUnits', { qty: formatQuantity(d.free_quantity, lang, true) })}
                </span>
              )}
              <span className="tabular-nums">−{money(d.amount)}</span>
            </span>
          ))}
        </span>
      )}
      {i.warranty_expires_on && (
        <span className={`block text-xs ${warrantyExpired(i.warranty_expires_on, today()) ? 'text-danger' : 'text-brand-700'}`}>
          {t(warrantyExpired(i.warranty_expires_on, today()) ? 'sales.warrantyExpired' : 'sales.warrantyUntil', {
            date: formatDate(i.warranty_expires_on, lang),
          })}
          {warrantyTenure(i, t) && <span className="text-ink-muted"> · {warrantyTenure(i, t)}</span>}
        </span>
      )}
    </>
  )
  const quantity = (i: SaleItem) => `${formatQuantity(i.quantity, lang, true)} ${i.unit_short_name}`
  // Promotions, the cashier's line discount and the invoice discount's share.
  const lineDiscount = (i: SaleItem) => dec(i.promotion_discount) + dec(i.line_discount) + dec(i.invoice_discount_alloc)
  const price = (i: SaleItem) => (
    <>
      {money(i.unit_price)}
      {i.price_overridden && <span className="block text-xs text-ink-muted line-through">{money(i.list_price)}</span>}
    </>
  )

  const itemColumns: Column<SaleItem>[] = [
    { key: 'no', header: '#', render: (i) => i.line_no },
    { key: 'product', header: t('sales.product'), className: 'min-w-[12rem]', render: productCell },
    { key: 'qty', header: t('common.quantity'), align: 'right', render: quantity },
    {
      key: 'returned',
      header: t('sales.returnedQty'),
      align: 'right',
      render: (i) => (dec(i.returned_quantity) === ZERO ? '—' : formatQuantity(i.returned_quantity, lang, true)),
    },
    {
      key: 'price',
      header: t('common.price'),
      align: 'right',
      render: price,
    },
    {
      key: 'discount',
      header: t('sales.discount'),
      align: 'right',
      render: (i) => (lineDiscount(i) === ZERO ? '—' : money(toFixed(lineDiscount(i), 2))),
    },
    { key: 'vat', header: t('sales.vat'), align: 'right', render: (i) => money(i.tax_amount) },
    { key: 'total', header: t('common.total'), align: 'right', render: (i) => <span className="font-semibold">{money(i.line_total)}</span> },
    ...(showCost
      ? ([
          { key: 'unitCost', header: t('sales.unitCost'), align: 'right', showFrom: 'xl', render: (i) => (i.unit_cost === null ? '—' : money(i.unit_cost)) },
          { key: 'cost', header: t('sales.costTotal'), align: 'right', render: (i) => (i.cost_total === null ? '—' : money(i.cost_total)) },
        ] satisfies Column<SaleItem>[])
      : []),
  ]

  // Phones and tablets: each line as a card, the same figures as the table's columns.
  const itemCard = (i: SaleItem) => (
    <div className="flex flex-col gap-2">
      <div className="flex items-start gap-3">
        <span className="mt-0.5 flex h-6 min-w-6 shrink-0 items-center justify-center rounded-full bg-brand-100 px-1.5 text-xs font-bold text-brand-700 tabular-nums">
          <span className="sr-only">#</span>
          {i.line_no}
        </span>
        <div className="min-w-0 flex-1 break-words">{productCell(i)}</div>
        <span className="font-semibold whitespace-nowrap tabular-nums">{money(i.line_total)}</span>
      </div>
      <dl className="ml-9 grid grid-cols-1 gap-x-4 gap-y-1 text-xs min-[400px]:grid-cols-2">
        <Pair label={t('common.quantity')} value={quantity(i)} />
        <Pair label={t('common.price')} value={price(i)} />
        {dec(i.returned_quantity) !== ZERO && (
          <Pair label={t('sales.returnedQty')} value={<span className="font-semibold text-danger">{formatQuantity(i.returned_quantity, lang, true)}</span>} />
        )}
        {lineDiscount(i) !== ZERO && <Pair label={t('sales.discount')} value={money(toFixed(lineDiscount(i), 2))} />}
        {dec(i.tax_amount) !== ZERO && <Pair label={t('sales.vat')} value={money(i.tax_amount)} />}
        {i.cost_total !== null && <Pair label={t('sales.costTotal')} value={money(i.cost_total)} />}
      </dl>
    </div>
  )

  const paymentCard = (p: SalePayment) => (
    <div className="flex items-start justify-between gap-3">
      <div className="min-w-0">
        <div className="font-medium">
          {p.method_name} <span className="font-normal text-ink-muted">· {p.payment_no}</span>
        </div>
        <div className="text-xs break-words text-ink-muted">
          {p.account_name}
          {p.reference && ` · ${p.reference}`}
        </div>
      </div>
      <div className="flex shrink-0 flex-col items-end gap-1">
        <span className="font-semibold tabular-nums">{money(p.amount)}</span>
        <Badge tone={p.status === 'posted' ? 'good' : 'bad'}>{t(`sales.paymentRowStatus.${p.status}`)}</Badge>
      </div>
    </div>
  )

  const paymentColumns: Column<SalePayment>[] = [
    { key: 'no', header: t('sales.paymentNo'), render: (p) => p.payment_no },
    { key: 'method', header: t('sales.method'), render: (p) => p.method_name },
    { key: 'account', header: t('sales.account'), render: (p) => p.account_name },
    { key: 'ref', header: t('sales.reference'), render: (p) => p.reference ?? '—' },
    {
      key: 'status',
      header: t('common.status'),
      render: (p) => <Badge tone={p.status === 'posted' ? 'good' : 'bad'}>{t(`sales.paymentRowStatus.${p.status}`)}</Badge>,
    },
    { key: 'amount', header: t('common.amount'), align: 'right', render: (p) => money(p.amount) },
  ]

  return (
    <>
      <ScreenOnly className="flex max-w-6xl flex-col gap-6">
        <Link to="/sales" className="inline-flex min-h-11 items-center self-start text-sm font-medium text-brand-700 hover:underline md:min-h-0">
          ← {t('sales.backToSales')}
        </Link>
        <PageHeader
          title={t('sales.invoiceTitle', { no: s.invoice_no })}
          subtitle={
            <span className="inline-flex flex-wrap items-center gap-2">
              <SaleStatusBadge status={s.status} />
              <PaymentStatusBadge status={s.payment_status} />
              {s.reprint_count > 0 && <Badge>{t('sales.receipt.reprint', { n: s.reprint_count })}</Badge>}
            </span>
          }
          actions={
            <>
              <Button variant="secondary" disabled={reprint.isPending} onClick={() => reprint.mutate(false)}>
                {reprint.isPending && !reprint.variables ? t('common.loading') : t('sales.reprint')}
              </Button>
              {canPrintNet && (
                <Button variant="secondary" disabled={reprint.isPending} title={t('sales.printNetHint')} onClick={() => reprint.mutate(true)}>
                  {reprint.isPending && reprint.variables ? t('common.loading') : t('sales.printNet')}
                </Button>
              )}
              {canReturn && (
                <Link to={`/sales/${s.id}/return`} className={`${linkButton} border border-border bg-surface text-ink hover:bg-canvas`}>
                  {t('sales.returnItems')}
                </Link>
              )}
              {canCancel && (
                <Button variant="secondary" className="text-danger" onClick={() => setCancelling(true)}>
                  {t('sales.cancelSale')}
                </Button>
              )}
            </>
          }
        />
        {reprint.isError && <Alert tone="danger">{problemMessage(reprint.error, t)}</Alert>}
        {s.status === 'cancelled' && (
          <Alert tone="warning">{t('sales.cancelledInfo', { date: formatDateTime(s.cancelled_at, lang), reason: s.cancel_reason ?? '—' })}</Alert>
        )}

        <Card className="grid grid-cols-2 gap-x-6 gap-y-3 text-sm lg:grid-cols-4">
          <Info label={t('sales.dateTime')} value={formatDateTime(s.sale_date || s.created_at, lang)} />
          <Info label={t('sales.businessDate')} value={formatDate(s.business_date, lang)} />
          <Info
            label={t('sales.customer')}
            value={
              s.is_walk_in ? (
                t('sales.walkIn')
              ) : (
                <Link to={`/customers/${s.customer_id}`} className="text-brand-700 hover:underline">
                  {s.customer_name}
                  {s.customer_phone && ` · ${s.customer_phone}`}
                </Link>
              )
            }
          />
          <Info label={t('sales.cashier')} value={s.cashier_name ?? '—'} />
          {s.note && <Info label={t('common.note')} value={s.note} />}
        </Card>

        <section className="flex flex-col gap-2">
          <h2 className="text-lg font-semibold">{t('sales.items')}</h2>
          <DataTable caption={t('sales.items')} columns={itemColumns} rows={s.items} rowKey={(i) => i.id} card={itemCard} />
        </section>

        <div className="grid grid-cols-1 gap-6 xl:grid-cols-[minmax(0,1fr)_22rem]">
          <div className="flex min-w-0 flex-col gap-6">
            <section className="flex flex-col gap-2">
              <h2 className="text-lg font-semibold">{t('sales.payments')}</h2>
              <DataTable
                caption={t('sales.payments')}
                columns={paymentColumns}
                rows={s.payments}
                rowKey={(p) => p.payment_id}
                empty={t('sales.noPayments')}
                card={paymentCard}
              />
            </section>
            <section className="flex flex-col gap-2">
              <h2 className="text-lg font-semibold">{t('sales.returnsTitle')}</h2>
              {s.returns.length === 0 ? (
                <p className="text-sm text-ink-muted">{t('sales.noReturns')}</p>
              ) : (
                <ul className="flex flex-col gap-1 text-sm">
                  {s.returns.map((r) => (
                    <li key={r.id} className="flex flex-wrap justify-between gap-2 rounded-lg border border-border bg-surface px-3 py-2">
                      <Link to={`/sales/returns/${r.id}`} className="font-medium text-brand-700 hover:underline">
                        {r.return_no}
                      </Link>
                      <span className="text-ink-muted">
                        {formatDate(r.business_date, lang)} · {t(`sales.refundModes.${r.refund_mode}`)}
                      </span>
                      <span className="tabular-nums">{money(r.total)}</span>
                    </li>
                  ))}
                </ul>
              )}
            </section>
          </div>
          {/* Beside payments from xl; below them (right-aligned on tablets) before that. */}
          <div className="grid w-full min-w-0 md:max-w-md md:justify-self-end xl:max-w-none">
            <SaleTotals sale={s} />
          </div>
        </div>
        {canCancel && <CancelSaleDialog sale={s} tenantId={tenantId} open={cancelling} onClose={() => setCancelling(false)} />}
      </ScreenOnly>
      <PrintOnly>
        <SaleReceipt sale={s} net={net ?? undefined} />
      </PrintOnly>
    </>
  )
}

/** A label and value on a phone card. */
function Pair({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div className="flex justify-between gap-2">
      <dt className="text-ink-muted">{label}</dt>
      <dd className="text-right tabular-nums">{value}</dd>
    </div>
  )
}
