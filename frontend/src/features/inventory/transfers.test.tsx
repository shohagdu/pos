import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { currentTenant, membership, mockApi, token, user } from '../../test/api'
import { renderRoute } from '../../test/render'

afterEach(() => vi.restoreAllMocks())

const perms = ['dashboard.view', 'inventory.view', 'inventory.transfer', 'inventory.transfer_receive', 'inventory.count', 'inventory.adjust', 'products.view']

function signedIn(permissions = perms) {
  const tenant = { ...currentTenant('t1', 'Rahim Store', permissions), features: ['pos.core', 'inventory.transfers'] }
  return {
    'POST /api/v1/auth/refresh': token('t1'),
    'GET /api/v1/auth/me': { user, memberships: [membership('t1', 'Rahim Store')], tenant },
    'GET /api/v1/categories': [],
    'GET /api/v1/branches': [
      { id: 'b1', code: 'MAIN', name: 'Main', address: null, phone: null, receipt_header: null, is_default: true, status: 'active', version: 1 },
      { id: 'b2', code: 'UTT', name: 'Uttara', address: null, phone: null, receipt_header: null, is_default: false, status: 'active', version: 1 },
    ],
    'GET /api/v1/stock-locations': [
      { id: 'loc1', branch_id: 'b1', branch_name: 'Main', code: 'MAIN', name: 'Shop floor', location_type: 'store', is_sellable: true, status: 'active' },
      { id: 'loc2', branch_id: 'b2', branch_name: 'Uttara', code: 'UTT', name: 'Uttara floor', location_type: 'store', is_sellable: true, status: 'active' },
    ],
  }
}

const summary = {
  id: 'tr1', transfer_no: 'TRF-0001', business_date: '2026-10-19', status: 'dispatched', from_branch_id: 'b1', from_branch_name: 'Main',
  from_location_id: 'loc1', from_location_name: 'Shop floor', to_branch_id: 'b2', to_branch_name: 'Uttara', to_location_id: 'loc2',
  to_location_name: 'Uttara floor', note: null, total_value: '4000.00', loss_value: '0.00', item_count: 2, created_by_name: 'Rahim Uddin',
  dispatched_at: '2026-10-19T04:00:00Z', received_at: null, version: 2, created_at: '2026-10-19T03:00:00Z',
}
const transfer = {
  ...summary,
  items: [
    { id: 'i1', variant_id: 'v1', product_name: 'Miniket Rice 5kg', sku: 'P000001', unit_short_name: 'pcs', quantity_sent: '10.000', quantity_received: null, unit_cost: '400.0000' },
    { id: 'i2', variant_id: 'v2', product_name: 'Lux Soap', sku: 'P000002', unit_short_name: 'pcs', quantity_sent: '5.000', quantity_received: null, unit_cost: null },
  ],
}

const product = {
  id: 'p1', variant_id: 'v1', product_type: 'standard', name: 'Miniket Rice 5kg', name_bn: null, description: null, sku: 'P000001',
  barcodes: [], category_id: null, category_name: null, brand_id: null, brand_name: null, unit_id: 'u-pcs', unit_short_name: 'pcs',
  allow_fraction: false, tax_rate_id: null, tax_rate_pct: null, track_inventory: true, purchase_price: '400.0000', selling_price: '450.0000',
  wholesale_price: null, min_selling_price: null, reorder_level: null, status: 'active', version: 1, created_at: '', updated_at: '',
}

