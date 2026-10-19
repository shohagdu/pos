import { useInfiniteQuery, useQuery } from '@tanstack/react-query'

import { apiFetch } from './client'
import { PAGE_SIZE, type Page, queryString } from './query'

export interface StockLocation {
  id: string
  branch_id: string
  branch_name: string
  code: string
  name: string
  location_type: string
  is_sellable: boolean
  status: string
}

export type StockStatusFilter = 'all' | 'in' | 'low' | 'out'

export interface StockFilter {
  location_id?: string
  q?: string
  category_id?: string
  brand_id?: string
  status?: StockStatusFilter
  variant_id?: string
}

export interface StockRow {
  variant_id: string
  product_id: string
  name: string
  name_bn: string | null
  sku: string
  unit_short_name: string
  allow_fraction: boolean
  category_name: string | null
  brand_name: string | null
  quantity: string
  /** `null` without `products.view_cost`. */
  avg_cost: string | null
  value: string | null
  selling_price: string
  reorder_level: string | null
  is_low: boolean
  last_movement_at: string | null
}

export interface StockSummary {
  items: number
  quantity: string
  value: string | null
  low_count: number
  out_count: number
}

export interface StockPage extends Page<StockRow> {
  summary: StockSummary
}

export interface ValuationRow {
  category_id: string | null
  category_name: string | null
  items: number
  quantity: string
  value: string
  retail_value: string
}

export interface Valuation {
  rows: ValuationRow[]
  total_quantity: string
  total_value: string
  total_retail_value: string
}

export interface MovementFilter {
  variant_id?: string
  location_id?: string
  from?: string
  to?: string
  movement_type?: string
}

export const MOVEMENT_TYPES = [
  'opening',
  'purchase_receipt',
  'purchase_return',
  'sale',
  'sale_cancel',
  'sale_return',
  'adjustment',
  'transfer_out',
  'transfer_in',
  'count_adjustment',
] as const

export interface Movement {
  id: string
  created_at: string
  business_date: string
  variant_id: string
  product_name: string
  sku: string
  unit_short_name: string
  location_id: string
  location_name: string
  movement_type: string
  /** Signed: + in, − out. */
  quantity: string
  balance_qty_after: string
  unit_cost: string | null
  total_cost: string | null
  reference_type: string
  reference_id: string
  reference_no: string | null
  reason: string | null
}

export const ADJUSTMENT_REASONS = ['damaged', 'lost', 'found', 'expired', 'correction', 'internal_use', 'transit_loss', 'other'] as const
export type AdjustmentReason = 'opening' | (typeof ADJUSTMENT_REASONS)[number]

export interface AdjustmentLineInput {
  variant_id: string
  /** Signed; opening stock is always positive. */
  quantity: string
  unit_cost?: string
  note?: string
}

export interface AdjustmentInput {
  reason: AdjustmentReason
  location_id?: string
  note?: string
  lines: AdjustmentLineInput[]
}

export interface AdjustmentSummary {
  id: string
  adjustment_no: string | null
  business_date: string
  reason: AdjustmentReason
  note: string | null
  status: string
  location_id: string
  location_name: string
  total_value: string | null
  item_count: number
  created_by_name: string | null
  created_at: string
}

export interface AdjustmentItem {
  id: string
  variant_id: string
  product_name: string
  sku: string
  unit_short_name: string
  quantity: string
  unit_cost: string | null
  note: string | null
}

export interface Adjustment extends AdjustmentSummary {
  items: AdjustmentItem[]
}

export interface AdjustmentFilter {
  reason?: string
  from?: string
  to?: string
}

export const TRANSFER_STATUSES = ['draft', 'dispatched', 'received', 'cancelled'] as const
export type TransferStatus = (typeof TRANSFER_STATUSES)[number]

export interface TransferLineInput {
  variant_id: string
  /** Base quantity, more than 0. */
  quantity: string
}

export interface TransferInput {
  /** Blank = the selected branch's default location. */
  from_location_id?: string
  to_location_id: string
  note?: string
  lines: TransferLineInput[]
  /** Dispatch right away instead of saving a draft. */
  dispatch?: boolean
}

export interface TransferSummary {
  id: string
  transfer_no: string | null
  business_date: string
  status: TransferStatus
  from_branch_id: string
  from_branch_name: string
  from_location_id: string
  from_location_name: string
  to_branch_id: string
  to_branch_name: string
  to_location_id: string
  to_location_name: string
  note: string | null
  /** `null` without `products.view_cost`. */
  total_value: string | null
  loss_value: string | null
  item_count: number
  created_by_name: string | null
  dispatched_at: string | null
  received_at: string | null
  version: number
  created_at: string
}

