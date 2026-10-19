import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { currentTenant, membership, mockApi, problem, token, user } from '../../test/api'
import { renderRoute } from '../../test/render'

afterEach(() => vi.restoreAllMocks())

const perms = ['dashboard.view', 'customers.view', 'customers.manage', 'customers.collect_payment', 'accounts.view']

const karim = {
  id: 'c1', code: 'C00001', name: 'Karim Mia', phone: '01711000002', email: null, address: null, credit_limit: '5000.00',
  opening_balance: '0.00', current_balance: '250.00', is_walk_in: false, notes: null, status: 'active', version: 2,
  created_at: '2026-10-01T00:00:00Z', updated_at: '2026-10-01T00:00:00Z',
}

function signedIn() {
  return {
    'POST /api/v1/auth/refresh': token('t1'),
    'GET /api/v1/auth/me': { user, memberships: [membership('t1', 'Rahim Store')], tenant: currentTenant('t1', 'Rahim Store', perms) },
    'GET /api/v1/customers/c1': karim,
    'GET /api/v1/customers/c1/statement': {
      party_type: 'customer', party_id: 'c1', party_name: 'Karim Mia', from: null, to: null, opening_balance: '0.00', entries: [],
      total_debit: '0.00', total_credit: '0.00', closing_balance: '250.00',
    },
    'GET /api/v1/payment-methods': [
      { id: 'pm-cash', name: 'Cash', method_type: 'cash', default_account_id: 'a-cash', default_account_name: 'Cash drawer', requires_reference: false, sort_order: 0, is_active: true },
      { id: 'pm-credit', name: 'Due', method_type: 'customer_credit', default_account_id: null, default_account_name: null, requires_reference: false, sort_order: 9, is_active: true },
    ],
    'GET /api/v1/payment-accounts': [
      { id: 'a-cash', account_type: 'cash', name: 'Cash drawer', account_number: null, bank_name: null, opening_balance: '0', current_balance: '0', ledger_account_id: null, is_default: true, status: 'active', version: 1, created_at: '' },
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
        Response.json(
          { id: 'pay1', payment_no: 'RCV-0001', amount: '250.00', unallocated_amount: '0.00', allocations: [] },
          { status: 201 },
        ),
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

  it('suggests deactivating when a customer with transactions is deleted', async () => {
    mockApi({ ...signedIn(), 'DELETE /api/v1/customers/c1': () => problem(409, 'IN_USE') })
    renderRoute('/customers/c1')
    await userEvent.click(await screen.findByRole('button', { name: 'Delete' }))
    await userEvent.click(screen.getByRole('button', { name: 'Yes' }))
    await waitFor(() => expect(screen.getByText(/Deactivate the customer instead/)).toBeInTheDocument())
  })
})
