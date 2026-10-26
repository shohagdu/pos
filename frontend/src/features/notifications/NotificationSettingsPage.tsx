import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { type Channel, type NotificationPreference, notificationKeys, notificationsApi, useNotificationPreferences } from '../../api/notifications'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { PageHeader } from '../../components/ui/PageHeader'
import { problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { QueryStatus } from '../money/shared'

const CHANNELS: Channel[] = ['in_app', 'email', 'sms']

/** Which channels each notification type uses (`/account/notifications`). */
export function NotificationSettingsPage() {
  const { me } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const prefs = useNotificationPreferences(tenantId)
  if (!prefs.data) return <QueryStatus query={prefs} />
  return <Settings tenantId={tenantId} saved={prefs.data} hasEmail={Boolean(me?.user.email)} />
}

function Settings({ tenantId, saved, hasEmail }: { tenantId: string; saved: NotificationPreference[]; hasEmail: boolean }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const [draft, setDraft] = useState(() => Object.fromEntries(saved.map((p) => [p.notif_type, p.channels])))
  const changed = saved.filter((p) => [...p.channels].sort().join() !== [...(draft[p.notif_type] ?? [])].sort().join())
  const save = useMutation({
    mutationFn: () => notificationsApi.savePreferences(changed.map((p) => ({ notif_type: p.notif_type, channels: draft[p.notif_type] ?? [] }))),
    onSuccess: (next) => queryClient.setQueryData(notificationKeys.preferences(tenantId), next),
  })
  const toggle = (type: string, channel: Channel, on: boolean) =>
    setDraft((d) => {
      const current = d[type] ?? []
      return { ...d, [type]: on ? [...current, channel] : current.filter((c) => c !== channel) }
    })
  const smsAvailable = saved.some((p) => p.available_channels.includes('sms'))

  return (
    <div className="flex max-w-3xl flex-col gap-6">
      <Link to="/notifications" className="text-sm font-medium text-brand-700 hover:underline">
        ← {t('notifications.title')}
      </Link>
      <PageHeader title={t('notifications.settings')} subtitle={t('notifications.settingsHint')} />
      {!hasEmail && <Alert>{t('notifications.noEmail')}</Alert>}
      {!smsAvailable && <p className="text-sm text-ink-muted">{t('notifications.smsUnavailable')}</p>}
      <div className="overflow-x-auto rounded-xl border border-border bg-surface">
        <table className="w-full text-sm">
          <caption className="sr-only">{t('notifications.settings')}</caption>
          <thead className="border-b border-border bg-canvas text-xs text-ink-muted uppercase">
            <tr>
              <th scope="col" className="px-4 py-2 text-left">{t('notifications.type')}</th>
              {CHANNELS.map((c) => (
                <th key={c} scope="col" className="px-4 py-2 text-center">
                  {t(`notifications.channels.${c}`)}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {saved.map((p) => (
              <tr key={p.notif_type} className="border-b border-border last:border-b-0">
                <th scope="row" className="px-4 py-3 text-left font-normal">
                  <span className="font-medium">{t(`notifications.types.${p.notif_type}`, { defaultValue: p.notif_type })}</span>
                  <span className="block text-xs text-ink-muted">{t(`notifications.typeHints.${p.notif_type}`, { defaultValue: '' })}</span>
                </th>
                {CHANNELS.map((c) => {
                  const available = p.available_channels.includes(c)
                  return (
                    <td key={c} className="px-4 py-3 text-center">
                      <input
                        type="checkbox"
                        className="size-5 accent-brand-600 disabled:opacity-40"
                        aria-label={t('notifications.channelFor', {
                          channel: t(`notifications.channels.${c}`),
                          type: t(`notifications.types.${p.notif_type}`, { defaultValue: p.notif_type }),
                        })}
                        disabled={!available}
                        checked={available && (draft[p.notif_type] ?? []).includes(c)}
                        onChange={(e) => toggle(p.notif_type, c, e.target.checked)}
                      />
                    </td>
                  )
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {save.isError && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}
      {save.isSuccess && changed.length === 0 && <Alert tone="success">{t('settings.saved')}</Alert>}
      <Button className="sm:self-end" disabled={changed.length === 0 || save.isPending} onClick={() => save.mutate()}>
        {save.isPending ? t('common.saving') : t('common.save')}
      </Button>
    </div>
  )
}
