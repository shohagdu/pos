import { useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, NavLink } from 'react-router'

import { useSession } from '../../features/auth/session'
import { BranchSwitcher } from '../../features/branches/BranchSwitcher'
import { Icon } from './Icon'
import { NAV, type NavItem } from './nav'

type Props = {
  /** Mobile drawer visibility. */
  open: boolean
  onClose: () => void
  /** Desktop icon-only mode. */
  collapsed: boolean
  onToggleCollapsed: () => void
}

export function Sidebar({ open, onClose, collapsed, onToggleCollapsed }: Props) {
  const { t } = useTranslation()
  const { me, can } = useSession()
  const allowed = (p: NavItem['permission']) => p === null || (Array.isArray(p) ? p.some((x) => can(x)) : can(p))
  const groups = NAV.map((g) => ({ ...g, items: g.items.filter((i) => allowed(i.permission)) })).filter((g) => g.items.length > 0)
  const multiple = (me?.memberships.length ?? 0) > 1
  const tenantName = me?.tenant?.name ?? ''
  // Labels stay in the DOM (sr-only) when collapsed so links keep their names.
  const narrow = collapsed ? 'md:sr-only' : ''

  useEffect(() => {
    if (!open) return
    const onKey = (e: KeyboardEvent) => e.key === 'Escape' && onClose()
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [open, onClose])

  return (
    <>
      {open && <div className="fixed inset-0 z-30 bg-ink/40 md:hidden print:hidden" onClick={onClose} aria-hidden="true" />}
      <aside
        className={`fixed inset-y-0 left-0 z-40 flex w-72 flex-col border-r border-border bg-surface transition-[transform,width] duration-200 md:sticky md:top-0 md:h-svh md:shrink-0 md:translate-x-0 print:hidden ${
          open ? 'translate-x-0 shadow-xl' : '-translate-x-full'
        } ${collapsed ? 'md:w-16' : 'md:w-64'}`}
      >
        <div className={`flex items-center gap-3 px-4 pt-4 pb-3 ${collapsed ? 'md:justify-center md:px-2' : ''}`}>
          <div
            className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-brand-600 text-base font-bold text-white"
            title={tenantName}
            aria-hidden="true"
          >
            {tenantName.trim().charAt(0).toUpperCase() || 'P'}
          </div>
          <div className={`min-w-0 flex-1 ${narrow}`}>
            <div className="truncate text-xs font-semibold tracking-wide text-brand-700 uppercase">{t('app.name')}</div>
            <div className="truncate text-sm font-semibold text-ink" title={tenantName}>
              {tenantName}
            </div>
            {multiple && (
              <Link to="/choose-business" onClick={onClose} className="text-xs text-brand-700 hover:underline">
                {t('tenant.switch')}
              </Link>
            )}
          </div>
          <button
            type="button"
            onClick={onClose}
            className="rounded-lg p-1.5 text-ink-muted hover:bg-canvas hover:text-ink md:hidden"
            aria-label={t('nav.closeMenu')}
          >
            <Icon name="close" />
          </button>
        </div>

        <div className={collapsed ? 'md:hidden' : ''}>
          <BranchSwitcher />
        </div>

        <nav aria-label={t('nav.main')} className="flex-1 overflow-y-auto px-2 pb-4">
          {groups.map((g, gi) => (
            <div key={g.section ?? gi} className={gi > 0 ? 'mt-4' : ''}>
              {g.section && (
                <div className={`px-3 pb-1 text-[11px] font-semibold tracking-wider text-ink-muted/80 uppercase ${collapsed ? 'md:hidden' : ''}`}>
                  {t(g.section)}
                </div>
              )}
              {g.section && collapsed && <div className="mx-3 mb-2 hidden border-t border-border md:block" aria-hidden="true" />}
              <ul className="flex flex-col gap-0.5">
                {g.items.map((item) => (
                  <li key={item.to}>
                    <NavLink
                      to={item.to}
                      end={item.to === '/'}
                      onClick={onClose}
                      title={collapsed ? t(item.key) : undefined}
                      className={({ isActive }) =>
                        `group relative flex min-h-10 items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors ${
                          collapsed ? 'md:justify-center md:px-0' : ''
                        } ${isActive ? 'bg-brand-50 text-brand-700' : 'text-ink-muted hover:bg-canvas hover:text-ink'}`
                      }
                    >
                      {({ isActive }) => (
                        <>
                          {isActive && <span className="absolute inset-y-1.5 left-0 w-1 rounded-r bg-brand-600" aria-hidden="true" />}
                          <Icon name={item.icon} className={`size-5 ${isActive ? 'text-brand-600' : 'text-ink-muted group-hover:text-ink'}`} />
                          <span className={`truncate ${narrow}`}>{t(item.key)}</span>
                        </>
                      )}
                    </NavLink>
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </nav>

        <div className="hidden border-t border-border p-2 md:block">
          <button
            type="button"
            onClick={onToggleCollapsed}
            className={`flex min-h-10 w-full items-center gap-3 rounded-lg px-3 text-sm font-medium text-ink-muted hover:bg-canvas hover:text-ink ${
              collapsed ? 'justify-center px-0' : ''
            }`}
            aria-label={collapsed ? t('nav.expand') : t('nav.collapse')}
            title={collapsed ? t('nav.expand') : undefined}
          >
            <Icon name={collapsed ? 'expand' : 'collapse'} />
            {!collapsed && <span>{t('nav.collapse')}</span>}
          </button>
        </div>
      </aside>
    </>
  )
}
