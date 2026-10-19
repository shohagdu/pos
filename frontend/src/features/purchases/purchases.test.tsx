import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { toFixed } from '../../lib/decimal'
import { currentTenant, membership, mockApi, token, user } from '../../test/api'
import { renderRoute } from '../../test/render'
import { costBill } from './costing'

afterEach(() => vi.restoreAllMocks())

const perms = ['dashboard.view', 'purchases.view', 'purchases.create', 'purchases.post', 'purchases.return', 'suppliers.view', 'suppliers.pay', 'products.view']

function signedIn(permissions = perms) {
  return {
    'POST /api/v1/auth/refresh': token('t1'),
    'GET /api/v1/auth/me': { user, memberships: [membership('t1', 'Rahim Store')], tenant: currentTenant('t1', 'Rahim Store', permissions) },
    'GET /api/v1/stock-locations': [{ id: 'loc1', branch_id: 'b1', code: 'MAIN', name: 'Main store', location_type: 'store', is_sellable: true, status: 'active' }],
    'GET /api/v1/payment-methods': [
      { id: 'pm-cash', name: 'Cash', method_type: 'cash', default_account_id: 'acc-cash', default_account_name: 'Cash drawer', requires_reference: false, sort_order: 0, is_active: true },
    ],
    'GET /api/v1/payment-accounts': [{ id: 'acc-cash', account_type: 'cash', name: 'Cash drawer', status: 'active' }],
  }
}

const supplier = { id: 's1', code: 'S0001', name: 'Karim Traders', contact_person: null, phone: '01711000002', email: null, address: null, opening_balance: '0', current_balance: '0', notes: null, status: 'active', version: 1, created_at: '', updated_at: '' }
const rice = {
  id: 'p1', variant_id: 'v1', product_type: 'standard', name: 'Miniket Rice 5kg', name_bn: null, sku: 'P000001', barcodes: [], unit_short_name: 'pcs',
  allow_fraction: false, tax_rate_pct: null, track_inventory: true, purchase_price: '400.0000', selling_price: '450.0000', status: 'active',
}

const draft = {
  id: 'pur1', purchase_no: null, supplier_id: 's1', supplier_name: 'Karim Traders', supplier_invoice_no: 'KT-77', purchase_date: '2026-10-18',
  location_id: 'loc1', location_name: 'Main store', status: 'draft', payment_status: 'unpaid', subtotal: '4200.00', item_discount_total: '0.00',
  bill_discount_amount: '0.00', tax_total: '0.00', shipping_cost: '50.00', other_cost: '0.00', grand_total: '4250.00', paid_total: '0.00',
  returned_total: '0.00', due_total: '4250.00', note: null, item_count: 1, posted_at: null, cancelled_at: null, cancel_reason: null,
  created_by_name: 'Rahim Uddin', version: 1, created_at: '2026-10-18T05:00:00Z',
  items: [{ id: 'i1', line_no: 1, variant_id: 'v1', product_name: 'Miniket Rice 5kg', sku: 'P000001', unit_short_name: 'pcs', quantity: '10.000',
    received_quantity: '0.000', returned_quantity: '0.000', unit_cost: '420.0000', discount_type: null, discount_value: null, line_discount: '0.00',
    bill_discount_alloc: '0.00', tax_rate_pct: '0.0000', tax_amount: '0.00', charges_alloc: '50.00', line_total: '4200.00', landed_unit_cost: '425.0000', previous_unit_cost: '400.0000' }],
  payments: [],
  returns: [],
}

