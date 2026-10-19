import { createContext, useContext } from 'react'

import type { Me } from '../../api/auth'
import type { TokenResponse } from '../../api/session'

export type SessionStatus = 'loading' | 'anonymous' | 'authenticated' | 'error'

export interface SessionValue {
  status: SessionStatus
  me: Me | undefined
  tenantId: string | null
  /** Permission check for the active business (`module.action`). */
  can: (permission: string) => boolean
  /** Plan feature check (`pos.core`, `reports.export`, …), SUB-R03 `can()`. */
  hasFeature: (feature: string) => boolean
  signIn: (tokens: TokenResponse) => void
  signOut: () => Promise<void>
  switchTenant: (tenantId: string | null) => Promise<void>
  retry: () => void
}

export const SessionContext = createContext<SessionValue | null>(null)

export function useSession(): SessionValue {
  const value = useContext(SessionContext)
  if (!value) throw new Error('useSession outside SessionProvider')
  return value
}

/** Where a signed-in user belongs next. */
export function landingPath(me: Me): string {
  if (!me.user.phone_verified) return '/verify'
  if (me.tenant) return '/'
  if (me.memberships.length === 0) return '/onboarding/business'
  return '/choose-business'
}
