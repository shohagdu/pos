import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { type PaymentStatus, type Sale, type SaleFilter, type SaleStatus, useSales } from '../../api/sales'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Checkbox } from '../../components/ui/Checkbox'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { DateRangeFilter } from '../../components/ui/DateRangeFilter'
import { PageHeader } from '../../components/ui/PageHeader'
import { Select } from '../../components/ui/Select'
import { Spinner } from '../../components/ui/Spinner'
import { StatCard } from '../../components/ui/StatCard'
import { TextField } from '../../components/ui/TextField'
import { downloadCsv } from '../../lib/csv'
import { formatDateTime, presetRange } from '../../lib/dates'
import { ZERO, dec } from '../../lib/decimal'
import { formatMoney } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { PAYMENT_STATUSES, SALE_STATUSES } from './constants'
import { PaymentStatusBadge, SaleStatusBadge } from './labels'
import { WarrantyStatus } from './WarrantyStatus'

const linkButton = 'inline-flex min-h-11 items-center rounded-lg px-4 text-sm font-semibold'

/** Sales list (E5): filters, summary and "load more" paging. */
export function SalesPage() {
  const { t, i18n } = useTranslation()
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const lang = i18n.language
  const [text, setText] = useState('')
  const [filter, setFilter] = useState<SaleFilter>(() => presetRange('thisMonth'))
  // Below lg the filters fold away behind a button; search stays visible.
  const [showFilters, setShowFilters] = useState(false)
  const activeFilters = [filter.status, filter.payment_status, filter.with_warranty].filter(Boolean).length
  const folded = showFilters ? '' : 'max-lg:hidden'
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
        <Link to={`/sales/${s.id}`} className="font-medium whitespace-nowrap text-brand-700 hover:underline">
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
    { key: 'cashier', header: t('sales.cashier'), showFrom: '2xl', render: (s) => s.cashier_name ?? '—' },
    { key: 'total', header: t('sales.grandTotal'), align: 'right', render: (s) => money(s.grand_total) },
    { key: 'paid', header: t('sales.paid'), align: 'right', showFrom: '2xl', render: (s) => money(s.paid_total) },
    { key: 'due', header: t('sales.due'), align: 'right', render: (s) => money(s.due_total) },
    { key: 'status', header: t('common.status'), render: (s) => <SaleStatusBadge status={s.status} /> },
    { key: 'payment', header: t('sales.payment'), render: (s) => <PaymentStatusBadge status={s.payment_status} /> },
    { key: 'warranty', header: t('sales.warranty'), render: (s) => <WarrantyStatus sale={s} /> },
  ]

  // Below xl: one card per sale; the whole card opens the invoice.
  const card = (s: Sale) => (
    <div className="relative flex flex-col gap-1.5">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <Link
            to={`/sales/${s.id}`}
            className="font-semibold text-brand-700 after:absolute after:inset-0 after:content-[''] hover:underline focus-visible:outline-none focus-visible:after:rounded-lg focus-visible:after:ring-2 focus-visible:after:ring-brand-500"
          >
            {s.invoice_no}
          </Link>
          <div className="text-xs text-ink-muted">{formatDateTime(s.sale_date || s.created_at, lang)}</div>
        </div>
        <span className="text-base font-bold whitespace-nowrap tabular-nums">{money(s.grand_total)}</span>
      </div>
      <div className="flex flex-wrap items-center justify-between gap-x-3 gap-y-1">
        <span className="min-w-0 break-words">
          {s.is_walk_in ? t('sales.walkIn') : s.customer_name}
          {s.customer_phone && !s.is_walk_in && <span className="text-ink-muted"> · {s.customer_phone}</span>}
        </span>
        {s.cashier_name && (
          <span className="text-xs text-ink-muted">
            {t('sales.cashier')}: {s.cashier_name}
          </span>
        )}
      </div>
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1.5 text-xs">
        <span className="tabular-nums">
          <span className="text-ink-muted">{t('sales.paid')}</span> {money(s.paid_total)}
        </span>
        <span className={`tabular-nums ${dec(s.due_total) > ZERO ? 'font-semibold text-danger' : ''}`}>
          <span className="font-normal text-ink-muted">{t('sales.due')}</span> {money(s.due_total)}
        </span>
        <span className="flex flex-wrap items-center gap-1.5">
          <SaleStatusBadge status={s.status} />
          <PaymentStatusBadge status={s.payment_status} />
        </span>
        {s.warranty_until && s.warranty_items > 0 && <WarrantyStatus sale={s} />}
      </div>
    </div>
  )

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
      { header: t('sales.warranty'), value: (s) => s.warranty_until },
    ])
  }

  return (
    <div className="flex max-w-6xl flex-col gap-6">
      <PageHeader
        title={t('sales.title')}
        subtitle={can('sales.view_all_users') ? undefined : t('sales.ownOnly')}
        actions={
          <>
            <Link to="/sales/warranties" className={`${linkButton} border border-border bg-surface text-ink hover:bg-canvas`}>
              {t('sales.warrantyLookup.title')}
            </Link>
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

      <div className="flex flex-col gap-3 lg:grid lg:grid-cols-[1fr_1fr_2fr]">
        <div id="sales-filters" className={`lg:col-span-3 ${folded}`}>
          <DateRangeFilter from={filter.from ?? ''} to={filter.to ?? ''} onChange={(r) => set(r)} />
        </div>
        <div className={`grid grid-cols-1 gap-3 sm:grid-cols-2 lg:contents ${folded}`}>
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
            options={[{ value: '', label: t('common.all') }, ...PAYMENT_STATUSES.map((s) => ({ value: s, label: t(`sales.paymentStatuses.${s}`) }))]}
          />
        </div>
        <form onSubmit={onSearch} role="search" className="order-first flex items-end gap-2 lg:order-none">
          <TextField
            className="min-w-0 flex-1"
            label={t('common.search')}
            type="search"
            value={text}
            placeholder={t('sales.searchHint')}
            onChange={(e) => setText(e.target.value)}
          />
          <Button type="submit">{t('common.search')}</Button>
          <Button variant="secondary" className="lg:hidden" aria-expanded={showFilters} aria-controls="sales-filters" onClick={() => setShowFilters((v) => !v)}>
            {activeFilters ? t('sales.filtersCount', { count: activeFilters }) : t('sales.filters')}
          </Button>
        </form>
        <Checkbox
          className={`lg:col-span-3 ${folded}`}
          label={t('sales.withWarranty')}
          checked={filter.with_warranty ?? false}
          onChange={(e) => set({ with_warranty: e.target.checked || undefined })}
        />
      </div>

      {summary && (
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 xl:grid-cols-5 max-sm:[&>*:last-child:nth-child(odd)]:col-span-2">
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
          <div className="-mt-2 flex justify-end">
            <Button variant="secondary" disabled={rows.length === 0} onClick={exportCsv}>
              {t('common.exportCsv')}
            </Button>
          </div>
          <DataTable caption={t('sales.title')} columns={columns} rows={rows} rowKey={(s) => s.id} empty={t('sales.noSales')} card={card} tableFrom="xl" />
        </>
      )}
      {sales.hasNextPage && (
        <Button variant="secondary" className="self-stretch sm:self-center" disabled={sales.isFetchingNextPage} onClick={() => void sales.fetchNextPage()}>
          {sales.isFetchingNextPage ? t('common.loading') : t('common.loadMore')}
        </Button>
      )}
    </div>
  )
}
