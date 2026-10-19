import { useTranslation } from 'react-i18next'

import { type PlanFeature, type UsageMeter, useSubscription } from '../../api/billing'
import { Alert } from '../../components/ui/Alert'
import { Card } from '../../components/ui/Card'
import { Spinner } from '../../components/ui/Spinner'
import { problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { Billing } from './Billing'
import { featureName } from './labels'

/**
 * SUB-R05: plan, state, renewal date, usage meters and included features; the
 * owner (`subscription.manage`) also renews here and exports the data.
 */
export function PlanPage() {
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const owner = can('subscription.manage')
  return (
    <div className="flex max-w-4xl flex-col gap-6">
      <PlanSummary tenantId={tenantId} />
      {owner && <Billing tenantId={tenantId} currentPlan={me?.tenant?.subscription?.plan_code ?? null} />}
    </div>
  )
}

function PlanSummary({ tenantId }: { tenantId: string }) {
  const { t, i18n } = useTranslation()
  const overview = useSubscription(tenantId)
  const date = (iso: string | null) =>
    iso ? new Intl.DateTimeFormat(i18n.language === 'bn' ? 'bn-BD' : 'en-GB', { dateStyle: 'long' }).format(new Date(iso)) : '—'

  if (overview.isPending) return <Spinner />
  if (overview.isError) return <Alert tone="danger">{problemMessage(overview.error, t)}</Alert>
  const { subscription: sub, usage, features } = overview.data
  const flags = features.filter((f) => f.kind === 'flag')
  const ends = sub.status === 'trialing' ? sub.trial_ends_at : sub.current_period_end

  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-2xl font-bold">{t('plan.title')}</h1>
      <Card className="flex flex-col gap-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <span className="text-lg font-semibold">{t(`plan.names.${sub.plan_code}`, { defaultValue: sub.plan_name })}</span>
          <span className="rounded bg-canvas px-2 py-0.5 text-sm font-semibold">{t(`plan.status.${sub.status}`)}</span>
        </div>
        <dl className="grid grid-cols-[auto_1fr] gap-x-6 gap-y-1 text-sm">
          <dt className="text-ink-muted">{sub.status === 'trialing' ? t('plan.trialEnds') : t('plan.periodEnds')}</dt>
          <dd>{date(ends)}</dd>
          {sub.days_left !== null && (
            <>
              <dt className="text-ink-muted">{t('plan.daysLeftLabel')}</dt>
              <dd>{t('plan.daysLeft', { count: sub.days_left })}</dd>
            </>
          )}
        </dl>
        <Alert tone={sub.status === 'trialing' || sub.status === 'active' ? 'info' : 'warning'}>{t(`plan.explain.${sub.status}`)}</Alert>
        <p className="text-sm text-ink-muted">{t('plan.renewSoon')}</p>
      </Card>

      <Card>
        <h2 className="mb-4 font-semibold">{t('plan.usage')}</h2>
        <ul className="flex flex-col gap-4">
          {usage.map((m) => (
            <Meter key={m.feature} meter={m} />
          ))}
        </ul>
      </Card>

      <Card>
        <h2 className="mb-4 font-semibold">{t('plan.features')}</h2>
        <ul className="grid gap-2 sm:grid-cols-2">
          {flags.map((f: PlanFeature) => (
            <li key={f.code} className={`flex items-center gap-2 text-sm ${f.enabled ? '' : 'text-ink-muted'}`}>
              <span aria-hidden className={f.enabled ? 'text-success' : ''}>
                {f.enabled ? '✓' : '—'}
              </span>
              {featureName(t, f)}
              <span className="sr-only">{f.enabled ? t('plan.included') : t('plan.notIncluded')}</span>
            </li>
          ))}
        </ul>
      </Card>
    </div>
  )
}

function Meter({ meter }: { meter: UsageMeter }) {
  const { t } = useTranslation()
  const name = featureName(t, { code: meter.feature, name: meter.name })
  const pct = meter.used !== null && meter.limit ? Math.min(100, (meter.used / meter.limit) * 100) : 0
  const full = meter.used !== null && meter.limit !== null && meter.used >= meter.limit
  return (
    <li className="flex flex-col gap-1.5">
      <div className="flex justify-between gap-2 text-sm">
        <span className="font-medium">{name}</span>
        <span className={full ? 'font-semibold text-danger' : 'text-ink-muted'}>
          {meter.used === null
            ? meter.limit === null
              ? t('plan.unlimited')
              : t('plan.upTo', { max: meter.limit })
            : meter.limit === null
              ? t('plan.usedUnlimited', { used: meter.used })
              : t('plan.usedOf', { used: meter.used, max: meter.limit })}
        </span>
      </div>
      {meter.used !== null && meter.limit !== null && (
        <div
          role="meter"
          aria-label={name}
          aria-valuemin={0}
          aria-valuemax={meter.limit}
          aria-valuenow={meter.used}
          className="h-2 overflow-hidden rounded-full bg-border"
        >
          <div className={`h-full ${full ? 'bg-danger' : 'bg-brand-600'}`} style={{ width: `${pct}%` }} />
        </div>
      )}
    </li>
  )
}
