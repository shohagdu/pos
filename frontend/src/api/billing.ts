import { useQuery } from '@tanstack/react-query'

import type { SubscriptionSummary } from './auth'
import { apiFetch } from './client'
import { getSession, refreshSession } from './session'

export interface PlanFeature {
  code: string
  name: string
  kind: 'flag' | 'limit'
  enabled: boolean
  /** For limits; `null` = unlimited. */
  limit: number | null
  overridden: boolean
}

export interface UsageMeter {
  feature: string
  name: string
  /** `null` while the module that owns the data does not exist yet. */
  used: number | null
  limit: number | null
}

export interface SubscriptionOverview {
  subscription: SubscriptionSummary
  features: PlanFeature[]
  usage: UsageMeter[]
}

export const billingKeys = { subscription: (tenantId: string) => ['billing', tenantId, 'subscription'] as const }

export function useSubscription(tenantId: string) {
  return useQuery({
    queryKey: billingKeys.subscription(tenantId),
    queryFn: () => apiFetch<SubscriptionOverview>('/api/v1/subscription'),
    enabled: tenantId !== '',
  })
}

// ------------------------------------------------- manual billing (SUB-R05)

export type BillingCycle = 'monthly' | 'quarterly' | 'annual' | 'lifetime'
export type ClaimMethod = 'bkash' | 'nagad' | 'rocket' | 'bank_transfer' | 'cash' | 'other'
export const CLAIM_METHODS: ClaimMethod[] = ['bkash', 'nagad', 'rocket', 'bank_transfer', 'cash', 'other']

export interface PlanOffer {
  id: string
  code: string
  name: string
  description: string | null
  prices: { billing_cycle: BillingCycle; amount: string }[]
  features: { code: string; name: string; kind: 'flag' | 'limit'; enabled: boolean; limit_value: number | null }[]
}

export interface SubscriptionInvoice {
  id: string
  tenant_id: string
  invoice_no: string
  plan_code: string
  plan_name: string
  billing_cycle: BillingCycle
  period_start: string
  period_end: string
  subtotal: string
  discount_amount: string
  total: string
  status: 'open' | 'paid' | 'void'
  paid_at: string | null
  created_at: string
}

export interface PaymentClaim {
  id: string
  tenant_id: string
  tenant_name: string
  invoice_id: string
  invoice_no: string
  plan_name: string
  billing_cycle: BillingCycle
  invoice_total: string
  method: ClaimMethod
  amount: string
  transaction_ref: string
  payer_account: string | null
  status: 'pending' | 'approved' | 'rejected'
  rejection_reason: string | null
  submitted_by_name: string | null
  reviewed_by_name: string | null
  reviewed_at: string | null
  created_at: string
}

export interface BillingOverview {
  plans: PlanOffer[]
  invoices: SubscriptionInvoice[]
  claims: PaymentClaim[]
  /** Short texts keyed by method (`bkash`, `nagad`, `bank_transfer`) and `note`. */
  payment_instructions: Record<string, string>
}

export interface ClaimInput {
  method: ClaimMethod
  amount: string
  transaction_ref: string
  payer_account?: string
}

export const billingApi = {
  overview: () => apiFetch<BillingOverview>('/api/v1/subscription/billing'),
  requestInvoice: (plan_code: string, billing_cycle: BillingCycle) =>
    apiFetch<SubscriptionInvoice>('/api/v1/subscription/invoices', { method: 'POST', body: { plan_code, billing_cycle } }),
  submitClaim: (invoiceId: string, input: ClaimInput) =>
    apiFetch<PaymentClaim>(`/api/v1/subscription/invoices/${invoiceId}/claims`, { method: 'POST', body: input }),
}

export const billingOverviewKey = (tenantId: string) => ['billing', tenantId, 'overview'] as const

export function useBillingOverview(tenantId: string, enabled = true) {
  return useQuery({ queryKey: billingOverviewKey(tenantId), queryFn: billingApi.overview, enabled: enabled && tenantId !== '' })
}

/**
 * Downloads the full data export (ZIP of CSVs, NFR-PRIV-01). Uses the same
 * bearer token as other calls; the browser saves the file.
 */
export async function downloadExport(): Promise<void> {
  const request = () => {
    const token = getSession().token
    return fetch('/api/v1/subscription/export', {
      headers: token ? { Authorization: `Bearer ${token}` } : {},
      credentials: 'same-origin',
    })
  }
  let response = await request()
  // An expired access token: refresh once, like apiFetch does.
  if (response.status === 401 && (await refreshSession())) response = await request()
  if (!response.ok) throw new Error(`export failed: ${response.status}`)
  const blob = await response.blob()
  const disposition = response.headers.get('content-disposition') ?? ''
  const name = /filename="([^"]+)"/.exec(disposition)?.[1] ?? 'pos-export.zip'
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = name
  document.body.appendChild(a)
  a.click()
  a.remove()
  URL.revokeObjectURL(url)
}
