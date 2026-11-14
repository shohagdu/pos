import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { currentTenant, membership, mockApi, problem, token, user } from '../../test/api'
import { renderRoute } from '../../test/render'
import { sale } from '../sales/fixtures'

afterEach(() => vi.restoreAllMocks())

const perms = ['dashboard.view', 'customers.view', 'customers.manage', 'customers.collect_payment', 'accounts.view']

const karim = {
  id: 'c1',
  code: 'C00001',
  name: 'Karim Mia',
  phone: '01711000002',
  email: null,
  address: null,
  credit_limit: '5000.00',
  opening_balance: '0.00',
  current_balance: '250.00',
  is_walk_in: false,
  notes: null,
  status: 'active',
  version: 2,
  created_at: '2026-10-01T00:00:00Z',
  updated_at: '2026-10-01T00:00:00Z',
}

function signedIn() {
  return {
    'POST /api/v1/auth/refresh': token('t1'),
    'GET /api/v1/auth/me': { user, memberships: [membership('t1', 'Rahim Store')], tenant: currentTenant('t1', 'Rahim Store', perms) },
    'GET /api/v1/customers/c1': karim,
    'GET /api/v1/customers/c1/statement': {
      party_type: 'customer',
      party_id: 'c1',
      party_name: 'Karim Mia',
      from: null,
      to: null,
      opening_balance: '0.00',
      entries: [],
      total_debit: '0.00',
      total_credit: '0.00',
      closing_balance: '250.00',
    },
    'GET /api/v1/payment-methods': [
      {
        id: 'pm-cash',
        name: 'Cash',
        method_type: 'cash',
        default_account_id: 'a-cash',
        default_account_name: 'Cash drawer',
        requires_reference: false,
        sort_order: 0,
        is_active: true,
      },
      {
        id: 'pm-credit',
        name: 'Due',
        method_type: 'customer_credit',
        default_account_id: null,
        default_account_name: null,
        requires_reference: false,
        sort_order: 9,
        is_active: true,
      },
    ],
    'GET /api/v1/payment-accounts': [
      {
        id: 'a-cash',
        account_type: 'cash',
        name: 'Cash drawer',
        account_number: null,
        bank_name: null,
        opening_balance: '0',
        current_balance: '0',
        ledger_account_id: null,
        is_default: true,
        status: 'active',
        version: 1,
        created_at: '',
      },
    ],
  }
}