export interface TransferItem {
  id: string
  variant_id: string
  product_name: string
  sku: string
  unit_short_name: string
  quantity_sent: string
  quantity_received: string | null
  /** Source average at dispatch; `null` before dispatch or without `products.view_cost`. */
  unit_cost: string | null
}

export interface Transfer extends TransferSummary {
  items: TransferItem[]
}

export interface TransferFilter {
  status?: string
  /** `out` (sent from the selected branch) or `in` (coming to it). */
  direction?: 'in' | 'out'
  from?: string
  to?: string
}

export interface ReceiveInput {
  /** Lines left out arrive in full. */
  lines?: { item_id: string; quantity_received: string }[]
  note?: string
}

export const COUNT_STATUSES = ['in_progress', 'posted', 'cancelled'] as const
export type CountStatus = (typeof COUNT_STATUSES)[number]

export interface CountInput {
  /** Blank = the selected branch's default location. */
  location_id?: string
  /** Count one category (with its sub-categories) only. */
  category_id?: string
  note?: string
}

export interface CountSummary {
  id: string
  count_no: string | null
  business_date: string
  status: CountStatus
  branch_id: string
  location_id: string
  location_name: string
  category_name: string | null
  note: string | null
  item_count: number
  counted_count: number
  /** Net value of the differences; `null` without `products.view_cost`. */
  variance_value: string | null
  created_by_name: string | null
  snapshot_at: string | null
  posted_at: string | null
  created_at: string
}

export interface CountItem {
  id: string
  variant_id: string
  product_name: string
  sku: string
  barcode: string | null
  unit_short_name: string
  allow_fraction: boolean
  system_qty: string
  counted_qty: string | null
  variance_qty: string | null
  unit_cost: string | null
}

export interface StockCount extends CountSummary {
  items: CountItem[]
}

export const inventoryApi = {
  locations: () => apiFetch<StockLocation[]>('/api/v1/stock-locations'),
  stock: (filter: StockFilter, cursor: string | null, limit = PAGE_SIZE) =>
    apiFetch<StockPage>(`/api/v1/stock${queryString({ ...filter, limit, cursor })}`),
  valuation: (locationId?: string) => apiFetch<Valuation>(`/api/v1/stock/valuation${queryString({ location_id: locationId })}`),
  movements: (filter: MovementFilter, cursor: string | null) =>
    apiFetch<Page<Movement>>(`/api/v1/stock/movements${queryString({ ...filter, limit: PAGE_SIZE, cursor })}`),
  adjustments: (filter: AdjustmentFilter, cursor: string | null) =>
    apiFetch<Page<AdjustmentSummary>>(`/api/v1/stock-adjustments${queryString({ ...filter, limit: PAGE_SIZE, cursor })}`),
  adjustment: (id: string) => apiFetch<Adjustment>(`/api/v1/stock-adjustments/${id}`),
  createAdjustment: (input: AdjustmentInput, idempotencyKey: string) =>
    apiFetch<Adjustment>('/api/v1/stock-adjustments', { method: 'POST', body: input, idempotencyKey }),
  transfers: (filter: TransferFilter, cursor: string | null) =>
    apiFetch<Page<TransferSummary>>(`/api/v1/stock-transfers${queryString({ ...filter, limit: PAGE_SIZE, cursor })}`),
  transfer: (id: string) => apiFetch<Transfer>(`/api/v1/stock-transfers/${id}`),
  createTransfer: (input: TransferInput, idempotencyKey: string) =>
    apiFetch<Transfer>('/api/v1/stock-transfers', { method: 'POST', body: input, idempotencyKey }),
  dispatchTransfer: (id: string, idempotencyKey: string) =>
    apiFetch<Transfer>(`/api/v1/stock-transfers/${id}/dispatch`, { method: 'POST', idempotencyKey }),
  receiveTransfer: (id: string, input: ReceiveInput, idempotencyKey: string) =>
    apiFetch<Transfer>(`/api/v1/stock-transfers/${id}/receive`, { method: 'POST', body: input, idempotencyKey }),
  cancelTransfer: (id: string) => apiFetch<Transfer>(`/api/v1/stock-transfers/${id}/cancel`, { method: 'POST' }),
  counts: (status: string | undefined, cursor: string | null) =>
    apiFetch<Page<CountSummary>>(`/api/v1/stock-counts${queryString({ status, limit: PAGE_SIZE, cursor })}`),
  count: (id: string) => apiFetch<StockCount>(`/api/v1/stock-counts/${id}`),
  createCount: (input: CountInput) => apiFetch<StockCount>('/api/v1/stock-counts', { method: 'POST', body: input }),
  /** `counted_qty: null` clears an entry. */
  enterCount: (id: string, lines: { item_id: string; counted_qty: string | null }[]) =>
    apiFetch<StockCount>(`/api/v1/stock-counts/${id}/items`, { method: 'PUT', body: { lines } }),
  postCount: (id: string, idempotencyKey: string) =>
    apiFetch<StockCount>(`/api/v1/stock-counts/${id}/post`, { method: 'POST', idempotencyKey }),
  cancelCount: (id: string) => apiFetch<StockCount>(`/api/v1/stock-counts/${id}/cancel`, { method: 'POST' }),
}

