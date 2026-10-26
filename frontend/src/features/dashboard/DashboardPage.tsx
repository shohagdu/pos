import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { type Dashboard, type Figures, useDashboard } from '../../api/reports'
import { useMeta, useReadiness } from '../../api/system'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Card } from '../../components/ui/Card'
import { DataTable } from '../../components/ui/DataTable'
import { Spinner } from '../../components/ui/Spinner'
import { type Status, StatusDot } from '../../components/ui/StatusDot'
import { isoDate } from '../../lib/dates'
import { dec } from '../../lib/decimal'
import { formatMoney, formatQuantity } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { SetupChecklist } from '../onboarding/SetupChecklist'
import { reportAccess } from '../reports/access'
import { SalesChart } from './SalesChart'
import { DIcon, GettingStarted, KpiCard, ListLink, type QuickAction, QuickActions, type StartStep } from './widgets'

/** Quick-action tiles, each shown only with its permission (`reports` = any report). */
const ACTIONS: (QuickAction & { permission: string })[] = [
  { key: 'newSale', to: '/pos', icon: 'pos', permission: 'pos.sell', primary: true },
  { key: 'addProduct', to: '/products/new', icon: 'products', permission: 'products.create' },
  { key: 'newPurchase', to: '/purchases/new', icon: 'purchases', permission: 'purchases.create' },
  { key: 'addExpense', to: '/expenses', icon: 'expenses', permission: 'expenses.create' },
  { key: 'addCustomer', to: '/customers/new', icon: 'customers', permission: 'customers.manage' },
  { key: 'reports', to: '/reports', icon: 'reports', permission: 'reports' },
]

/** Getting-started steps for a shop without any sales yet. */
const START_STEPS: (StartStep & { permission: string })[] = [
  { key: 'products', to: '/products/new', permission: 'products.create' },
  { key: 'stock', to: '/inventory/adjustments/new?reason=opening', permission: 'inventory.opening_stock' },
  { key: 'sale', to: '/pos', permission: 'pos.sell' },
]

const SALE_TONES: Record<string, 'good' | 'warn' | 'bad' | 'neutral'> = {
  completed: 'good',
  partially_returned: 'warn',
  returned: 'neutral',
  cancelled: 'bad',
}

function dateLocale(lang: string) {
  return lang === 'bn' ? 'bn-BD' : 'en-GB'
}

function partOfDay(hour: number): 'morning' | 'afternoon' | 'evening' {
  if (hour >= 5 && hour < 12) return 'morning'
  if (hour >= 12 && hour < 17) return 'afternoon'
  return 'evening'
}

export function DashboardPage() {
  const { t, i18n } = useTranslation()
  const { me, can } = useSession()
  const anyReport = Object.values(reportAccess(can)).some(Boolean)
  const actions = ACTIONS.filter((a) => (a.permission === 'reports' ? anyReport : can(a.permission)))
  // Fixed for the page's lifetime; the greeting needn't tick over live.
  const [now] = useState(() => new Date())
  const firstName = me?.user.full_name.trim().split(/\s+/)[0] ?? ''
  const todayLabel = new Intl.DateTimeFormat(dateLocale(i18n.language), { weekday: 'long', day: 'numeric', month: 'long', year: 'numeric' }).format(now)

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-xl font-bold sm:text-2xl">{t(`dashboard.greeting.${partOfDay(now.getHours())}`, { name: firstName })}</h1>
        <p className="text-sm text-ink-muted">
          <time dateTime={isoDate(now)}>{todayLabel}</time>
          {me?.tenant && <> · {me.tenant.name}</>}
        </p>
      </div>
      {me?.tenant && <QuickActions actions={actions} />}
      <SetupChecklist />
      {me?.tenant && can('dashboard.view') && <Overview tenantId={me.tenant.tenant_id} />}
      <SystemStatus />
    </div>
  )
}

