import { useInfiniteQuery, useQuery } from '@tanstack/react-query'

import { apiFetch } from './client'

export interface TaxRate {
  id: string
  name: string
  /** Percent as a decimal string ("15.0000"). */
  rate_pct: string
  is_default: boolean
  status: 'active' | 'inactive'
}

export interface TaxRateInput {
  name: string
  rate_pct: string
  is_default?: boolean
  status?: TaxRate['status']
}

export interface Unit {
  id: string
  name: string
  short_name: string
  allow_fraction: boolean
  status: 'active' | 'inactive'
}

export interface UnitInput {
  name: string
  short_name: string
  allow_fraction?: boolean
  status?: Unit['status']
}

/** Deepest category level (0-based): three levels in all. */
export const MAX_CATEGORY_DEPTH = 2

export interface Category {
  id: string
  parent_id: string | null
  name: string
  /** 0 = top level; at most 2 (three levels). */
  depth: number
  sort_order: number
  status: 'active' | 'inactive'
  product_count: number
}

export interface CategoryInput {
  name: string
  parent_id?: string | null
  sort_order?: number
  status?: Category['status']
}

export interface Brand {
  id: string
  name: string
  status: 'active' | 'inactive'
  product_count: number
}

export interface BrandInput {
  name: string
  status?: Brand['status']
}

export const PRODUCT_TYPES = ['standard', 'service'] as const
export type ProductType = (typeof PRODUCT_TYPES)[number]

export interface Product {
  id: string
  variant_id: string
  product_type: ProductType
  name: string
  name_bn: string | null
  description: string | null
  sku: string
  barcodes: string[]
  category_id: string | null
  category_name: string | null
  brand_id: string | null
  brand_name: string | null
  unit_id: string
  unit_short_name: string
  allow_fraction: boolean
  tax_rate_id: string | null
  tax_rate_pct: string | null
  track_inventory: boolean
  /** `null` without `products.view_cost`. */
  purchase_price: string | null
  selling_price: string
  wholesale_price: string | null
  min_selling_price: string | null
  reorder_level: string | null
  status: 'active' | 'archived'
  version: number
  created_at: string
  updated_at: string
}

export interface ProductInput {
  product_type?: ProductType
  name: string
  name_bn?: string
  description?: string
  /** Blank = next automatic code (P000001); on update, blank keeps the SKU. */
  sku?: string
  barcodes: string[]
  category_id?: string
  brand_id?: string
  unit_id: string
  /** Blank = the business's default tax rate. */
  tax_rate_id?: string
  track_inventory?: boolean
  /** Only with `products.view_cost`; blank on update keeps the cost. */
  purchase_price?: string
  selling_price: string
  wholesale_price?: string
  min_selling_price?: string
  reorder_level?: string
}

export type ProductStatusFilter = 'active' | 'archived' | 'all'

export interface ProductFilter {
  q?: string
  category_id?: string
  brand_id?: string
  status?: ProductStatusFilter
}

export interface ProductPage {
  data: Product[]
  next_cursor: string | null
}

export interface PriceChange {
  price_field: 'selling_price' | 'purchase_price' | 'wholesale_price' | 'min_selling_price'
  old_value: string | null
  new_value: string | null
  changed_by: string | null
  changed_at: string
}

const PAGE_SIZE = 50

function productQuery(filter: ProductFilter, cursor: string | null): string {
  const params = new URLSearchParams()
  for (const [key, value] of Object.entries(filter)) {
    if (value) params.set(key, value)
  }
  params.set('limit', String(PAGE_SIZE))
  if (cursor) params.set('cursor', cursor)
  return params.toString()
}

