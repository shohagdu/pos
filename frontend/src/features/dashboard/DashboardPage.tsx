import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { type Figures, useDashboard } from '../../api/reports'
import { useMeta, useReadiness } from '../../api/system'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Card } from '../../components/ui/Card'
import { DataTable } from '../../components/ui/DataTable'
import { Spinner } from '../../components/ui/Spinner'
import { StatCard } from '../../components/ui/StatCard'
import { type Status, StatusDot } from '../../components/ui/StatusDot'
import { dec } from '../../lib/decimal'
import { formatMoney, formatQuantity } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { SetupChecklist } from '../onboarding/SetupChecklist'
import { reportAccess } from '../reports/access'
import { SalesChart } from './SalesChart'

const ACTIONS = [
  { key: 'newSale', to: '/pos', permission: 'pos.sell', primary: true },
  { key: 'newPurchase', to: '/purchases/new', permission: 'purchases.create' },
  { key: 'addExpense', to: '/expenses', permission: 'expenses.create' },
  { key: 'reports', to: '/reports', permission: 'reports' },
] as const

const SALE_TONES: Record<string, 'good' | 'warn' | 'bad' | 'neutral'> = {
  completed: 'good',
  partially_returned: 'warn',
  returned: 'neutral',
  cancelled: 'bad',
}

export function DashboardPage() {
  const { t } = useTranslation()
  const { me, can } = useSession()
  const anyReport = Object.values(reportAccess(can)).some(Boolean)
  const actions = ACTIONS.filter((a) => (a.permission === 'reports' ? anyReport : can(a.permission)))

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h1 className="text-2xl font-bold">{t('dashboard.title')}</h1>
          <p className="text-sm text-ink-muted">{t('dashboard.welcome', { name: me?.user.full_name ?? '' })}</p>
        </div>
        {me?.tenant && (
          <nav aria-label={t('dashboard.quickActions')} className="flex flex-wrap gap-2">
            {actions.map((a) => (
              <Link
                key={a.key}
                to={a.to}
                className={`inline-flex min-h-11 items-center rounded-lg px-4 text-sm font-semibold ${'primary' in a ? 'bg-brand-600 text-white hover:bg-brand-700' : 'border border-border bg-surface text-ink hover:bg-canvas'}`}
              >
                {t(`dashboard.actions.${a.key}`)}
              </Link>
            ))}
          </nav>
        )}
      </div>
      <SetupChecklist />
      {me?.tenant && can('dashboard.view') && <Overview tenantId={me.tenant.tenant_id} />}
      <SystemStatus />
    </div>
  )
}

