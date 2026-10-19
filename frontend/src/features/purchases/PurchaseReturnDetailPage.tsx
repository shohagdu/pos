import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useParams } from 'react-router'

import { type PurchaseReturnDetail, usePurchaseReturn } from '../../api/purchasing'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { PageHeader } from '../../components/ui/PageHeader'
import { Spinner } from '../../components/ui/Spinner'
import { formatDate, formatDateTime } from '../../lib/dates'
import { formatMoney, formatQuantity } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { LINK } from './helpers'

type Item = PurchaseReturnDetail['items'][number]

/** `/purchases/returns/:id`. */
export function PurchaseReturnDetailPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { id = '' } = useParams()
  const { me } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const ret = usePurchaseReturn(tenantId, id)
  if (ret.isPending) return <Spinner />
  if (ret.isError) return <Alert tone="danger">{problemMessage(ret.error, t)}</Alert>
  const r = ret.data
  const money = (v: string) => formatMoney(v, lang)
  const columns: Column<Item>[] = [
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
    { key: 'qty', header: t('common.quantity'), align: 'right', render: (i) => formatQuantity(i.base_quantity, lang, true) },
    { key: 'cost', header: t('purchases.returns.unitCostOut'), align: 'right', render: (i) => formatMoney(i.unit_cost_out, lang) },
    { key: 'total', header: t('purchases.lineTotal'), align: 'right', render: (i) => money(i.line_total) },
  ]
  return (
    <div className="flex max-w-5xl flex-col gap-6">
      <Link to="/purchases/returns" className={`text-sm print:hidden ${LINK}`}>
        ← {t('purchases.returns.title')}
      </Link>
      <PageHeader
        title={r.return_no}
        subtitle={t('purchases.returns.subtitle')}
        actions={
          <Button variant="secondary" className="print:hidden" onClick={() => window.print()}>
            {t('common.print')}
          </Button>
        }
      />
      <div className="grid gap-6 lg:grid-cols-[2fr_1fr]">
        <Card>
          <dl className="grid gap-x-6 gap-y-3 text-sm sm:grid-cols-2">
            <Info label={t('purchases.supplier')}>
              <Link to={`/suppliers/${r.supplier_id}`} className={LINK}>
                {r.supplier_name}
              </Link>
            </Info>
            <Info label={t('purchases.returns.bill')}>
              <Link to={`/purchases/${r.purchase_id}`} className={LINK}>
                {r.purchase_no ?? '—'}
              </Link>
            </Info>
            <Info label={t('common.date')}>{formatDate(r.business_date, lang)}</Info>
            <Info label={t('purchases.createdBy')}>{`${r.created_by_name ?? '—'} · ${formatDateTime(r.created_at, lang)}`}</Info>
            <Info label={t('common.reason')}>{r.reason}</Info>
            <Info label={t('purchases.return.settlement')}>{t(`purchases.return.modes.${r.settlement_mode}`)}</Info>
          </dl>
        </Card>
        <Card>
          <dl className="flex flex-col gap-1 text-sm" aria-label={t('common.totals')}>
            <Row label={t('common.total')} value={money(r.total)} strong />
            <Row label={t('purchases.returns.vatIncluded')} value={money(r.tax_total)} />
            <Row label={t('purchases.returns.appliedToDue')} value={money(r.applied_to_due)} />
            <Row label={t('purchases.returns.refunded')} value={money(r.refunded_amount)} />
            <Row label={t('purchases.returns.credited')} value={money(r.credited_amount)} />
          </dl>
        </Card>
      </div>
      <DataTable caption={t('purchases.return.lines')} columns={columns} rows={r.items} rowKey={(i) => i.id} />
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

function Row({ label, value, strong = false }: { label: string; value: string; strong?: boolean }) {
  return (
    <div className={`flex justify-between gap-4 ${strong ? 'font-bold' : ''}`}>
      <dt className={strong ? '' : 'text-ink-muted'}>{label}</dt>
      <dd className="tabular-nums">{value}</dd>
    </div>
  )
}
