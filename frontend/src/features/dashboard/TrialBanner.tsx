import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { useSession } from '../auth/session'

const TONES = {
  info: 'bg-brand-50 text-brand-700',
  warning: 'bg-warning/15 text-ink',
  danger: 'bg-danger/10 text-danger',
} as const

/** Subscription state banner (Requirement Analysis §9.5). Hidden during the free trial. */
export function TrialBanner() {
  const { t } = useTranslation()
  const { me } = useSession()
  const sub = me?.tenant?.subscription
  if (!sub || sub.status === 'active' || sub.status === 'trialing' || sub.status === 'suspended' || sub.status === 'terminated') return null
  const tone = sub.status === 'cancelled' ? 'info' : sub.status === 'grace' ? 'warning' : 'danger'
  return (
    <div className={`flex flex-wrap items-center justify-center gap-x-3 px-4 py-2 text-center text-sm sm:px-8 ${TONES[tone]}`}>
      <span>{t(`subscription.banner.${sub.status}`, { count: sub.days_left ?? 0 })}</span>
      <Link to="/plan" className="font-semibold underline">
        {t('subscription.seePlan')}
      </Link>
    </div>
  )
}
