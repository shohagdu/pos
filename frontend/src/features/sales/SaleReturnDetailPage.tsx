import { useTranslation } from 'react-i18next'
import { Link, useParams } from 'react-router'

import { type SaleReturnItem, useSaleReturn } from '../../api/sales'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { PageHeader } from '../../components/ui/PageHeader'
import { Spinner } from '../../components/ui/Spinner'
import { ZERO, dec } from '../../lib/decimal'
import { formatDate, formatDateTime } from '../../lib/dates'
import { formatMoney, formatQuantity } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { Info } from './Info'
import { PrintOnly, ScreenOnly } from './print'
import { ReturnReceipt } from './Receipt'
import { usePrint } from './usePrint'

/** One sales return with its items and a printable return receipt. */
export function SaleReturnDetailPage() {
  const { id = '' } = useParams()
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { me } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const ret = useSaleReturn(tenantId, id)
  const print = usePrint()

  if (ret.isPending) return <Spinner />
  if (ret.isError) return <Alert tone="danger">{problemMessage(ret.error, t)}</Alert>
  const r = ret.data
  const money = (v: string) => formatMoney(v, lang)

  const columns: Column<SaleReturnItem>[] = [
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
      key: 'condition',
      header: t('sales.condition'),
      render: (i) => <Badge tone={i.item_condition === 'damaged' ? 'warn' : 'neutral'}>{t(`sales.conditions.${i.item_condition}`)}</Badge>,
    },
    { key: 'vat', header: t('sales.vat'), align: 'right', render: (i) => money(i.tax_amount) },
    { key: 'total', header: t('common.total'), align: 'right', render: (i) => money(i.line_total) },
  ]

  const amounts: [string, string][] = [
    [t('sales.subtotal'), r.subtotal],
    [t('sales.vat'), r.tax_total],
    [t('common.total'), r.total],
    [t('sales.receipt.appliedToDue'), r.applied_to_due],
    [t('sales.receipt.refunded'), r.refunded_amount],
    [t('sales.receipt.credited'), r.credited_amount],
  ]
  if (r.cost_total !== null) amounts.push([t('sales.costTotal'), r.cost_total])

  return (
    <>
      <ScreenOnly className="flex max-w-5xl flex-col gap-6">
        <Link to="/sales/returns" className="text-sm font-medium text-brand-700 hover:underline">
          ← {t('sales.returnsTitle')}
        </Link>
        <PageHeader
          title={t('sales.returnDetailTitle', { no: r.return_no })}
          actions={<Button onClick={print}>{t('common.print')}</Button>}
        />
        <Card className="grid gap-x-6 gap-y-3 text-sm sm:grid-cols-2 lg:grid-cols-3">
          <Info
            label={t('sales.invoiceNo')}
            value={
              <Link to={`/sales/${r.sale_id}`} className="text-brand-700 hover:underline">
                {r.invoice_no}
              </Link>
            }
          />
          <Info label={t('sales.customer')} value={r.customer_name} />
          <Info label={t('sales.businessDate')} value={formatDate(r.business_date, lang)} />
          <Info label={t('sales.dateTime')} value={formatDateTime(r.created_at, lang)} />
          <Info label={t('sales.createdBy')} value={r.created_by_name ?? '—'} />
          <Info label={t('sales.refundMode')} value={t(`sales.refundModes.${r.refund_mode}`)} />
          <Info label={t('common.reason')} value={r.reason} />
        </Card>
        <div className="grid gap-6 lg:grid-cols-[1fr_20rem]">
          <DataTable caption={t('sales.items')} columns={columns} rows={r.items} rowKey={(i) => i.id} />
          <Card className="self-start p-4">
            <dl className="flex flex-col gap-1 text-sm">
              {amounts
                .filter(([, v], i) => i < 3 || dec(v) !== ZERO)
                .map(([label, v]) => (
                  <div key={label} className="flex justify-between gap-3">
                    <dt>{label}</dt>
                    <dd className="tabular-nums">{money(v)}</dd>
                  </div>
                ))}
            </dl>
          </Card>
        </div>
      </ScreenOnly>
      <PrintOnly>
        <ReturnReceipt ret={r} />
      </PrintOnly>
    </>
  )
}
