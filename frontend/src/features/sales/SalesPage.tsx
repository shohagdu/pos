import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { type PaymentStatus, type Sale, type SaleFilter, type SaleStatus, useSales } from '../../api/sales'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { DateRangeFilter } from '../../components/ui/DateRangeFilter'
import { PageHeader } from '../../components/ui/PageHeader'
import { Select } from '../../components/ui/Select'
import { Spinner } from '../../components/ui/Spinner'
import { StatCard } from '../../components/ui/StatCard'
import { TextField } from '../../components/ui/TextField'
import { downloadCsv } from '../../lib/csv'
import { formatDateTime, presetRange } from '../../lib/dates'
import { formatMoney } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { PAYMENT_STATUSES, SALE_STATUSES } from './constants'
import { PaymentStatusBadge, SaleStatusBadge } from './labels'

const linkButton = 'inline-flex min-h-11 items-center rounded-lg px-4 text-sm font-semibold'

/** Sales list (E5): filters, summary and "load more" paging. */
export function SalesPage() {
  const { t, i18n } = useTranslation()
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const lang = i18n.language
  const [text, setText] = useState('')
  const [filter, setFilter] = useState<SaleFilter>(() => presetRange('thisMonth'))
  const sales = useSales(tenantId, filter)
  const set = (patch: Partial<SaleFilter>) => setFilter((f) => ({ ...f, ...patch }))

  function onSearch(e: FormEvent) {
    e.preventDefault()
    set({ q: text.trim() || undefined })
  }

  const rows = sales.data?.pages.flatMap((p) => p.data) ?? []
  const summary = sales.data?.pages[0]?.summary
  const money = (v: string) => formatMoney(v, lang)

  const columns: Column<Sale>[] = [
    {
      key: 'invoice',
      header: t('sales.invoiceNo'),
      render: (s) => (
        <Link to={`/sales/${s.id}`} className="font-medium text-brand-700 hover:underline">
          {s.invoice_no}
        </Link>
      ),
    },
    { key: 'date', header: t('sales.dateTime'), render: (s) => <span className="whitespace-nowrap">{formatDateTime(s.sale_date || s.created_at, lang)}</span> },
    {
      key: 'customer',
      header: t('sales.customer'),
      render: (s) => (
        <>
          {s.is_walk_in ? t('sales.walkIn') : s.customer_name}
          {s.customer_phone && !s.is_walk_in && <span className="block text-xs text-ink-muted">{s.customer_phone}</span>}
        </>
      ),
    },
    { key: 'cashier', header: t('sales.cashier'), render: (s) => s.cashier_name ?? '—' },
    { key: 'total', header: t('sales.grandTotal'), align: 'right', render: (s) => money(s.grand_total) },
    { key: 'paid', header: t('sales.paid'), align: 'right', render: (s) => money(s.paid_total) },
    { key: 'due', header: t('sales.due'), align: 'right', render: (s) => money(s.due_total) },
    { key: 'status', header: t('common.status'), render: (s) => <SaleStatusBadge status={s.status} /> },
    { key: 'payment', header: t('sales.payment'), render: (s) => <PaymentStatusBadge status={s.payment_status} /> },
  ]

  function exportCsv() {
    downloadCsv(`sales-${filter.from ?? ''}-${filter.to ?? ''}`, rows, [
      { header: t('sales.invoiceNo'), value: (s) => s.invoice_no },
      { header: t('sales.dateTime'), value: (s) => s.sale_date || s.created_at },
      { header: t('sales.customer'), value: (s) => (s.is_walk_in ? t('sales.walkIn') : s.customer_name) },
      { header: t('sales.phone'), value: (s) => s.customer_phone },
      { header: t('sales.cashier'), value: (s) => s.cashier_name },
      { header: t('sales.grandTotal'), value: (s) => s.grand_total },
      { header: t('sales.paid'), value: (s) => s.paid_total },
      { header: t('sales.due'), value: (s) => s.due_total },
      { header: t('sales.returned'), value: (s) => s.returned_total },
      { header: t('common.status'), value: (s) => t(`sales.statuses.${s.status}`) },
      { header: t('sales.payment'), value: (s) => t(`sales.paymentStatuses.${s.payment_status}`) },
    ])
  }

  return (
    <div className="flex max-w-6xl flex-col gap-6">
      <PageHeader
        title={t('sales.title')}
        subtitle={can('sales.view_all_users') ? undefined : t('sales.ownOnly')}
        actions={
          <>
            <Link to="/sales/returns" className={`${linkButton} border border-border bg-surface text-ink hover:bg-canvas`}>
              {t('sales.returnsTitle')}
            </Link>
            {can('pos.sell') && (
              <Link to="/pos" className={`${linkButton} bg-brand-600 text-white hover:bg-brand-700`}>
                {t('sales.newSale')}
              </Link>
            )}
          </>
        }
      />

      <div className="flex flex-col gap-3">
        <DateRangeFilter from={filter.from ?? ''} to={filter.to ?? ''} onChange={(r) => set(r)} />
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-[1fr_1fr_2fr]">
          <Select
            label={t('common.status')}
            value={filter.status ?? ''}
            onChange={(e) => set({ status: e.target.value as SaleStatus | '' })}
            options={[{ value: '', label: t('common.all') }, ...SALE_STATUSES.map((s) => ({ value: s, label: t(`sales.statuses.${s}`) }))]}
          />
          <Select
            label={t('sales.payment')}
            value={filter.payment_status ?? ''}
            onChange={(e) => set({ payment_status: e.target.value as PaymentStatus | '' })}
            options={[
              { value: '', label: t('common.all') },
              ...PAYMENT_STATUSES.map((s) => ({ value: s, label: t(`sales.paymentStatuses.${s}`) })),
            ]}
          />
          <form onSubmit={onSearch} role="search" className="flex items-end gap-2 sm:col-span-2 lg:col-span-1">
            <TextField
              className="flex-1"
              label={t('common.search')}
              type="search"
              value={text}
              placeholder={t('sales.searchHint')}
              onChange={(e) => setText(e.target.value)}
            />
            <Button type="submit">{t('common.search')}</Button>
          </form>
        </div>
      </div>

      {summary && (
        <div className="grid grid-cols-2 gap-3 md:grid-cols-5">
          <StatCard label={t('sales.summary.count')} value={summary.count} />
          <StatCard label={t('sales.summary.total')} value={money(summary.grand_total)} />
          <StatCard label={t('sales.summary.paid')} value={money(summary.paid_total)} tone="good" />
          <StatCard label={t('sales.summary.due')} value={money(summary.due_total)} tone="bad" />
          <StatCard label={t('sales.summary.returned')} value={money(summary.returned_total)} />
        </div>
      )}

      {sales.isPending ? (
        <Spinner />
      ) : sales.isError ? (
        <Alert tone="danger">{problemMessage(sales.error, t)}</Alert>
      ) : (
        <>
          <div className="flex justify-end">
            <Button variant="secondary" disabled={rows.length === 0} onClick={exportCsv}>
              {t('common.exportCsv')}
            </Button>
          </div>
          <DataTable caption={t('sales.title')} columns={columns} rows={rows} rowKey={(s) => s.id} empty={t('sales.noSales')} />
        </>
      )}
      {sales.hasNextPage && (
        <Button variant="secondary" className="self-center" disabled={sales.isFetchingNextPage} onClick={() => void sales.fetchNextPage()}>
          {sales.isFetchingNextPage ? t('common.loading') : t('common.loadMore')}
        </Button>
      )}
    </div>
  )
}