export const catalogApi = {
  taxRates: () => apiFetch<TaxRate[]>('/api/v1/tax-rates'),
  createTaxRate: (input: TaxRateInput) => apiFetch<TaxRate>('/api/v1/tax-rates', { method: 'POST', body: input }),
  updateTaxRate: (id: string, input: TaxRateInput) => apiFetch<TaxRate>(`/api/v1/tax-rates/${id}`, { method: 'PUT', body: input }),

  units: () => apiFetch<Unit[]>('/api/v1/units'),
  createUnit: (input: UnitInput) => apiFetch<Unit>('/api/v1/units', { method: 'POST', body: input }),
  updateUnit: (id: string, input: UnitInput) => apiFetch<Unit>(`/api/v1/units/${id}`, { method: 'PUT', body: input }),

  categories: () => apiFetch<Category[]>('/api/v1/categories'),
  createCategory: (input: CategoryInput) => apiFetch<Category>('/api/v1/categories', { method: 'POST', body: input }),
  updateCategory: (id: string, input: CategoryInput) =>
    apiFetch<Category>(`/api/v1/categories/${id}`, { method: 'PUT', body: input }),
  deleteCategory: (id: string) => apiFetch<void>(`/api/v1/categories/${id}`, { method: 'DELETE' }),

  brands: () => apiFetch<Brand[]>('/api/v1/brands'),
  createBrand: (input: BrandInput) => apiFetch<Brand>('/api/v1/brands', { method: 'POST', body: input }),
  updateBrand: (id: string, input: BrandInput) => apiFetch<Brand>(`/api/v1/brands/${id}`, { method: 'PUT', body: input }),
  deleteBrand: (id: string) => apiFetch<void>(`/api/v1/brands/${id}`, { method: 'DELETE' }),

  products: (filter: ProductFilter, cursor: string | null) =>
    apiFetch<ProductPage>(`/api/v1/products?${productQuery(filter, cursor)}`),
  product: (id: string) => apiFetch<Product>(`/api/v1/products/${id}`),
  createProduct: (input: ProductInput) => apiFetch<Product>('/api/v1/products', { method: 'POST', body: input }),
  updateProduct: (id: string, version: number, input: ProductInput) =>
    apiFetch<Product>(`/api/v1/products/${id}`, { method: 'PUT', body: input, ifMatch: version }),
  archiveProduct: (id: string) => apiFetch<Product>(`/api/v1/products/${id}/archive`, { method: 'POST' }),
  restoreProduct: (id: string) => apiFetch<Product>(`/api/v1/products/${id}/restore`, { method: 'POST' }),
  deleteProduct: (id: string) => apiFetch<void>(`/api/v1/products/${id}`, { method: 'DELETE' }),
  priceHistory: (id: string) => apiFetch<PriceChange[]>(`/api/v1/products/${id}/price-history`),
}

export const catalogKeys = {
  all: (tenantId: string) => ['catalog', tenantId] as const,
  taxRates: (tenantId: string) => ['catalog', tenantId, 'tax-rates'] as const,
  units: (tenantId: string) => ['catalog', tenantId, 'units'] as const,
  categories: (tenantId: string) => ['catalog', tenantId, 'categories'] as const,
  brands: (tenantId: string) => ['catalog', tenantId, 'brands'] as const,
  products: (tenantId: string) => ['catalog', tenantId, 'products'] as const,
  productList: (tenantId: string, filter: ProductFilter) => ['catalog', tenantId, 'products', 'list', filter] as const,
  product: (tenantId: string, id: string) => ['catalog', tenantId, 'products', id] as const,
  priceHistory: (tenantId: string, id: string) => ['catalog', tenantId, 'products', id, 'price-history'] as const,
}

export const useTaxRates = (tenantId: string) =>
  useQuery({ queryKey: catalogKeys.taxRates(tenantId), queryFn: catalogApi.taxRates })
export const useUnits = (tenantId: string) => useQuery({ queryKey: catalogKeys.units(tenantId), queryFn: catalogApi.units })
export const useCategories = (tenantId: string) =>
  useQuery({ queryKey: catalogKeys.categories(tenantId), queryFn: catalogApi.categories })
export const useBrands = (tenantId: string) => useQuery({ queryKey: catalogKeys.brands(tenantId), queryFn: catalogApi.brands })
export const useProduct = (tenantId: string, id: string) =>
  useQuery({ queryKey: catalogKeys.product(tenantId, id), queryFn: () => catalogApi.product(id) })
export const usePriceHistory = (tenantId: string, id: string) =>
  useQuery({ queryKey: catalogKeys.priceHistory(tenantId, id), queryFn: () => catalogApi.priceHistory(id) })

/** Products page by page (`next_cursor`), newest first. */
export const useProducts = (tenantId: string, filter: ProductFilter) =>
  useInfiniteQuery({
    queryKey: catalogKeys.productList(tenantId, filter),
    queryFn: ({ pageParam }) => catalogApi.products(filter, pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (last) => last.next_cursor,
  })

/** Categories in tree order (parents before their children) with their depth. */
export function categoryTree(categories: Category[]): Category[] {
  const children = new Map<string | null, Category[]>()
  for (const c of categories) {
    const list = children.get(c.parent_id) ?? []
    list.push(c)
    children.set(c.parent_id, list)
  }
  const out: Category[] = []
  const walk = (parent: string | null) => {
    const list = (children.get(parent) ?? []).sort((a, b) => a.sort_order - b.sort_order || a.name.localeCompare(b.name))
    for (const c of list) {
      out.push(c)
      walk(c.id)
    }
  }
  walk(null)
  return out
}
