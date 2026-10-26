import { useInfiniteQuery, useQuery } from '@tanstack/react-query'

import { apiFetch } from './client'
import { PAGE_SIZE, type Page, queryString } from './query'

export const NOTIFICATION_TYPES = [
  'stock.low',
  'customer.due',
  'supplier.due',
  'subscription.expiring',
  'sales.daily_summary',
  'security.alert',
  'approval.request',
] as const

export type Channel = 'in_app' | 'email' | 'sms'

export interface AppNotification {
  id: string
  notif_type: string
  title: string
  body: string | null
  /** Page in the app to open, e.g. `/inventory?status=low`. */
  link: string | null
  data: Record<string, unknown>
  read_at: string | null
  created_at: string
}

export interface NotificationPage extends Page<AppNotification> {
  unread_count: number
}

export interface NotificationPreference {
  notif_type: string
  /** Empty = off. */
  channels: Channel[]
  default_channels: Channel[]
  /** SMS needs the plan feature `notifications.sms`. */
  available_channels: Channel[]
}

export const notificationsApi = {
  list: (unreadOnly: boolean, cursor: string | null) =>
    apiFetch<NotificationPage>(`/api/v1/notifications${queryString({ unread_only: unreadOnly || undefined, limit: PAGE_SIZE, cursor })}`),
  unreadCount: () => apiFetch<{ unread_count: number }>('/api/v1/notifications/unread-count'),
  markRead: (id: string) => apiFetch<void>(`/api/v1/notifications/${id}/read`, { method: 'POST' }),
  markAllRead: () => apiFetch<void>('/api/v1/notifications/read-all', { method: 'POST' }),
  preferences: () => apiFetch<NotificationPreference[]>('/api/v1/notification-preferences'),
  savePreferences: (changes: { notif_type: string; channels: Channel[] }[]) =>
    apiFetch<NotificationPreference[]>('/api/v1/notification-preferences', { method: 'PUT', body: changes }),
}

export const notificationKeys = {
  all: (tenantId: string) => ['notifications', tenantId] as const,
  list: (tenantId: string, unreadOnly: boolean) => ['notifications', tenantId, 'list', unreadOnly] as const,
  unread: (tenantId: string) => ['notifications', tenantId, 'unread'] as const,
  preferences: (tenantId: string) => ['notifications', tenantId, 'preferences'] as const,
}

/** Polled by the header bell; refetched when the window regains focus. */
export const useUnreadCount = (tenantId: string, enabled = true) =>
  useQuery({
    queryKey: notificationKeys.unread(tenantId),
    queryFn: notificationsApi.unreadCount,
    enabled,
    refetchInterval: 60_000,
  })

export const useNotifications = (tenantId: string, unreadOnly: boolean) =>
  useInfiniteQuery({
    queryKey: notificationKeys.list(tenantId, unreadOnly),
    queryFn: ({ pageParam }) => notificationsApi.list(unreadOnly, pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (last) => last.next_cursor,
  })

export const useNotificationPreferences = (tenantId: string) =>
  useQuery({ queryKey: notificationKeys.preferences(tenantId), queryFn: notificationsApi.preferences })
