import { screen, within } from '@testing-library/react'
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest'

import i18n from '../../i18n'
import dashboardEn from '../dashboard/i18n/en.json'
import reportsEn from '../reports/i18n/en.json'

import { currentTenant, membership, mockApi, token, user } from '../../test/api'
import { renderRoute } from '../../test/render'

afterEach(() => vi.restoreAllMocks())

// Register this feature's strings explicitly (deep-merged with the base ones).
beforeAll(() => {
  i18n.addResourceBundle('en', 'translation', dashboardEn, true, true)
  i18n.addResourceBundle('en', 'translation', reportsEn, true, true)
})

const figures = (sales: string, profit: string | null) => ({
  invoices: 3,
  sales,
  net_sales: sales,
  cash_received: sales,
  expenses: '10.00',
  gross_profit: profit,
  net_profit: profit,
})

const dashboard = (profit: string | null) => ({
  today: '2026-10-18',
  today_figures: figures('300.00', profit),
  month_figures: figures('9000.00', profit),
  receivables: '1200.00',
  payables: '800.00',
  money_balance: '5000.00',
  low_stock_count: 2,
  out_of_stock_count: 1,
  top_products: [{ variant_id: 'v1', product_name: 'Miniket Rice 5kg', unit_short_name: 'pcs', quantity: '12', net_sales: '5400.00' }],
  recent_sales: [{ id: 's1', invoice_no: 'INV-0001', customer_name: 'Walk-in', grand_total: '300.00', due_total: '0', status: 'completed', sale_date: '2026-10-18' }],
  last_14_days: [
    { date: '2026-10-17', sales: '100.00' },
    { date: '2026-10-18', sales: '300.00' },
  ],
})

function signedIn(perms: string[], profit: string | null, data: object = dashboard(profit)) {
  return {
    'POST /api/v1/auth/refresh': token('t1'),
    'GET /api/v1/auth/me': { user, memberships: [membership('t1', 'Rahim Store')], tenant: currentTenant('t1', 'Rahim Store', perms) },
    'GET /api/v1/dashboard': data,
    'GET /readyz': { status: 'ready' },
    'GET /api/v1/meta': { name: 'pos-api', version: '0.1.0', environment: 'test' },
  }
}

describe('dashboard', () => {
  it('shows figures, balances, chart text, top products and recent sales', async () => {
    mockApi(signedIn(['dashboard.view', 'dashboard.view_financials', 'pos.sell'], '120.00'))
    renderRoute('/')
    const today = await screen.findByRole('region', { name: 'Today' })
    expect(within(today).getByText('Net profit')).toBeInTheDocument()
    expect(screen.getByRole('link', { name: /Low stock/ })).toHaveAttribute('href', '/inventory?status=low')
    expect(screen.getByRole('table', { name: 'Sales, last 14 days' })).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'INV-0001' })).toHaveAttribute('href', '/sales/s1')
    expect(screen.getByText('Miniket Rice 5kg')).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'New sale' })).toHaveAttribute('href', '/pos')
    expect(screen.queryByRole('link', { name: 'New purchase' })).not.toBeInTheDocument()
    expect(screen.getByRole('heading', { level: 1, name: /Rahim/ })).toBeInTheDocument()
    expect(screen.getByText('Yesterday: ৳100.00')).toBeInTheDocument()
    expect(screen.getByRole('link', { name: /Out of stock/ })).toHaveAttribute('href', '/inventory?status=out')
    expect(screen.getByRole('link', { name: /Customers owe/ })).toHaveAttribute('href', '/customers')
    expect(screen.queryByRole('region', { name: /get your shop selling/ })).not.toBeInTheDocument()
  })

  it('shows only the quick actions the user is allowed to use', async () => {
    mockApi(signedIn(['dashboard.view', 'pos.sell', 'products.create', 'purchases.create', 'expenses.create', 'customers.manage', 'reports.sales'], null))
    renderRoute('/')
    const nav = await screen.findByRole('navigation', { name: 'Quick actions' })
    const links = within(nav).getAllByRole('link')
    expect(links.map((l) => [l.textContent, l.getAttribute('href')])).toEqual([
      ['New sale', '/pos'],
      ['Add product', '/products/new'],
      ['New purchase', '/purchases/new'],
      ['Record expense', '/expenses'],
      ['Add customer', '/customers/new'],
      ['Reports', '/reports'],
    ])
  })

  it('hides quick actions without permissions', async () => {
    mockApi(signedIn(['dashboard.view'], null))
    renderRoute('/')
    await screen.findByRole('region', { name: 'Today' })
    expect(screen.queryByRole('navigation', { name: 'Quick actions' })).not.toBeInTheDocument()
    expect(screen.queryByRole('link', { name: 'New sale' })).not.toBeInTheDocument()
  })

  it('guides a new shop without sales through getting started', async () => {
    const empty = {
      ...dashboard(null),
      today_figures: { ...figures('0', null), invoices: 0 },
      month_figures: { ...figures('0', null), invoices: 0 },
      top_products: [],
      recent_sales: [],
      last_14_days: [
        { date: '2026-10-17', sales: '0' },
        { date: '2026-10-18', sales: '0' },
      ],
    }
    mockApi(signedIn(['dashboard.view', 'pos.sell', 'products.create'], null, empty))
    renderRoute('/')
    const start = await screen.findByRole('region', { name: /get your shop selling/ })
    expect(within(start).getByRole('link', { name: 'Add products' })).toHaveAttribute('href', '/products/new')
    expect(within(start).getByRole('link', { name: 'Start selling' })).toHaveAttribute('href', '/pos')
    // Opening stock needs its own permission.
    expect(within(start).queryByRole('link', { name: 'Add opening stock' })).not.toBeInTheDocument()
    expect(screen.getByText('No sales in the last 14 days yet.')).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'New sale' })).toHaveAttribute('href', '/pos')
  })

  it('hides profit without the financials permission', async () => {
    mockApi(signedIn(['dashboard.view'], null))
    renderRoute('/')
    const today = await screen.findByRole('region', { name: 'Today' })
    expect(within(today).getByText('Net sales')).toBeInTheDocument()
    expect(screen.queryByText('Net profit')).not.toBeInTheDocument()
    expect(screen.queryByText('Gross profit')).not.toBeInTheDocument()
  })
})
