import { useQuery, useQueryClient } from '@tanstack/react-query'
import { type ReactNode, useCallback, useEffect, useMemo, useState, useSyncExternalStore } from 'react'

import { authApi, authKeys } from '../../api/auth'
import { getSession, refreshSession, setTokens, subscribeSession, type TokenResponse } from '../../api/session'
import { setBusinessCurrency } from '../../lib/format'
import { SessionContext, type SessionStatus, type SessionValue } from './session'

export function SessionProvider({ children }: { children: ReactNode }) {
  const queryClient = useQueryClient()
  const session = useSyncExternalStore(subscribeSession, getSession)
  // On load the access token is gone (memory only); the refresh cookie may still be valid.
  const [restored, setRestored] = useState(session.token !== null)

  useEffect(() => {
    if (restored) return
    refreshSession()
      .catch(() => false)
      .finally(() => setRestored(true))
  }, [restored])

  const me = useQuery({
    queryKey: authKeys.me(session.tenantId),
    queryFn: async () => {
      const me = await authApi.me()
      // Set before anything renders, so every amount shows the business's currency.
      setBusinessCurrency(me.tenant?.currency_code)
      return me
    },
    enabled: session.token !== null,
    staleTime: 60_000,
  })

  const signIn = useCallback((tokens: TokenResponse) => setTokens(tokens), [])

  const signOut = useCallback(async () => {
    try {
      await authApi.logout()
    } catch {
      // Signing out locally still works when the server is unreachable.
    }
    setTokens(null)
    queryClient.clear()
  }, [queryClient])

  const switchTenant = useCallback(async (tenantId: string | null) => {
    setTokens(await authApi.switchTenant(tenantId))
  }, [])

  const value = useMemo<SessionValue>(() => {
    let status: SessionStatus
    if (!restored) status = 'loading'
    else if (!session.token) status = 'anonymous'
    else if (me.data) status = 'authenticated'
    else if (me.isError) status = 'error'
    else status = 'loading'
    const permissions = new Set(me.data?.tenant?.permissions ?? [])
    const features = new Set(me.data?.tenant?.features ?? [])
    return {
      status,
      me: me.data,
      tenantId: session.tenantId,
      can: (p) => permissions.has(p),
      hasFeature: (f) => features.has(f),
      signIn,
      signOut,
      switchTenant,
      retry: () => void me.refetch(),
    }
  }, [restored, session, me, signIn, signOut, switchTenant])

  return <SessionContext.Provider value={value}>{children}</SessionContext.Provider>
}
