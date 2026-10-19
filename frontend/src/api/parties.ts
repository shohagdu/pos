import { useInfiniteQuery, useQuery } from '@tanstack/react-query'

import { apiFetch } from './client'
import { PAGE_SIZE, type Page, queryString } from './query'

export type PartyStatus = 'active' | 'inactive'

export interface Customer {
  id: string
  code: string
  name: string
  phone: string | null
  email: string | null
  address: string | null
  /** `null` = no limit. */
  credit_limit: string | null
  opening_balance: string
  /** Positive = owes us; negative = advance held. */
  current_balance: string
  is_walk_in: boolean
  notes: string | null
  status: PartyStatus
  version: number
  created_at: string
  updated_at: string
}

export interface CustomerInput {
  name: string
  phone?: string
  email?: string
  address?: string
  credit_limit?: string
  /** Create only. */
  opening_balance?: string
  notes?: string
  /** Update only. */
  status?: PartyStatus
}

export interface PartyFilter {
  q?: string
  status?: 'active' | 'inactive' | 'all'
  due_only?: boolean
}

export interface CustomerPage extends Page<Customer> {
  total_due: string
}

export interface Supplier {
  id: string
  code: string
  name: string
  contact_person: string | null
  phone: string | null
  email: string | null
  address: string | null
  opening_balance: string
  /** Positive = we owe them; negative = advance paid. */
  current_balance: string
  notes: string | null
  status: PartyStatus
  version: number
  created_at: string
  updated_at: string
}

export interface SupplierInput {
  name: string
  contact_person?: string
  phone?: string
  email?: string
  address?: string
  opening_balance?: string
  notes?: string
  status?: PartyStatus
}

export interface SupplierPage extends Page<Supplier> {
  total_payable: string
}

export interface StatementEntry {
  id: string
  business_date: string
  created_at: string
  entry_type: string
  reference_type: string
  reference_id: string
  reference_no: string | null
  note: string | null
  debit: string
  credit: string
  balance: string
}

export interface Statement {
  party_type: 'customer' | 'supplier'
  party_id: string
  party_name: string
  from: string | null
  to: string | null
  opening_balance: string
  entries: StatementEntry[]
  total_debit: string
  total_credit: string
  closing_balance: string
}

export const partiesApi = {
  customers: (filter: PartyFilter, cursor: string | null, limit = PAGE_SIZE) =>
    apiFetch<CustomerPage>(`/api/v1/customers${queryString({ ...filter, limit, cursor })}`),
  walkIn: () => apiFetch<Customer>('/api/v1/customers/walk-in'),
  customer: (id: string) => apiFetch<Customer>(`/api/v1/customers/${id}`),
  createCustomer: (input: CustomerInput) => apiFetch<Customer>('/api/v1/customers', { method: 'POST', body: input }),
  updateCustomer: (id: string, version: number, input: CustomerInput) =>
    apiFetch<Customer>(`/api/v1/customers/${id}`, { method: 'PUT', body: input, ifMatch: version }),
  deleteCustomer: (id: string) => apiFetch<void>(`/api/v1/customers/${id}`, { method: 'DELETE' }),
  customerStatement: (id: string, from?: string, to?: string) =>
    apiFetch<Statement>(`/api/v1/customers/${id}/statement${queryString({ from, to })}`),

  suppliers: (filter: PartyFilter, cursor: string | null, limit = PAGE_SIZE) =>
    apiFetch<SupplierPage>(`/api/v1/suppliers${queryString({ ...filter, limit, cursor })}`),
  supplier: (id: string) => apiFetch<Supplier>(`/api/v1/suppliers/${id}`),
  createSupplier: (input: SupplierInput) => apiFetch<Supplier>('/api/v1/suppliers', { method: 'POST', body: input }),
  updateSupplier: (id: string, version: number, input: SupplierInput) =>
    apiFetch<Supplier>(`/api/v1/suppliers/${id}`, { method: 'PUT', body: input, ifMatch: version }),
  deleteSupplier: (id: string) => apiFetch<void>(`/api/v1/suppliers/${id}`, { method: 'DELETE' }),
  supplierStatement: (id: string, from?: string, to?: string) =>
    apiFetch<Statement>(`/api/v1/suppliers/${id}/statement${queryString({ from, to })}`),
}

export const partyKeys = {
  customers: (tenantId: string) => ['parties', tenantId, 'customers'] as const,
  customerList: (tenantId: string, filter: PartyFilter) => ['parties', tenantId, 'customers', 'list', filter] as const,
  customer: (tenantId: string, id: string) => ['parties', tenantId, 'customers', id] as const,
  walkIn: (tenantId: string) => ['parties', tenantId, 'customers', 'walk-in'] as const,
  customerStatement: (tenantId: string, id: string, from?: string, to?: string) =>
    ['parties', tenantId, 'customers', id, 'statement', from ?? '', to ?? ''] as const,
  suppliers: (tenantId: string) => ['parties', tenantId, 'suppliers'] as const,
  supplierList: (tenantId: string, filter: PartyFilter) => ['parties', tenantId, 'suppliers', 'list', filter] as const,
  supplier: (tenantId: string, id: string) => ['parties', tenantId, 'suppliers', id] as const,
  supplierStatement: (tenantId: string, id: string, from?: string, to?: string) =>
    ['parties', tenantId, 'suppliers', id, 'statement', from ?? '', to ?? ''] as const,
}

export const useCustomers = (tenantId: string, filter: PartyFilter) =>
  useInfiniteQuery({
    queryKey: partyKeys.customerList(tenantId, filter),
    queryFn: ({ pageParam }) => partiesApi.customers(filter, pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (last) => last.next_cursor,
  })

export const useCustomer = (tenantId: string, id: string) =>
  useQuery({ queryKey: partyKeys.customer(tenantId, id), queryFn: () => partiesApi.customer(id) })

export const useWalkInCustomer = (tenantId: string) =>
  useQuery({ queryKey: partyKeys.walkIn(tenantId), queryFn: partiesApi.walkIn, staleTime: Infinity })

export const useCustomerStatement = (tenantId: string, id: string, from?: string, to?: string) =>
  useQuery({
    queryKey: partyKeys.customerStatement(tenantId, id, from, to),
    queryFn: () => partiesApi.customerStatement(id, from, to),
  })

export const useSuppliers = (tenantId: string, filter: PartyFilter) =>
  useInfiniteQuery({
    queryKey: partyKeys.supplierList(tenantId, filter),
    queryFn: ({ pageParam }) => partiesApi.suppliers(filter, pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (last) => last.next_cursor,
  })

export const useSupplier = (tenantId: string, id: string) =>
  useQuery({ queryKey: partyKeys.supplier(tenantId, id), queryFn: () => partiesApi.supplier(id) })

export const useSupplierStatement = (tenantId: string, id: string, from?: string, to?: string) =>
  useQuery({
    queryKey: partyKeys.supplierStatement(tenantId, id, from, to),
    queryFn: () => partiesApi.supplierStatement(id, from, to),
  })
