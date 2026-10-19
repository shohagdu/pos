import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { usePlatformMetrics } from '../../api/platform'
import { Alert } from '../../components/ui/Alert'
import { Card } from '../../components/ui/Card'
import { PageHeader } from '../../components/ui/PageHeader'
import { Spinner } from '../../components/ui/Spinner'
import { StatCard } from '../../components/ui/StatCard'
import { formatMoney } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { LINK, planLabel, stateLabel } from './labels'

const STATES = ['trialing', 'active', 'grace', 'read_only', 'suspended', 'cancelled', 'business_suspended'] as const

/** Where a state count links to (the tenants list, filtered). */
function stateLink(state: string): string {
  return state === 'business_suspended' ? '/admin/tenants?status=suspended' : `/admin/tenants?subscription_status=${state}`
}

/** Platform overview (ADM-R08): revenue, sign-ups and businesses by state and plan. */
export function AdminDashboardPage() {
  const { t, i18n } = useTranslation()
  const metrics = usePlatformMetrics()
  const lang = i18n.language

  if (metrics.isPending) return <Spinner />
  if (metrics.isError) return <Alert tone="danger">{problemMessage(metrics.error, t)}</Alert>
  const m = metrics.data
  const plans = Object.entries(m.by_plan)

  return (
    <div className="flex max-w-6xl flex-col gap-6">
      <PageHeader title={t('admin.dashboard.title')} />
      <section aria-label={t('admin.dashboard.headline')} className="grid grid-cols-2 gap-3 lg:grid-cols-4">
        <StatCard label={t('admin.dashboard.businesses')} value={m.businesses} />
        <StatCard label={t('admin.dashboard.mrr')} value={formatMoney(m.mrr, lang)} tone="good" />
        <StatCard label={t('admin.dashboard.arr')} value={formatMoney(m.arr, lang)} />
        <StatCard label={t('admin.dashboard.revenue30')} value={formatMoney(m.revenue_30d, lang)} />
        <StatCard label={t('admin.dashboard.signups7')} value={m.signups_7d} />
        <StatCard label={t('admin.dashboard.signups30')} value={m.signups_30d} />
        <Link to="/admin/claims" className="rounded-xl focus-visible:outline-2 focus-visible:outline-brand-600">
          <StatCard
            label={t('admin.dashboard.pendingClaims')}
            value={m.pending_claims}
            tone={m.pending_claims > 0 ? 'bad' : 'default'}
            hint={<span className="text-brand-700">{t('admin.dashboard.reviewClaims')}</span>}
          />
        </Link>
      </section>

      <section className="flex flex-col gap-3">
        <h2 className="text-lg font-semibold">{t('admin.dashboard.byStatus')}</h2>
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-4 lg:grid-cols-7">
          {STATES.map((s) => (
            <Link
              key={s}
              to={stateLink(s)}
              className="rounded-xl border border-border bg-surface p-4 shadow-sm hover:border-brand-500 focus-visible:outline-2 focus-visible:outline-brand-600"
            >
              <div className="text-xs font-medium text-ink-muted">{stateLabel(t, s)}</div>
              <div className="mt-1 text-xl font-bold tabular-nums">{m.by_status[s] ?? 0}</div>
            </Link>
          ))}
        </div>
      </section>

      <Card className="flex flex-col gap-3">
        <h2 className="text-lg font-semibold">{t('admin.dashboard.byPlan')}</h2>
        {plans.length === 0 ? (
          <p className="text-sm text-ink-muted">{t('common.none')}</p>
        ) : (
          <table className="w-full text-sm">
            <thead className="text-xs text-ink-muted uppercase">
              <tr>
                <th scope="col" className="py-2 text-left">{t('admin.tenants.plan')}</th>
                <th scope="col" className="py-2 text-right">{t('admin.dashboard.businesses')}</th>
              </tr>
            </thead>
            <tbody>
              {plans.map(([code, count]) => (
                <tr key={code} className="border-t border-border">
                  <td className="py-2">
                    <Link to={`/admin/tenants?plan_code=${encodeURIComponent(code)}`} className={LINK}>
                      {planLabel(t, code)}
                    </Link>
                    <span className="ml-2 font-mono text-xs text-ink-muted">{code}</span>
                  </td>
                  <td className="py-2 text-right tabular-nums">{count}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </Card>
    </div>
  )
}
