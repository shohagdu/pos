import { keepPreviousData, useQuery } from '@tanstack/react-query'

import type { Promotion as EnginePromotion } from '../lib/promotions'
import { apiFetch } from './client'
import { queryString } from './query'

export type PromotionType = 'product_discount' | 'flash' | 'bxgy' | 'coupon'
export type PromotionStatus = 'draft' | 'active' | 'paused' | 'ended'
/** What a promotion is now (dates, limits and status together). */
export type PromotionState = 'draft' | 'scheduled' | 'running' | 'paused' | 'expired' | 'sold_out' | 'ended'
export type Weekday = 'mon' | 'tue' | 'wed' | 'thu' | 'fri' | 'sat' | 'sun'

export interface Schedule {
  /** Business local time `YYYY-MM-DDTHH:MM:SS`. */
  starts_at: string | null
  ends_at: string | null
  active_days: Weekday[] | null
  /** `HH:MM:SS`. */
  active_time_from: string | null
  active_time_to: string | null
}

/** A live automatic promotion for the POS (`GET /pos/promotions`). */
export type LivePromotion = EnginePromotion & Schedule

export interface TargetInput {
  target_type: 'product' | 'category' | 'brand' | 'all_products'
  target_id?: string | null
  is_exclusion?: boolean
}

export interface TargetOut extends TargetInput {
  target_id: string | null
  is_exclusion: boolean
  name: string | null
}

export interface BxgyInput {
  buy_quantity: string
  get_quantity: string
  get_target_type: 'same_product' | 'product' | 'category'
  get_target_id?: string | null
  get_discount_type: 'free' | 'percent'
  get_discount_value?: string | null
  repeat: boolean
  auto_add_free_item: boolean
}

export interface BxgyOut extends BxgyInput {
  get_target_name: string | null
  get_discount_value: string
}

export interface PromotionInput {
  name: string
  name_bn?: string | null
  type: PromotionType
  discount_type?: string | null
  discount_value?: string | null
  max_discount_amount?: string | null
  min_purchase_amount?: string | null
  starts_at?: string | null
  ends_at?: string | null
  active_days?: Weekday[] | null
  active_time_from?: string | null
  active_time_to?: string | null
  priority?: number
  stackable?: boolean
  applies_to_customer?: 'all' | 'registered_only'
  total_usage_limit?: number | null
  per_customer_limit?: number | null
  quantity_limit?: string | null
  bxgy?: BxgyInput | null
  targets: TargetInput[]
  branch_ids?: string[]
  status?: 'draft' | 'active'
}

export interface PromotionSummary {
  id: string
  name: string
  name_bn: string | null
  type: PromotionType
  discount_type: string
  discount_value: string
  status: PromotionStatus
  state: PromotionState
  starts_at: string | null
  ends_at: string | null
  priority: number
  usage_count: number
  total_usage_limit: number | null
  quantity_used: string
  quantity_limit: string | null
  coupon_count: number
  branches: string[]
  first_used_at: string | null
  created_at: string
}

export interface PromotionDetail extends Schedule {
  id: string
  name: string
  name_bn: string | null
  type: PromotionType
  discount_type: string
  discount_value: string
  max_discount_amount: string | null
  min_purchase_amount: string | null
  priority: number
  stackable: boolean
  applies_to_customer: 'all' | 'registered_only'
  total_usage_limit: number | null
  usage_count: number
  per_customer_limit: number | null
  quantity_limit: string | null
  quantity_used: string
  bxgy: BxgyOut | null
  targets: TargetOut[]
  branch_ids: string[]
  status: PromotionStatus
  state: PromotionState
  /** Used by a sale: type, benefit and products can no longer change. */
  locked: boolean
  coupon_count: number
  matched_products: number
  version: number
  created_at: string
  updated_at: string
}

export interface PromotionFilter {
  type?: PromotionType | ''
  state?: PromotionState | 'all' | ''
  branch_id?: string
  q?: string
}

export interface Coupon {
  id: string
  promotion_id: string
  code: string
  type: 'public' | 'unique'
  usage_limit: number | null
  used_count: number
  assigned_customer_id: string | null
  customer_name: string | null
  expires_at: string | null
  status: 'active' | 'used' | 'disabled'
  created_at: string
}

export interface CouponInput {
  code: string
  usage_limit?: number | null
  expires_at?: string | null
  assigned_customer_id?: string | null
}

export interface GenerateInput {
  count: number
  prefix?: string
  usage_limit?: number
  expires_at?: string | null
}

export interface Redemption {
  id: string
  code: string
  sale_id: string
  invoice_no: string
  customer_name: string
  cashier_name: string | null
  branch_name: string
  discount_amount: string
  status: 'applied' | 'reversed'
  created_at: string
}

