import { useInfiniteQuery, useQuery } from '@tanstack/react-query'

import { apiFetch } from './client'
import type { PaymentDetail } from './money'
import { PAGE_SIZE, type Page, queryString } from './query'

export interface PurchaseLineInput {
  variant_id: string
  quantity: string
  /** Per unit, excluding VAT. */
  unit_cost: string
  discount_type?: 'amount' | 'percent'
  discount_value?: string
  tax_rate_pct?: string
}

export interface PurchaseInput {
  supplier_id: string
  supplier_invoice_no?: string
  purchase_date?: string
  location_id?: string
  lines: PurchaseLineInput[]
  bill_discount?: string
  shipping_cost?: string
  other_cost?: string
  note?: string
}

export interface PayInput {
  method_id: string
  amount: string
  account_id?: string
  reference?: string
  note?: string
}

export type PurchaseStatus = 'draft' | 'received' | 'cancelled'

export interface Purchase {
  id: string
  purchase_no: string | null
  supplier_id: string
  supplier_name: string
  supplier_invoice_no: string | null
  purchase_date: string
  location_id: string
  location_name: string
  status: PurchaseStatus
  payment_status: 'unpaid' | 'partial' | 'paid' | 'refunded'
  subtotal: string
  item_discount_total: string
  bill_discount_amount: string
  tax_total: string
  shipping_cost: string
  other_cost: string
  grand_total: string
  paid_total: string
  returned_total: string
  due_total: string
  note: string | null
  item_count: number
  posted_at: string | null
  cancelled_at: string | null
  cancel_reason: string | null
  created_by_name: string | null
  version: number
  created_at: string
}

export interface PurchaseItem {
  id: string
  line_no: number
  variant_id: string
  product_name: string
  sku: string
  unit_short_name: string
  quantity: string
  received_quantity: string
  returned_quantity: string
  unit_cost: string
  discount_type: 'amount' | 'percent' | null
  discount_value: string | null
  line_discount: string
  bill_discount_alloc: string
  tax_rate_pct: string
  tax_amount: string
  charges_alloc: string
  line_total: string
  landed_unit_cost: string
  /** Last purchase price when the line was entered (PUR-R04). */
  previous_unit_cost: string | null
}

export interface PurchasePayment {
  payment_id: string
  payment_no: string
  method_name: string
  account_name: string
  amount: string
  reference: string | null
  status: 'posted' | 'voided'
  created_at: string
}

export interface PurchaseDetail extends Purchase {
  items: PurchaseItem[]
  payments: PurchasePayment[]
  returns: { id: string; return_no: string; business_date: string; total: string }[]
}

export interface PurchaseFilter {
  supplier_id?: string
  status?: PurchaseStatus | ''
  payment_status?: string
  from?: string
  to?: string
  q?: string
  due_only?: boolean
}

export interface PurchasePage extends Page<Purchase> {
  summary: { count: number; grand_total: string; paid_total: string; due_total: string }
}

export interface LastPrice {
  variant_id: string
  last_purchase_price: string | null
  last_purchase_date: string | null
  last_purchase_qty: string | null
  last_supplier_name: string | null
  purchase_price: string
}

export interface SupplierPaymentInput {
  supplier_id: string
  amount: string
  method_id: string
  account_id?: string
  reference?: string
  note?: string
  allocations?: { purchase_id: string; amount: string }[]
}

export interface PurchaseReturnInput {
  purchase_id: string
  reason: string
  settlement_mode?: 'reduce_payable' | 'refund'
  refund_method_id?: string
  refund_account_id?: string
  /** Quantities in base units. */
  lines: { purchase_item_id: string; quantity: string }[]
}

export interface PurchaseReturn {
  id: string
  return_no: string
  purchase_id: string
  purchase_no: string | null
  supplier_id: string
  supplier_name: string
  business_date: string
  reason: string
  settlement_mode: 'reduce_payable' | 'refund'
  total: string
  tax_total: string
  cost_total: string
  applied_to_due: string
  refunded_amount: string
  credited_amount: string
  created_by_name: string | null
  created_at: string
}

export interface PurchaseReturnDetail extends PurchaseReturn {
  items: {
    id: string
    purchase_item_id: string
    variant_id: string
    product_name: string
    sku: string
    base_quantity: string
    line_total: string
    unit_cost_out: string
  }[]
}