function Overview({ tenantId }: { tenantId: string }) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const dashboard = useDashboard(tenantId)
  const money = (v: string) => formatMoney(v, lang)

  if (dashboard.isPending) return <Spinner />
  if (dashboard.isError) return <Alert tone="danger">{problemMessage(dashboard.error, t)}</Alert>
  const d = dashboard.data

  const figures = (title: string, f: Figures) => (
    <section aria-label={title} className="flex flex-col gap-2">
      <h2 className="font-semibold">{title}</h2>
      <div className="grid grid-cols-2 gap-3 md:grid-cols-3 xl:grid-cols-6">
        <StatCard label={t('dashboard.sales')} value={money(f.sales)} hint={t('dashboard.invoices', { count: f.invoices })} />
        <StatCard label={t('dashboard.netSales')} value={money(f.net_sales)} />
        <StatCard label={t('dashboard.cashReceived')} value={money(f.cash_received)} />
        <StatCard label={t('dashboard.expenses')} value={money(f.expenses)} />
        {f.gross_profit !== null && <StatCard label={t('dashboard.grossProfit')} value={money(f.gross_profit)} />}
        {f.net_profit !== null && (
          <StatCard label={t('dashboard.netProfit')} value={money(f.net_profit)} tone={dec(f.net_profit) < 0n ? 'bad' : 'good'} />
        )}
      </div>
    </section>
  )

  return (
    <>
      {figures(t('dashboard.today'), d.today_figures)}
      {figures(t('dashboard.thisMonth'), d.month_figures)}
      <section aria-label={t('dashboard.balances')} className="grid grid-cols-2 gap-3 md:grid-cols-3 xl:grid-cols-5">
        <LinkStat to="/customers" label={t('dashboard.receivables')} value={money(d.receivables)} />
        <LinkStat to="/suppliers" label={t('dashboard.payables')} value={money(d.payables)} />
        <LinkStat to="/accounts" label={t('dashboard.moneyBalance')} value={money(d.money_balance)} />
        <LinkStat to="/inventory?status=low" label={t('dashboard.lowStock')} value={d.low_stock_count} tone={d.low_stock_count > 0 ? 'bad' : 'default'} />
        <LinkStat to="/inventory?status=out" label={t('dashboard.outOfStock')} value={d.out_of_stock_count} tone={d.out_of_stock_count > 0 ? 'bad' : 'default'} />
      </section>
      <div className="grid gap-6 lg:grid-cols-2">
        <Card className="p-4">
          <SalesChart days={d.last_14_days} />
        </Card>
        <section aria-labelledby="dash-top" className="flex flex-col gap-2">
          <h2 id="dash-top" className="font-semibold">
            {t('dashboard.topProducts')}
          </h2>
          <DataTable
            caption={t('dashboard.topProducts')}
            rows={d.top_products}
            rowKey={(p) => p.variant_id}
            empty={t('dashboard.noSales')}
            columns={[
              { key: 'name', header: t('dashboard.product'), render: (p) => p.product_name },
              { key: 'qty', header: t('common.quantity'), align: 'right', render: (p) => `${formatQuantity(p.quantity, lang, true)} ${p.unit_short_name}` },
              { key: 'net', header: t('dashboard.netSales'), align: 'right', render: (p) => money(p.net_sales) },
            ]}
          />
        </section>
      </div>
      <section aria-labelledby="dash-recent" className="flex flex-col gap-2">
        <h2 id="dash-recent" className="font-semibold">
          {t('dashboard.recentSales')}
        </h2>
        <DataTable
          caption={t('dashboard.recentSales')}
          rows={d.recent_sales}
          rowKey={(s) => s.id}
          empty={t('dashboard.noSales')}
          columns={[
            {
              key: 'invoice',
              header: t('dashboard.invoice'),
              render: (s) => (
                <Link to={`/sales/${s.id}`} className="font-mono text-xs font-semibold text-brand-700 hover:underline">
                  {s.invoice_no}
                </Link>
              ),
            },
            { key: 'customer', header: t('dashboard.customer'), render: (s) => s.customer_name },
            { key: 'total', header: t('common.total'), align: 'right', render: (s) => money(s.grand_total) },
            { key: 'due', header: t('dashboard.due'), align: 'right', render: (s) => (dec(s.due_total) > 0n ? money(s.due_total) : '—') },
            {
              key: 'status',
              header: t('common.status'),
              render: (s) => <Badge tone={SALE_TONES[s.status] ?? 'neutral'}>{t(`dashboard.saleStatuses.${s.status}`, { defaultValue: s.status })}</Badge>,
            },
          ]}
        />
      </section>
    </>
  )
}

function LinkStat({ to, label, value, tone }: { to: string; label: string; value: ReactNode; tone?: 'default' | 'good' | 'bad' }) {
  return (
    <Link to={to} className="block rounded-xl hover:ring-2 hover:ring-brand-100 focus-visible:outline-2 focus-visible:outline-brand-600">
      <StatCard label={label} value={value} tone={tone} />
    </Link>
  )
}

function SystemStatus() {
  const { t } = useTranslation()
  const meta = useMeta()
  const ready = useReadiness()
  const status = (q: { isPending: boolean; isError: boolean }): Status => (q.isPending ? 'pending' : q.isError ? 'down' : 'ok')
  const label = (s: Status) => (s === 'pending' ? t('dashboard.checking') : s === 'ok' ? t('dashboard.ok') : t('dashboard.down'))
  const apiStatus = status(meta)
  const dbStatus = status(ready)

  return (
    <section aria-labelledby="dash-status" className="mt-4 rounded-xl border border-border bg-surface px-4 py-3 text-sm">
      <h2 id="dash-status" className="sr-only">
        {t('dashboard.systemStatus')}
      </h2>
      <dl className="flex flex-wrap items-center gap-x-6 gap-y-2 text-xs">
        <div className="flex items-center gap-2">
          <dt className="text-ink-muted">{t('dashboard.api')}</dt>
          <dd>
            <StatusDot status={apiStatus} label={label(apiStatus)} />
          </dd>
        </div>
        <div className="flex items-center gap-2">
          <dt className="text-ink-muted">{t('dashboard.database')}</dt>
          <dd>
            <StatusDot status={dbStatus} label={label(dbStatus)} />
          </dd>
        </div>
        {meta.data && (
          <>
            <div className="flex items-center gap-2">
              <dt className="text-ink-muted">{t('dashboard.version')}</dt>
              <dd className="font-mono">{meta.data.version}</dd>
            </div>
            <div className="flex items-center gap-2">
              <dt className="text-ink-muted">{t('dashboard.environment')}</dt>
              <dd>{meta.data.environment}</dd>
            </div>
          </>
        )}
      </dl>
    </section>
  )
}
