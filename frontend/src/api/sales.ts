import { useInfiniteQuery, useQuery } from '@tanstack/react-query'

import { apiFetch } from './client'
import type { WarrantyUnit } from './catalog'
import type { PaymentDetail } from './money'
import { PAGE_SIZE, type Page, queryString } from './query'

export type DiscountType = 'amount' | 'percent'

export interface PosItem {
  variant_id: string
  product_id: string
  name: string
  name_bn: string | null
  sku: string
  barcode: string | null
  unit_short_name: string
  allow_fraction: boolean
  track_inventory: boolean
  /** The product's warranty; `null` = none (missing on carts held before warranties). */
  warranty_period?: number | null
  warranty_unit?: WarrantyUnit | null
  selling_price: string
  min_selling_price: string | null
  tax_rate_pct: string
  /** Stock at the sale location; `null` for services. */
  stock: string | null
  /** At or below this stock the product is running low; missing on older held carts. */
  reorder_level?: string | null
  /** The product photo (`GET /files/{id}`); missing on carts held before photos were shown. */
  image_file_id?: string | null
  /** The search text matched a barcode or SKU exactly (scan → add). */
  exact: boolean
}

/** A top-level category for the POS grid filter (`GET /pos/categories`). */
export interface PosCategory {
  id: string
  name: string
  product_count: number
}

/** Settings the POS needs to price and print (`GET /pos/context`). */
export interface PosContext {
  prices_include_tax: boolean
  /** "0.00" = none, else the step ("0.50", "1.00"). */
  cash_rounding: string
  allow_negative_stock: boolean
  /** Show the product grid beside the cart (Settings → Invoice, tax & POS). */
  show_product_panel: boolean
  /** Show photos on the product cards. */
  show_product_images: boolean
  /** Discounts above this % need `pos.discount_over_limit`. */
  cashier_max_discount_pct: string
  walk_in_customer_id: string
  branch_id: string
  location_id: string
  business_date: string
  receipt: ReceiptInfo
}

export interface SaleLineInput {
  variant_id: string
  quantity: string
  /** Blank = list price; another price needs `pos.price_override`. */
  unit_price?: string
  discount_type?: DiscountType
  discount_value?: string
}

export interface SalePaymentInput {
  method_id: string
  /** Tendered; cash above the total comes back as change. */
  amount: string
  account_id?: string
  reference?: string
}

export interface SaleInput {
  customer_id?: string
  lines: SaleLineInput[]
  invoice_discount_type?: DiscountType
  invoice_discount_value?: string
  shipping_charge?: string
  other_charge?: string
  payments: SalePaymentInput[]
  note?: string
  /** What the POS showed; the server refuses a mismatch over 0.01. */
  expected_total?: string
}

export type SaleStatus = 'completed' | 'partially_returned' | 'returned' | 'cancelled'
export type PaymentStatus = 'unpaid' | 'partial' | 'paid' | 'refunded'

export interface Sale {
  id: string
  invoice_no: string
  sale_date: string
  business_date: string
  customer_id: string
  customer_name: string
  customer_phone: string | null
  is_walk_in: boolean
  cashier_id: string
  cashier_name: string | null
  status: SaleStatus
  payment_status: PaymentStatus
  prices_include_tax: boolean
  subtotal: string
  item_discount_total: string
  invoice_discount_type: DiscountType | null
  invoice_discount_value: string | null
  invoice_discount_amount: string
  tax_total: string
  shipping_charge: string
  other_charge: string
  rounding_adjustment: string
  grand_total: string
  paid_total: string
  returned_total: string
  due_total: string
  change_given: string
  /** COGS; `null` without `products.view_cost`. */
  cost_total: string | null
  note: string | null
  reprint_count: number
  cancelled_at: string | null
  cancel_reason: string | null
  item_count: number
  /** Lines sold with a warranty. */
  warranty_items: number
  /** The latest day any of them runs out (`YYYY-MM-DD`). */
  warranty_until: string | null
  created_at: string
}