export const purchasingApi = {
  purchases: (filter: PurchaseFilter, cursor: string | null) =>
    apiFetch<PurchasePage>(`/api/v1/purchases${queryString({ ...filter, limit: PAGE_SIZE, cursor })}`),
  purchase: (id: string) => apiFetch<PurchaseDetail>(`/api/v1/purchases/${id}`),
  createPurchase: (input: PurchaseInput) => apiFetch<PurchaseDetail>('/api/v1/purchases', { method: 'POST', body: input }),
  updatePurchase: (id: string, version: number, input: PurchaseInput) =>
    apiFetch<PurchaseDetail>(`/api/v1/purchases/${id}`, { method: 'PUT', body: input, ifMatch: version }),
  postPurchase: (id: string, payment: PayInput | undefined, idempotencyKey: string) =>
    apiFetch<PurchaseDetail>(`/api/v1/purchases/${id}/post`, { method: 'POST', body: payment ? { payment } : {}, idempotencyKey }),
  cancelPurchase: (id: string, reason: string) =>
    apiFetch<PurchaseDetail>(`/api/v1/purchases/${id}/cancel`, { method: 'POST', body: { reason } }),
  lastPrices: (variantIds: string[]) =>
    apiFetch<LastPrice[]>(`/api/v1/purchases/last-prices${queryString({ variant_ids: variantIds.join(',') })}`),
  paySupplier: (input: SupplierPaymentInput, idempotencyKey: string) =>
    apiFetch<PaymentDetail>('/api/v1/supplier-payments', { method: 'POST', body: input, idempotencyKey }),
  createReturn: (input: PurchaseReturnInput, idempotencyKey: string) =>
    apiFetch<PurchaseReturnDetail>('/api/v1/purchase-returns', { method: 'POST', body: input, idempotencyKey }),
  returns: (filter: { purchase_id?: string; supplier_id?: string; from?: string; to?: string }, cursor: string | null) =>
    apiFetch<Page<PurchaseReturn>>(`/api/v1/purchase-returns${queryString({ ...filter, limit: PAGE_SIZE, cursor })}`),
  purchaseReturn: (id: string) => apiFetch<PurchaseReturnDetail>(`/api/v1/purchase-returns/${id}`),
}

export const purchasingKeys = {
  all: (tenantId: string) => ['purchasing', tenantId] as const,
  list: (tenantId: string, filter: PurchaseFilter) => ['purchasing', tenantId, 'list', filter] as const,
  purchase: (tenantId: string, id: string) => ['purchasing', tenantId, 'purchase', id] as const,
  lastPrices: (tenantId: string, ids: string[]) => ['purchasing', tenantId, 'last-prices', ids.join(',')] as const,
  returns: (tenantId: string, filter: object) => ['purchasing', tenantId, 'returns', filter] as const,
  purchaseReturn: (tenantId: string, id: string) => ['purchasing', tenantId, 'return', id] as const,
}

export const usePurchases = (tenantId: string, filter: PurchaseFilter) =>
  useInfiniteQuery({
    queryKey: purchasingKeys.list(tenantId, filter),
    queryFn: ({ pageParam }) => purchasingApi.purchases(filter, pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (last) => last.next_cursor,
  })
export const usePurchase = (tenantId: string, id: string, enabled = true) =>
  useQuery({ queryKey: purchasingKeys.purchase(tenantId, id), queryFn: () => purchasingApi.purchase(id), enabled })
export const useLastPrices = (tenantId: string, ids: string[]) =>
  useQuery({ queryKey: purchasingKeys.lastPrices(tenantId, ids), queryFn: () => purchasingApi.lastPrices(ids), enabled: ids.length > 0 })
export const usePurchaseReturns = (tenantId: string, filter: { purchase_id?: string; supplier_id?: string; from?: string; to?: string }) =>
  useInfiniteQuery({
    queryKey: purchasingKeys.returns(tenantId, filter),
    queryFn: ({ pageParam }) => purchasingApi.returns(filter, pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (last) => last.next_cursor,
  })
export const usePurchaseReturn = (tenantId: string, id: string) =>
  useQuery({ queryKey: purchasingKeys.purchaseReturn(tenantId, id), queryFn: () => purchasingApi.purchaseReturn(id) })