describe('stock transfers', () => {
  it('lists transfers with a direction filter', async () => {
    const calls = mockApi({ ...signedIn(), 'GET /api/v1/stock-transfers': { data: [summary], next_cursor: null } })
    renderRoute('/inventory/transfers')
    const table = await screen.findByRole('table')
    expect(within(table).getByRole('link', { name: 'TRF-0001' })).toHaveAttribute('href', '/inventory/transfers/tr1')
    expect(within(table).getByText('In transit')).toBeInTheDocument()
    await userEvent.selectOptions(screen.getByLabelText('Direction'), 'in')
    await waitFor(() => expect(calls.some((c) => c.path === '/api/v1/stock-transfers' && c.search.includes('direction=in'))).toBe(true))
    expect(screen.getByRole('link', { name: 'New transfer' })).toHaveAttribute('href', '/inventory/transfers/new')
  })

  it('creates and dispatches a transfer with an Idempotency-Key', async () => {
    const calls = mockApi({
      ...signedIn(),
      'GET /api/v1/products': { data: [product], next_cursor: null },
      'POST /api/v1/stock-transfers': () => Response.json(transfer, { status: 201 }),
      'GET /api/v1/stock-transfers/tr1': transfer,
    })
    const { router } = renderRoute('/inventory/transfers/new')
    await userEvent.type(await screen.findByLabelText('Search products'), 'rice')
    await userEvent.click(screen.getByRole('button', { name: 'Search' }))
    await userEvent.click(await screen.findByRole('button', { name: 'Add Miniket Rice 5kg' }))
    await userEvent.type(screen.getByLabelText('Quantity for Miniket Rice 5kg'), '10')
    await userEvent.click(screen.getByRole('button', { name: 'Save and dispatch' }))
    expect(await screen.findByText('Choose where the stock goes.')).toBeInTheDocument()
    await userEvent.selectOptions(screen.getByLabelText('To'), 'loc2')
    await userEvent.click(screen.getByRole('button', { name: 'Save and dispatch' }))

    await waitFor(() => expect(router.state.location.pathname).toBe('/inventory/transfers/tr1'))
    const post = calls.find((c) => c.method === 'POST' && c.path === '/api/v1/stock-transfers')
    expect(post?.body).toEqual({ to_location_id: 'loc2', lines: [{ variant_id: 'v1', quantity: '10' }], dispatch: true })
    expect(post?.headers.get('Idempotency-Key')).toBeTruthy()
  })

  it('receives a transfer sending only the lines that arrived short', async () => {
    const received = { ...transfer, status: 'received', loss_value: '400.00', items: transfer.items.map((i) => ({ ...i, quantity_received: i.id === 'i1' ? '9.000' : '5.000' })) }
    let current: object = transfer
    const calls = mockApi({
      ...signedIn(),
      'GET /api/v1/stock-transfers/tr1': () => Response.json(current),
      'POST /api/v1/stock-transfers/tr1/receive': () => Response.json((current = received)),
    })
    renderRoute('/inventory/transfers/tr1')
    await userEvent.click(await screen.findByRole('button', { name: 'Receive' }))
    const qty = screen.getByLabelText('Received quantity for Miniket Rice 5kg')
    expect(qty).toHaveValue('10')
    await userEvent.clear(qty)
    await userEvent.type(qty, '9')
    await userEvent.click(screen.getByRole('button', { name: 'Confirm receipt' }))
    await waitFor(() => expect(calls.some((c) => c.path === '/api/v1/stock-transfers/tr1/receive')).toBe(true))
    const post = calls.find((c) => c.path === '/api/v1/stock-transfers/tr1/receive')
    expect(post?.body).toEqual({ lines: [{ item_id: 'i1', quantity_received: '9' }] })
    expect(await screen.findByText('Transit loss')).toBeInTheDocument()
  })
})

describe('stock counts', () => {
  const count = {
    id: 'c1', count_no: 'CNT-0001', business_date: '2026-10-19', status: 'in_progress', branch_id: 'b1', location_id: 'loc1', location_name: 'Shop floor',
    category_name: null, note: null, item_count: 2, counted_count: 1, variance_value: null, created_by_name: 'Rahim Uddin',
    snapshot_at: '2026-10-19T03:00:00Z', posted_at: null, created_at: '2026-10-19T03:00:00Z',
    items: [
      { id: 'ci1', variant_id: 'v1', product_name: 'Miniket Rice 5kg', sku: 'P000001', barcode: null, unit_short_name: 'pcs', allow_fraction: false,
        system_qty: '10.000', counted_qty: '10.000', variance_qty: '0.000', unit_cost: '400.0000' },
      { id: 'ci2', variant_id: 'v2', product_name: 'Lux Soap', sku: 'P000002', barcode: '8901', unit_short_name: 'pcs', allow_fraction: false,
        system_qty: '5.000', counted_qty: null, variance_qty: null, unit_cost: '30.0000' },
    ],
  }

  it('saves only changed entries, clearing a blanked one', async () => {
    const saved = { ...count, items: count.items.map((i) => ({ ...i, counted_qty: i.id === 'ci2' ? '4.000' : null })) }
    let current: object = count
    const calls = mockApi({
      ...signedIn(),
      'GET /api/v1/stock-counts/c1': () => Response.json(current),
      'PUT /api/v1/stock-counts/c1/items': () => Response.json((current = saved)),
    })
    renderRoute('/inventory/counts/c1')
    const soap = await screen.findByLabelText('Counted quantity for Lux Soap')
    await userEvent.type(soap, '4')
    expect(screen.getByText('-1')).toBeInTheDocument()
    await userEvent.clear(screen.getByLabelText('Counted quantity for Miniket Rice 5kg'))
    expect(screen.getByRole('button', { name: 'Post count' })).toBeDisabled()
    await userEvent.click(screen.getByRole('button', { name: 'Save entries' }))
    await waitFor(() => expect(calls.some((c) => c.method === 'PUT')).toBe(true))
    const put = calls.find((c) => c.method === 'PUT')
    expect(put?.body).toEqual({ lines: [{ item_id: 'ci1', counted_qty: null }, { item_id: 'ci2', counted_qty: '4' }] })
    expect(await screen.findByText('Entries saved.')).toBeInTheDocument()
  })
})