export interface SaleItem {
  id: string
  line_no: number
  variant_id: string
  product_name: string
  sku: string
  unit_short_name: string
  track_inventory: boolean
  /** The product's warranty when it was sold; `null` = none. */
  warranty_period: number | null
  warranty_unit: WarrantyUnit | null
  /** The warranty's last day (`YYYY-MM-DD`): the sale's business date + tenure. */
  warranty_expires_on: string | null
  quantity: string
  returned_quantity: string
  list_price: string
  unit_price: string
  price_overridden: boolean
  gross_amount: string
  discount_type: DiscountType | null
  discount_value: string | null
  line_discount: string
  invoice_discount_alloc: string
  taxable_amount: string
  tax_rate_pct: string
  tax_amount: string
  line_total: string
  unit_cost: string | null
  cost_total: string | null
}

export interface SalePayment {
  payment_id: string
  payment_no: string
  method_name: string
  account_name: string
  amount: string
  reference: string | null
  source: string
  status: 'posted' | 'voided'
  created_at: string
}

export interface SaleReturnRef {
  id: string
  return_no: string
  business_date: string
  refund_mode: RefundMode
  total: string
}

export interface ReceiptInfo {
  business_name: string
  branch_name: string
  address: string | null
  phone: string | null
  /** `BD` or `IT`: which registration lines print, and their labels. */
  country_code?: string
  legal_name?: string | null
  /** BIN; in Italy the Partita IVA. */
  bin_number: string | null
  /** Italy only. */
  tax_code?: string | null
  rea_number?: string | null
  registry_office?: string | null
  receipt_header: string | null
  receipt_footer: string | null
  receipt_paper: '80mm' | '58mm' | 'a4' | string
  auto_print: boolean
}

export interface SaleDetail extends Sale {
  items: SaleItem[]
  payments: SalePayment[]
  returns: SaleReturnRef[]
  receipt: ReceiptInfo
}

export interface SaleFilter {
  from?: string
  to?: string
  status?: SaleStatus | ''
  payment_status?: PaymentStatus | ''
  customer_id?: string
  cashier_id?: string
  q?: string
  due_only?: boolean
  /** Only sales with an item under warranty. */
  with_warranty?: boolean
}

export interface WarrantyItem {
  /** The sale line. */
  id: string
  sale_id: string
  invoice_no: string
  /** The day the warranty started. */
  business_date: string
  customer_id: string
  customer_name: string
  customer_phone: string | null
  is_walk_in: boolean
  variant_id: string
  product_name: string
  sku: string
  unit_short_name: string
  quantity: string
  returned_quantity: string
  warranty_period: number
  warranty_unit: WarrantyUnit
  warranty_expires_on: string
}

export interface WarrantyFilter {
  customer_id?: string
  status?: 'active' | 'expired' | ''
  /** Invoice number, product name, SKU or customer phone. */
  q?: string
}

export interface WarrantyPage extends Page<WarrantyItem> {
  /** Today in the business's timezone; active while `warranty_expires_on >= today`. */
  today: string
}

export interface SalePage extends Page<Sale> {
  summary: { count: number; grand_total: string; paid_total: string; due_total: string; returned_total: string }
}

export type RefundMode = 'cash' | 'original_method' | 'customer_credit'

export interface ReturnInput {
  sale_id: string
  reason: string
  refund_mode: RefundMode
  refund_method_id?: string
  lines: { sale_item_id: string; quantity: string; condition?: 'restock' | 'damaged' }[]
}

export interface SaleReturn {
  id: string
  return_no: string
  sale_id: string
  invoice_no: string
  customer_id: string
  customer_name: string
  business_date: string
  reason: string
  refund_mode: RefundMode
  subtotal: string
  tax_total: string
  total: string
  applied_to_due: string
  refunded_amount: string
  credited_amount: string
  cost_total: string | null
  created_by_name: string | null
  created_at: string
}