export type PromotionAction = 'activate' | 'pause' | 'resume' | 'end' | 'duplicate'

export const promotionsApi = {
  list: (filter: PromotionFilter) => apiFetch<{ data: PromotionSummary[]; total: number }>(`/api/v1/promotions${queryString({ ...filter, limit: 200 })}`),
  get: (id: string) => apiFetch<PromotionDetail>(`/api/v1/promotions/${id}`),
  create: (input: PromotionInput) => apiFetch<PromotionDetail>('/api/v1/promotions', { method: 'POST', body: input }),
  update: (id: string, version: number, input: PromotionInput) =>
    apiFetch<PromotionDetail>(`/api/v1/promotions/${id}`, { method: 'PUT', body: input, ifMatch: version }),
  remove: (id: string) => apiFetch<void>(`/api/v1/promotions/${id}`, { method: 'DELETE' }),
  action: (id: string, action: PromotionAction) => apiFetch<PromotionDetail>(`/api/v1/promotions/${id}/${action}`, { method: 'POST' }),
  targetCount: (targets: TargetInput[]) =>
    apiFetch<{ matched_products: number }>('/api/v1/promotions/target-count', { method: 'POST', body: { targets } }),
  coupons: (id: string, filter: { q?: string; status?: string; limit?: number; offset?: number }) =>
    apiFetch<{ data: Coupon[]; total: number }>(`/api/v1/promotions/${id}/coupons${queryString(filter)}`),
  addCoupon: (id: string, input: CouponInput) => apiFetch<Coupon>(`/api/v1/promotions/${id}/coupons`, { method: 'POST', body: input }),
  generate: (id: string, input: GenerateInput) => apiFetch<Coupon[]>(`/api/v1/promotions/${id}/coupons/generate`, { method: 'POST', body: input }),
  couponAction: (couponId: string, action: 'disable' | 'enable') => apiFetch<Coupon>(`/api/v1/coupons/${couponId}/${action}`, { method: 'POST' }),
  redemptions: (id: string) => apiFetch<Redemption[]>(`/api/v1/promotions/${id}/redemptions`),
  posPromotions: (customerId: string | null) => apiFetch<LivePromotion[]>(`/api/v1/pos/promotions${queryString({ customer_id: customerId })}`),
  validateCoupon: (code: string, customerId: string | null) =>
    apiFetch<EnginePromotion>('/api/v1/pos/coupons/validate', { method: 'POST', body: { code, customer_id: customerId ?? undefined } }),
}

export const promotionKeys = {
  all: (tenantId: string) => ['promotions', tenantId] as const,
  list: (tenantId: string, filter: PromotionFilter) => ['promotions', tenantId, 'list', filter] as const,
  detail: (tenantId: string, id: string) => ['promotions', tenantId, 'detail', id] as const,
  coupons: (tenantId: string, id: string, filter: object) => ['promotions', tenantId, 'coupons', id, filter] as const,
  redemptions: (tenantId: string, id: string) => ['promotions', tenantId, 'redemptions', id] as const,
  pos: (tenantId: string, customerId: string | null) => ['promotions', tenantId, 'pos', customerId] as const,
}

export const usePromotions = (tenantId: string, filter: PromotionFilter) =>
  useQuery({ queryKey: promotionKeys.list(tenantId, filter), queryFn: () => promotionsApi.list(filter), placeholderData: keepPreviousData })
export const usePromotion = (tenantId: string, id: string, enabled = true) =>
  useQuery({ queryKey: promotionKeys.detail(tenantId, id), queryFn: () => promotionsApi.get(id), enabled })
export const useCoupons = (tenantId: string, id: string, filter: { q?: string; status?: string }, enabled = true) =>
  useQuery({ queryKey: promotionKeys.coupons(tenantId, id, filter), queryFn: () => promotionsApi.coupons(id, { ...filter, limit: 1000 }), enabled })
export const useRedemptions = (tenantId: string, id: string, enabled = true) =>
  useQuery({ queryKey: promotionKeys.redemptions(tenantId, id), queryFn: () => promotionsApi.redemptions(id), enabled })
/** Live promotions for the POS; refreshed every few minutes and when the customer changes. */
export const usePosPromotions = (tenantId: string, customerId: string | null, enabled = true) =>
  useQuery({
    queryKey: promotionKeys.pos(tenantId, customerId),
    queryFn: () => promotionsApi.posPromotions(customerId),
    enabled,
    staleTime: 60_000,
    refetchInterval: 3 * 60_000,
    placeholderData: keepPreviousData,
  })
