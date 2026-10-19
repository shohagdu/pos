import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate } from 'react-router'

import { type Purchase, type PurchaseFilter, type PurchaseStatus, usePurchases } from '../../api/purchasing'
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
import { formatDate } from '../../lib/dates'
import { formatMoney } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { BUTTON_LINK, LINK, SECONDARY_LINK } from './helpers'
import { PaymentBadge, StatusBadge } from './shared'

const STATUSES: PurchaseStatus[] = ['draft', 'received', 'cancelled']
const PAYMENT_STATUSES = ['unpaid', 'partial', 'paid', 'refunded'] as const

/** `/purchases`: supplier bills with filters, totals and CSV. */
export function PurchasesPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const navigate = useNavigate()
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const [text, setText] = useState('')
  const [filter, setFilter] = useState<PurchaseFilter>({})
  const purchases = usePurchases(tenantId, filter)
  const set = (patch: Partial<PurchaseFilter>) => setFilter((f) => ({ ...f, ...patch }))

  function onSearch(e: FormEvent) {
    e.preventDefault()
    set({ q: text.trim() || undefined })
  }

  const rows = purchases.data?.pages.flatMap((p) => p.data) ?? []
  const summary = purchases.data?.pages[0]?.summary
  const money = (v: string) => formatMoney(v, lang)
  const number = (p: Purchase) => p.purchase_no ?? t('purchases.draft')

  const columns: Column<Purchase>[] = [
    {
      key: 'no',
      header: t('purchases.number'),
      render: (p) => (
        <Link to={`/purchases/${p.id}`} className={LINK}>
          {number(p)}
        </Link>
      ),
    },
    { key: 'date', header: t('common.date'), render: (p) => formatDate(p.purchase_date, lang) },
    { key: 'supplier', header: t('purchases.supplier'), render: (p) => p.supplier_name },
    { key: 'bill', header: t('purchases.supplierInvoiceNo'), render: (p) => p.supplier_invoice_no ?? '—' },
    { key: 'total', header: t('common.total'), align: 'right', render: (p) => money(p.grand_total) },
    { key: 'paid', header: t('purchases.paid'), align: 'right', render: (p) => money(p.paid_total) },
    { key: 'due', header: t('purchases.due'), align: 'right', render: (p) => money(p.due_total) },
    {
      key: 'status',
      header: t('common.status'),
      render: (p) => (
        <span className="flex flex-wrap gap-1">
          <StatusBadge status={p.status} />
          {p.status === 'received' && <PaymentBadge status={p.payment_status} />}
        </span>
      ),
    },
  ]

  function exportCsv() {
    downloadCsv(`purchases-${filter.from ?? ''}-${filter.to ?? ''}`, rows, [
      { header: t('purchases.number'), value: number },
      { header: t('common.date'), value: (p) => p.purchase_date },
      { header: t('purchases.supplier'), value: (p) => p.supplier_name },
      { header: t('purchases.supplierInvoiceNo'), value: (p) => p.supplier_invoice_no },
      { header: t('common.total'), value: (p) => p.grand_total },
      { header: t('purchases.paid'), value: (p) => p.paid_total },
      { header: t('purchases.due'), value: (p) => p.due_total },
      { header: t('common.status'), value: (p) => t(`purchases.statuses.${p.status}`) },
      { header: t('purchases.paymentStatus'), value: (p) => t(`purchases.paymentStatuses.${p.payment_status}`) },
    ])
  }

  return (
    <div className="flex max-w-6xl flex-col gap-6">
      <PageHeader
        title={t('purchases.title')}
        actions={
          <>
            <Link to="/purchases/returns" className={SECONDARY_LINK}>
              {t('purchases.returns.title')}
            </Link>
            {can('purchases.create') && (
              <Link to="/purchases/new" className={BUTTON_LINK}>
                {t('purchases.new')}
              </Link>
            )}
          </>
        }
      />
      <div className="flex flex-col gap-3 print:hidden">
        <form onSubmit={onSearch} className="flex flex-wrap items-end gap-2" role="search">
          <TextField
            className="min-w-60 flex-1"
            label={t('common.search')}
            type="search"
            value={text}
            onChange={(e) => setText(e.target.value)}
            placeholder={t('purchases.searchHint')}
          />
          <Button type="submit">{t('common.search')}</Button>
        </form>
        <div className="flex flex-wrap items-end gap-3">
          <DateRangeFilter
            from={filter.from ?? ''}
            to={filter.to ?? ''}
            onChange={({ from, to }) => set({ from: from || undefined, to: to || undefined })}
          />
          <Select
            label={t('common.status')}
            value={filter.status ?? ''}
            onChange={(e) => set({ status: (e.target.value || undefined) as PurchaseStatus | undefined })}
            options={[{ value: '', label: t('common.all') }, ...STATUSES.map((s) => ({ value: s, label: t(`purchases.statuses.${s}`) }))]}
          />
          <Select
            label={t('purchases.paymentStatus')}
            value={filter.payment_status ?? ''}
            onChange={(e) => set({ payment_status: e.target.value || undefined })}
            options={[{ value: '', label: t('common.all') }, ...PAYMENT_STATUSES.map((s) => ({ value: s, label: t(`purchases.paymentStatuses.${s}`) }))]}
          />
          <Button variant="secondary" onClick={exportCsv} disabled={rows.length === 0}>
            {t('common.exportCsv')}
          </Button>
        </div>
      </div>
      {summary && (
        <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
          <StatCard label={t('purchases.summary.count')} value={summary.count} />
          <StatCard label={t('purchases.summary.total')} value={money(summary.grand_total)} />
          <StatCard label={t('purchases.summary.paid')} value={money(summary.paid_total)} tone="good" />
          <StatCard label={t('purchases.summary.due')} value={money(summary.due_total)} tone="bad" />
        </div>
      )}
      {purchases.isPending ? (
        <Spinner />
      ) : purchases.isError ? (
        <Alert tone="danger">{problemMessage(purchases.error, t)}</Alert>
      ) : (
        <DataTable
          caption={t('purchases.title')}
          columns={columns}
          rows={rows}
          rowKey={(p) => p.id}
          onRowClick={(p) => navigate(`/purchases/${p.id}`)}
          empty={filter.q || filter.status || filter.from ? t('common.noResults') : t('purchases.none')}
        />
      )}
      {purchases.hasNextPage && (
        <Button variant="secondary" className="self-center print:hidden" disabled={purchases.isFetchingNextPage} onClick={() => void purchases.fetchNextPage()}>
          {purchases.isFetchingNextPage ? t('common.loading') : t('common.loadMore')}
        </Button>
      )}
    </div>
  )
}