export interface SaleReturnItem {
  id: string
  sale_item_id: string
  variant_id: string
  product_name: string
  sku: string
  unit_short_name: string
  quantity: string
  item_condition: 'restock' | 'damaged'
  tax_amount: string
  line_total: string
}

export interface SaleReturnDetail extends SaleReturn {
  items: SaleReturnItem[]
  receipt: ReceiptInfo
}

export interface ReturnFilter {
  sale_id?: string
  customer_id?: string
  from?: string
  to?: string
  q?: string
}

export interface HeldCart {
  id: string
  label: string | null
  customer_id: string | null
  customer_name: string | null
  /** Opaque POS cart state. */
  cart: Record<string, unknown>
  held_by: string
  held_by_name: string | null
  expires_at: string
  created_at: string
}

export interface CollectionInput {
  customer_id: string
  amount: string
  method_id: string
  account_id?: string
  reference?: string
  note?: string
  /** Blank = oldest invoices first. */
  allocations?: { sale_id: string; amount: string }[]
}

export const salesApi = {
  posContext: () => apiFetch<PosContext>('/api/v1/pos/context'),
  posProducts: (q: string, limit = 20, offset = 0, categoryId: string | null = null) =>
    apiFetch<PosItem[]>(`/api/v1/pos/products${queryString({ q, limit, offset: offset || undefined, category_id: categoryId })}`),
  posCategories: () => apiFetch<PosCategory[]>('/api/v1/pos/categories'),
  createSale: (input: SaleInput, idempotencyKey: string) =>
    apiFetch<SaleDetail>('/api/v1/sales', { method: 'POST', body: input, idempotencyKey }),
  sales: (filter: SaleFilter, cursor: string | null) =>
    apiFetch<SalePage>(`/api/v1/sales${queryString({ ...filter, limit: PAGE_SIZE, cursor })}`),
  sale: (id: string) => apiFetch<SaleDetail>(`/api/v1/sales/${id}`),
  warranties: (filter: WarrantyFilter, cursor: string | null) =>
    apiFetch<WarrantyPage>(`/api/v1/sale-warranties${queryString({ ...filter, limit: PAGE_SIZE, cursor })}`),
  reprint: (id: string) => apiFetch<SaleDetail>(`/api/v1/sales/${id}/reprint`, { method: 'POST' }),
  cancel: (id: string, reason: string) => apiFetch<SaleDetail>(`/api/v1/sales/${id}/cancel`, { method: 'POST', body: { reason } }),

  createReturn: (input: ReturnInput, idempotencyKey: string) =>
    apiFetch<SaleReturnDetail>('/api/v1/sale-returns', { method: 'POST', body: input, idempotencyKey }),
  returns: (filter: ReturnFilter, cursor: string | null) =>
    apiFetch<Page<SaleReturn> & { total: string }>(`/api/v1/sale-returns${queryString({ ...filter, limit: PAGE_SIZE, cursor })}`),
  saleReturn: (id: string) => apiFetch<SaleReturnDetail>(`/api/v1/sale-returns/${id}`),

  heldCarts: () => apiFetch<HeldCart[]>('/api/v1/held-carts'),
  holdCart: (input: { label?: string; customer_id?: string; cart: Record<string, unknown> }) =>
    apiFetch<HeldCart>('/api/v1/held-carts', { method: 'POST', body: input }),
  removeHeldCart: (id: string) => apiFetch<HeldCart>(`/api/v1/held-carts/${id}`, { method: 'DELETE' }),

  collect: (input: CollectionInput, idempotencyKey: string) =>
    apiFetch<PaymentDetail>('/api/v1/collections', { method: 'POST', body: input, idempotencyKey }),
}

