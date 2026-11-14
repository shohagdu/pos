import { useEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useLocation } from 'react-router'

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
  // The most specific entry wins: /sales/warranties marks "Warranty lookup",
  // not "Sales", while /sales/123 still marks "Sales".
  const { pathname } = useLocation()
  const covers = (to: string) => (to === '/' ? pathname === '/' : pathname === to || pathname.startsWith(`${to}/`))
  const activeTo = groups
    .flatMap((g) => g.items.map((i) => i.to))
    .filter(covers)
    .sort((a, b) => b.length - a.length)[0]
  const multiple = (me?.memberships.length ?? 0) > 1
  const tenantName = me?.tenant?.name ?? ''
  // Labels stay in the DOM (sr-only) when collapsed so links keep their names.
  const narrow = collapsed ? 'lg:sr-only' : ''

  // Below lg the sidebar is a drawer: Esc closes it, focus moves into it and
  // Tab stays inside while it is open.
  const drawer = useRef<HTMLElement>(null)
  useEffect(() => {
    if (!open) return
    const panel = drawer.current
    const opener = document.activeElement instanceof HTMLElement ? document.activeElement : null
    panel?.querySelector<HTMLElement>('[data-drawer-close]')?.focus()
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
      else if (e.key === 'Tab' && panel) {
        const items = [...panel.querySelectorAll<HTMLElement>('a[href], button, select, input')].filter(
          (el) => !el.hasAttribute('disabled') && el.offsetParent !== null,
        )
        const first = items[0]
        const last = items.at(-1)
        if (e.shiftKey && document.activeElement === first) {
          e.preventDefault()
          last?.focus()
        } else if (!e.shiftKey && document.activeElement === last) {
          e.preventDefault()
          first?.focus()
        }
      }
    }
    window.addEventListener('keydown', onKey)
    return () => {
      window.removeEventListener('keydown', onKey)
      opener?.focus()
    }
  }, [open, onClose])

  return (
    <>
      {open && <div className="fixed inset-0 z-30 bg-ink/40 lg:hidden print:hidden" onClick={onClose} aria-hidden="true" />}
      <aside
        ref={drawer}
        className={`fixed inset-y-0 left-0 z-40 flex w-72 flex-col border-r border-border bg-surface transition-[transform,width] duration-200 lg:sticky lg:top-0 lg:h-svh lg:shrink-0 lg:translate-x-0 print:hidden ${
          open ? 'translate-x-0 shadow-xl' : '-translate-x-full'
        } ${collapsed ? 'lg:w-16' : 'lg:w-64'}`}
      >
        <div className={`flex items-center gap-3 px-4 pt-4 pb-3 ${collapsed ? 'lg:justify-center lg:px-2' : ''}`}>
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
            data-drawer-close
            className="flex size-11 items-center justify-center rounded-lg text-ink-muted hover:bg-canvas hover:text-ink lg:hidden"
            aria-label={t('nav.closeMenu')}
          >
            <Icon name="close" />
          </button>
        </div>

        <div className={collapsed ? 'lg:hidden' : ''}>
          <BranchSwitcher />
        </div>

        <nav aria-label={t('nav.main')} className="flex-1 overflow-y-auto px-2 pb-4">
          {groups.map((g, gi) => (
            <div key={g.section ?? gi} className={gi > 0 ? 'mt-4' : ''}>
              {g.section && (
                <div className={`px-3 pb-1 text-[11px] font-semibold tracking-wider text-ink-muted/80 uppercase ${collapsed ? 'lg:hidden' : ''}`}>
                  {t(g.section)}
                </div>
              )}
              {g.section && collapsed && <div className="mx-3 mb-2 hidden border-t border-border lg:block" aria-hidden="true" />}
              <ul className="flex flex-col gap-0.5">
                {g.items.map((item) => {
                  const isActive = item.to === activeTo
                  return (
                    <li key={item.to}>
                      <Link
                        to={item.to}
                        aria-current={isActive ? 'page' : undefined}
                        onClick={onClose}
                        title={collapsed ? t(item.key) : undefined}
                        className={`group relative flex min-h-11 items-center lg:min-h-10 gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors ${
                          collapsed ? 'lg:justify-center lg:px-0' : ''
                        } ${isActive ? 'bg-brand-50 text-brand-700' : 'text-ink-muted hover:bg-canvas hover:text-ink'}`}
                      >
                        {isActive && <span className="absolute inset-y-1.5 left-0 w-1 rounded-r bg-brand-600" aria-hidden="true" />}
                        <Icon name={item.icon} className={`size-5 ${isActive ? 'text-brand-600' : 'text-ink-muted group-hover:text-ink'}`} />
                        <span className={`truncate ${narrow}`}>{t(item.key)}</span>
                      </Link>
                    </li>
                  )
                })}
              </ul>
            </div>
          ))}
        </nav>

        <div className="hidden border-t border-border p-2 lg:block">
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
