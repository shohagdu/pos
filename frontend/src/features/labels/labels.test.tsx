import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { currentTenant, membership, mockApi, token, user } from '../../test/api'
import { renderRoute } from '../../test/render'
import { layoutById, paginate } from './layouts'

afterEach(() => {
  vi.restoreAllMocks()
  localStorage.clear()
})

const perms = ['dashboard.view', 'purchases.view', 'products.view', 'products.update']

function signedIn(permissions = perms) {
  return {
    'POST /api/v1/auth/refresh': token('t1'),
    'GET /api/v1/auth/me': { user, memberships: [membership('t1', 'Rahim Store')], tenant: currentTenant('t1', 'Rahim Store', permissions) },
  }
}

const item = (id: string, variant: string, name: string, quantity: string, received: string, returned = '0.000') => ({
  id, line_no: 1, variant_id: variant, product_name: name, sku: 'P1', unit_short_name: 'pcs', quantity, received_quantity: received,
  returned_quantity: returned, unit_cost: '10.0000', discount_type: null, discount_value: null, line_discount: '0.00', bill_discount_alloc: '0.00',
  tax_rate_pct: '0.0000', tax_amount: '0.00', charges_alloc: '0.00', line_total: '0.00', landed_unit_cost: '10.0000', previous_unit_cost: null,
})

const purchase = {
  id: 'pur1', purchase_no: 'PUR-2611-00007', supplier_id: 's1', supplier_name: 'Karim Traders', supplier_invoice_no: null, purchase_date: '2026-11-05',
  location_id: 'loc1', location_name: 'Main store', status: 'received', payment_status: 'unpaid', subtotal: '0', item_discount_total: '0',
  bill_discount_amount: '0', tax_total: '0', shipping_cost: '0', other_cost: '0', grand_total: '0', received_total: '0', paid_total: '0',
  returned_total: '0', due_total: '0', note: null, item_count: 2, expected_date: null, ordered_at: null, approved_at: null, approved_by_name: null,
  posted_at: '2026-11-05T05:00:00Z', closed_at: null, cancelled_at: null, cancel_reason: null, created_by_name: 'Rahim Uddin', version: 1,
  created_at: '2026-11-05T05:00:00Z',
  // 3 soaps in stock (one returned); 1.5 kg sugar → 2 labels.
  items: [item('i1', 'v1', 'Lux Soap', '4.000', '4.000', '1.000'), item('i2', 'v2', 'Sugar', '1.500', '1.500')],
  receipts: [], payments: [], returns: [],
}

const soap = { variant_id: 'v1', product_id: 'p1', product_name: 'Lux Soap', product_name_bn: null, variant_name: null, sku: 'P000001', barcode: '8901030865275', selling_price: '55.0000', unit_short_name: 'pcs' }
const sugar = { variant_id: 'v2', product_id: 'p2', product_name: 'Sugar', product_name_bn: null, variant_name: null, sku: 'P000002', barcode: null, selling_price: '140.0000', unit_short_name: 'kg' }

describe('barcode labels', () => {
  it('pages sheets and skips used positions', () => {
    const sheet = layoutById('a4-24')
    expect(paginate([1, 2, 3], sheet, 22).map((p) => p.length)).toEqual([24, 1])
    expect(paginate([1, 2, 3], sheet, 22)[0]!.slice(22)).toEqual([1, 2])
    // Rolls ignore skip: one label per page.
    expect(paginate([1, 2], layoutById('roll-38x25'), 5)).toEqual([[1], [2]])
  })

  it('labels a purchase: copies from stock received, generates missing barcodes', async () => {
    const calls = mockApi({
      ...signedIn(),
      'GET /api/v1/purchases/pur1': purchase,
      'POST /api/v1/labels': [soap, sugar],
      'POST /api/v1/labels/generate-barcodes': [soap, { ...sugar, barcode: '2000000000015' }],
    })
    renderRoute('/labels?purchase=pur1')
    expect(await screen.findByRole('heading', { name: 'Barcode labels' })).toBeInTheDocument()
    expect(calls.find((c) => c.path === '/api/v1/labels')?.body).toEqual({ variant_ids: ['v1', 'v2'] })
    expect(screen.getByLabelText('Labels for Lux Soap')).toHaveValue(3)
    expect(screen.getByLabelText('Labels for Sugar')).toHaveValue(2)
    // Sugar has no barcode: its SKU prints until one is generated.
    expect(screen.getByText('No barcode: prints the SKU')).toBeInTheDocument()
    expect(screen.getAllByTestId('label')).toHaveLength(5)
    expect(screen.getByRole('button', { name: 'Print 5 labels' })).toBeEnabled()

    await userEvent.click(screen.getByRole('button', { name: 'Generate barcode' }))
    await waitFor(() => expect(screen.queryByText('No barcode: prints the SKU')).not.toBeInTheDocument())
    expect(calls.find((c) => c.path === '/api/v1/labels/generate-barcodes')?.body).toEqual({ variant_ids: ['v2'] })
    const labels = screen.getAllByTestId('label')
    expect(within(labels[4]!).getByRole('img', { name: '2000000000015' })).toHaveAttribute('data-symbology', 'ean13')

    const soapCopies = screen.getByLabelText('Labels for Lux Soap')
    await userEvent.clear(soapCopies)
    await userEvent.type(soapCopies, '1')
    expect(screen.getAllByTestId('label')).toHaveLength(3)

    await userEvent.click(screen.getByRole('button', { name: 'Remove Lux Soap' }))
    expect(screen.getAllByTestId('label')).toHaveLength(2)
  })

  it('leaves out products without a barcode when the SKU fallback is off', async () => {
    mockApi({ ...signedIn(['dashboard.view', 'products.view']), 'POST /api/v1/labels': [soap, sugar] })
    renderRoute('/labels?variants=v1,v2')
    await userEvent.click(await screen.findByLabelText('Use the SKU when there is no barcode'))
    expect(screen.getByText('No barcode: skipped')).toBeInTheDocument()
    expect(screen.getAllByTestId('label')).toHaveLength(1)
    // Without products.update there is no generate button.
    expect(screen.queryByRole('button', { name: /Generate/ })).not.toBeInTheDocument()
  })

  it('opens from the product list with that product', async () => {
    const calls = mockApi({
      ...signedIn(['dashboard.view', 'products.view']),
      'GET /api/v1/products': { data: [{ id: 'p1', variant_id: 'v1', name: 'Lux Soap', name_bn: null, sku: 'P000001', category_name: null, selling_price: '55.0000', unit_short_name: 'pcs', status: 'active', barcodes: [] }], next_cursor: null },
      'GET /api/v1/categories': [],
      'GET /api/v1/brands': [],
      'POST /api/v1/labels': [soap],
    })
    renderRoute('/products')
    await userEvent.click(await screen.findByRole('link', { name: 'Print barcode for Lux Soap' }))
    expect(await screen.findByLabelText('Labels for Lux Soap')).toHaveValue(1)
    expect(calls.find((c) => c.path === '/api/v1/labels')?.body).toEqual({ variant_ids: ['v1'] })
    expect(screen.getByRole('link', { name: '← Products' })).toHaveAttribute('href', '/products')
  })
})
