import { useTranslation } from 'react-i18next'
import { Link, NavLink, Outlet } from 'react-router'

import { usePlatformMetrics, useStaff } from '../../api/platform'
import { LanguageSwitcher } from '../../components/LanguageSwitcher'
import { Card } from '../../components/ui/Card'
import { Spinner } from '../../components/ui/Spinner'
import { useSession } from '../auth/session'

const NAV = [
  { to: '/admin', key: 'admin.nav.dashboard', end: true },
  { to: '/admin/tenants', key: 'admin.nav.tenants', end: false },
  { to: '/admin/claims', key: 'admin.nav.claims', end: false },
  { to: '/admin/plans', key: 'admin.nav.plans', end: false },
  { to: '/admin/settings', key: 'admin.nav.settings', end: false },
] as const

/** Super Admin console shell (Requirement Analysis §7.3): platform staff only. */
export function AdminLayout() {
  const { t } = useTranslation()
  const staff = useStaff()

  if (staff.isPending) {
    return (
      <div className="flex min-h-svh items-center justify-center">
        <Spinner />
      </div>
    )
  }
  if (staff.isError) {
    return (
      <div className="flex min-h-svh items-center justify-center bg-canvas p-4">
        <Card className="flex max-w-md flex-col gap-3 text-center">
          <h1 className="text-xl font-bold">{t('admin.staffOnlyTitle')}</h1>
          <p className="text-sm text-ink-muted">{t('admin.staffOnlyBody')}</p>
          <Link to="/" className="font-semibold text-brand-700 hover:underline">
            {t('admin.backToApp')}
          </Link>
        </Card>
      </div>
    )
  }
  return <ConsoleShell name={staff.data.full_name} role={staff.data.role} />
}

function ConsoleShell({ name, role }: { name: string; role: string }) {
  const { t } = useTranslation()
  const { signOut } = useSession()
  const metrics = usePlatformMetrics()
  const pending = metrics.data?.pending_claims ?? 0

  return (
    <div className="flex min-h-svh flex-col bg-canvas md:flex-row">
      <aside className="bg-ink text-surface md:w-60 md:shrink-0 print:hidden">
        <div className="px-4 pt-4">
          <div className="text-lg font-bold">{t('app.name')}</div>
          <div className="text-xs font-semibold tracking-wide text-brand-100 uppercase">{t('admin.console')}</div>
        </div>
        <nav aria-label={t('admin.navLabel')} className="flex flex-wrap gap-1 p-2 md:flex-col md:flex-nowrap">
          {NAV.map((item) => (
            <NavLink
              key={item.to}
              to={item.to}
              end={item.end}
              className={({ isActive }) =>
                `flex items-center justify-between gap-2 rounded-lg px-3 py-2 text-sm font-medium ${isActive ? 'bg-surface/15 text-surface' : 'text-surface/75 hover:bg-surface/10 hover:text-surface'}`
              }
            >
              {t(item.key)}
              {item.to === '/admin/claims' && pending > 0 && (
                <span
                  className="rounded-full bg-warning px-2 py-0.5 text-xs font-bold text-ink tabular-nums"
                  aria-label={t('admin.pendingBadge', { count: pending })}
                >
                  {pending}
                </span>
              )}
            </NavLink>
          ))}
        </nav>
      </aside>
      <div className="flex min-w-0 flex-1 flex-col">
        <header className="flex flex-wrap items-center justify-end gap-3 border-b border-ink/20 bg-ink/95 px-4 py-3 text-surface sm:px-8 print:hidden">
          <span className="text-sm">{name}</span>
          <span className="rounded bg-brand-600 px-2 py-0.5 text-xs font-semibold text-white">
            {t(`admin.roles.${role}`, { defaultValue: role })}
          </span>
          <Link to="/" className="text-sm font-medium text-brand-100 hover:underline">
            {t('admin.backToApp')}
          </Link>
          <LanguageSwitcher />
          <button
            type="button"
            onClick={() => void signOut()}
            className="min-h-9 rounded-lg px-3 text-sm font-semibold text-surface/85 hover:bg-surface/10 hover:text-surface"
          >
            {t('auth.signOut')}
          </button>
        </header>
        <main className="flex-1 px-4 py-6 sm:px-8 print:p-0">
          <Outlet />
        </main>
      </div>
    </div>
  )
}
