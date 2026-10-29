import { useCallback, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, Outlet, useLocation } from 'react-router'

import { useSession } from '../../features/auth/session'
import { LANGUAGES } from '../../i18n'
import { NotificationBell } from '../../features/notifications/NotificationBell'
import { SuspendedNotice } from '../../features/billing/SuspendedNotice'
import { TrialBanner } from '../../features/dashboard/TrialBanner'
import { Icon } from './Icon'
import { navKeyFor } from './nav'
import { Sidebar } from './Sidebar'
import { UserMenu } from './UserMenu'

// Header titles for pages that are not in the sidebar.
const EXTRA_TITLES: Record<string, string> = {
  notifications: 'notifications.title',
  account: 'sessions.title',
  admin: 'nav.admin',
}

// Pages with their own "New sale" button (or the till itself) skip the header shortcut.
const OWN_NEW_SALE = ['/', '/pos', '/sales']

const COLLAPSED_KEY = 'pos.sidebar.collapsed'

function readCollapsed() {
  try {
    return localStorage.getItem(COLLAPSED_KEY) === '1'
  } catch {
    return false
  }
}

export function AppLayout() {
  const { t, i18n } = useTranslation()
  const { me, can } = useSession()
  const [menuOpen, setMenuOpen] = useState(false)
  const [collapsed, setCollapsed] = useState(readCollapsed)
  const closeMenu = useCallback(() => setMenuOpen(false), [])
  const toggleCollapsed = () =>
    setCollapsed((c) => {
      try {
        localStorage.setItem(COLLAPSED_KEY, c ? '0' : '1')
      } catch {
        // Storage unavailable: the choice just won't survive a reload.
      }
      return !c
    })
  const { pathname } = useLocation()
  // Suspended: only the plan page is usable (the API refuses everything else).
  const suspended = me?.tenant?.subscription?.status === 'suspended' && pathname !== '/plan'
  const titleKey = navKeyFor(pathname) ?? EXTRA_TITLES[pathname.split('/')[1] ?? '']
  const title = titleKey ? t(titleKey) : ''

  return (
    <div className="flex min-h-svh flex-col md:flex-row">
      <Sidebar open={menuOpen} onClose={closeMenu} collapsed={collapsed} onToggleCollapsed={toggleCollapsed} />
      <div className="flex min-w-0 flex-1 flex-col">
        <div className="print:hidden">
          <TrialBanner />
        </div>
        <header className="sticky top-0 z-20 flex items-center gap-2 border-b border-border bg-surface/95 px-4 py-2 backdrop-blur sm:gap-3 sm:px-8 print:hidden">
          <button
            type="button"
            onClick={() => setMenuOpen(true)}
            className="-ml-1.5 rounded-lg p-1.5 text-ink-muted hover:bg-canvas hover:text-ink md:hidden"
            aria-label={t('nav.openMenu')}
            aria-expanded={menuOpen}
          >
            <Icon name="menu" className="size-6" />
          </button>
          <div className="min-w-0 flex-1 truncate text-base font-semibold text-ink">{title}</div>
          {can('pos.sell') && !OWN_NEW_SALE.includes(pathname) && (
            <Link
              to="/pos"
              className="inline-flex min-h-9 items-center gap-1.5 rounded-lg bg-brand-600 px-3 text-sm font-semibold text-white hover:bg-brand-700"
              aria-label={t('nav.newSale')}
            >
              <Icon name="plus" className="size-4" />
              <span className="hidden sm:inline">{t('nav.newSale')}</span>
            </Link>
          )}
          <label className="relative inline-flex min-h-9 items-center gap-1.5 rounded-lg px-2 text-sm font-medium text-ink-muted hover:bg-canvas hover:text-ink">
            <Icon name="language" className="size-5" />
            <span className="sr-only">{t('common.language')}</span>
            {/* Each option is written in its own language. */}
            <select
              value={i18n.language}
              onChange={(e) => void i18n.changeLanguage(e.target.value)}
              title={t('common.language')}
              className="cursor-pointer appearance-none bg-transparent text-sm font-medium outline-none max-sm:absolute max-sm:inset-0 max-sm:opacity-0"
            >
              {LANGUAGES.map((l) => (
                <option key={l.code} value={l.code} lang={l.code}>
                  {l.label}
                </option>
              ))}
            </select>
          </label>
          <NotificationBell />
          <UserMenu />
        </header>
        <main className="flex-1 px-4 py-6 sm:px-8 print:p-0">
          {suspended ? <SuspendedNotice /> : <Outlet />}
        </main>
      </div>
    </div>
  )
}
