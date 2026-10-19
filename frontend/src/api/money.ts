import { useInfiniteQuery, useQuery } from '@tanstack/react-query'

import { apiFetch } from './client'
import { PAGE_SIZE, type Page, queryString } from './query'

export const ACCOUNT_TYPES = ['cash', 'bank', 'mobile_wallet', 'card'] as const
export type AccountType = (typeof ACCOUNT_TYPES)[number]

export interface PaymentAccount {
  id: string
  account_type: AccountType
  name: string
  account_number: string | null
  bank_name: string | null
  opening_balance: string
  /** 0 for members without `accounts.view`. */
  current_balance: string
  ledger_account_id: string | null
  is_default: boolean
  status: 'active' | 'inactive'
  version: number
  created_at: string
}

export interface PaymentAccountInput {
  name: string
  account_type?: AccountType
  account_number?: string
  bank_name?: string
  opening_balance?: string
  status?: 'active' | 'inactive'
}

export interface PaymentMethod {
  id: string
  name: string
  method_type: 'cash' | 'card' | 'mobile_wallet' | 'bank_transfer' | 'cheque' | 'customer_credit' | 'other'
  default_account_id: string | null
  default_account_name: string | null
  requires_reference: boolean
  sort_order: number
  is_active: boolean
}

export interface PaymentMethodPatch {
  default_account_id?: string
  requires_reference?: boolean
  is_active?: boolean
  sort_order?: number
}

export interface BookLine {
  id: string
  business_date: string
  created_at: string
  txn_type: string
  reference_type: string
  reference_id: string
  reference_no: string | null
  note: string | null
  money_in: string
  money_out: string
  balance_after: string
}

export interface Book {
  account: PaymentAccount
  from: string | null
  to: string | null
  opening_balance: string
  lines: BookLine[]
  total_in: string
  total_out: string
  closing_balance: string
}

export interface Payment {
  id: string
  payment_no: string
  direction: 'in' | 'out'
  payment_date: string
  business_date: string
  party_type: 'customer' | 'supplier' | null
  party_id: string | null
  party_name: string | null
  account_id: string
  account_name: string
  payment_method_id: string
  method_name: string
  amount: string
  unallocated_amount: string
  reference: string | null
  note: string | null
  source: 'pos' | 'collection' | 'supplier_payment' | 'refund' | 'expense' | 'income' | 'manual'
  status: 'posted' | 'voided'
  void_reason: string | null
  created_by_name: string | null
  created_at: string
}

export interface PaymentAllocation {
  document_type: string
  document_id: string
  document_no: string | null
  amount: string
  reversed_at: string | null
}

export interface PaymentDetail extends Payment {
  allocations: PaymentAllocation[]
}

export interface PaymentFilter {
  direction?: 'in' | 'out'
  party_type?: 'customer' | 'supplier'
  party_id?: string
  account_id?: string
  source?: string
  from?: string
  to?: string
  status?: 'posted' | 'voided' | 'all'
}

export interface TransferInput {
  from_account_id: string
  to_account_id: string
  amount: string
  fee_amount?: string
  reference?: string
  note?: string
}

export interface Transfer {
  id: string
  transfer_no: string
  from_account_id: string
  from_account_name: string
  to_account_id: string
  to_account_name: string
  amount: string
  fee_amount: string
  business_date: string
  reference: string | null
  note: string | null
  status: string
  created_at: string
}

export interface ExpenseCategory {
  id: string
  name: string
  ledger_account_id: string | null
  ledger_code: string | null
  is_system: boolean
  status: 'active' | 'inactive'
  expense_count: number
}

export interface ExpenseInput {
  category_id: string
  amount: string
  tax_amount?: string
  account_id?: string
  business_date?: string
  payee?: string
  reference?: string
  note?: string
}

export interface Expense {
  id: string
  expense_no: string
  business_date: string
  category_id: string
  category_name: string
  amount: string
  tax_amount: string
  account_id: string
  account_name: string
  payment_id: string | null
  payee: string | null
  reference: string | null
  note: string | null
  status: 'posted' | 'voided'
  void_reason: string | null
  created_by_name: string | null
  created_at: string
}

export interface ExpenseFilter {
  category_id?: string
  account_id?: string
  from?: string
  to?: string
  q?: string
  status?: 'posted' | 'voided' | 'all'
}

export interface ExpensePage extends Page<Expense> {
  total_amount: string
}

export interface IncomeCategory {
  id: string
  name: string
  is_system: boolean
  status: 'active' | 'inactive'
  income_count: number
}

