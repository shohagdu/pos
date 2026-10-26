import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'

import { type AppNotification, notificationKeys, notificationsApi, useNotifications } from '../../api/notifications'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { PageHeader } from '../../components/ui/PageHeader'
import { StateTabs } from '../../components/ui/Tabs'
import { formatDateTime } from '../../lib/dates'
import { problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { LinkButton, LoadMore, QueryStatus } from '../money/shared'

type View = 'all' | 'unread'

/** The member's notifications (`/notifications`, NOT-R01). */
export function NotificationsPage() {
  const { t, i18n } = useTranslation()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const { me } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const [view, setView] = useState<View>('all')
  const list = useNotifications(tenantId, view === 'unread')
  const rows = list.data?.pages.flatMap((p) => p.data) ?? []
  const unread = list.data?.pages[0]?.unread_count ?? 0
  const refresh = () => queryClient.invalidateQueries({ queryKey: notificationKeys.all(tenantId) })
  const readAll = useMutation({ mutationFn: notificationsApi.markAllRead, onSuccess: refresh })
  const open = useMutation({
    mutationFn: async (n: AppNotification) => {
      if (!n.read_at) await notificationsApi.markRead(n.id)
      return n
    },
    onSuccess: async (n) => {
      await refresh()
      if (n.link) navigate(n.link)
    },
  })

  return (
    <div className="flex max-w-3xl flex-col gap-6">
      <PageHeader
        title={t('notifications.title')}
        subtitle={unread > 0 ? t('notifications.unreadCount', { count: unread }) : t('notifications.allRead')}
        actions={
          <>
            <LinkButton to="/account/notifications">{t('notifications.settings')}</LinkButton>
            <Button variant="secondary" disabled={unread === 0 || readAll.isPending} onClick={() => readAll.mutate()}>
              {t('notifications.markAllRead')}
            </Button>
          </>
        }
      />
      <StateTabs
        label={t('notifications.title')}
        value={view}
        onChange={setView}
        items={[
          { value: 'all', label: t('notifications.all') },
          { value: 'unread', label: t('notifications.unread') },
        ]}
      />
      {(readAll.isError || open.isError) && <Alert tone="danger">{problemMessage(readAll.error ?? open.error, t)}</Alert>}
      {list.isSuccess ? (
        rows.length === 0 ? (
          <p className="text-sm text-ink-muted">{view === 'unread' ? t('notifications.noneUnread') : t('notifications.none')}</p>
        ) : (
          <ul className="flex flex-col divide-y divide-border rounded-xl border border-border bg-surface">
            {rows.map((n) => (
              <li key={n.id}>
                <button
                  type="button"
                  onClick={() => open.mutate(n)}
                  className="flex w-full items-start gap-3 px-4 py-3 text-left hover:bg-canvas focus-visible:outline-2 focus-visible:outline-brand-600"
                >
                  <span
                    aria-hidden
                    className={`mt-1.5 size-2.5 shrink-0 rounded-full ${n.read_at ? 'bg-transparent' : 'bg-brand-600'}`}
                  />
                  <span className="flex min-w-0 flex-1 flex-col gap-0.5">
                    <span className={`text-sm ${n.read_at ? '' : 'font-semibold'}`}>
                      {!n.read_at && <span className="sr-only">{t('notifications.unread')}: </span>}
                      {n.title}
                    </span>
                    {n.body && <span className="text-sm text-ink-muted">{n.body}</span>}
                    <span className="text-xs text-ink-muted">
                      {t(`notifications.types.${n.notif_type}`, { defaultValue: n.notif_type })} · {formatDateTime(n.created_at, i18n.language)}
                    </span>
                  </span>
                </button>
              </li>
            ))}
          </ul>
        )
      ) : (
        <QueryStatus query={list} />
      )}
      <LoadMore query={list} />
    </div>
  )
}