export const salesKeys = {
  all: (tenantId: string) => ['sales', tenantId] as const,
  posContext: (tenantId: string) => ['sales', tenantId, 'pos-context'] as const,
  posProducts: (tenantId: string, q: string) => ['sales', tenantId, 'pos-products', q] as const,
  posCatalog: (tenantId: string, categoryId: string | null, q: string) => ['sales', tenantId, 'pos-catalog', categoryId, q] as const,
  posCategories: (tenantId: string) => ['sales', tenantId, 'pos-categories'] as const,
  list: (tenantId: string, filter: SaleFilter) => ['sales', tenantId, 'list', filter] as const,
  sale: (tenantId: string, id: string) => ['sales', tenantId, 'sale', id] as const,
  returns: (tenantId: string, filter: ReturnFilter) => ['sales', tenantId, 'returns', filter] as const,
  saleReturn: (tenantId: string, id: string) => ['sales', tenantId, 'return', id] as const,
  heldCarts: (tenantId: string) => ['sales', tenantId, 'held-carts'] as const,
  warranties: (tenantId: string, filter: WarrantyFilter) => ['sales', tenantId, 'warranties', filter] as const,
}

export const usePosContext = (tenantId: string) =>
  useQuery({ queryKey: salesKeys.posContext(tenantId), queryFn: salesApi.posContext, staleTime: 5 * 60_000 })
export const usePosProducts = (tenantId: string, q: string, enabled = true) =>
  useQuery({ queryKey: salesKeys.posProducts(tenantId, q), queryFn: () => salesApi.posProducts(q), enabled, staleTime: 10_000 })
/** Page size of the POS product grid (the API allows at most 50). */
export const POS_CATALOG_PAGE = 48
/**
 * Active products for the POS grid, a page at a time by name: optionally one
 * category, and `q` matching name or SKU (partial, case-insensitive).
 */
export const usePosCatalog = (tenantId: string, categoryId: string | null, q = '') =>
  useInfiniteQuery({
    queryKey: salesKeys.posCatalog(tenantId, categoryId, q),
    queryFn: ({ pageParam }) => salesApi.posProducts(q, POS_CATALOG_PAGE, pageParam, categoryId),
    placeholderData: (previous) => previous,
    initialPageParam: 0,
    getNextPageParam: (last, pages) => (last.length < POS_CATALOG_PAGE ? undefined : pages.length * POS_CATALOG_PAGE),
    staleTime: 30_000,
  })
export const usePosCategories = (tenantId: string) =>
  useQuery({ queryKey: salesKeys.posCategories(tenantId), queryFn: salesApi.posCategories, staleTime: 5 * 60_000 })
export const useSales = (tenantId: string, filter: SaleFilter) =>
  useInfiniteQuery({
    queryKey: salesKeys.list(tenantId, filter),
    queryFn: ({ pageParam }) => salesApi.sales(filter, pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (last) => last.next_cursor,
  })
export const useSale = (tenantId: string, id: string) =>
  useQuery({ queryKey: salesKeys.sale(tenantId, id), queryFn: () => salesApi.sale(id) })
export const useSaleReturns = (tenantId: string, filter: ReturnFilter) =>
  useInfiniteQuery({
    queryKey: salesKeys.returns(tenantId, filter),
    queryFn: ({ pageParam }) => salesApi.returns(filter, pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (last) => last.next_cursor,
  })
export const useSaleReturn = (tenantId: string, id: string) =>
  useQuery({ queryKey: salesKeys.saleReturn(tenantId, id), queryFn: () => salesApi.saleReturn(id) })
export const useWarranties = (tenantId: string, filter: WarrantyFilter) =>
  useInfiniteQuery({
    queryKey: salesKeys.warranties(tenantId, filter),
    queryFn: ({ pageParam }) => salesApi.warranties(filter, pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (last) => last.next_cursor,
  })
export const useHeldCarts = (tenantId: string) => useQuery({ queryKey: salesKeys.heldCarts(tenantId), queryFn: salesApi.heldCarts })
