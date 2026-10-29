import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'

import { authApi, authKeys, type SessionInfo, useSessions } from '../../api/auth'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { Spinner } from '../../components/ui/Spinner'
import { describeDevice } from '../../lib/device'
import { problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { dateLocale } from '../../lib/dates'

/** US-AUTH-05 for the user's own devices. */
export function SessionsPage() {
  const { t, i18n } = useTranslation()
  const queryClient = useQueryClient()
  const { signOut } = useSession()
  const sessions = useSessions()
  const revoke = useMutation({
    mutationFn: authApi.revokeSession,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: authKeys.sessions }),
  })
  const when = (iso: string) =>
    new Intl.DateTimeFormat(dateLocale(i18n.language), { dateStyle: 'medium', timeStyle: 'short' }).format(new Date(iso))

  return (
    <div className="flex max-w-2xl flex-col gap-6">
      <div>
        <h1 className="text-2xl font-bold">{t('sessions.title')}</h1>
        <p className="text-sm text-ink-muted">{t('sessions.intro')}</p>
      </div>
      {sessions.isPending && <Spinner />}
      {sessions.isError && <Alert tone="danger">{problemMessage(sessions.error, t)}</Alert>}
      {revoke.isError && <Alert tone="danger">{problemMessage(revoke.error, t)}</Alert>}
      <ul className="flex flex-col gap-3">
        {sessions.data?.map((s: SessionInfo) => (
          <li key={s.id}>
            <Card className="flex flex-wrap items-center justify-between gap-4 p-4">
              <div className="flex flex-col gap-0.5">
                <span className="font-medium">
                  {describeDevice(s.user_agent)}
                  {s.current && <span className="ml-2 rounded bg-brand-50 px-2 py-0.5 text-xs font-semibold text-brand-700">{t('sessions.thisDevice')}</span>}
                </span>
                <span className="text-sm text-ink-muted">
                  {t('sessions.lastActive', { when: when(s.last_used_at) })}
                  {s.ip_address && ` · ${s.ip_address}`}
                </span>
                <span className="text-xs text-ink-muted">{t('sessions.signedIn', { when: when(s.started_at) })}</span>
              </div>
              {s.current ? (
                <Button variant="secondary" onClick={() => void signOut()}>
                  {t('auth.signOut')}
                </Button>
              ) : (
                <Button variant="secondary" onClick={() => revoke.mutate(s.id)} disabled={revoke.isPending}>
                  {t('sessions.revoke')}
                </Button>
              )}
            </Card>
          </li>
        ))}
      </ul>
    </div>
  )
}
