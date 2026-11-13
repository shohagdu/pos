import { screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
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

  it('ranks the best-selling and most profitable products', async () => {
    const product = (id: string, name: string, quantity: string, net: string, profit: string) => ({
      variant_id: id, product_name: name, sku: id.toUpperCase(), category_name: 'Grocery', unit_short_name: 'pcs',
      quantity, net_sales: net, cogs: '0.00', gross_profit: profit, margin_pct: '10.00',
    })
    const calls = mockApi({
      ...signedIn(['reports.sales', 'products.view_cost']),
      'GET /api/v1/categories': [],
      'GET /api/v1/reports/sales-by-product': {
        from: '2026-10-01', to: '2026-10-18',
        rows: [product('v-rice', 'Miniket Rice 5kg', '10.000', '4500.00', '500.00'), product('v-salt', 'Iodized Salt 1kg', '40.000', '1680.00', '280.00'), product('v-tea', 'Black Tea 400g', '2.000', '520.00', '640.00')],
        total_net_sales: '6700.00', total_cogs: '5280.00', total_gross_profit: '1420.00',
      },
    })
    renderRoute('/reports/sales')
    await userEvent.click(await screen.findByRole('tab', { name: 'Best sellers' }))
    let table = await screen.findByRole('table', { name: 'Best sellers' })
    // Ranked by quantity sold: salt (40), rice (10), tea (2).
    let names = within(table).getAllByRole('row').slice(1).map((r) => within(r).getAllByRole('cell')[1]?.textContent)
    expect(names).toEqual(['Iodized Salt 1kgV-SALT', 'Miniket Rice 5kgV-RICE', 'Black Tea 400gV-TEA'])
    await userEvent.selectOptions(screen.getByLabelText('Rank by'), 'net_sales')
    names = within(table).getAllByRole('row').slice(1).map((r) => within(r).getAllByRole('cell')[1]?.textContent)
    expect(names[0]).toBe('Miniket Rice 5kgV-RICE')

    await userEvent.click(screen.getByRole('tab', { name: 'Top profit' }))
    table = await screen.findByRole('table', { name: 'Top profit' })
    names = within(table).getAllByRole('row').slice(1).map((r) => within(r).getAllByRole('cell')[1]?.textContent)
    expect(names).toEqual(['Black Tea 400gV-TEA', 'Miniket Rice 5kgV-RICE', 'Iodized Salt 1kgV-SALT'])
    // One request serves all three product views.
    expect(calls.filter((c) => c.path === '/api/v1/reports/sales-by-product')).toHaveLength(1)
  })

  it('offers Top profit only to members who may see cost', async () => {
    mockApi({ ...signedIn(['reports.sales']), 'GET /api/v1/reports/sales-summary': { from: '2026-10-01', to: '2026-10-18', group: 'day', rows: [], totals: { ...row('', '0.00'), invoices: 0 }, payments: [], cancelled_count: 0 } })
    renderRoute('/reports/sales')
    expect(await screen.findByRole('tab', { name: 'Best sellers' })).toBeInTheDocument()
    expect(screen.queryByRole('tab', { name: 'Top profit' })).not.toBeInTheDocument()
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
