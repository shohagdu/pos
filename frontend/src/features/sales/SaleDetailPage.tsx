import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
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
import { SaleTotals } from './SaleTotals'
import { usePrint } from './usePrint'

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

  const reprint = useMutation({
    mutationFn: () => salesApi.reprint(id),
    onSuccess: (detail) => {
      queryClient.setQueryData(salesKeys.sale(tenantId, id), detail)
      print()
    },
  })

  if (sale.isPending) return <Spinner />
  if (sale.isError) return <Alert tone="danger">{problemMessage(sale.error, t)}</Alert>
  const s = sale.data
  const money = (v: string) => formatMoney(v, lang)
  const showCost = s.items.some((i) => i.unit_cost !== null)
  const businessDate = ctx.data?.business_date ?? today()
  const canCancel =
    can('sales.cancel') && s.status === 'completed' && dec(s.returned_total) === ZERO && s.business_date === businessDate
  const canReturn = can('sales.return') && (s.status === 'completed' || s.status === 'partially_returned')

  const itemColumns: Column<SaleItem>[] = [
    { key: 'no', header: '#', render: (i) => i.line_no },
    {
      key: 'product',
      header: t('sales.product'),
      render: (i) => (
        <>
          <span className="font-medium">{i.product_name}</span>
          <span className="block font-mono text-xs text-ink-muted">{i.sku}</span>
        </>
      ),
    },
    { key: 'qty', header: t('common.quantity'), align: 'right', render: (i) => `${formatQuantity(i.quantity, lang, true)} ${i.unit_short_name}` },
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
      render: (i) => (
        <>
          {money(i.unit_price)}
          {i.price_overridden && <span className="block text-xs text-ink-muted line-through">{money(i.list_price)}</span>}
        </>
      ),
    },
    {
      key: 'discount',
      header: t('sales.discount'),
      align: 'right',
      render: (i) => {
        const d = dec(i.line_discount) + dec(i.invoice_discount_alloc)
        return d === ZERO ? '—' : money(toFixed(d, 2))
      },
    },
    { key: 'vat', header: t('sales.vat'), align: 'right', render: (i) => money(i.tax_amount) },
    { key: 'total', header: t('common.total'), align: 'right', render: (i) => <span className="font-semibold">{money(i.line_total)}</span> },
    ...(showCost
      ? ([
          { key: 'unitCost', header: t('sales.unitCost'), align: 'right', render: (i) => (i.unit_cost === null ? '—' : money(i.unit_cost)) },
          { key: 'cost', header: t('sales.costTotal'), align: 'right', render: (i) => (i.cost_total === null ? '—' : money(i.cost_total)) },
        ] satisfies Column<SaleItem>[])
      : []),
  ]

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
        <Link to="/sales" className="text-sm font-medium text-brand-700 hover:underline">
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
              <Button variant="secondary" disabled={reprint.isPending} onClick={() => reprint.mutate()}>
                {reprint.isPending ? t('common.loading') : t('sales.reprint')}
              </Button>
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
          <Alert tone="warning">
            {t('sales.cancelledInfo', { date: formatDateTime(s.cancelled_at, lang), reason: s.cancel_reason ?? '—' })}
          </Alert>
        )}

        <Card className="grid gap-x-6 gap-y-3 text-sm sm:grid-cols-2 lg:grid-cols-4">
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
          <DataTable caption={t('sales.items')} columns={itemColumns} rows={s.items} rowKey={(i) => i.id} />
        </section>

        <div className="grid gap-6 lg:grid-cols-[1fr_22rem]">
          <div className="flex flex-col gap-6">
            <section className="flex flex-col gap-2">
              <h2 className="text-lg font-semibold">{t('sales.payments')}</h2>
              <DataTable caption={t('sales.payments')} columns={paymentColumns} rows={s.payments} rowKey={(p) => p.payment_id} empty={t('sales.noPayments')} />
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
          <SaleTotals sale={s} />
        </div>
        {canCancel && <CancelSaleDialog sale={s} tenantId={tenantId} open={cancelling} onClose={() => setCancelling(false)} />}
      </ScreenOnly>
      <PrintOnly>
        <SaleReceipt sale={s} />
      </PrintOnly>
    </>
  )
}
