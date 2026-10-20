import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type ReactNode, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate, useParams } from 'react-router'

import { ApiError } from '../../api/client'
import { type Payment, usePayments } from '../../api/money'
import { type StatementEntry, type Supplier, partiesApi, partyKeys, useSupplier, useSupplierStatement } from '../../api/parties'
import { type Purchase, usePurchases } from '../../api/purchasing'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { ConfirmButton } from '../../components/ui/ConfirmButton'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { DateRangeFilter } from '../../components/ui/DateRangeFilter'
import { PageHeader } from '../../components/ui/PageHeader'
import { Spinner } from '../../components/ui/Spinner'
import { StateTabs } from '../../components/ui/Tabs'
import { dec } from '../../lib/decimal'
import { formatDate } from '../../lib/dates'
import { formatMoney } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { LINK, SECONDARY_LINK } from '../purchases/helpers'
import { PaySupplierDialog, PaymentBadge, StatusBadge as PurchaseStatusBadge, VoidPaymentDialog } from '../purchases/shared'
import { supplierInput } from './input'
import { BalanceBadge, StatusBadge } from './shared'

type Tab = 'statement' | 'bills' | 'payments'

/** `/suppliers/:id`: balance, actions and statement / bills / payments. */
export function SupplierDetailPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { id = '' } = useParams()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const supplier = useSupplier(tenantId, id)
  const [tab, setTab] = useState<Tab>('statement')
  const [paying, setPaying] = useState(false)

  const refresh = () => queryClient.invalidateQueries({ queryKey: partyKeys.suppliers(tenantId) })
  const toggle = useMutation({
    mutationFn: (s: Supplier) => partiesApi.updateSupplier(s.id, s.version, supplierInput(s, { status: s.status === 'active' ? 'inactive' : 'active' })),
    onSuccess: async (saved) => {
      queryClient.setQueryData(partyKeys.supplier(tenantId, saved.id), saved)
      await refresh()
    },
    onError: (err) => {
      if (err instanceof ApiError && err.status === 412) void refresh()
    },
  })
  const remove = useMutation({
    mutationFn: () => partiesApi.deleteSupplier(id),
    onSuccess: async () => {
      await refresh()
      navigate('/suppliers', { replace: true })
    },
  })

  if (supplier.isPending) return <Spinner />
  if (supplier.isError) return <Alert tone="danger">{problemMessage(supplier.error, t)}</Alert>
  const s = supplier.data
  const balance = dec(s.current_balance)
  const inUse = remove.error instanceof ApiError && remove.error.problem.code === 'IN_USE'

  return (
    <div className="flex max-w-6xl flex-col gap-6">
      <Link to="/suppliers" className={`text-sm print:hidden ${LINK}`}>
        ← {t('suppliers.title')}
      </Link>
      <PageHeader
        title={s.name}
        subtitle={
          <span className="flex flex-wrap items-center gap-2">
            <span className="font-mono">{s.code}</span>
            <StatusBadge status={s.status} />
          </span>
        }
        actions={
          <div className="flex flex-wrap gap-2 print:hidden">
            {can('suppliers.pay') && s.status === 'active' && <Button onClick={() => setPaying(true)}>{t('suppliers.pay')}</Button>}
            {can('suppliers.manage') && (
              <>
                <Link to={`/suppliers/${s.id}/edit`} className={SECONDARY_LINK}>
                  {t('common.edit')}
                </Link>
                <Button variant="secondary" disabled={toggle.isPending} onClick={() => toggle.mutate(s)}>
                  {s.status === 'active' ? t('suppliers.deactivate') : t('suppliers.activate')}
                </Button>
                <ConfirmButton disabled={remove.isPending} onConfirm={() => remove.mutate()}>
                  {t('common.delete')}
                </ConfirmButton>
              </>
            )}
          </div>
        }
      />
      {inUse ? (
        <Alert tone="warning">{t('suppliers.inUse')}</Alert>
      ) : (
        remove.isError && <Alert tone="danger">{problemMessage(remove.error, t)}</Alert>
      )}
      {toggle.isError && <Alert tone="danger">{problemMessage(toggle.error, t)}</Alert>}

      <Card className="grid gap-4 sm:grid-cols-[1fr_auto]">
        <dl className="grid gap-x-6 gap-y-3 text-sm sm:grid-cols-2">
          <Info label={t('suppliers.contactPerson')}>{s.contact_person ?? '—'}</Info>
          <Info label={t('suppliers.phone')}>{s.phone ? <a href={`tel:${s.phone}`} className={LINK}>{s.phone}</a> : '—'}</Info>
          <Info label={t('suppliers.email')}>{s.email ?? '—'}</Info>
          <Info label={t('suppliers.address')}>{s.address ?? '—'}</Info>
          <Info label={t('suppliers.openingBalance')}>{formatMoney(s.opening_balance, lang)}</Info>
          {s.notes && <Info label={t('suppliers.notes')}>{s.notes}</Info>}
        </dl>
        <div className="flex flex-col items-start gap-1 sm:items-end">
          <span className="text-xs font-medium text-ink-muted uppercase">{t('suppliers.balance')}</span>
          <span className={`text-2xl font-bold tabular-nums ${balance > 0n ? 'text-danger' : 'text-ink'}`}>{formatMoney(s.current_balance.replace(/^-/, ''), lang)}</span>
          <BalanceBadge balance={s.current_balance} />
        </div>
      </Card>

      <div className="print:hidden">
        <StateTabs
          label={t('suppliers.tabs.label')}
          value={tab}
          onChange={setTab}
          items={[
            { value: 'statement', label: t('suppliers.tabs.statement') },
            { value: 'bills', label: t('suppliers.tabs.bills') },
            { value: 'payments', label: t('suppliers.tabs.payments') },
          ]}
        />
      </div>
      {tab === 'statement' && <StatementTab tenantId={tenantId} supplierId={s.id} />}
      {tab === 'bills' && <BillsTab tenantId={tenantId} supplierId={s.id} />}
      {tab === 'payments' && <PaymentsTab tenantId={tenantId} supplierId={s.id} />}

      {paying && (
        <PaySupplierDialog tenantId={tenantId} supplierId={s.id} defaultAmount={balance > 0n ? s.current_balance : ''} onClose={() => setPaying(false)} />
      )}
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

type StatementRow = { kind: 'opening' | 'closing'; balance: string } | { kind: 'entry'; entry: StatementEntry }

function StatementTab({ tenantId, supplierId }: { tenantId: string; supplierId: string }) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const [range, setRange] = useState({ from: '', to: '' })
  const statement = useSupplierStatement(tenantId, supplierId, range.from || undefined, range.to || undefined)
  const money = (v: string) => formatMoney(v, lang)
  const amount = (v: string) => (dec(v) === 0n ? '' : money(v))

  const columns: Column<StatementRow>[] = [
    { key: 'date', header: t('common.date'), render: (r) => (r.kind === 'entry' ? formatDate(r.entry.business_date, lang) : '') },
    {
      key: 'type',
      header: t('suppliers.statement.type'),
      render: (r) =>
        r.kind === 'entry' ? t(`suppliers.entryTypes.${r.entry.entry_type}`, { defaultValue: r.entry.entry_type }) : <strong>{t(`suppliers.statement.${r.kind}`)}</strong>,
    },
    {
      key: 'ref',
      header: t('suppliers.statement.reference'),
      render: (r) => {
        if (r.kind !== 'entry') return ''
        const e = r.entry
        const label = e.reference_no ?? '—'
        if (e.reference_type === 'purchase') return <Link to={`/purchases/${e.reference_id}`} className={LINK}>{label}</Link>
        if (e.reference_type === 'purchase_return') return <Link to={`/purchases/returns/${e.reference_id}`} className={LINK}>{label}</Link>
        return label
      },
    },
    { key: 'debit', header: t('suppliers.statement.debit'), align: 'right', render: (r) => (r.kind === 'entry' ? amount(r.entry.debit) : '') },
    { key: 'credit', header: t('suppliers.statement.credit'), align: 'right', render: (r) => (r.kind === 'entry' ? amount(r.entry.credit) : '') },
    {
      key: 'balance',
      header: t('suppliers.balance'),
      align: 'right',
      render: (r) => {
        const b = r.kind === 'entry' ? r.entry.balance : r.balance
        return r.kind === 'entry' ? money(b) : <strong>{money(b)}</strong>
      },
    },
  ]

  return (
    <section className="flex flex-col gap-4">
      <div className="flex flex-wrap items-end justify-between gap-3 print:hidden">
        <DateRangeFilter from={range.from} to={range.to} onChange={setRange} />
        <Button variant="secondary" onClick={() => window.print()}>
          {t('common.print')}
        </Button>
      </div>
      <p className="text-xs text-ink-muted">{t('suppliers.statement.hint')}</p>
      {statement.isPending ? (
        <Spinner />
      ) : statement.isError ? (
        <Alert tone="danger">{problemMessage(statement.error, t)}</Alert>
      ) : (
        <>
          <h2 className="hidden text-lg font-semibold print:block">
            {t('suppliers.statement.title', { name: statement.data.party_name })}
            {(range.from || range.to) && ` · ${formatDate(range.from, lang)} – ${formatDate(range.to, lang)}`}
          </h2>
          <DataTable
            caption={t('suppliers.tabs.statement')}
            columns={columns}
            rows={[
              { kind: 'opening', balance: statement.data.opening_balance },
              ...statement.data.entries.map((entry) => ({ kind: 'entry' as const, entry })),
              { kind: 'closing', balance: statement.data.closing_balance },
            ]}
            rowKey={(r, i) => (r.kind === 'entry' ? r.entry.id : `${r.kind}-${i}`)}
            footer={['', t('common.totals'), '', money(statement.data.total_debit), money(statement.data.total_credit), '']}
          />
        </>
      )}
    </section>
  )
}

