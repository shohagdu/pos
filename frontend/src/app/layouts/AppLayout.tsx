import { useTranslation } from 'react-i18next'
import { Link, NavLink, Outlet, useLocation } from 'react-router'

import { LanguageSwitcher } from '../../components/LanguageSwitcher'
import { Button } from '../../components/ui/Button'
import { useStaff } from '../../api/platform'
import { useSession } from '../../features/auth/session'
import { BranchSwitcher } from '../../features/branches/BranchSwitcher'
import { SuspendedNotice } from '../../features/billing/SuspendedNotice'
import { TrialBanner } from '../../features/dashboard/TrialBanner'

// Each entry names the permissions that show it (any of them; `null` = every
// member). Further modules add themselves here.
const NAV: { to: string; key: string; permission: string | string[] | null }[] = [
  { to: '/', key: 'nav.dashboard', permission: null },
  { to: '/pos', key: 'nav.pos', permission: 'pos.sell' },
  { to: '/sales', key: 'nav.sales', permission: 'sales.view' },
  { to: '/purchases', key: 'nav.purchases', permission: 'purchases.view' },
  { to: '/inventory', key: 'nav.inventory', permission: 'inventory.view' },
  { to: '/products', key: 'nav.products', permission: 'products.view' },
  { to: '/customers', key: 'nav.customers', permission: 'customers.view' },
  { to: '/suppliers', key: 'nav.suppliers', permission: 'suppliers.view' },
  { to: '/accounts', key: 'nav.money', permission: 'accounts.view' },
  { to: '/expenses', key: 'nav.expenses', permission: 'expenses.view' },
  { to: '/incomes', key: 'nav.income', permission: ['income.create', 'expenses.view'] },
  { to: '/shifts', key: 'nav.shifts', permission: 'pos.sell' },
  {
    to: '/reports',
    key: 'nav.reports',
    permission: ['reports.sales', 'reports.purchases', 'reports.inventory', 'reports.financial', 'reports.parties', 'accounting.view'],
  },
  { to: '/accounting/journals', key: 'nav.accounting', permission: 'accounting.view' },
  { to: '/users', key: 'nav.users', permission: 'users.view' },
  { to: '/audit', key: 'nav.audit', permission: 'audit.view' },
  { to: '/settings', key: 'nav.settings', permission: ['settings.manage', 'branches.manage'] },
  { to: '/plan', key: 'nav.plan', permission: null },
]

export function AppLayout() {
  const { t } = useTranslation()
  const { me, can, signOut } = useSession()
  const staff = useStaff()
  const items = NAV.filter(
    (item) =>
      item.permission === null || (Array.isArray(item.permission) ? item.permission.some((p) => can(p)) : can(item.permission)),
  )
  const multiple = (me?.memberships.length ?? 0) > 1
  const { pathname } = useLocation()
  // Suspended: only the plan page is usable (the API refuses everything else).
  const suspended = me?.tenant?.subscription?.status === 'suspended' && pathname !== '/plan'

  return (
    <div className="flex min-h-svh flex-col md:flex-row">
      <aside className="border-b border-border bg-surface md:w-60 md:shrink-0 md:border-r md:border-b-0 print:hidden">
        <div className="px-4 pt-4 text-lg font-bold text-brand-700">{t('app.name')}</div>
        <div className="px-4 pb-3 text-sm">
          <span className="font-semibold text-ink">{me?.tenant?.name}</span>
          {multiple && (
            <Link to="/choose-business" className="ml-2 text-brand-700 hover:underline">
              {t('tenant.switch')}
            </Link>
          )}
        </div>
        <BranchSwitcher />
        <nav aria-label={t('nav.main')} className="flex flex-wrap gap-1 px-2 pb-2 md:flex-col md:flex-nowrap">
          {items.map((item) => (
            <NavLink
              key={item.to}
              to={item.to}
              end={item.to === '/'}
              className={({ isActive }) =>
                `rounded-lg px-3 py-2 text-sm font-medium ${isActive ? 'bg-brand-50 text-brand-700' : 'text-ink-muted hover:bg-canvas hover:text-ink'}`
              }
            >
              {t(item.key)}
            </NavLink>
          ))}
        </nav>
      </aside>
      <div className="flex min-w-0 flex-1 flex-col">
        <div className="print:hidden">
          <TrialBanner />
        </div>
        <header className="flex flex-wrap items-center justify-end gap-3 border-b border-border bg-surface px-4 py-3 sm:px-8 print:hidden">
          {staff.data && (
            <Link to="/admin" className="text-sm font-semibold text-brand-700 hover:underline">
              {t('nav.admin')}
            </Link>
          )}
          <span className="text-sm text-ink-muted">{me?.user.full_name}</span>
          <Link to="/account/sessions" className="text-sm font-medium text-brand-700 hover:underline">
            {t('sessions.title')}
          </Link>
          <LanguageSwitcher />
          <Button variant="ghost" onClick={() => void signOut()}>
            {t('auth.signOut')}
          </Button>
        </header>
        <main className="flex-1 px-4 py-6 sm:px-8 print:p-0">
          {suspended ? <SuspendedNotice /> : <Outlet />}
        </main>
      </div>
    </div>
  )
}
