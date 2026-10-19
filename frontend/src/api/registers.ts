import { useQuery } from '@tanstack/react-query'

import { apiFetch } from './client'
import { queryString } from './query'

export interface Register {
  id: string
  branch_id: string
  branch_name: string
  name: string
  cash_account_id: string
  cash_account_name: string
  status: 'active' | 'inactive'
  /** The shift open on this register now, if any. */
  open_shift_id: string | null
  open_shift_user: string | null
}

export interface RegisterInput {
  name: string
  /** Blank = the branch's default cash account. */
  cash_account_id?: string
  status?: 'active' | 'inactive'
}

export interface Shift {
  id: string
  shift_no: number
  register_id: string
  register_name: string
  branch_id: string
  status: 'open' | 'closed'
  opened_by: string
  opened_by_name: string | null
  opened_at: string
  opening_cash: string
  closed_at: string | null
  closed_by_name: string | null
  /** Set when the shift closes. */
  expected_cash: string | null
  counted_cash: string | null
  /** counted − expected: + over, − short. */
  difference: string | null
  note: string | null
}

export interface ShiftFilter {
  register_id?: string
  status?: 'open' | 'closed'
  from?: string
  to?: string
}

export interface ShiftMethodRow {
  method_name: string
  count: number
  money_in: string
  money_out: string
}

/** X report while open (`kind: "X"`), Z report once closed (`kind: "Z"`). */
export interface ShiftReport {
  kind: 'X' | 'Z'
  shift: Shift
  sales_count: number
  sales_total: string
  discount_total: string
  tax_total: string
  due_total: string
  cancelled_count: number
  returns_count: number
  returns_total: string
  payments: ShiftMethodRow[]
  cash_in: string
  cash_out: string
  expected_cash: string
}

export const registersApi = {
  registers: () => apiFetch<Register[]>('/api/v1/registers'),
  createRegister: (input: RegisterInput) => apiFetch<Register>('/api/v1/registers', { method: 'POST', body: input }),
  updateRegister: (id: string, input: RegisterInput) => apiFetch<Register>(`/api/v1/registers/${id}`, { method: 'PUT', body: input }),
  shifts: (filter: ShiftFilter) => apiFetch<Shift[]>(`/api/v1/shifts${queryString({ ...filter, limit: 100 })}`),
  /** The member's open shift in the selected branch, or `null`. */
  currentShift: () => apiFetch<Shift | null>('/api/v1/shifts/current'),
  openShift: (registerId: string, openingCash: string) =>
    apiFetch<Shift>('/api/v1/shifts', { method: 'POST', body: { register_id: registerId, opening_cash: openingCash } }),
  report: (id: string) => apiFetch<ShiftReport>(`/api/v1/shifts/${id}/report`),
  closeShift: (id: string, countedCash: string, note?: string) =>
    apiFetch<ShiftReport>(`/api/v1/shifts/${id}/close`, { method: 'POST', body: { counted_cash: countedCash, note } }),
}

export const registerKeys = {
  all: (tenantId: string) => ['registers', tenantId] as const,
  registers: (tenantId: string) => ['registers', tenantId, 'list'] as const,
  shifts: (tenantId: string, filter: ShiftFilter) => ['registers', tenantId, 'shifts', filter] as const,
  current: (tenantId: string) => ['registers', tenantId, 'current'] as const,
  report: (tenantId: string, id: string) => ['registers', tenantId, 'report', id] as const,
}

export const useRegisters = (tenantId: string, enabled = true) =>
  useQuery({ queryKey: registerKeys.registers(tenantId), queryFn: registersApi.registers, enabled })

export const useShifts = (tenantId: string, filter: ShiftFilter) =>
  useQuery({ queryKey: registerKeys.shifts(tenantId, filter), queryFn: () => registersApi.shifts(filter) })

export const useCurrentShift = (tenantId: string, enabled = true) =>
  useQuery({ queryKey: registerKeys.current(tenantId), queryFn: registersApi.currentShift, enabled })

export const useShiftReport = (tenantId: string, id: string) =>
  useQuery({ queryKey: registerKeys.report(tenantId, id), queryFn: () => registersApi.report(id) })
