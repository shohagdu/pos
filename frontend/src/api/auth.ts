import { useQuery } from '@tanstack/react-query'

import { apiFetch } from './client'
import type { TokenResponse } from './session'

export interface OtpChallenge {
  destination: string
  expires_in: number
  /** Only in development (AUTH_EXPOSE_OTP). */
  dev_code?: string
}

export interface UserProfile {
  id: string
  full_name: string
  phone: string | null
  email: string | null
  phone_verified: boolean
  locale: string
}

export interface MembershipSummary {
  tenant_id: string
  membership_id: string
  tenant_name: string
  tenant_slug: string
  tenant_status: string
  is_owner: boolean
}

export type SubscriptionStatus = 'trialing' | 'active' | 'grace' | 'read_only' | 'suspended' | 'cancelled' | 'terminated'

export interface SubscriptionSummary {
  /** Effective state now (derived from the dates). */
  status: SubscriptionStatus
  stored_status: SubscriptionStatus
  plan_code: string
  plan_name: string
  trial_ends_at: string | null
  current_period_end: string | null
  /** `lifetime` = one-time licence with no end date. */
  billing_cycle?: string
  days_left: number | null
}

export interface CurrentTenant {
  tenant_id: string
  membership_id: string
  name: string
  slug: string
  is_owner: boolean
  permissions: string[]
  subscription: SubscriptionSummary | null
  /** Plan features that are switched on. */
  features: string[]
  /** Branches the member may work in; `null` (or absent) = every branch. */
  branch_ids?: string[] | null
  /** ISO 4217 code money is kept in (formatting only). */
  currency_code?: string
}

export interface Me {
  user: UserProfile
  memberships: MembershipSummary[]
  tenant: CurrentTenant | null
}

export interface SessionInfo {
  id: string
  tenant_id: string | null
  ip_address: string | null
  user_agent: string | null
  started_at: string
  last_used_at: string
  expires_at: string
  current: boolean
}

export interface RegisterInput {
  full_name: string
  phone: string
  email?: string
  password: string
  locale?: string
}

const post = <T>(path: string, body: unknown = {}, auth = false) =>
  apiFetch<T>(`/api/v1/auth${path}`, { method: 'POST', body, auth })

export const authApi = {
  register: (input: RegisterInput) => post<OtpChallenge>('/register', input),
  verifyPhone: (phone: string, code: string) => post<TokenResponse>('/verify-phone', { phone, code }),
  resendVerification: (phone: string) => post<OtpChallenge>('/verify-phone/resend', { phone }),
  login: (identifier: string, password: string) => post<TokenResponse>('/login', { identifier, password }),
  logout: () => post<void>('/logout'),
  switchTenant: (tenantId: string | null) => post<TokenResponse>('/switch-tenant', { tenant_id: tenantId }, true),
  forgotPassword: (identifier: string) => post<OtpChallenge>('/password/forgot', { identifier }),
  resetPassword: (identifier: string, code: string, newPassword: string) =>
    post<void>('/password/reset', { identifier, code, new_password: newPassword }),
  me: () => apiFetch<Me>('/api/v1/auth/me'),
  sessions: () => apiFetch<SessionInfo[]>('/api/v1/auth/sessions'),
  revokeSession: (id: string) => apiFetch<void>(`/api/v1/auth/sessions/${id}`, { method: 'DELETE' }),
}

export const authKeys = {
  me: (tenantId: string | null) => ['auth', 'me', tenantId ?? 'none'] as const,
  sessions: ['auth', 'sessions'] as const,
}

export function useSessions() {
  return useQuery({ queryKey: authKeys.sessions, queryFn: authApi.sessions })
}