export interface IncomeInput {
  category_id: string
  amount: string
  /** Blank = the branch's default cash account. */
  account_id?: string
  business_date?: string
  source?: string
  reference?: string
  note?: string
}

export interface Income {
  id: string
  income_no: string
  business_date: string
  category_id: string
  category_name: string
  amount: string
  account_id: string
  account_name: string
  payment_id: string | null
  source: string | null
  reference: string | null
  note: string | null
  status: 'posted' | 'voided'
  void_reason: string | null
  created_by_name: string | null
  created_at: string
}

export interface IncomeFilter {
  category_id?: string
  from?: string
  to?: string
  status?: 'posted' | 'voided' | 'all'
}

export interface IncomePage extends Page<Income> {
  total_amount: string
}

export const moneyApi = {
  accounts: (includeInactive = false) =>
    apiFetch<PaymentAccount[]>(`/api/v1/payment-accounts${queryString({ include_inactive: includeInactive || undefined })}`),
  account: (id: string) => apiFetch<PaymentAccount>(`/api/v1/payment-accounts/${id}`),
  createAccount: (input: PaymentAccountInput) => apiFetch<PaymentAccount>('/api/v1/payment-accounts', { method: 'POST', body: input }),
  updateAccount: (id: string, version: number, input: PaymentAccountInput) =>
    apiFetch<PaymentAccount>(`/api/v1/payment-accounts/${id}`, { method: 'PUT', body: input, ifMatch: version }),
  book: (id: string, from?: string, to?: string) => apiFetch<Book>(`/api/v1/payment-accounts/${id}/book${queryString({ from, to })}`),

  methods: (includeInactive = false) =>
    apiFetch<PaymentMethod[]>(`/api/v1/payment-methods${queryString({ include_inactive: includeInactive || undefined })}`),
  updateMethod: (id: string, patch: PaymentMethodPatch) =>
    apiFetch<PaymentMethod>(`/api/v1/payment-methods/${id}`, { method: 'PATCH', body: patch }),

  payments: (filter: PaymentFilter, cursor: string | null) =>
    apiFetch<Page<Payment>>(`/api/v1/payments${queryString({ ...filter, limit: PAGE_SIZE, cursor })}`),
  payment: (id: string) => apiFetch<PaymentDetail>(`/api/v1/payments/${id}`),
  voidPayment: (id: string, reason: string) =>
    apiFetch<PaymentDetail>(`/api/v1/payments/${id}/void`, { method: 'POST', body: { reason } }),

  transfers: (cursor: string | null, accountId?: string) =>
    apiFetch<Page<Transfer>>(`/api/v1/account-transfers${queryString({ account_id: accountId, limit: PAGE_SIZE, cursor })}`),
  createTransfer: (input: TransferInput, idempotencyKey: string) =>
    apiFetch<Transfer>('/api/v1/account-transfers', { method: 'POST', body: input, idempotencyKey }),

  expenseCategories: () => apiFetch<ExpenseCategory[]>('/api/v1/expense-categories'),
  createExpenseCategory: (name: string) =>
    apiFetch<ExpenseCategory>('/api/v1/expense-categories', { method: 'POST', body: { name } }),
  updateExpenseCategory: (id: string, name: string, status: 'active' | 'inactive') =>
    apiFetch<ExpenseCategory>(`/api/v1/expense-categories/${id}`, { method: 'PUT', body: { name, status } }),
  expenses: (filter: ExpenseFilter, cursor: string | null) =>
    apiFetch<ExpensePage>(`/api/v1/expenses${queryString({ ...filter, limit: PAGE_SIZE, cursor })}`),
  expense: (id: string) => apiFetch<Expense>(`/api/v1/expenses/${id}`),
  createExpense: (input: ExpenseInput, idempotencyKey: string) =>
    apiFetch<Expense>('/api/v1/expenses', { method: 'POST', body: input, idempotencyKey }),
  voidExpense: (id: string, reason: string) => apiFetch<Expense>(`/api/v1/expenses/${id}/void`, { method: 'POST', body: { reason } }),

  incomeCategories: () => apiFetch<IncomeCategory[]>('/api/v1/income-categories'),
  createIncomeCategory: (name: string) => apiFetch<IncomeCategory>('/api/v1/income-categories', { method: 'POST', body: { name } }),
  updateIncomeCategory: (id: string, name: string, status: 'active' | 'inactive') =>
    apiFetch<IncomeCategory>(`/api/v1/income-categories/${id}`, { method: 'PUT', body: { name, status } }),
  incomes: (filter: IncomeFilter, cursor: string | null) =>
    apiFetch<IncomePage>(`/api/v1/incomes${queryString({ ...filter, limit: PAGE_SIZE, cursor })}`),
  income: (id: string) => apiFetch<Income>(`/api/v1/incomes/${id}`),
  createIncome: (input: IncomeInput, idempotencyKey: string) =>
    apiFetch<Income>('/api/v1/incomes', { method: 'POST', body: input, idempotencyKey }),
  voidIncome: (id: string, reason: string) => apiFetch<Income>(`/api/v1/incomes/${id}/void`, { method: 'POST', body: { reason } }),
}

