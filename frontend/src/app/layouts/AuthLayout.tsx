import { useTranslation } from 'react-i18next'
import { Outlet } from 'react-router'

import { LanguageSwitcher } from '../../components/LanguageSwitcher'

export function AuthLayout() {
  const { t } = useTranslation()
  return (
    <div className="flex min-h-svh flex-col">
      <header className="flex items-center justify-between px-4 py-4 sm:px-8">
        <span className="text-lg font-bold text-brand-700">{t('app.name')}</span>
        <LanguageSwitcher />
      </header>
      <main className="flex flex-1 items-start justify-center px-4 pt-8 pb-16 sm:pt-16">
        <div className="w-full max-w-md">
          <Outlet />
        </div>
      </main>
    </div>
  )
}
