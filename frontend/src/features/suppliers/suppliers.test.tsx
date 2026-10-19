import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { currentTenant, membership, mockApi, problem, token, user } from '../../test/api'
import { renderRoute } from '../../test/render'

afterEach(() => vi.restoreAllMocks())

const perms = ['dashboard.view', 'suppliers.view', 'suppliers.manage', 'suppliers.pay', 'purchases.view']

function signedIn(permissions = perms) {
  return {
    'POST /api/v1/auth/refresh': token('t1'),
    'GET /api/v1/auth/me': { user, memberships: [membership('t1', 'Rahim Store')], tenant: currentTenant('t1', 'Rahim Store', permissions) },
  }
}

const karim = {
  id: 's1', code: 'S0001', name: 'Karim Traders', contact_person: 'Karim', phone: '01711000002', email: null, address: 'Chawkbazar',
  opening_balance: '1000.0000', current_balance: '3500.0000', notes: null, status: 'active', version: 2,
  created_at: '2026-10-01T00:00:00Z', updated_at: '2026-10-01T00:00:00Z',
}

describe('suppliers', () => {
  it('lists suppliers with the total payable', async () => {
    mockApi({ ...signedIn(), 'GET /api/v1/suppliers': { data: [karim], next_cursor: null, total_payable: '3500.0000' } })
    renderRoute('/suppliers')
    const table = await screen.findByRole('table')
    expect(within(table).getByRole('link', { name: 'Karim Traders' })).toHaveAttribute('href', '/suppliers/s1')
    expect(within(table).getByText('Payable')).toBeInTheDocument()
    expect(screen.getByText('Total payable').parentElement).toHaveTextContent('3,500.00')
  })

  it('creates a supplier with an opening balance', async () => {
    const calls = mockApi({
      ...signedIn(),
      'POST /api/v1/suppliers': (call) => Response.json({ ...karim, id: 's2', ...(call.body as object) }, { status: 201 }),
      'GET /api/v1/suppliers/s2': { ...karim, id: 's2' },
      'GET /api/v1/suppliers/s2/statement': { party_type: 'supplier', party_id: 's2', party_name: 'Karim Traders', from: null, to: null, opening_balance: '0', entries: [], total_debit: '0', total_credit: '0', closing_balance: '0' },
    })
    const { router } = renderRoute('/suppliers/new')
    await userEvent.type(await screen.findByLabelText('Name'), 'Karim Traders')
    await userEvent.type(screen.getByLabelText('Phone'), '01711000002')
    await userEvent.type(screen.getByLabelText('Opening balance'), '1000')
    await userEvent.click(screen.getByRole('button', { name: 'Save' }))
    await waitFor(() => expect(router.state.location.pathname).toBe('/suppliers/s2'))
    expect(calls.find((c) => c.method === 'POST' && c.path === '/api/v1/suppliers')?.body).toEqual({ name: 'Karim Traders', phone: '01711000002', opening_balance: '1000' })
  })

  it('shows the statement with opening, running and closing balances', async () => {
    mockApi({
      ...signedIn(),
      'GET /api/v1/suppliers/s1': karim,
      'GET /api/v1/suppliers/s1/statement': {
        party_type: 'supplier', party_id: 's1', party_name: 'Karim Traders', from: null, to: null, opening_balance: '1000.0000',
        entries: [
          { id: 'e1', business_date: '2026-10-05', created_at: '', entry_type: 'purchase', reference_type: 'purchase', reference_id: 'pur1', reference_no: 'PUR-0001', note: null, debit: '4250.0000', credit: '0', balance: '5250.0000' },
          { id: 'e2', business_date: '2026-10-06', created_at: '', entry_type: 'payment_out', reference_type: 'payment', reference_id: 'pay1', reference_no: 'PAY-0001', note: null, debit: '0', credit: '1750.0000', balance: '3500.0000' },
        ],
        total_debit: '4250.0000', total_credit: '1750.0000', closing_balance: '3500.0000',
      },
    })
    renderRoute('/suppliers/s1')
    const table = await screen.findByRole('table')
    const rows = within(table).getAllByRole('row')
    expect(rows[1]).toHaveTextContent(/Opening balance.*1,000\.00/)
    expect(rows[2]).toHaveTextContent(/Purchase.*PUR-0001.*4,250\.00.*5,250\.00/)
    expect(within(rows[2]!).getByRole('link', { name: 'PUR-0001' })).toHaveAttribute('href', '/purchases/pur1')
    expect(rows[3]).toHaveTextContent(/Payment.*1,750\.00.*3,500\.00/)
    expect(rows[4]).toHaveTextContent(/Closing balance.*3,500\.00/)
  })

  it('suggests deactivating when a supplier with transactions is deleted', async () => {
    mockApi({
      ...signedIn(),
      'GET /api/v1/suppliers/s1': karim,
      'GET /api/v1/suppliers/s1/statement': { party_type: 'supplier', party_id: 's1', party_name: 'Karim Traders', from: null, to: null, opening_balance: '0', entries: [], total_debit: '0', total_credit: '0', closing_balance: '0' },
      'DELETE /api/v1/suppliers/s1': () => problem(409, 'IN_USE'),
    })
    renderRoute('/suppliers/s1')
    await userEvent.click(await screen.findByRole('button', { name: 'Delete' }))
    await userEvent.click(screen.getByRole('button', { name: 'Yes' }))
    expect(await screen.findByText(/Deactivate it instead/)).toBeInTheDocument()
  })
})
