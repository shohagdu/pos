import { useTranslation } from 'react-i18next'

import type { PaymentClaim, SubscriptionInvoice } from '../../api/billing'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { DataTable } from '../../components/ui/DataTable'
import { formatDate, formatDateTime } from '../../lib/dates'
import { formatMoney } from '../../lib/format'

const INVOICE_TONE = { open: 'warn', paid: 'good', void: 'neutral' } as const
const CLAIM_TONE = { pending: 'warn', approved: 'good', rejected: 'bad' } as const

/** "Invoices and payments": subscription invoices and the payments reported for them. */
export function Invoices({
  invoices,
  claims,
  onReport,
}: {
  invoices: SubscriptionInvoice[]
  claims: PaymentClaim[]
  onReport: (invoice: SubscriptionInvoice) => void
}) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const pending = new Set(claims.filter((c) => c.status === 'pending').map((c) => c.invoice_id))
  const planName = (code: string, name: string) => t(`plan.names.${code}`, { defaultValue: name })

  return (
    <section aria-labelledby="invoices-title" className="flex flex-col gap-4">
      <h2 id="invoices-title" className="text-lg font-semibold">
        {t('billing.invoices.title')}
      </h2>
      <h3 className="font-semibold">{t('billing.invoices.invoices')}</h3>
      <DataTable
        caption={t('billing.invoices.invoices')}
        rows={invoices}
        rowKey={(r) => r.id}
        empty={t('billing.invoices.noInvoices')}
        columns={[
          { key: 'no', header: t('billing.invoice.number'), render: (r) => <span className="font-mono text-xs">{r.invoice_no}</span> },
          { key: 'plan', header: t('billing.invoice.plan'), render: (r) => planName(r.plan_code, r.plan_name) },
          { key: 'cycle', header: t('billing.invoice.cycle'), render: (r) => t(`billing.cycles.${r.billing_cycle}`) },
          {
            key: 'period',
            header: t('billing.invoice.period'),
            className: 'whitespace-nowrap',
            render: (r) => `${formatDate(r.period_start, lang)} → ${formatDate(r.period_end, lang)}`,
          },
          { key: 'total', header: t('billing.invoice.total'), align: 'right', render: (r) => formatMoney(r.total, lang) },
          { key: 'status', header: t('common.status'), render: (r) => <Badge tone={INVOICE_TONE[r.status]}>{t(`billing.invoiceStatus.${r.status}`)}</Badge> },
          { key: 'paid', header: t('billing.invoice.paidAt'), render: (r) => formatDate(r.paid_at, lang) },
          {
            key: 'action',
            header: <span className="sr-only">{t('common.actions')}</span>,
            render: (r) =>
              r.status !== 'open' ? null : pending.has(r.id) ? (
                <span className="text-xs text-ink-muted">{t('billing.invoices.awaiting')}</span>
              ) : (
                <Button variant="secondary" onClick={() => onReport(r)} aria-label={t('billing.invoices.reportFor', { no: r.invoice_no })}>
                  {t('billing.invoices.report')}
                </Button>
              ),
          },
        ]}
      />
      <h3 className="font-semibold">{t('billing.invoices.claims')}</h3>
      <DataTable
        caption={t('billing.invoices.claims')}
        rows={claims}
        rowKey={(r) => r.id}
        empty={t('billing.invoices.noClaims')}
        columns={[
          { key: 'date', header: t('common.date'), className: 'whitespace-nowrap', render: (r) => formatDateTime(r.created_at, lang) },
          { key: 'no', header: t('billing.invoice.number'), render: (r) => <span className="font-mono text-xs">{r.invoice_no}</span> },
          { key: 'method', header: t('billing.claim.method'), render: (r) => t(`billing.methods.${r.method}`) },
          { key: 'amount', header: t('common.amount'), align: 'right', render: (r) => formatMoney(r.amount, lang) },
          { key: 'ref', header: t('billing.claim.reference'), render: (r) => <span className="font-mono text-xs">{r.transaction_ref}</span> },
          { key: 'status', header: t('common.status'), render: (r) => <Badge tone={CLAIM_TONE[r.status]}>{t(`billing.claimStatus.${r.status}`)}</Badge> },
          { key: 'reason', header: t('billing.invoices.rejection'), render: (r) => r.rejection_reason ?? '—' },
        ]}
      />
    </section>
  )
}
