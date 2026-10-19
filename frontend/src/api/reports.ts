import { useQuery } from '@tanstack/react-query'

import { apiFetch } from './client'
import { queryString } from './query'

export interface DateRange {
  from?: string
  to?: string
  /** Period reports: `day` (default) or `month`. */
  group?: 'day' | 'month'
  category_id?: string
  supplier_id?: string
}

export interface Figures {
  invoices: number
  sales: string
  net_sales: string
  cash_received: string
  expenses: string
  /** `null` without `dashboard.view_financials`. */
  gross_profit: string | null
  net_profit: string | null
}

export interface Dashboard {
  today: string
  today_figures: Figures
  month_figures: Figures
  receivables: string
  payables: string
  money_balance: string
  low_stock_count: number
  out_of_stock_count: number
  top_products: { variant_id: string; product_name: string; unit_short_name: string; quantity: string; net_sales: string }[]
  recent_sales: { id: string; invoice_no: string; customer_name: string; grand_total: string; due_total: string; status: string; sale_date: string }[]
  last_14_days: { date: string; sales: string }[]
}

export interface SalesPeriodRow {
  period: string
  invoices: number
  gross: string
  discount: string
  tax: string
  grand_total: string
  returns: string
  net_sales: string
  cogs: string | null
  gross_profit: string | null
  collected: string
  due: string
}

export interface MethodTotal {
  method_name: string
  money_in: string
  money_out: string
  count: number
}

export interface SalesSummary {
  from: string
  to: string
  group: 'day' | 'month'
  rows: SalesPeriodRow[]
  totals: SalesPeriodRow
  payments: MethodTotal[]
  cancelled_count: number
}

export interface ProductSalesRow {
  variant_id: string
  product_name: string
  sku: string
  category_name: string | null
  unit_short_name: string
  quantity: string
  net_sales: string
  cogs: string | null
  gross_profit: string | null
  margin_pct: string | null
}

export interface ProductSales {
  from: string
  to: string
  rows: ProductSalesRow[]
  total_net_sales: string
  total_cogs: string | null
  total_gross_profit: string | null
}

export interface CategorySalesRow {
  category_id: string | null
  category_name: string | null
  quantity: string
  net_sales: string
  cogs: string | null
  gross_profit: string | null
  margin_pct: string | null
}

export interface CashierSalesRow {
  cashier_id: string
  cashier_name: string | null
  invoices: number
  grand_total: string
  discounts: string
  collected: string
  cancelled: number
}

export interface PaymentSummary {
  from: string
  to: string
  by_method: MethodTotal[]
  by_account: { account_id: string; account_name: string; money_in: string; money_out: string }[]
}

export interface PurchaseReport {
  from: string
  to: string
  by_supplier: {
    supplier_id: string
    supplier_name: string
    bills: number
    grand_total: string
    returned_total: string
    paid_total: string
    due_total: string
  }[]
  by_product: {
    variant_id: string
    product_name: string
    sku: string
    unit_short_name: string
    quantity: string
    net_cost: string
    avg_unit_cost: string
    last_unit_cost: string | null
  }[]
  bills: number
  grand_total: string
  returned_total: string
  paid_total: string
  due_total: string
  tax_total: string
}

export interface ExpenseSummary {
  from: string
  to: string
  by_category: { category_id: string; category_name: string; count: number; amount: string; tax_amount: string }[]
  by_month: { period: string; count: number; amount: string }[]
  count: number
  total: string
}

export interface ProfitEstimate {
  from: string
  to: string
  net_sales: string
  cogs: string
  gross_profit: string
  expenses: string
  net_profit: string
  net_margin_pct: string | null
}

export interface AgingTotals {
  current: string
  days_31_60: string
  days_61_90: string
  over_90: string
  /** Balance not tied to an open document (opening balances, advances). */
  other: string
  total: string
}

export interface AgingRow extends AgingTotals {
  party_id: string
  code: string | null
  name: string
  phone: string | null
  oldest_date: string | null
}

export interface AgingReport {
  party: 'customers' | 'suppliers'
  as_of: string
  rows: AgingRow[]
  totals: AgingTotals
}

export interface VatRow {
  /** `null` = VAT on expenses (no rate). */
  rate_pct: string | null
  taxable: string
  vat: string
}

export interface VatSummary {
  from: string
  to: string
  is_vat_registered: boolean
  output: VatRow[]
  input: VatRow[]
  output_vat: string
  input_vat: string
  net_vat: string
}

export interface DiscountByCashier {
  cashier_id: string
  cashier_name: string | null
  invoices: number
  discounted_invoices: number
  gross: string
  item_discount: string
  invoice_discount: string
  total_discount: string
}

