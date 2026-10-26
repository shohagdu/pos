// The Super Admin console (platform staff only; Requirement Analysis §7.3).

import { useInfiniteQuery, useQuery } from '@tanstack/react-query'

import type { PaymentClaim, SubscriptionInvoice } from './billing'
import { apiFetch } from './client'
import { PAGE_SIZE, queryString } from './query'

export type StaffRole = 'platform_owner' | 'billing' | 'support'
export type SubscriptionState = 'trialing' | 'active' | 'grace' | 'read_only' | 'suspended' | 'cancelled' | 'terminated'

export interface Staff {
  user_id: string
  full_name: string
  role: StaffRole
  created_at: string
}

/** What a role may do (mirrors the server, ADM-R11). */
export const canAdminister = (role?: StaffRole) => role === 'platform_owner'
export const canBill = (role?: StaffRole) => role === 'platform_owner' || role === 'billing'

export interface PlatformMetrics {
  businesses: number
  by_status: Record<string, number>
  mrr: string
  arr: string
  signups_30d: number
  signups_7d: number
  revenue_30d: string
  pending_claims: number
  by_plan: Record<string, number>
}

export interface TenantRow {
  id: string
  name: string
  slug: string
  business_type: string
  /** Business status: `active` or `suspended`. */
  status: string
  phone: string | null
  email: string | null
  district: string | null
  plan_code: string | null
  plan_name: string | null
  stored_subscription_status: string | null
  /** Effective (derived from the dates). */
  subscription_status: SubscriptionState | null
  owner_name: string | null
  owner_phone: string | null
  trial_ends_at: string | null
  current_period_end: string | null
  grace_ends_at: string | null
  read_only_ends_at: string | null
  created_at: string
}

export interface TenantFilter {
  q?: string
  subscription_status?: string
  status?: string
  plan_code?: string
}

export interface TenantPage {
  data: TenantRow[]
  next_cursor: string | null
  counts: Record<string, number>
  total: number
}

export interface Override {
  id: string
  feature_code: string
  feature_name: string
  kind: 'flag' | 'limit'
  enabled: boolean | null
  limit_value: number | null
  expires_at: string | null
  reason: string
  created_by_name: string | null
  created_at: string
}

export interface OverrideInput {
  feature_code: string
  enabled?: boolean
  limit_value?: number
  expires_at?: string
  reason: string
}

export interface TenantDetail extends TenantRow {
  suspended_reason: string | null
  billing_cycle: string | null
  price_amount: string | null
  days_left: number | null
  last_activity_at: string | null
  members: { user_id: string; full_name: string; phone: string | null; email: string | null; is_owner: boolean; status: string }[]
  usage: { code: string; used: number; limit: number | null }[]
  events: {
    id: string
    event_type: string
    from_status: string | null
    to_status: string | null
    from_plan: string | null
    to_plan: string | null
    actor_type: string
    actor_name: string | null
    reason: string | null
    created_at: string
  }[]
  invoices: SubscriptionInvoice[]
  claims: PaymentClaim[]
  overrides: Override[]
  entitlements: { code: string; name: string; kind: 'flag' | 'limit'; enabled: boolean; limit: number | null; overridden: boolean }[]
}

export interface AdminPlan {
  id: string
  code: string
  name: string
  description: string | null
  is_public: boolean
  is_trial: boolean
  trial_days: number | null
  sort_order: number
  status: 'active' | 'retired'
  subscribers: number
  prices: { billing_cycle: 'monthly' | 'quarterly' | 'annual' | 'lifetime'; amount: string }[]
  features: { code: string; name: string; kind: 'flag' | 'limit'; enabled: boolean; limit_value: number | null }[]
  updated_at: string
}

export interface PlanInput {
  code?: string
  name: string
  description?: string
  is_public: boolean
  sort_order?: number
  status?: 'active' | 'retired'
  trial_days?: number
  /** Cycle → price; a missing cycle is not sold. */
  prices: Partial<Record<'monthly' | 'quarterly' | 'annual' | 'lifetime', string>>
  features: { code: string; enabled: boolean; limit_value?: number | null }[]
}

export type LicenceKind = 'trial' | 'annual' | 'lifetime'

export interface LicenceInput {
  kind: LicenceKind
  /** Annual and lifetime: the plan sold. */
  plan_code?: string
  /** Trial: days to add (1–365). */
  days?: number
  /** Annual: years (1–10). */
  years?: number
  /** What the business paid. */
  price_amount?: string
  reason: string
}

export interface ClaimPage {
  data: PaymentClaim[]
  next_cursor: string | null
  pending_count: number
}