function hasNoSales(d: Dashboard) {
  return d.recent_sales.length === 0 && d.month_figures.invoices === 0 && d.last_14_days.every((x) => dec(x.sales) === 0n)
}

function Overview({ tenantId }: { tenantId: string }) {
  const { t, i18n } = useTranslation()
  const { can } = useSession()
  const lang = i18n.language
  const dashboard = useDashboard(tenantId)
  const money = (v: string) => formatMoney(v, lang)
  const count = (n: number) => n.toLocaleString(dateLocale(lang))

  if (dashboard.isPending) return <Spinner />
  if (dashboard.isError) return <Alert tone="danger">{problemMessage(dashboard.error, t)}</Alert>
  const d = dashboard.data

  const steps = hasNoSales(d) ? START_STEPS.filter((s) => can(s.permission)) : []
  const todayIndex = d.last_14_days.findIndex((x) => x.date === d.today)
  const yesterday = todayIndex > 0 ? d.last_14_days[todayIndex - 1] : undefined

  const figures = (title: string, f: Figures, salesHint?: string) => (
    <section aria-label={title} className="flex flex-col gap-3">
      <h2 className="font-semibold">{title}</h2>
      <div className="grid grid-cols-1 gap-3 min-[420px]:grid-cols-2 md:grid-cols-3 xl:grid-cols-6">
        <KpiCard
          icon="sales"
          label={t('dashboard.sales')}
          value={money(f.sales)}
          hint={
            <>
              {t('dashboard.invoices', { count: f.invoices, formatted: count(f.invoices) })}
              {salesHint && <span className="block">{salesHint}</span>}
            </>
          }
        />
        <KpiCard icon="invoice" label={t('dashboard.netSales')} value={money(f.net_sales)} hint={t('dashboard.netSalesHint')} />
        <KpiCard icon="money" label={t('dashboard.cashReceived')} value={money(f.cash_received)} />
        <KpiCard icon="expenses" label={t('dashboard.expenses')} value={money(f.expenses)} />
        {f.gross_profit !== null && <KpiCard icon="profit" label={t('dashboard.grossProfit')} value={money(f.gross_profit)} tone="good" />}
        {f.net_profit !== null && (
          <KpiCard icon="profit" label={t('dashboard.netProfit')} value={money(f.net_profit)} tone={dec(f.net_profit) < 0n ? 'bad' : 'good'} />
        )}
      </div>
    </section>
  )

  const stockIssues = (d.low_stock_count > 0 ? 1 : 0) + (d.out_of_stock_count > 0 ? 1 : 0)

  return (
    <>
      <GettingStarted steps={steps} />
      {figures(t('dashboard.today'), d.today_figures, yesterday ? t('dashboard.yesterday', { amount: money(yesterday.sales) }) : undefined)}
      <div className="grid gap-6 lg:grid-cols-3">
        <Card className="p-4 lg:col-span-2">
          <SalesChart days={d.last_14_days} today={d.today} />
        </Card>
        <div className="flex flex-col gap-6">
          <section aria-labelledby="dash-attention" className="rounded-xl border border-border bg-surface p-3 shadow-sm">
            <h2 id="dash-attention" className="flex items-center justify-between gap-2 px-2 pt-1 pb-2 font-semibold">
              <span className="flex items-center gap-2">
                <DIcon name={stockIssues > 0 ? 'alert' : 'check'} className={`size-5 ${stockIssues > 0 ? 'text-danger' : 'text-success'}`} />
                {t('dashboard.attention')}
              </span>
              {stockIssues === 0 && <Badge tone="good">{t('dashboard.allGood')}</Badge>}
            </h2>
            <ul className="flex flex-col">
              <ListLink
                to="/inventory?status=low"
                icon="inventory"
                label={t('dashboard.lowStock')}
                detail={t('dashboard.lowStockHint')}
                value={count(d.low_stock_count)}
                tone={d.low_stock_count > 0 ? 'warn' : 'default'}
              />
              <ListLink
                to="/inventory?status=out"
                icon="alert"
                label={t('dashboard.outOfStock')}
                detail={t('dashboard.outOfStockHint')}
                value={count(d.out_of_stock_count)}
                tone={d.out_of_stock_count > 0 ? 'bad' : 'default'}
              />
            </ul>
          </section>
          <section aria-label={t('dashboard.balances')} className="rounded-xl border border-border bg-surface p-3 shadow-sm">
            <h2 className="px-2 pt-1 pb-2 font-semibold">{t('dashboard.balances')}</h2>
            <ul className="flex flex-col">
              <ListLink
                to="/customers"
                icon="receivable"
                label={t('dashboard.receivables')}
                detail={t('dashboard.receivablesHint')}
                value={money(d.receivables)}
                tone={dec(d.receivables) > 0n ? 'warn' : 'default'}
              />
              <ListLink to="/suppliers" icon="suppliers" label={t('dashboard.payables')} detail={t('dashboard.payablesHint')} value={money(d.payables)} />
              <ListLink
                to="/accounts"
                icon="money"
                label={t('dashboard.moneyBalance')}
                detail={t('dashboard.moneyBalanceHint')}
                value={money(d.money_balance)}
                tone={dec(d.money_balance) < 0n ? 'bad' : 'default'}
              />
            </ul>
          </section>
        </div>
      </div>
      {figures(t('dashboard.thisMonth'), d.month_figures)}
      <div className="grid gap-6 lg:grid-cols-2">
        <section aria-labelledby="dash-top" className="flex min-w-0 flex-col gap-2">
          <h2 id="dash-top" className="font-semibold">
            {t('dashboard.topProducts')}
          </h2>
          <DataTable
            caption={t('dashboard.topProducts')}
            rows={d.top_products}
            rowKey={(p) => p.variant_id}
            empty={<EmptyNote text={t('dashboard.noTopProducts')} />}
            columns={[
              { key: 'name', header: t('dashboard.product'), render: (p) => p.product_name },
              { key: 'qty', header: t('common.quantity'), align: 'right', render: (p) => `${formatQuantity(p.quantity, lang, true)} ${p.unit_short_name}` },
              { key: 'net', header: t('dashboard.netSales'), align: 'right', render: (p) => money(p.net_sales) },
            ]}
          />
        </section>
        <section aria-labelledby="dash-recent" className="flex min-w-0 flex-col gap-2">
          <div className="flex items-center justify-between gap-2">
            <h2 id="dash-recent" className="font-semibold">
              {t('dashboard.recentSales')}
            </h2>
            {d.recent_sales.length > 0 && (
              <Link to="/sales" className="text-sm font-semibold text-brand-700 hover:underline">
                {t('dashboard.viewAllSales')}
              </Link>
            )}
          </div>
          <DataTable
            caption={t('dashboard.recentSales')}
            rows={d.recent_sales}
            rowKey={(s) => s.id}
            empty={<EmptyNote text={t('dashboard.noSales')} />}
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
              {
                key: 'due',
                header: t('dashboard.due'),
                align: 'right',
                render: (s) => (dec(s.due_total) > 0n ? <span className="font-semibold text-danger">{money(s.due_total)}</span> : '—'),
              },
              {
                key: 'status',
                header: t('common.status'),
                render: (s) => <Badge tone={SALE_TONES[s.status] ?? 'neutral'}>{t(`dashboard.saleStatuses.${s.status}`, { defaultValue: s.status })}</Badge>,
              },
            ]}
          />
        </section>
      </div>
    </>
  )
}

function EmptyNote({ text }: { text: string }) {
  return (
    <span className="flex items-center gap-3 rounded-xl border border-dashed border-border bg-surface px-4 py-6 text-ink-muted">
      <DIcon name="empty" className="size-6" />
      {text}
    </span>
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