function BillsTab({ tenantId, supplierId }: { tenantId: string; supplierId: string }) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const navigate = useNavigate()
  const bills = usePurchases(tenantId, { supplier_id: supplierId })
  const rows = bills.data?.pages.flatMap((p) => p.data) ?? []
  const money = (v: string) => formatMoney(v, lang)
  const columns: Column<Purchase>[] = [
    {
      key: 'no',
      header: t('purchases.number'),
      render: (p) => (
        <Link to={`/purchases/${p.id}`} className={LINK}>
          {p.purchase_no ?? t('purchases.draft')}
        </Link>
      ),
    },
    { key: 'date', header: t('common.date'), render: (p) => formatDate(p.purchase_date, lang) },
    { key: 'bill', header: t('purchases.supplierInvoiceNo'), render: (p) => p.supplier_invoice_no ?? '—' },
    { key: 'total', header: t('common.total'), align: 'right', render: (p) => money(p.grand_total) },
    { key: 'paid', header: t('purchases.paid'), align: 'right', render: (p) => money(p.paid_total) },
    { key: 'due', header: t('purchases.due'), align: 'right', render: (p) => money(p.due_total) },
    {
      key: 'status',
      header: t('common.status'),
      render: (p) => (
        <span className="flex flex-wrap gap-1">
          <PurchaseStatusBadge status={p.status} />
          {(p.status === 'received' || p.status === 'partially_received') && <PaymentBadge status={p.payment_status} />}
        </span>
      ),
    },
  ]
  if (bills.isPending) return <Spinner />
  if (bills.isError) return <Alert tone="danger">{problemMessage(bills.error, t)}</Alert>
  return (
    <section className="flex flex-col gap-4">
      <DataTable caption={t('suppliers.tabs.bills')} columns={columns} rows={rows} rowKey={(p) => p.id} onRowClick={(p) => navigate(`/purchases/${p.id}`)} empty={t('suppliers.noBills')} />
      {bills.hasNextPage && (
        <Button variant="secondary" className="self-center" disabled={bills.isFetchingNextPage} onClick={() => void bills.fetchNextPage()}>
          {bills.isFetchingNextPage ? t('common.loading') : t('common.loadMore')}
        </Button>
      )}
    </section>
  )
}

