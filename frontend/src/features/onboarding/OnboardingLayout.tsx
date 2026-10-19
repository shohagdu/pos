import { useTranslation } from 'react-i18next'
import { Outlet, useLocation } from 'react-router'

import { LanguageSwitcher } from '../../components/LanguageSwitcher'
import { useSession } from '../auth/session'
import { WIZARD_STEPS } from './steps'

export function OnboardingLayout() {
  const { t } = useTranslation()
  const { me } = useSession()
  const current = useLocation().pathname.split('/').pop() ?? ''
  const index = Math.max(0, WIZARD_STEPS.indexOf(current as (typeof WIZARD_STEPS)[number]))

  return (
    <div className="flex min-h-svh flex-col">
      <header className="flex items-center justify-between gap-4 px-4 py-4 sm:px-8">
        <span className="text-lg font-bold text-brand-700">{me?.tenant?.name ?? t('app.name')}</span>
        <LanguageSwitcher />
      </header>
      <main className="flex flex-1 justify-center px-4 pt-4 pb-16 sm:pt-8">
        <div className="flex w-full max-w-xl flex-col gap-6">
          <ol aria-label={t('onboarding.progress')} className="flex gap-1.5">
            {WIZARD_STEPS.map((step, i) => (
              <li key={step} className="flex flex-1 flex-col gap-1.5" aria-current={i === index ? 'step' : undefined}>
                <span className={`h-1.5 rounded-full ${i <= index ? 'bg-brand-600' : 'bg-border'}`} />
                <span className={`hidden text-xs sm:block ${i === index ? 'font-semibold text-ink' : 'text-ink-muted'}`}>
                  {t(`onboarding.steps.${step}`)}
                </span>
              </li>
            ))}
          </ol>
          <Outlet />
        </div>
      </main>
    </div>
  )
}