export const platformApi = {
  me: () => apiFetch<Staff>('/api/v1/admin/me'),
  metrics: () => apiFetch<PlatformMetrics>('/api/v1/admin/metrics'),
  tenants: (filter: TenantFilter, cursor: string | null) =>
    apiFetch<TenantPage>(`/api/v1/admin/tenants${queryString({ ...filter, limit: PAGE_SIZE, cursor })}`),
  tenant: (id: string) => apiFetch<TenantDetail>(`/api/v1/admin/tenants/${id}`),
  suspend: (id: string, reason: string) =>
    apiFetch<TenantDetail>(`/api/v1/admin/tenants/${id}/suspend`, { method: 'POST', body: { reason } }),
  reactivate: (id: string, reason: string) =>
    apiFetch<TenantDetail>(`/api/v1/admin/tenants/${id}/reactivate`, { method: 'POST', body: { reason } }),
  grantLicence: (tenantId: string, input: LicenceInput) =>
    apiFetch<TenantDetail>(`/api/v1/admin/tenants/${tenantId}/licence`, { method: 'POST', body: input }),
  setOverride: (tenantId: string, input: OverrideInput) =>
    apiFetch<Override>(`/api/v1/admin/tenants/${tenantId}/overrides`, { method: 'PUT', body: input }),
  removeOverride: (tenantId: string, overrideId: string) =>
    apiFetch<void>(`/api/v1/admin/tenants/${tenantId}/overrides/${overrideId}`, { method: 'DELETE' }),
  plans: () => apiFetch<AdminPlan[]>('/api/v1/admin/plans'),
  createPlan: (input: PlanInput) => apiFetch<AdminPlan>('/api/v1/admin/plans', { method: 'POST', body: input }),
  updatePlan: (id: string, input: PlanInput) => apiFetch<AdminPlan>(`/api/v1/admin/plans/${id}`, { method: 'PUT', body: input }),
  claims: (status: string, cursor: string | null, tenantId?: string) =>
    apiFetch<ClaimPage>(`/api/v1/admin/claims${queryString({ status, tenant_id: tenantId, limit: PAGE_SIZE, cursor })}`),
  approveClaim: (id: string) => apiFetch<PaymentClaim>(`/api/v1/admin/claims/${id}/approve`, { method: 'POST' }),
  rejectClaim: (id: string, reason: string) =>
    apiFetch<PaymentClaim>(`/api/v1/admin/claims/${id}/reject`, { method: 'POST', body: { reason } }),
  paymentInstructions: () => apiFetch<Record<string, string>>('/api/v1/admin/settings/payment-instructions'),
  updatePaymentInstructions: (value: Record<string, string>) =>
    apiFetch<Record<string, string>>('/api/v1/admin/settings/payment-instructions', { method: 'PUT', body: value }),
}

export const platformKeys = {
  all: ['platform'] as const,
  me: ['platform', 'me'] as const,
  metrics: ['platform', 'metrics'] as const,
  tenants: (filter: TenantFilter) => ['platform', 'tenants', filter] as const,
  tenant: (id: string) => ['platform', 'tenant', id] as const,
  plans: ['platform', 'plans'] as const,
  claims: (status: string) => ['platform', 'claims', status] as const,
  paymentInstructions: ['platform', 'payment-instructions'] as const,
}

/** The signed-in user's staff record; errors (403) mean "not staff". */
export const useStaff = (enabled = true) =>
  useQuery({ queryKey: platformKeys.me, queryFn: platformApi.me, retry: false, staleTime: 5 * 60_000, enabled })
export const usePlatformMetrics = () => useQuery({ queryKey: platformKeys.metrics, queryFn: platformApi.metrics })
export const useAdminTenants = (filter: TenantFilter) =>
  useInfiniteQuery({
    queryKey: platformKeys.tenants(filter),
    queryFn: ({ pageParam }) => platformApi.tenants(filter, pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (last) => last.next_cursor,
  })
export const useAdminTenant = (id: string) => useQuery({ queryKey: platformKeys.tenant(id), queryFn: () => platformApi.tenant(id) })
export const useAdminPlans = () => useQuery({ queryKey: platformKeys.plans, queryFn: platformApi.plans })
export const useAdminClaims = (status: string) =>
  useInfiniteQuery({
    queryKey: platformKeys.claims(status),
    queryFn: ({ pageParam }) => platformApi.claims(status, pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (last) => last.next_cursor,
  })
export const usePaymentInstructions = () =>
  useQuery({ queryKey: platformKeys.paymentInstructions, queryFn: platformApi.paymentInstructions })
