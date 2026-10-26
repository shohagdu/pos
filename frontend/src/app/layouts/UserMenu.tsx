import { useEffect, useId, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { useStaff } from '../../api/platform'
import { useSession } from '../../features/auth/session'
import { Icon, type IconName } from './Icon'

function initials(name: string) {
  const parts = name.trim().split(/\s+/).filter(Boolean)
  return ((parts[0]?.charAt(0) ?? '') + (parts.length > 1 ? (parts.at(-1)?.charAt(0) ?? '') : '')).toUpperCase() || '?'
}

/** Avatar button opening the signed-in user's account panel. */
export function UserMenu() {
  const { t } = useTranslation()
  const { me, signOut } = useSession()
  const staff = useStaff()
  const [open, setOpen] = useState(false)
  const root = useRef<HTMLDivElement>(null)
  const panelId = useId()
  const name = me?.user.full_name ?? ''
  const contact = me?.user.phone ?? me?.user.email ?? ''

  useEffect(() => {
    if (!open) return
    const onDown = (e: PointerEvent) => {
      if (!root.current?.contains(e.target as Node)) setOpen(false)
    }
    const onKey = (e: KeyboardEvent) => e.key === 'Escape' && setOpen(false)
    document.addEventListener('pointerdown', onDown)
    window.addEventListener('keydown', onKey)
    return () => {
      document.removeEventListener('pointerdown', onDown)
      window.removeEventListener('keydown', onKey)
    }
  }, [open])

  const item = 'flex min-h-10 w-full items-center gap-3 rounded-lg px-3 text-left text-sm font-medium hover:bg-canvas'
  const link = (to: string, icon: IconName, label: string) => (
    <Link to={to} onClick={() => setOpen(false)} className={`${item} text-ink`}>
      <Icon name={icon} className="size-5 text-ink-muted" />
      {label}
    </Link>
  )

  return (
    <div ref={root} className="relative">
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        aria-expanded={open}
        aria-controls={panelId}
        aria-label={t('nav.account')}
        className="flex items-center gap-2 rounded-full p-0.5 pr-2 hover:bg-canvas"
      >
        <span className="flex size-9 items-center justify-center rounded-full bg-brand-100 text-sm font-semibold text-brand-700" aria-hidden="true">
          {initials(name)}
        </span>
        <span className="hidden max-w-40 truncate text-sm font-medium text-ink lg:inline">{name}</span>
        <Icon name="chevronDown" className="size-4 text-ink-muted" />
      </button>
      {open && (
        <div id={panelId} className="absolute right-0 z-50 mt-2 w-72 rounded-xl border border-border bg-surface p-2 shadow-lg">
          <div className="flex items-center gap-3 px-3 py-2">
            <span className="flex size-10 shrink-0 items-center justify-center rounded-full bg-brand-100 font-semibold text-brand-700" aria-hidden="true">
              {initials(name)}
            </span>
            <div className="min-w-0">
              <div className="truncate text-sm font-semibold text-ink">{name}</div>
              {contact && <div className="truncate text-xs text-ink-muted">{contact}</div>}
              {me?.tenant?.name && <div className="truncate text-xs text-ink-muted">{me.tenant.name}</div>}
            </div>
          </div>
          <div className="my-1 border-t border-border" />
          {staff.data && link('/admin', 'shield', t('nav.admin'))}
          {link('/account/sessions', 'devices', t('sessions.title'))}
          <div className="my-1 border-t border-border" />
          <button type="button" onClick={() => void signOut()} className={`${item} text-danger`}>
            <Icon name="signOut" className="size-5" />
            {t('auth.signOut')}
          </button>
        </div>
      )}
    </div>
  )
}
