import { useQuery } from '@tanstack/react-query'

import { apiFetch } from './client'

export const BUSINESS_TYPES = ['retail', 'grocery', 'fashion', 'electronics', 'wholesale', 'pharmacy', 'restaurant', 'other'] as const
export type BusinessType = (typeof BUSINESS_TYPES)[number]

export const TIMEZONES = ['Asia/Dhaka', 'Asia/Kolkata', 'Asia/Kathmandu', 'UTC'] as const
export const RECEIPT_PAPERS = ['80mm', '58mm', 'a4'] as const
/** Mirrors `pos_tenancy::settings::CURRENCIES` (2-decimal currencies only). */
export const CURRENCIES = ['BDT', 'INR', 'PKR', 'NPR', 'LKR', 'USD', 'EUR', 'GBP', 'AED', 'SAR', 'QAR', 'MYR', 'SGD', 'AUD', 'CAD'] as const

export interface NewBusiness {
  name: string
  business_type: BusinessType
  address?: string
  district?: string
  phone?: string
  email?: string
  bin_number?: string
  trade_license_no?: string
  locale?: string
}

export interface ProvisionedTenant {
  tenant_id: string
  slug: string
  name: string
  branch_id: string
  membership_id: string
}

export interface BusinessSettings {
  name: string
  slug: string
  legal_name: string | null
  business_type: BusinessType
  phone: string | null
  email: string | null
  address: string | null
  district: string | null
  bin_number: string | null
  trade_license_no: string | null
  currency_code: string
  timezone: string
  locale: string
  fiscal_year_start_month: number
  invoice_prefix: string
  receipt_paper: (typeof RECEIPT_PAPERS)[number]
  receipt_footer: string | null
  auto_print_receipt: boolean
  is_vat_registered: boolean
  prices_include_tax: boolean
  version: number
}

export type SettingsPatch = Partial<Omit<BusinessSettings, 'slug' | 'version'>>

export interface Branch {
  id: string
  code: string
  name: string
  address: string | null
  phone: string | null
  receipt_header: string | null
  is_default: boolean
  status: string
  version: number
}

export type BranchPatch = Partial<Pick<Branch, 'name' | 'address' | 'phone' | 'receipt_header'>>

export interface NewBranch {
  code: string
  name: string
  address?: string
  phone?: string
  receipt_header?: string
}

export const ONBOARDING_STEPS = ['business', 'regional', 'branch', 'invoice', 'products', 'staff'] as const
export type OnboardingStep = (typeof ONBOARDING_STEPS)[number]

export interface OnboardingStatus {
  steps: { key: OnboardingStep; done: boolean }[]
  dismissed: boolean
  complete: boolean
}

export const tenancyApi = {
  createTenant: (input: NewBusiness) => apiFetch<ProvisionedTenant>('/api/v1/tenants', { method: 'POST', body: input }),
  settings: () => apiFetch<BusinessSettings>('/api/v1/settings'),
  updateSettings: (version: number, patch: SettingsPatch) =>
    apiFetch<BusinessSettings>('/api/v1/settings', { method: 'PATCH', body: patch, ifMatch: version }),
  branches: () => apiFetch<Branch[]>('/api/v1/branches'),
  updateBranch: (id: string, version: number, patch: BranchPatch) =>
    apiFetch<Branch>(`/api/v1/branches/${id}`, { method: 'PATCH', body: patch, ifMatch: version }),
  createBranch: (input: NewBranch) => apiFetch<Branch>('/api/v1/branches', { method: 'POST', body: input }),
  activateBranch: (id: string) => apiFetch<Branch>(`/api/v1/branches/${id}/activate`, { method: 'POST' }),
  deactivateBranch: (id: string) => apiFetch<Branch>(`/api/v1/branches/${id}/deactivate`, { method: 'POST' }),
  onboarding: () => apiFetch<OnboardingStatus>('/api/v1/onboarding'),
  completeStep: (step: OnboardingStep) =>
    apiFetch<OnboardingStatus>(`/api/v1/onboarding/steps/${step}`, { method: 'POST' }),
  dismissOnboarding: () => apiFetch<OnboardingStatus>('/api/v1/onboarding/dismiss', { method: 'POST' }),
}

// Keys include the tenant so switching business never shows the previous one's data.
export const tenancyKeys = {
  settings: (tenantId: string) => ['tenancy', tenantId, 'settings'] as const,
  branches: (tenantId: string) => ['tenancy', tenantId, 'branches'] as const,
  onboarding: (tenantId: string) => ['tenancy', tenantId, 'onboarding'] as const,
}

export function useSettings(tenantId: string) {
  return useQuery({ queryKey: tenancyKeys.settings(tenantId), queryFn: tenancyApi.settings })
}

export function useBranches(tenantId: string) {
  return useQuery({ queryKey: tenancyKeys.branches(tenantId), queryFn: tenancyApi.branches })
}

export function useOnboarding(tenantId: string, enabled = true) {
  return useQuery({ queryKey: tenancyKeys.onboarding(tenantId), queryFn: tenancyApi.onboarding, enabled })
}