describe('purchases', () => {
  it('computes landed cost like the server (§8.4)', () => {
    const line = (quantity: string, unit_cost: string) => ({ quantity, unit_cost, discount_type: 'amount' as const, discount_value: '', tax_rate_pct: '0' })
    const bill = costBill([line('2', '100'), line('1', '100')], '30', '9', '0', false)
    expect(bill.lines.map((l) => toFixed(l.bill_discount))).toEqual(['20.00', '10.00'])
    expect(bill.lines.map((l) => toFixed(l.charges))).toEqual(['6.00', '3.00'])
    expect(toFixed(bill.lines[0]!.landed_unit_cost, 4)).toBe('93.0000')
    expect(toFixed(bill.grand_total)).toBe('279.00')
    const vat = { ...line('4', '25'), tax_rate_pct: '15' }
    expect(toFixed(costBill([vat], '', '', '', false).lines[0]!.landed_unit_cost, 4)).toBe('28.7500')
    expect(toFixed(costBill([vat], '', '', '', true).lines[0]!.landed_unit_cost, 4)).toBe('25.0000')
  })

  it('saves a draft with the lines, shows the last price and variance', async () => {
    const calls = mockApi({
      ...signedIn(),
      'GET /api/v1/suppliers': { data: [supplier], next_cursor: null, total_payable: '0' },
      'GET /api/v1/products': { data: [rice], next_cursor: null },
      'GET /api/v1/purchases/last-prices': [
        { variant_id: 'v1', last_purchase_price: '400.0000', last_purchase_date: '2026-10-01', last_purchase_qty: '20.000', last_supplier_name: 'Karim Traders', purchase_price: '400.0000' },
      ],
      'POST /api/v1/purchases': () => Response.json(draft, { status: 201 }),
      'GET /api/v1/purchases/pur1': draft,
    })
    const { router } = renderRoute('/purchases/new')
    await screen.findByRole('option', { name: /Karim Traders/ })
    await userEvent.selectOptions(screen.getByLabelText('Supplier'), 's1')
    await userEvent.type(screen.getByLabelText('Supplier bill no (optional)'), 'KT-77')
    await userEvent.type(screen.getByLabelText('Add product'), 'rice{Enter}')
    await userEvent.click(await screen.findByRole('button', { name: 'Add Miniket Rice 5kg' }))

    const qty = screen.getByLabelText('Qty Miniket Rice 5kg')
    await userEvent.clear(qty)
    await userEvent.type(qty, '10')
    const cost = screen.getByLabelText('Unit cost Miniket Rice 5kg')
    await userEvent.clear(cost)
    await userEvent.type(cost, '420')
    await userEvent.type(screen.getByLabelText('Shipping'), '50')

    expect(await screen.findByText(/Last: .*400\.00/)).toBeInTheDocument()
    expect(screen.getByText(/\+5\.0%/)).toBeInTheDocument()
    const totals = screen.getByLabelText('Totals')
    expect(within(totals).getByText(/4,250\.00/)).toBeInTheDocument()
    expect(screen.getByText(/425\.00/)).toBeInTheDocument()

    await userEvent.click(screen.getByRole('button', { name: 'Save draft' }))
    await waitFor(() => expect(router.state.location.pathname).toBe('/purchases/pur1'))
    expect(calls.find((c) => c.method === 'POST' && c.path === '/api/v1/purchases')?.body).toEqual({
      supplier_id: 's1',
      supplier_invoice_no: 'KT-77',
      purchase_date: expect.stringMatching(/^\d{4}-\d{2}-\d{2}$/),
      location_id: 'loc1',
      lines: [{ variant_id: 'v1', quantity: '10', unit_cost: '420', tax_rate_pct: '0' }],
      shipping_cost: '50',
    })
  })

  it('posts a draft with a payment and an Idempotency-Key', async () => {
    const calls = mockApi({
      ...signedIn(),
      'GET /api/v1/purchases/pur1': draft,
      'POST /api/v1/purchases/pur1/post': () => Response.json({ ...draft, purchase_no: 'PUR-0001', status: 'received', paid_total: '4250.00', due_total: '0.00', payment_status: 'paid' }),
    })
    renderRoute('/purchases/pur1')
    await userEvent.click(await screen.findByRole('button', { name: 'Post bill' }))
    const dialog = screen.getByRole('dialog', { name: 'Post bill' })
    await userEvent.click(within(dialog).getByLabelText('Record a payment now'))
    await within(dialog).findByRole('option', { name: 'Cash' })
    await userEvent.selectOptions(within(dialog).getByLabelText('Payment method'), 'pm-cash')
    expect(within(dialog).getByLabelText('Amount')).toHaveValue('4250')
    await userEvent.click(within(dialog).getByRole('button', { name: 'Post' }))

    await waitFor(() => expect(calls.some((c) => c.method === 'POST' && c.path === '/api/v1/purchases/pur1/post')).toBe(true))
    const post = calls.find((c) => c.method === 'POST' && c.path === '/api/v1/purchases/pur1/post')
    expect(post?.headers.get('Idempotency-Key')).toMatch(/^[0-9a-f-]{36}$/)
    expect(post?.body).toEqual({ payment: { method_id: 'pm-cash', amount: '4250', account_id: 'acc-cash' } })
  })
})
