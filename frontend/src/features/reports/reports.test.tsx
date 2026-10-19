import { screen, within } from '@testing-library/react'
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest'

import i18n from '../../i18n'
import reportsEn from '../reports/i18n/en.json'
import accountingEn from '../accounting/i18n/en.json'

import { currentTenant, membership, mockApi, token, user } from '../../test/api'
import { renderRoute } from '../../test/render'

afterEach(() => vi.restoreAllMocks())

// Register this feature's strings explicitly (deep-merged with the base ones).
beforeAll(() => {
  i18n.addResourceBundle('en', 'translation', reportsEn, true, true)
  i18n.addResourceBundle('en', 'translation', accountingEn, true, true)
})

function signedIn(perms: string[]) {
  return {
    'POST /api/v1/auth/refresh': token('t1'),
    'GET /api/v1/auth/me': { user, memberships: [membership('t1', 'Rahim Store')], tenant: currentTenant('t1', 'Rahim Store', perms) },
  }
}

const accounts = [
  { id: 'a-assets', code: '1000', name: 'Assets', account_type: 'asset', normal_side: 'debit', parent_id: null, system_key: null, is_postable: false, status: 'active' },
  { id: 'a-cash', code: '1010', name: 'Cash in hand', account_type: 'asset', normal_side: 'debit', parent_id: 'a-assets', system_key: 'cash', is_postable: true, status: 'active' },
  { id: 'a-sales', code: '4000', name: 'Sales', account_type: 'income', normal_side: 'credit', parent_id: null, system_key: 'sales', is_postable: true, status: 'active' },
]

const row = (period: string, net: string) => ({
  period,
  invoices: 2,
  gross: net,
  discount: '0.00',
  tax: '0.00',
  grand_total: net,
  returns: '0.00',
  net_sales: net,
  cogs: '100.00',
  gross_profit: '50.00',
  collected: net,
  due: '0.00',
})

describe('reports', () => {
  it('shows a trial balance with its rows and a Balanced badge', async () => {
    const calls = mockApi({
      ...signedIn(['reports.financial']),
      'GET /api/v1/ledger-accounts': accounts,
      'GET /api/v1/reports/trial-balance': {
        from: '2026-10-01',
        to: '2026-10-18',
        rows: [
          { account_id: 'a-cash', code: '1010', name: 'Cash in hand', account_type: 'asset', opening: '0', debit: '1500.00', credit: '0', closing_debit: '1500.00', closing_credit: '0' },
          { account_id: 'a-sales', code: '4000', name: 'Sales', account_type: 'income', opening: '0', debit: '0', credit: '1500.00', closing_debit: '0', closing_credit: '1500.00' },
        ],
        total_debit: '1500.00',
        total_credit: '1500.00',
        total_closing_debit: '1500.00',
        total_closing_credit: '1500.00',
        balanced: true,
      },
    })
    renderRoute('/reports/trial-balance')
    const table = await screen.findByRole('table', { name: 'Trial balance' })
    expect(within(table).getByText('Cash in hand')).toBeInTheDocument()
    expect(within(table).getByRole('link', { name: '4000' })).toHaveAttribute('href', expect.stringContaining('/reports/general-ledger?account_id=a-sales'))
    expect(screen.getByText('Balanced')).toBeInTheDocument()
    expect(screen.getByText('Generated', { exact: false })).toHaveTextContent('Rahim Uddin')
    expect(calls.find((c) => c.path === '/api/v1/reports/trial-balance')?.search).toMatch(/from=\d{4}-\d{2}-01&to=/)
  })

  it('shows the sales summary with a totals row and hides cost columns when not returned', async () => {
    mockApi({
      ...signedIn(['reports.sales']),
      'GET /api/v1/reports/sales-summary': {
        from: '2026-10-01',
        to: '2026-10-18',
        group: 'day',
        rows: [{ ...row('2026-10-17', '150.00'), cogs: null, gross_profit: null }, { ...row('2026-10-18', '250.00'), cogs: null, gross_profit: null }],
        totals: { ...row('', '400.00'), invoices: 4, cogs: null, gross_profit: null },
        payments: [{ method_name: 'Cash', money_in: '400.00', money_out: '0.00', count: 4 }],
        cancelled_count: 1,
      },
    })
    renderRoute('/reports/sales')
    const table = await screen.findByRole('table', { name: 'Summary' })
    const footer = within(table).getAllByRole('row').at(-1)!
    expect(within(footer).getByText('Totals')).toBeInTheDocument()
    expect(within(footer).getAllByText(/400\.00/).length).toBeGreaterThan(0)
    expect(within(table).queryByRole('columnheader', { name: 'COGS' })).not.toBeInTheDocument()
    expect(screen.getByText('1 cancelled invoice is not included.')).toBeInTheDocument()
    expect(screen.getByRole('table', { name: 'Payments by method' })).toBeInTheDocument()
  })

  it('shows only the report cards the member may open', async () => {
    mockApi(signedIn(['reports.sales', 'expenses.view']))
    renderRoute('/reports')
    const business = await screen.findByRole('region', { name: 'Business reports' })
    expect(within(business).getByRole('link', { name: /^Sales/ })).toHaveAttribute('href', '/reports/sales')
    expect(within(business).getByRole('link', { name: /^Expenses/ })).toHaveAttribute('href', '/reports/expenses')
    expect(within(business).queryByRole('link', { name: /^Profit/ })).not.toBeInTheDocument()
    expect(within(business).queryByRole('link', { name: /^Purchases/ })).not.toBeInTheDocument()
    expect(screen.queryByRole('region', { name: 'Accounting' })).not.toBeInTheDocument()
    expect(screen.queryByRole('link', { name: /^Trial balance/ })).not.toBeInTheDocument()
  })
})