export interface DiscountedInvoice {
  sale_id: string
  invoice_no: string
  business_date: string
  customer_name: string
  cashier_name: string | null
  subtotal: string
  discount: string
  grand_total: string
}

export interface DiscountReport {
  from: string
  to: string
  gross: string
  item_discount: string
  invoice_discount: string
  total_discount: string
  discount_pct: string
  by_cashier: DiscountByCashier[]
  top_invoices: DiscountedInvoice[]
}

export type MovementClass = 'fast' | 'slow' | 'non_moving'

export interface ProductMovementRow {
  variant_id: string
  product_name: string
  sku: string
  category_name: string | null
  unit_short_name: string
  class: MovementClass
  quantity_sold: string
  net_sales: string
  daily_rate: string
  stock_qty: string
  /** `null` without `products.view_cost`. */
  stock_value: string | null
  days_of_stock: string | null
  last_sold_on: string | null
}

export interface ProductMovement {
  from: string
  to: string
  fast_count: number
  slow_count: number
  non_moving_count: number
  non_moving_value: string | null
  rows: ProductMovementRow[]
}

const get = <T,>(path: string, range: DateRange) => apiFetch<T>(`${path}${queryString({ ...range })}`)

export const reportsApi = {
  dashboard: () => apiFetch<Dashboard>('/api/v1/dashboard'),
  salesSummary: (range: DateRange) => get<SalesSummary>('/api/v1/reports/sales-summary', range),
  salesByProduct: (range: DateRange) => get<ProductSales>('/api/v1/reports/sales-by-product', range),
  salesByCategory: (range: DateRange) => get<CategorySalesRow[]>('/api/v1/reports/sales-by-category', range),
  salesByCashier: (range: DateRange) => get<CashierSalesRow[]>('/api/v1/reports/sales-by-cashier', range),
  payments: (range: DateRange) => get<PaymentSummary>('/api/v1/reports/payments', range),
  purchases: (range: DateRange) => get<PurchaseReport>('/api/v1/reports/purchases', range),
  expenses: (range: DateRange) => get<ExpenseSummary>('/api/v1/reports/expenses', range),
  profit: (range: DateRange) => get<ProfitEstimate>('/api/v1/reports/profit', range),
  vatSummary: (range: DateRange) => get<VatSummary>('/api/v1/reports/vat-summary', range),
  discounts: (range: DateRange) => get<DiscountReport>('/api/v1/reports/discounts', range),
  productMovement: (range: DateRange & { class?: MovementClass }) =>
    apiFetch<ProductMovement>(`/api/v1/reports/product-movement${queryString({ ...range })}`),
  aging: (party: 'customers' | 'suppliers', asOf?: string) =>
    apiFetch<AgingReport>(`/api/v1/reports/aging${queryString({ party, as_of: asOf })}`),
}

export const reportKeys = {
  dashboard: (tenantId: string) => ['reports', tenantId, 'dashboard'] as const,
  report: (tenantId: string, name: string, range: DateRange) => ['reports', tenantId, name, range] as const,
}

export const useDashboard = (tenantId: string, enabled = true) =>
  useQuery({ queryKey: reportKeys.dashboard(tenantId), queryFn: reportsApi.dashboard, enabled, refetchInterval: 60_000 })

/** What each date-range report returns. */
export interface ReportResults {
  salesSummary: SalesSummary
  salesByProduct: ProductSales
  salesByCategory: CategorySalesRow[]
  salesByCashier: CashierSalesRow[]
  payments: PaymentSummary
  purchases: PurchaseReport
  expenses: ExpenseSummary
  profit: ProfitEstimate
  vatSummary: VatSummary
  discounts: DiscountReport
  productMovement: ProductMovement
}

/** Any date-range report by name (`salesSummary`, `purchases`, …). */
export function useReport<K extends keyof ReportResults>(tenantId: string, name: K, range: DateRange, enabled = true) {
  const fetcher = reportsApi[name] as (range: DateRange) => Promise<ReportResults[K]>
  return useQuery<ReportResults[K]>({
    queryKey: reportKeys.report(tenantId, name, range),
    queryFn: () => fetcher(range),
    enabled,
  })
}

export const useAgingReport = (tenantId: string, party: 'customers' | 'suppliers', asOf: string, enabled = true) =>
  useQuery({
    queryKey: ['reports', tenantId, 'aging', party, asOf] as const,
    queryFn: () => reportsApi.aging(party, asOf || undefined),
    enabled,
  })
