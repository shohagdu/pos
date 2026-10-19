import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { type Payment, moneyApi, moneyKeys, usePayments } from '../../api/money'
import { type StatementEntry, useCustomerStatement } from '../../api/parties'
import { type Sale, useSales } from '../../api/sales'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { DateRangeFilter } from '../../components/ui/DateRangeFilter'
import { downloadCsv } from '../../lib/csv'
import { dec } from '../../lib/decimal'
import { formatDate, formatDateTime, startOfMonth, today } from '../../lib/dates'
import { formatMoney } from '../../lib/format'
import { ReasonDialog } from '../money/ReasonDialog'
import { LoadMore, QueryStatus, TextLink } from '../money/shared'
import { isPositive, stamp, useTenant, referencePath } from '../money/utils'

/** Customer statement with opening/closing balance rows. */
export function StatementTab({ customerId, customerName }: { customerId: string; customerName: string }) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { tenantId } = useTenant()
  const [range, setRange] = useState({ from: startOfMonth(), to: today() })
  const statement = useCustomerStatement(tenantId, customerId, range.from || undefined, range.to || undefined)
  const s = statement.data
  const money = (v: string) => (dec(v) === 0n ? '' : formatMoney(v, lang))
  const typeLabel = (type: string) => t(`customers.entryTypes.${type}`, { defaultValue: type })

  const columns: Column<StatementEntry>[] = [
    { key: 'date', header: t('common.date'), render: (e) => formatDate(e.business_date, lang) },
    { key: 'type', header: t('customers.entryType'), render: (e) => typeLabel(e.entry_type) },
    {
      key: 'ref',
      header: t('customers.referenceNo'),
      render: (e) => {
        const path = referencePath(e.reference_type, e.reference_id)
        return path && e.reference_no ? <TextLink to={path}>{e.reference_no}</TextLink> : (e.reference_no ?? '—')
      },
    },
    { key: 'note', header: t('common.note'), render: (e) => e.note ?? '' },
    { key: 'debit', header: t('customers.debit'), align: 'right', render: (e) => money(e.debit) },
    { key: 'credit', header: t('customers.credit'), align: 'right', render: (e) => money(e.credit) },
    { key: 'balance', header: t('customers.balance'), align: 'right', render: (e) => formatMoney(e.balance, lang) },
  ]

  function exportCsv() {
    if (!s) return
    downloadCsv(`statement-${customerName}-${stamp()}`, s.entries, [
      { header: t('common.date'), value: (e) => e.business_date },
      { header: t('customers.entryType'), value: (e) => typeLabel(e.entry_type) },
      { header: t('customers.referenceNo'), value: (e) => e.reference_no },
      { header: t('common.note'), value: (e) => e.note },
      { header: t('customers.debit'), value: (e) => e.debit },
      { header: t('customers.credit'), value: (e) => e.credit },
      { header: t('customers.balance'), value: (e) => e.balance },
    ])
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-end gap-3 print:hidden">
        <DateRangeFilter from={range.from} to={range.to} onChange={setRange} />
        <div className="ms-auto flex gap-2">
          <Button variant="secondary" disabled={!s} onClick={exportCsv}>
            {t('common.exportCsv')}
          </Button>
          <Button variant="secondary" disabled={!s} onClick={() => window.print()}>
            {t('common.print')}
          </Button>
        </div>
      </div>
      <p className="hidden text-sm print:block">
        {t('customers.statementFor', { name: customerName })} · {formatDate(range.from, lang)} – {formatDate(range.to, lang)}
      </p>
      {!s ? (
        <QueryStatus query={statement} />
      ) : (
        <div className="overflow-x-auto rounded-xl border border-border bg-surface">
          <table className="w-full text-sm">
            <caption className="sr-only">{t('customers.statement')}</caption>
            <thead className="border-b border-border bg-canvas text-xs text-ink-muted uppercase">
              <tr>
                {columns.map((c) => (
                  <th key={c.key} scope="col" className={`px-3 py-2.5 font-semibold whitespace-nowrap ${c.align === 'right' ? 'text-right' : 'text-left'}`}>
                    {c.header}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              <tr className="border-b border-border bg-canvas/50 font-medium">
                <td className="px-3 py-2.5" colSpan={6}>
                  {t('customers.openingBalance')}
                </td>
                <td className="px-3 py-2.5 text-right tabular-nums">{formatMoney(s.opening_balance, lang)}</td>
              </tr>
              {s.entries.length === 0 && (
                <tr>
                  <td colSpan={7} className="px-3 py-2.5 text-ink-muted">
                    {t('customers.noEntries')}
                  </td>
                </tr>
              )}
              {s.entries.map((e, i) => (
                <tr key={e.id} className="border-b border-border">
                  {columns.map((c) => (
                    <td key={c.key} className={`px-3 py-2.5 ${c.align === 'right' ? 'text-right tabular-nums whitespace-nowrap' : ''}`}>
                      {c.render(e, i)}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
            <tfoot className="border-t-2 border-border bg-canvas font-semibold">
              <tr>
                <td className="px-3 py-2.5" colSpan={4}>
                  {t('customers.closingBalance')}
                </td>
                <td className="px-3 py-2.5 text-right tabular-nums">{formatMoney(s.total_debit, lang)}</td>
                <td className="px-3 py-2.5 text-right tabular-nums">{formatMoney(s.total_credit, lang)}</td>
                <td className="px-3 py-2.5 text-right tabular-nums">{formatMoney(s.closing_balance, lang)}</td>
              </tr>
            </tfoot>
          </table>
        </div>
      )}
    </div>
  )
}

const PAYMENT_TONE = { unpaid: 'bad', partial: 'warn', paid: 'good', refunded: 'neutral' } as const

/** The customer's invoices. */
export function InvoicesTab({ customerId }: { customerId: string }) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { tenantId } = useTenant()
  const sales = useSales(tenantId, { customer_id: customerId })
  const rows = sales.data?.pages.flatMap((p) => p.data) ?? []
  const columns: Column<Sale>[] = [
    { key: 'no', header: t('customers.invoiceNo'), render: (s) => <TextLink to={`/sales/${s.id}`}>{s.invoice_no}</TextLink> },
    { key: 'date', header: t('common.date'), render: (s) => formatDate(s.business_date, lang) },
    { key: 'total', header: t('common.total'), align: 'right', render: (s) => formatMoney(s.grand_total, lang) },
    { key: 'paid', header: t('customers.paid'), align: 'right', render: (s) => formatMoney(s.paid_total, lang) },
    {
      key: 'due',
      header: t('customers.due'),
      align: 'right',
      render: (s) => <span className={isPositive(s.due_total) ? 'font-semibold text-danger' : ''}>{formatMoney(s.due_total, lang)}</span>,
    },
    {
      key: 'pstatus',
      header: t('customers.paymentStatus'),
      render: (s) => <Badge tone={PAYMENT_TONE[s.payment_status]}>{t(`customers.paymentStatuses.${s.payment_status}`)}</Badge>,
    },
    { key: 'status', header: t('common.status'), render: (s) => t(`customers.saleStatuses.${s.status}`, { defaultValue: s.status }) },
  ]
  if (!sales.isSuccess) return <QueryStatus query={sales} />
  return (
    <div className="flex flex-col gap-4">
      <DataTable caption={t('customers.invoices')} columns={columns} rows={rows} rowKey={(s) => s.id} empty={t('customers.noInvoices')} />
      <LoadMore query={sales} />
    </div>
  )
}

/** Payments received from (or refunded to) the customer, with void. */
export function PaymentsTab({ customerId }: { customerId: string }) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { tenantId, can } = useTenant()
  const queryClient = useQueryClient()
  const payments = usePayments(tenantId, { party_type: 'customer', party_id: customerId, status: 'all' })
  const rows = payments.data?.pages.flatMap((p) => p.data) ?? []
  const [voiding, setVoiding] = useState<Payment | null>(null)
  const voidPayment = useMutation({
    mutationFn: ({ id, reason }: { id: string; reason: string }) => moneyApi.voidPayment(id, reason),
    onSuccess: async () => {
      setVoiding(null)
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: moneyKeys.all(tenantId) }),
        queryClient.invalidateQueries({ queryKey: ['parties', tenantId] }),
        queryClient.invalidateQueries({ queryKey: ['sales', tenantId] }),
        queryClient.invalidateQueries({ queryKey: ['reports', tenantId] }),
      ])
    },
  })
  const canVoid = can('payments.void')

  const columns: Column<Payment>[] = [
    { key: 'no', header: t('customers.paymentNo'), render: (p) => <span className="font-medium">{p.payment_no}</span> },
    { key: 'date', header: t('common.date'), render: (p) => formatDateTime(p.payment_date, lang) },
    { key: 'method', header: t('customers.method'), render: (p) => p.method_name },
    { key: 'account', header: t('customers.account'), render: (p) => p.account_name },
    { key: 'source', header: t('customers.source'), render: (p) => t(`money.sources.${p.source}`, { defaultValue: p.source }) },
    {
      key: 'amount',
      header: t('common.amount'),
      align: 'right',
      render: (p) => (
        <span className={p.status === 'voided' ? 'text-ink-muted line-through' : ''}>
          {p.direction === 'out' ? '−' : ''}
          {formatMoney(p.amount, lang)}
        </span>
      ),
    },
    {
      key: 'unallocated',
      header: t('customers.unallocated'),
      align: 'right',
      render: (p) => (isPositive(p.unallocated_amount) ? formatMoney(p.unallocated_amount, lang) : ''),
    },
    { key: 'reference', header: t('customers.reference'), render: (p) => p.reference ?? '' },
    {
      key: 'status',
      header: t('common.status'),
      render: (p) => (
        <span title={p.void_reason ?? undefined}>
          <Badge tone={p.status === 'posted' ? 'good' : 'bad'}>{t(`money.paymentStatuses.${p.status}`)}</Badge>
        </span>
      ),
    },
    {
      key: 'actions',
      header: <span className="sr-only">{t('common.actions')}</span>,
      render: (p) =>
        canVoid && p.source === 'collection' && p.status === 'posted' ? (
          <Button
            variant="ghost"
            className="min-h-9 text-danger"
            onClick={() => {
              voidPayment.reset()
              setVoiding(p)
            }}
            aria-label={t('customers.voidPaymentNo', { no: p.payment_no })}
          >
            {t('customers.void')}
          </Button>
        ) : null,
    },
  ]

  if (!payments.isSuccess) return <QueryStatus query={payments} />
  return (
    <div className="flex flex-col gap-4">
      <DataTable caption={t('customers.payments')} columns={columns} rows={rows} rowKey={(p) => p.id} empty={t('customers.noPayments')} />
      <LoadMore query={payments} />
      {voiding && (
        <ReasonDialog
          title={t('customers.voidPaymentNo', { no: voiding.payment_no })}
          description={t('customers.voidHint', { amount: formatMoney(voiding.amount, lang) })}
          confirmLabel={t('customers.void')}
          pending={voidPayment.isPending}
          error={voidPayment.error}
          onConfirm={(reason) => voidPayment.mutate({ id: voiding.id, reason })}
          onClose={() => setVoiding(null)}
        />
      )}
    </div>
  )
}
