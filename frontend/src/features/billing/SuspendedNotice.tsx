import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { Card } from '../../components/ui/Card'
import { useSession } from '../auth/session'

/** §9.5 "suspended": the app only shows the plan and how to renew. */
export function SuspendedNotice() {
  const { t } = useTranslation()
  const { me } = useSession()
  return (
    <Card className="flex max-w-xl flex-col gap-4">
      <h1 className="text-xl font-bold">{t('subscription.suspendedTitle', { business: me?.tenant?.name ?? '' })}</h1>
      <p className="text-sm">{t('subscription.suspendedBody')}</p>
      <p className="text-sm text-ink-muted">{t('subscription.suspendedData')}</p>
      <Link to="/plan" className="inline-flex min-h-11 items-center justify-center rounded-lg bg-brand-600 px-4 text-sm font-semibold text-white hover:bg-brand-700 sm:self-start">
        {t('subscription.seePlan')}
      </Link>
    </Card>
  )
}
