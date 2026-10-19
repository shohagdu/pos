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

function signedIn(perms: string[], profit: string | null) {
  return {
    'POST /api/v1/auth/refresh': token('t1'),
    'GET /api/v1/auth/me': { user, memberships: [membership('t1', 'Rahim Store')], tenant: currentTenant('t1', 'Rahim Store', perms) },
    'GET /api/v1/dashboard': dashboard(profit),
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
