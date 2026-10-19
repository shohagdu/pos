import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate } from 'react-router'

import { type PurchaseReturn, usePurchaseReturns } from '../../api/purchasing'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { DateRangeFilter } from '../../components/ui/DateRangeFilter'
import { PageHeader } from '../../components/ui/PageHeader'
import { Spinner } from '../../components/ui/Spinner'
import { downloadCsv } from '../../lib/csv'
import { formatDate } from '../../lib/dates'
import { formatMoney } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { LINK } from './helpers'

/** `/purchases/returns`: goods sent back to suppliers. */
export function PurchaseReturnsPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const navigate = useNavigate()
  const { me } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const [range, setRange] = useState({ from: '', to: '' })
  const returns = usePurchaseReturns(tenantId, { from: range.from || undefined, to: range.to || undefined })
  const rows = returns.data?.pages.flatMap((p) => p.data) ?? []
  const money = (v: string) => formatMoney(v, lang)

  const columns: Column<PurchaseReturn>[] = [
    {
      key: 'no',
      header: t('purchases.returns.number'),
      render: (r) => (
        <Link to={`/purchases/returns/${r.id}`} className={LINK}>
          {r.return_no}
        </Link>
      ),
    },
    { key: 'date', header: t('common.date'), render: (r) => formatDate(r.business_date, lang) },
    {
      key: 'bill',
      header: t('purchases.returns.bill'),
      render: (r) => (
        <Link to={`/purchases/${r.purchase_id}`} className={LINK} onClick={(e) => e.stopPropagation()}>
          {r.purchase_no ?? '—'}
        </Link>
      ),
    },
    { key: 'supplier', header: t('purchases.supplier'), render: (r) => r.supplier_name },
    { key: 'mode', header: t('purchases.return.settlement'), render: (r) => t(`purchases.return.modes.${r.settlement_mode}`) },
    { key: 'total', header: t('common.total'), align: 'right', render: (r) => money(r.total) },
  ]

  return (
    <div className="flex max-w-6xl flex-col gap-6">
      <Link to="/purchases" className={`text-sm print:hidden ${LINK}`}>
        ← {t('purchases.title')}
      </Link>
      <PageHeader title={t('purchases.returns.title')} />
      <div className="flex flex-wrap items-end justify-between gap-3 print:hidden">
        <DateRangeFilter from={range.from} to={range.to} onChange={setRange} />
        <Button
          variant="secondary"
          disabled={rows.length === 0}
          onClick={() =>
            downloadCsv(`purchase-returns-${range.from}-${range.to}`, rows, [
              { header: t('purchases.returns.number'), value: (r) => r.return_no },
              { header: t('common.date'), value: (r) => r.business_date },
              { header: t('purchases.returns.bill'), value: (r) => r.purchase_no },
              { header: t('purchases.supplier'), value: (r) => r.supplier_name },
              { header: t('common.reason'), value: (r) => r.reason },
              { header: t('purchases.return.settlement'), value: (r) => t(`purchases.return.modes.${r.settlement_mode}`) },
              { header: t('common.total'), value: (r) => r.total },
            ])
          }
        >
          {t('common.exportCsv')}
        </Button>
      </div>
      {returns.isPending ? (
        <Spinner />
      ) : returns.isError ? (
        <Alert tone="danger">{problemMessage(returns.error, t)}</Alert>
      ) : (
        <DataTable
          caption={t('purchases.returns.title')}
          columns={columns}
          rows={rows}
          rowKey={(r) => r.id}
          onRowClick={(r) => navigate(`/purchases/returns/${r.id}`)}
          empty={t('purchases.returns.none')}
        />
      )}
      {returns.hasNextPage && (
        <Button variant="secondary" className="self-center print:hidden" disabled={returns.isFetchingNextPage} onClick={() => void returns.fetchNextPage()}>
          {returns.isFetchingNextPage ? t('common.loading') : t('common.loadMore')}
        </Button>
      )}
    </div>
  )
}