describe('customers', () => {
  it('lists customers with their dues', async () => {
    mockApi({ ...signedIn(), 'GET /api/v1/customers': { data: [karim], next_cursor: null, total_due: '250.00' } })
    renderRoute('/customers')
    const table = await screen.findByRole('table')
    expect(within(table).getByRole('link', { name: 'Karim Mia' })).toHaveAttribute('href', '/customers/c1')
    expect(within(table).getByText('owes')).toBeInTheDocument()
    expect(screen.getByText('Total receivable')).toBeInTheDocument()
  })

  it('receives a payment through /collections', async () => {
    const calls = mockApi({
      ...signedIn(),
      'POST /api/v1/collections': () =>
        Response.json({ id: 'pay1', payment_no: 'RCV-0001', amount: '250.00', unallocated_amount: '0.00', allocations: [] }, { status: 201 }),
    })
    renderRoute('/customers/c1')
    await userEvent.click(await screen.findByRole('button', { name: 'Receive payment' }))
    const dialog = await screen.findByRole('dialog')
    expect(within(dialog).getByLabelText('Amount')).toHaveValue('250')
    expect(within(within(dialog).getByLabelText('Payment method')).queryByRole('option', { name: 'Due' })).not.toBeInTheDocument()
    await userEvent.click(within(dialog).getByRole('button', { name: 'Receive' }))
    expect(await screen.findByText(/Payment RCV-0001/)).toBeInTheDocument()
    const post = calls.find((c) => c.method === 'POST' && c.path === '/api/v1/collections')
    expect(post?.body).toEqual({ customer_id: 'c1', amount: '250', method_id: 'pm-cash' })
    expect(post?.headers.get('Idempotency-Key')).toBeTruthy()
  })

  it('shows the warranty of each invoice in the history', async () => {
    const invoice = { ...sale, customer_name: 'Karim Mia' }
    mockApi({
      ...signedIn(),
      'GET /api/v1/sales': {
        data: [
          { ...invoice, warranty_items: 1, warranty_until: '2099-01-18' },
          { ...invoice, id: 's2', invoice_no: 'INV-0002', warranty_items: 1, warranty_until: '2020-01-18' },
          { ...invoice, id: 's3', invoice_no: 'INV-0003' },
        ],
        next_cursor: null,
        summary: { count: 3, grand_total: '0', paid_total: '0', due_total: '0', returned_total: '0' },
      },
    })
    renderRoute('/customers/c1')
    await userEvent.click(await screen.findByRole('tab', { name: 'Invoices' }))
    const table = await screen.findByRole('table', { name: 'Invoices' })
    const row = (no: string) => within(table).getByRole('link', { name: no }).closest('tr')!
    expect(within(row('INV-0001')).getByText('Until 18 Jan 2099')).toBeInTheDocument()
    expect(within(row('INV-0002')).getByText('Expired 18 Jan 2020')).toHaveClass('text-danger')
    expect(within(row('INV-0003')).getByText('—')).toBeInTheDocument()
  })

  it('lists the items bought with a warranty and whether they are still covered', async () => {
    const item = {
      id: 'si1',
      sale_id: 's1',
      invoice_no: 'INV-0001',
      business_date: '2026-10-18',
      customer_id: 'c1',
      customer_name: 'Karim Mia',
      customer_phone: '01711000002',
      is_walk_in: false,
      variant_id: 'v1',
      product_name: 'Power Bank',
      sku: 'P000009',
      unit_short_name: 'pcs',
      quantity: '1.000',
      returned_quantity: '0.000',
      warranty_period: 3,
      warranty_unit: 'month',
      warranty_expires_on: '2027-01-18',
    }
    const calls = mockApi({
      ...signedIn(),
      'GET /api/v1/sale-warranties': {
        data: [
          item,
          { ...item, id: 'si2', product_name: 'Cable', warranty_period: 7, warranty_unit: 'day', warranty_expires_on: '2026-10-25' },
          { ...item, id: 'si3', product_name: 'Charger', warranty_expires_on: '2026-11-20' },
          { ...item, id: 'si4', product_name: 'Earphones', returned_quantity: '1.000' },
        ],
        next_cursor: null,
        today: '2026-11-07',
      },
    })
    renderRoute('/customers/c1')
    await userEvent.click(await screen.findByRole('tab', { name: 'Warranties' }))
    const table = await screen.findByRole('table', { name: 'Warranties' })
    const row = (name: string) => within(table).getByText(name).closest('tr')!
    expect(within(row('Power Bank')).getByRole('link', { name: 'INV-0001' })).toHaveAttribute('href', '/sales/s1')
    expect(within(row('Power Bank')).getByText('3 months')).toBeInTheDocument()
    expect(within(row('Power Bank')).getByText('18 Jan 2027')).toBeInTheDocument()
    expect(within(row('Power Bank')).getByText('72 days left')).toBeInTheDocument()
    expect(within(row('Cable')).getByText('Expired')).toBeInTheDocument()
    expect(within(row('Charger')).getByText('13 days left')).toBeInTheDocument()
    expect(within(row('Earphones')).getByText('Returned')).toBeInTheDocument()
    expect(calls.find((c) => c.path === '/api/v1/sale-warranties')?.search).toContain('customer_id=c1')

    await userEvent.selectOptions(screen.getByLabelText('Status'), 'expired')
    await waitFor(() => expect(calls.some((c) => c.path === '/api/v1/sale-warranties' && c.search.includes('status=expired'))).toBe(true))
  })

  it('suggests deactivating when a customer with transactions is deleted', async () => {
    mockApi({ ...signedIn(), 'DELETE /api/v1/customers/c1': () => problem(409, 'IN_USE') })
    renderRoute('/customers/c1')
    await userEvent.click(await screen.findByRole('button', { name: 'Delete' }))
    await userEvent.click(screen.getByRole('button', { name: 'Yes' }))
    await waitFor(() => expect(screen.getByText(/Deactivate the customer instead/)).toBeInTheDocument())
  })
})
