import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { currentTenant, membership, mockApi, token, user } from '../../test/api'
import { renderRoute } from '../../test/render'

afterEach(() => vi.restoreAllMocks())

const perms = ['dashboard.view', 'inventory.view', 'inventory.adjust', 'inventory.opening_stock', 'products.view', 'products.view_cost']

function signedIn(permissions = perms) {
  return {
    'POST /api/v1/auth/refresh': token('t1'),
    'GET /api/v1/auth/me': { user, memberships: [membership('t1', 'Rahim Store')], tenant: currentTenant('t1', 'Rahim Store', permissions) },
    'GET /api/v1/categories': [],
    'GET /api/v1/stock-locations': [
      { id: 'loc1', branch_id: 'b1', code: 'MAIN', name: 'Shop floor', location_type: 'store', is_sellable: true, status: 'active' },
    ],
  }
}

const stockRow = {
  variant_id: 'v1', product_id: 'p1', name: 'Miniket Rice 5kg', name_bn: 'মিনিকেট চাল', sku: 'P000001', unit_short_name: 'pcs',
  allow_fraction: false, category_name: 'Rice', brand_name: null, quantity: '3.000', avg_cost: '400.0000', value: '1200.0000',
  selling_price: '450.0000', reorder_level: '5.000', is_low: true, last_movement_at: null,
}

const product = (over: object) => ({
  id: 'p1', variant_id: 'v1', product_type: 'standard', name: 'Miniket Rice 5kg', name_bn: null, description: null, sku: 'P000001',
  barcodes: [], category_id: null, category_name: null, brand_id: null, brand_name: null, unit_id: 'u-pcs', unit_short_name: 'pcs',
  allow_fraction: false, tax_rate_id: null, tax_rate_pct: null, track_inventory: true, purchase_price: '400.0000', selling_price: '450.0000',
  wholesale_price: null, min_selling_price: null, reorder_level: null, status: 'active', version: 1, created_at: '', updated_at: '',
  ...over,
})

describe('inventory', () => {
  it('shows stock with the low-stock badge and summary', async () => {
    const calls = mockApi({
      ...signedIn(),
      'GET /api/v1/stock': { data: [stockRow], next_cursor: null, summary: { items: 1, quantity: '3', value: '1200', low_count: 1, out_count: 0 } },
    })
    renderRoute('/inventory?status=low')
    const table = await screen.findByRole('table')
    expect(within(table).getByText('Miniket Rice 5kg')).toBeInTheDocument()
    expect(within(table).getByText('Low stock')).toBeInTheDocument()
    expect(within(table).getByRole('link', { name: 'Movements' })).toHaveAttribute('href', '/inventory/movements?variant_id=v1')
    expect(calls.find((c) => c.path === '/api/v1/stock')?.search).toContain('status=low')
    expect(screen.getByRole('link', { name: 'Opening stock' })).toHaveAttribute('href', '/inventory/adjustments/new?reason=opening')
  })

  it('posts opening stock with positive lines, cost and an Idempotency-Key', async () => {
    const created = {
      id: 'adj1', adjustment_no: 'ADJ-0001', business_date: '2026-10-18', reason: 'opening', note: null, status: 'posted', location_id: 'loc1',
      location_name: 'Shop floor', total_value: '4000.00', item_count: 1, created_by_name: 'Rahim Uddin', created_at: '2026-10-18T04:00:00Z',
      items: [{ id: 'i1', variant_id: 'v1', product_name: 'Miniket Rice 5kg', sku: 'P000001', unit_short_name: 'pcs', quantity: '10.000', unit_cost: '400.0000', note: null }],
    }
    const calls = mockApi({
      ...signedIn(),
      'GET /api/v1/products': { data: [product({}), product({ id: 'p2', variant_id: 'v2', name: 'Delivery charge', track_inventory: false })], next_cursor: null },
      'POST /api/v1/stock-adjustments': () => Response.json(created, { status: 201 }),
      'GET /api/v1/stock-adjustments/adj1': created,
    })
    const { router } = renderRoute('/inventory/adjustments/new?reason=opening')
    await userEvent.type(await screen.findByLabelText('Search products'), 'rice')
    await userEvent.click(screen.getByRole('button', { name: 'Search' }))
    await userEvent.click(await screen.findByRole('button', { name: 'Add Delivery charge' }))
    expect(screen.getByText(/Delivery charge does not track stock/)).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Add Miniket Rice 5kg' }))
    expect(screen.getByLabelText('Unit cost for Miniket Rice 5kg')).toHaveValue('400')
    await userEvent.type(screen.getByLabelText('Quantity for Miniket Rice 5kg'), '10')
    await userEvent.click(screen.getByRole('button', { name: 'Save opening stock' }))

    await waitFor(() => expect(router.state.location.pathname).toBe('/inventory/adjustments/adj1'))
    const post = calls.find((c) => c.method === 'POST' && c.path === '/api/v1/stock-adjustments')
    expect(post?.body).toEqual({ reason: 'opening', lines: [{ variant_id: 'v1', quantity: '10', unit_cost: '400' }] })
    expect(post?.headers.get('Idempotency-Key')).toBeTruthy()
    expect(await screen.findByText('ADJ-0001', { exact: false })).toBeInTheDocument()
  })
})
