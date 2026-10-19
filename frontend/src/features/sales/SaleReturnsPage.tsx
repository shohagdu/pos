import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { type ReturnFilter, type SaleReturn, useSaleReturns } from '../../api/sales'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { DateRangeFilter } from '../../components/ui/DateRangeFilter'
import { PageHeader } from '../../components/ui/PageHeader'
import { Spinner } from '../../components/ui/Spinner'
import { StatCard } from '../../components/ui/StatCard'
import { TextField } from '../../components/ui/TextField'
import { formatDate, presetRange } from '../../lib/dates'
import { formatMoney } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'

/** Sales returns list. */
export function SaleReturnsPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { me } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const [text, setText] = useState('')
  const [filter, setFilter] = useState<ReturnFilter>(() => presetRange('thisMonth'))
  const returns = useSaleReturns(tenantId, filter)
  const money = (v: string) => formatMoney(v, lang)

  function onSearch(e: FormEvent) {
    e.preventDefault()
    setFilter((f) => ({ ...f, q: text.trim() || undefined }))
  }

  const rows = returns.data?.pages.flatMap((p) => p.data) ?? []
  const total = returns.data?.pages[0]?.total
  const columns: Column<SaleReturn>[] = [
    {
      key: 'no',
      header: t('sales.returnNo'),
      render: (r) => (
        <Link to={`/sales/returns/${r.id}`} className="font-medium text-brand-700 hover:underline">
          {r.return_no}
        </Link>
      ),
    },
    { key: 'date', header: t('common.date'), render: (r) => <span className="whitespace-nowrap">{formatDate(r.business_date, lang)}</span> },
    {
      key: 'invoice',
      header: t('sales.invoiceNo'),
      render: (r) => (
        <Link to={`/sales/${r.sale_id}`} className="text-brand-700 hover:underline">
          {r.invoice_no}
        </Link>
      ),
    },
    { key: 'customer', header: t('sales.customer'), render: (r) => r.customer_name },
    { key: 'mode', header: t('sales.refundMode'), render: (r) => t(`sales.refundModes.${r.refund_mode}`) },
    { key: 'by', header: t('sales.createdBy'), render: (r) => r.created_by_name ?? '—' },
    { key: 'total', header: t('common.total'), align: 'right', render: (r) => money(r.total) },
  ]

  return (
    <div className="flex max-w-6xl flex-col gap-6">
      <Link to="/sales" className="text-sm font-medium text-brand-700 hover:underline">
        ← {t('sales.backToSales')}
      </Link>
      <PageHeader title={t('sales.returnsTitle')} />
      <div className="flex flex-wrap items-end gap-3">
        <DateRangeFilter from={filter.from ?? ''} to={filter.to ?? ''} onChange={(r) => setFilter((f) => ({ ...f, ...r }))} />
        <form onSubmit={onSearch} role="search" className="flex min-w-64 flex-1 items-end gap-2">
          <TextField
            className="flex-1"
            label={t('common.search')}
            type="search"
            value={text}
            placeholder={t('sales.returnSearchHint')}
            onChange={(e) => setText(e.target.value)}
          />
          <Button type="submit">{t('common.search')}</Button>
        </form>
      </div>
      {total !== undefined && (
        <div className="grid max-w-xs">
          <StatCard label={t('sales.summary.returned')} value={money(total)} />
        </div>
      )}
      {returns.isPending ? (
        <Spinner />
      ) : returns.isError ? (
        <Alert tone="danger">{problemMessage(returns.error, t)}</Alert>
      ) : (
        <DataTable caption={t('sales.returnsTitle')} columns={columns} rows={rows} rowKey={(r) => r.id} empty={t('sales.noReturns')} />
      )}
      {returns.hasNextPage && (
        <Button variant="secondary" className="self-center" disabled={returns.isFetchingNextPage} onClick={() => void returns.fetchNextPage()}>
          {returns.isFetchingNextPage ? t('common.loading') : t('common.loadMore')}
        </Button>
      )}
    </div>
  )
}
