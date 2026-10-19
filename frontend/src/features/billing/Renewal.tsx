import { useTranslation } from 'react-i18next'

import type { BillingCycle, PlanOffer } from '../../api/billing'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { StateTabs } from '../../components/ui/Tabs'
import { formatMoney } from '../../lib/format'
import { featureName } from './labels'
import { CYCLES, perMonth, priceFor, savings } from './pricing'

const KEY_LIMITS = ['users.max', 'branches.max', 'products.max']

/** "Choose a plan / Renew": plan cards with prices for the chosen billing cycle. */
export function Renewal({
  plans,
  currentPlan,
  cycle,
  onCycle,
  onChoose,
  busyPlan,
}: {
  plans: PlanOffer[]
  currentPlan: string | null
  cycle: BillingCycle
  onCycle: (cycle: BillingCycle) => void
  onChoose: (plan: PlanOffer) => void
  busyPlan: string | null
}) {
  const { t } = useTranslation()
  return (
    <section aria-labelledby="renew-title" className="flex flex-col gap-4">
      <div>
        <h2 id="renew-title" className="text-lg font-semibold">
          {t('billing.choose.title')}
        </h2>
        <p className="text-sm text-ink-muted">{t('billing.choose.subtitle')}</p>
      </div>
      <StateTabs
        label={t('billing.choose.cycle')}
        value={cycle}
        onChange={onCycle}
        items={CYCLES.map((c) => ({ value: c, label: t(`billing.cycles.${c}`) }))}
      />
      {plans.length === 0 ? (
        <p className="text-sm text-ink-muted">{t('billing.choose.none')}</p>
      ) : (
        <ul className="grid gap-4 md:grid-cols-2">
          {plans.map((plan) => (
            <PlanCard
              key={plan.id}
              plan={plan}
              cycle={cycle}
              current={plan.code === currentPlan}
              busy={busyPlan === plan.code}
              disabled={busyPlan !== null}
              onChoose={() => onChoose(plan)}
            />
          ))}
        </ul>
      )}
    </section>
  )
}

function PlanCard({
  plan,
  cycle,
  current,
  busy,
  disabled,
  onChoose,
}: {
  plan: PlanOffer
  cycle: BillingCycle
  current: boolean
  busy: boolean
  disabled: boolean
  onChoose: () => void
}) {
  const { t, i18n } = useTranslation()
  const price = priceFor(plan, cycle)
  const saved = savings(plan, cycle)
  const name = t(`plan.names.${plan.code}`, { defaultValue: plan.name })
  const limits = KEY_LIMITS.map((code) => ({ code, feature: plan.features.find((f) => f.code === code) })).filter((l) => l.feature)
  const flags = plan.features.filter((f) => f.kind === 'flag' && f.enabled)

  return (
    <li className="flex flex-col gap-3 rounded-xl border border-border bg-surface p-5 shadow-sm" aria-label={name}>
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h3 className="text-lg font-semibold">{name}</h3>
        {current && <Badge tone="info">{t('billing.choose.current')}</Badge>}
      </div>
      {plan.description && <p className="text-sm text-ink-muted">{plan.description}</p>}
      {price === null ? (
        <p className="text-sm text-ink-muted">{t('billing.choose.notOffered')}</p>
      ) : (
        <div className="flex flex-col gap-0.5">
          <span className="text-2xl font-bold tabular-nums">
            {formatMoney(price, i18n.language)}
            <span className="text-sm font-normal text-ink-muted"> / {t(`billing.per.${cycle}`)}</span>
          </span>
          {cycle !== 'monthly' && (
            <span className="text-sm text-ink-muted">{t('billing.choose.perMonth', { amount: formatMoney(perMonth(price, cycle), i18n.language) })}</span>
          )}
          {saved && (
            <span className="text-sm font-semibold text-brand-700">
              {t('billing.choose.save', { amount: formatMoney(saved.amount, i18n.language), pct: saved.pct })}
            </span>
          )}
        </div>
      )}
      {limits.length > 0 && (
        <dl className="grid grid-cols-[1fr_auto] gap-x-4 gap-y-1 text-sm">
          {limits.map(({ code, feature }) => (
            <div key={code} className="contents">
              <dt className="text-ink-muted">{featureName(t, { code, name: feature!.name })}</dt>
              <dd className="text-right tabular-nums">{feature!.limit_value === null ? t('plan.unlimited') : feature!.limit_value}</dd>
            </div>
          ))}
        </dl>
      )}
      {flags.length > 0 && (
        <ul className="flex flex-col gap-1 text-sm" aria-label={t('plan.features')}>
          {flags.map((f) => (
            <li key={f.code} className="flex items-center gap-2">
              <span aria-hidden className="text-success">
                ✓
              </span>
              {featureName(t, f)}
            </li>
          ))}
        </ul>
      )}
      <Button className="mt-auto" disabled={price === null || disabled} onClick={onChoose} aria-label={t('billing.choose.payFor', { plan: name })}>
        {busy ? t('common.pleaseWait') : current ? t('billing.choose.renew') : t('billing.choose.pay')}
      </Button>
    </li>
  )
}