export const moneyKeys = {
  all: (tenantId: string) => ['money', tenantId] as const,
  accounts: (tenantId: string, includeInactive = false) => ['money', tenantId, 'accounts', includeInactive] as const,
  account: (tenantId: string, id: string) => ['money', tenantId, 'account', id] as const,
  book: (tenantId: string, id: string, from?: string, to?: string) => ['money', tenantId, 'book', id, from ?? '', to ?? ''] as const,
  methods: (tenantId: string, includeInactive = false) => ['money', tenantId, 'methods', includeInactive] as const,
  payments: (tenantId: string, filter: PaymentFilter) => ['money', tenantId, 'payments', filter] as const,
  payment: (tenantId: string, id: string) => ['money', tenantId, 'payment', id] as const,
  transfers: (tenantId: string, accountId?: string) => ['money', tenantId, 'transfers', accountId ?? ''] as const,
  expenseCategories: (tenantId: string) => ['money', tenantId, 'expense-categories'] as const,
  expenses: (tenantId: string, filter: ExpenseFilter) => ['money', tenantId, 'expenses', filter] as const,
  expense: (tenantId: string, id: string) => ['money', tenantId, 'expense', id] as const,
  incomeCategories: (tenantId: string) => ['money', tenantId, 'income-categories'] as const,
  incomes: (tenantId: string, filter: IncomeFilter) => ['money', tenantId, 'incomes', filter] as const,
}

export const usePaymentAccounts = (tenantId: string, includeInactive = false) =>
  useQuery({ queryKey: moneyKeys.accounts(tenantId, includeInactive), queryFn: () => moneyApi.accounts(includeInactive) })
export const usePaymentAccount = (tenantId: string, id: string) =>
  useQuery({ queryKey: moneyKeys.account(tenantId, id), queryFn: () => moneyApi.account(id) })
export const useBook = (tenantId: string, id: string, from?: string, to?: string) =>
  useQuery({ queryKey: moneyKeys.book(tenantId, id, from, to), queryFn: () => moneyApi.book(id, from, to) })
export const usePaymentMethods = (tenantId: string, includeInactive = false) =>
  useQuery({ queryKey: moneyKeys.methods(tenantId, includeInactive), queryFn: () => moneyApi.methods(includeInactive) })
export const usePayment = (tenantId: string, id: string) =>
  useQuery({ queryKey: moneyKeys.payment(tenantId, id), queryFn: () => moneyApi.payment(id) })
export const usePayments = (tenantId: string, filter: PaymentFilter, enabled = true) =>
  useInfiniteQuery({
    queryKey: moneyKeys.payments(tenantId, filter),
    queryFn: ({ pageParam }) => moneyApi.payments(filter, pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (last) => last.next_cursor,
    enabled,
  })
export const useTransfers = (tenantId: string, accountId?: string) =>
  useInfiniteQuery({
    queryKey: moneyKeys.transfers(tenantId, accountId),
    queryFn: ({ pageParam }) => moneyApi.transfers(pageParam, accountId),
    initialPageParam: null as string | null,
    getNextPageParam: (last) => last.next_cursor,
  })
export const useExpenseCategories = (tenantId: string) =>
  useQuery({ queryKey: moneyKeys.expenseCategories(tenantId), queryFn: moneyApi.expenseCategories })
export const useExpenses = (tenantId: string, filter: ExpenseFilter) =>
  useInfiniteQuery({
    queryKey: moneyKeys.expenses(tenantId, filter),
    queryFn: ({ pageParam }) => moneyApi.expenses(filter, pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (last) => last.next_cursor,
  })
export const useIncomeCategories = (tenantId: string, enabled = true) =>
  useQuery({ queryKey: moneyKeys.incomeCategories(tenantId), queryFn: moneyApi.incomeCategories, enabled })
export const useIncomes = (tenantId: string, filter: IncomeFilter) =>
  useInfiniteQuery({
    queryKey: moneyKeys.incomes(tenantId, filter),
    queryFn: ({ pageParam }) => moneyApi.incomes(filter, pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (last) => last.next_cursor,
  })