function PaymentsTab({ tenantId, supplierId }: { tenantId: string; supplierId: string }) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { can } = useSession()
  const [voiding, setVoiding] = useState<string | null>(null)
  const payments = usePayments(tenantId, { party_type: 'supplier', party_id: supplierId, status: 'all' })
  const rows = payments.data?.pages.flatMap((p) => p.data) ?? []
  const columns: Column<Payment>[] = [
    { key: 'no', header: t('purchases.paymentNo'), render: (p) => p.payment_no },
    { key: 'date', header: t('common.date'), render: (p) => formatDate(p.business_date, lang) },
    { key: 'method', header: t('purchases.pay.method'), render: (p) => `${p.method_name} · ${p.account_name}` },
    { key: 'ref', header: t('purchases.pay.reference'), render: (p) => p.reference ?? '—' },
    { key: 'amount', header: t('common.amount'), align: 'right', render: (p) => formatMoney(p.amount, lang) },
    {
      key: 'advance',
      header: t('suppliers.unallocated'),
      align: 'right',
      render: (p) => (dec(p.unallocated_amount) === 0n ? '—' : formatMoney(p.unallocated_amount, lang)),
    },
    {
      key: 'status',
      header: t('common.status'),
      render: (p) => <Badge tone={p.status === 'posted' ? 'good' : 'bad'}>{t(`purchases.paymentRowStatuses.${p.status}`)}</Badge>,
    },
    {
      key: 'actions',
      header: <span className="sr-only">{t('common.actions')}</span>,
      align: 'right',
      render: (p) =>
        p.status === 'posted' && p.source === 'supplier_payment' && can('payments.void') ? (
          <Button variant="ghost" className="min-h-9" onClick={() => setVoiding(p.id)}>
            {t('purchases.voidPayment.button')}
          </Button>
        ) : null,
    },
  ]
  if (payments.isPending) return <Spinner />
  if (payments.isError) return <Alert tone="danger">{problemMessage(payments.error, t)}</Alert>
  return (
    <section className="flex flex-col gap-4">
      <DataTable caption={t('suppliers.tabs.payments')} columns={columns} rows={rows} rowKey={(p) => p.id} empty={t('purchases.noPayments')} />
      {payments.hasNextPage && (
        <Button variant="secondary" className="self-center" disabled={payments.isFetchingNextPage} onClick={() => void payments.fetchNextPage()}>
          {payments.isFetchingNextPage ? t('common.loading') : t('common.loadMore')}
        </Button>
      )}
      {voiding && <VoidPaymentDialog tenantId={tenantId} paymentId={voiding} onClose={() => setVoiding(null)} />}
    </section>
  )
}
