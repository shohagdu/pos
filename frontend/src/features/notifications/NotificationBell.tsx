import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { useUnreadCount } from '../../api/notifications'
import { useSession } from '../auth/session'

/** Header link to the notification centre with the unread count. */
export function NotificationBell() {
  const { t } = useTranslation()
  const { me } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const unread = useUnreadCount(tenantId, tenantId !== '').data?.unread_count ?? 0
  const label = unread > 0 ? t('notifications.bellUnread', { count: unread }) : t('notifications.title')
  return (
    <Link
      to="/notifications"
      aria-label={label}
      title={label}
      className="relative inline-flex size-10 items-center justify-center rounded-lg text-ink-muted hover:bg-canvas hover:text-ink"
    >
      <svg aria-hidden viewBox="0 0 24 24" className="size-5" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
        <path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9" />
        <path d="M10.3 21a1.94 1.94 0 0 0 3.4 0" />
      </svg>
      {unread > 0 && (
        <span className="absolute -top-0.5 -right-0.5 min-w-5 rounded-full bg-danger px-1 text-center text-xs leading-5 font-semibold text-white">
          {unread > 99 ? '99+' : unread}
        </span>
      )}
    </Link>
  )
}
