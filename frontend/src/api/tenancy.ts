import { useQuery } from '@tanstack/react-query'

import { apiFetch } from './client'

export const BUSINESS_TYPES = ['retail', 'grocery', 'fashion', 'electronics', 'wholesale', 'pharmacy', 'restaurant', 'other'] as const
export type BusinessType = (typeof BUSINESS_TYPES)[number]

export const TIMEZONES = ['Asia/Dhaka', 'Asia/Kolkata', 'Asia/Kathmandu', 'Europe/Rome', 'UTC'] as const
/** Mirrors `pos_tenancy::settings::COUNTRIES`: where the business is. */
export const COUNTRIES = ['BD', 'IT'] as const
export type CountryCode = (typeof COUNTRIES)[number]
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
  /** `BD` (default) or `IT`: Italy starts with EUR, Europe/Rome and IVA rates. */
  country_code?: CountryCode
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
  /** BIN in Bangladesh, Partita IVA in Italy. */
  bin_number: string | null
  trade_license_no: string | null
  country_code: CountryCode
  /** Italy: codice fiscale. */
  tax_code: string | null
  /** Italy: R.E.A. number and its Registro Imprese office. */
  rea_number: string | null
  registry_office: string | null
  currency_code: string
  timezone: string
  locale: string
  fiscal_year_start_month: number
  invoice_prefix: string
  /** Automatic SKUs: prefix + `sku_digits` zero-padded digits (`P0042`). */
  sku_prefix: string
  sku_digits: number
  /** Automatic barcodes: prefix + `barcode_digits` digits (`200000042`). */
  barcode_prefix: string
  barcode_digits: number
  receipt_paper: (typeof RECEIPT_PAPERS)[number]
  receipt_footer: string | null
  auto_print_receipt: boolean
  /** Show the product grid beside the cart on the POS screen. */
  show_product_panel: boolean
  /** Show photos on the POS product cards (only matters with the panel on). */
  show_product_images: boolean
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

/** Documents per type (`sales`, `purchases`, …); zero counts are left out. */
export type DocumentCounts = Partial<Record<string, number>>

/** Made by "Clear all transactions" (`clear`) or set aside by a restore (`restore`). */
export interface DataArchive {
  id: string
  kind: 'clear' | 'restore'
  status: 'held' | 'restored'
  documents: DocumentCounts
  total_rows: number
  created_by_name: string | null
  created_at: string
  restored_by_name: string | null
  restored_at: string | null
}

export interface DataResetOverview {
  live: DocumentCounts
  archives: DataArchive[]
}

export const tenancyApi = {
  /** Owner only. */
  dataReset: () => apiFetch<DataResetOverview>('/api/v1/data-reset'),
  clearTransactions: () => apiFetch<DataArchive>('/api/v1/data-reset/clear', { method: 'POST', body: {} }),
  restoreArchive: (id: string) => apiFetch<DataArchive>(`/api/v1/data-reset/archives/${id}/restore`, { method: 'POST', body: {} }),
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
  dataReset: (tenantId: string) => ['tenancy', tenantId, 'data-reset'] as const,
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