export const inventoryKeys = {
  all: (tenantId: string) => ['inventory', tenantId] as const,
  locations: (tenantId: string) => ['inventory', tenantId, 'locations'] as const,
  stock: (tenantId: string, filter: StockFilter) => ['inventory', tenantId, 'stock', filter] as const,
  valuation: (tenantId: string, locationId?: string) => ['inventory', tenantId, 'valuation', locationId ?? ''] as const,
  movements: (tenantId: string, filter: MovementFilter) => ['inventory', tenantId, 'movements', filter] as const,
  adjustments: (tenantId: string, filter: AdjustmentFilter) => ['inventory', tenantId, 'adjustments', filter] as const,
  adjustment: (tenantId: string, id: string) => ['inventory', tenantId, 'adjustment', id] as const,
  transfers: (tenantId: string, filter: TransferFilter) => ['inventory', tenantId, 'transfers', filter] as const,
  transfer: (tenantId: string, id: string) => ['inventory', tenantId, 'transfer', id] as const,
  counts: (tenantId: string, status?: string) => ['inventory', tenantId, 'counts', status ?? ''] as const,
  count: (tenantId: string, id: string) => ['inventory', tenantId, 'count', id] as const,
}

export const useStockLocations = (tenantId: string) =>
  useQuery({ queryKey: inventoryKeys.locations(tenantId), queryFn: inventoryApi.locations })

export const useStock = (tenantId: string, filter: StockFilter) =>
  useInfiniteQuery({
    queryKey: inventoryKeys.stock(tenantId, filter),
    queryFn: ({ pageParam }) => inventoryApi.stock(filter, pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (last) => last.next_cursor,
  })

export const useValuation = (tenantId: string, locationId?: string, enabled = true) =>
  useQuery({ queryKey: inventoryKeys.valuation(tenantId, locationId), queryFn: () => inventoryApi.valuation(locationId), enabled })

export const useMovements = (tenantId: string, filter: MovementFilter) =>
  useInfiniteQuery({
    queryKey: inventoryKeys.movements(tenantId, filter),
    queryFn: ({ pageParam }) => inventoryApi.movements(filter, pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (last) => last.next_cursor,
  })

export const useAdjustments = (tenantId: string, filter: AdjustmentFilter) =>
  useInfiniteQuery({
    queryKey: inventoryKeys.adjustments(tenantId, filter),
    queryFn: ({ pageParam }) => inventoryApi.adjustments(filter, pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (last) => last.next_cursor,
  })

export const useAdjustment = (tenantId: string, id: string) =>
  useQuery({ queryKey: inventoryKeys.adjustment(tenantId, id), queryFn: () => inventoryApi.adjustment(id) })

export const useTransfers = (tenantId: string, filter: TransferFilter) =>
  useInfiniteQuery({
    queryKey: inventoryKeys.transfers(tenantId, filter),
    queryFn: ({ pageParam }) => inventoryApi.transfers(filter, pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (last) => last.next_cursor,
  })

export const useTransfer = (tenantId: string, id: string) =>
  useQuery({ queryKey: inventoryKeys.transfer(tenantId, id), queryFn: () => inventoryApi.transfer(id) })

export const useCounts = (tenantId: string, status?: string) =>
  useInfiniteQuery({
    queryKey: inventoryKeys.counts(tenantId, status),
    queryFn: ({ pageParam }) => inventoryApi.counts(status, pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (last) => last.next_cursor,
  })

export const useCount = (tenantId: string, id: string) =>
  useQuery({ queryKey: inventoryKeys.count(tenantId, id), queryFn: () => inventoryApi.count(id) })
