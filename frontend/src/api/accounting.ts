import { useInfiniteQuery, useQuery } from '@tanstack/react-query'

import { apiFetch } from './client'
import { PAGE_SIZE, type Page, queryString } from './query'

export type AccountKind = 'asset' | 'liability' | 'equity' | 'income' | 'expense'

export interface LedgerAccount {
  id: string
  code: string
  name: string
  account_type: AccountKind
  normal_side: 'debit' | 'credit'
  parent_id: string | null
  system_key: string | null
  is_postable: boolean
  status: string
}

export interface JournalEntry {
  id: string
  entry_no: string
  entry_date: string
  source_type: string
  source_id: string | null
  source_no: string | null
  memo: string | null
  total_debit: string
  total_credit: string
  status: 'posted' | 'reversed'
  reversal_of_id: string | null
  created_at: string
}

export interface JournalLine {
  account_id: string
  account_code: string
  account_name: string
  debit: string
  credit: string
  party_type: string | null
  party_id: string | null
  memo: string | null
}

export interface JournalDetail extends JournalEntry {
  lines: JournalLine[]
}

export interface ManualJournalInput {
  date?: string
  memo: string
  lines: { account_id: string; debit?: string; credit?: string; memo?: string }[]
}

export interface FiscalPeriod {
  id: string
  start_date: string
  end_date: string
  status: 'open' | 'locked'
  locked_at: string | null
}

export interface TrialBalanceRow {
  account_id: string
  code: string
  name: string
  account_type: AccountKind
  /** Debit − credit before `from`. */
  opening: string
  debit: string
  credit: string
  closing_debit: string
  closing_credit: string
}

export interface TrialBalance {
  from: string | null
  to: string
  rows: TrialBalanceRow[]
  total_debit: string
  total_credit: string
  total_closing_debit: string
  total_closing_credit: string
  balanced: boolean
}

export interface StatementLine {
  account_id: string
  code: string
  name: string
  amount: string
}

export interface Section {
  lines: StatementLine[]
  total: string
}

export interface ProfitAndLoss {
  from: string | null
  to: string
  revenue: Section
  cost_of_sales: Section
  gross_profit: string
  other_income: Section
  expenses: Section
  net_profit: string
}

export interface BalanceSheet {
  as_of: string
  assets: Section
  liabilities: Section
  equity: Section
  current_earnings: string
  total_assets: string
  total_liabilities_and_equity: string
  balanced: boolean
}

export interface GeneralLedger {
  account: LedgerAccount
  from: string | null
  to: string
  opening: string
  rows: {
    entry_id: string
    entry_no: string
    entry_date: string
    source_type: string
    source_id: string | null
    source_no: string | null
    memo: string | null
    debit: string
    credit: string
    balance: string
  }[]
  closing: string
  truncated: boolean
}

export const accountingApi = {
  ledgerAccounts: () => apiFetch<LedgerAccount[]>('/api/v1/ledger-accounts'),
  journals: (filter: { from?: string; to?: string; source_type?: string }, cursor: string | null) =>
    apiFetch<Page<JournalEntry>>(`/api/v1/journals${queryString({ ...filter, limit: PAGE_SIZE, cursor })}`),
  journal: (id: string) => apiFetch<JournalDetail>(`/api/v1/journals/${id}`),
  createJournal: (input: ManualJournalInput) => apiFetch<JournalDetail>('/api/v1/journals', { method: 'POST', body: input }),
  reverseJournal: (id: string) => apiFetch<JournalDetail>(`/api/v1/journals/${id}/reverse`, { method: 'POST' }),
  periods: () => apiFetch<FiscalPeriod[]>('/api/v1/fiscal-periods'),
  lockPeriod: (month: string) => apiFetch<FiscalPeriod>('/api/v1/fiscal-periods/lock', { method: 'POST', body: { month } }),
  reopenPeriod: (month: string) => apiFetch<FiscalPeriod>('/api/v1/fiscal-periods/reopen', { method: 'POST', body: { month } }),
  trialBalance: (from?: string, to?: string) => apiFetch<TrialBalance>(`/api/v1/reports/trial-balance${queryString({ from, to })}`),
  profitAndLoss: (from?: string, to?: string) =>
    apiFetch<ProfitAndLoss>(`/api/v1/reports/profit-and-loss${queryString({ from, to })}`),
  balanceSheet: (asOf?: string) => apiFetch<BalanceSheet>(`/api/v1/reports/balance-sheet${queryString({ as_of: asOf })}`),
  generalLedger: (accountId: string, from?: string, to?: string) =>
    apiFetch<GeneralLedger>(`/api/v1/reports/general-ledger${queryString({ account_id: accountId, from, to })}`),
}

export const accountingKeys = {
  all: (tenantId: string) => ['accounting', tenantId] as const,
  ledgerAccounts: (tenantId: string) => ['accounting', tenantId, 'ledger-accounts'] as const,
  journals: (tenantId: string, filter: object) => ['accounting', tenantId, 'journals', filter] as const,
  journal: (tenantId: string, id: string) => ['accounting', tenantId, 'journal', id] as const,
  periods: (tenantId: string) => ['accounting', tenantId, 'periods'] as const,
  trialBalance: (tenantId: string, from?: string, to?: string) => ['accounting', tenantId, 'tb', from ?? '', to ?? ''] as const,
  profitAndLoss: (tenantId: string, from?: string, to?: string) => ['accounting', tenantId, 'pl', from ?? '', to ?? ''] as const,
  balanceSheet: (tenantId: string, asOf?: string) => ['accounting', tenantId, 'bs', asOf ?? ''] as const,
  generalLedger: (tenantId: string, accountId: string, from?: string, to?: string) =>
    ['accounting', tenantId, 'gl', accountId, from ?? '', to ?? ''] as const,
}

export const useLedgerAccounts = (tenantId: string) =>
  useQuery({ queryKey: accountingKeys.ledgerAccounts(tenantId), queryFn: accountingApi.ledgerAccounts })
export const useJournals = (tenantId: string, filter: { from?: string; to?: string; source_type?: string }) =>
  useInfiniteQuery({
    queryKey: accountingKeys.journals(tenantId, filter),
    queryFn: ({ pageParam }) => accountingApi.journals(filter, pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (last) => last.next_cursor,
  })
export const useJournal = (tenantId: string, id: string) =>
  useQuery({ queryKey: accountingKeys.journal(tenantId, id), queryFn: () => accountingApi.journal(id) })
export const usePeriods = (tenantId: string) => useQuery({ queryKey: accountingKeys.periods(tenantId), queryFn: accountingApi.periods })
export const useTrialBalance = (tenantId: string, from?: string, to?: string) =>
  useQuery({ queryKey: accountingKeys.trialBalance(tenantId, from, to), queryFn: () => accountingApi.trialBalance(from, to) })
export const useProfitAndLoss = (tenantId: string, from?: string, to?: string) =>
  useQuery({ queryKey: accountingKeys.profitAndLoss(tenantId, from, to), queryFn: () => accountingApi.profitAndLoss(from, to) })
export const useBalanceSheet = (tenantId: string, asOf?: string) =>
  useQuery({ queryKey: accountingKeys.balanceSheet(tenantId, asOf), queryFn: () => accountingApi.balanceSheet(asOf) })
export const useGeneralLedger = (tenantId: string, accountId: string, from?: string, to?: string) =>
  useQuery({
    queryKey: accountingKeys.generalLedger(tenantId, accountId, from, to),
    queryFn: () => accountingApi.generalLedger(accountId, from, to),
    enabled: accountId !== '',
  })
