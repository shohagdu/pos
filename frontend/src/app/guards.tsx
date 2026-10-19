import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Navigate, Outlet, useLocation } from 'react-router'

import { Button } from '../components/ui/Button'
import { Spinner } from '../components/ui/Spinner'
import { landingPath, useSession } from '../features/auth/session'

function FullPage({ children }: { children: ReactNode }) {
  return <div className="flex min-h-svh flex-col items-center justify-center gap-4 px-4 text-center">{children}</div>
}

function SessionError() {
  const { t } = useTranslation()
  const { retry, signOut } = useSession()
  return (
    <FullPage>
      <p className="text-ink-muted">{t('errors.network')}</p>
      <div className="flex gap-2">
        <Button onClick={retry}>{t('common.retry')}</Button>
        <Button variant="secondary" onClick={() => void signOut()}>
          {t('auth.signOut')}
        </Button>
      </div>
    </FullPage>
  )
}

/** Signed-in, verified users only. Others go to sign-in or verification. */
export function RequireSession() {
  const { status, me } = useSession()
  const location = useLocation()
  if (status === 'loading') return <FullPage><Spinner /></FullPage>
  if (status === 'error') return <SessionError />
  if (status === 'anonymous' || !me) return <Navigate to="/login" replace state={{ from: location.pathname }} />
  if (!me.user.phone_verified) return <Navigate to="/verify" replace />
  return <Outlet />
}

/** Screens inside a business: the token must be scoped to a tenant. */
export function RequireTenant() {
  const { me } = useSession()
  if (!me) return null
  if (!me.tenant) return <Navigate to={landingPath(me)} replace />
  return <Outlet />
}

/** Sign-in and sign-up screens; signed-in users move on. */
export function GuestOnly() {
  const { status, me } = useSession()
  const location = useLocation()
  if (status === 'loading') return <FullPage><Spinner /></FullPage>
  if (status === 'authenticated' && me) {
    const from = (location.state as { from?: string } | null)?.from
    // Invitation links work before the user has any business.
    const usable = from && from !== '/login' && (me.tenant || from.startsWith('/invite'))
    const target = usable ? from : landingPath(me)
    return <Navigate to={target} replace />
  }
  return <Outlet />
}
