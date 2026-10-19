import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { currentTenant, membership, mockApi, token, user } from '../../test/api'
import { renderRoute } from '../../test/render'

afterEach(() => vi.restoreAllMocks())

const account = (id: string, name: string, type: string, balance: string, isDefault = false) => ({
  id, account_type: type, name, account_number: null, bank_name: null, opening_balance: '0.00', current_balance: balance,
  ledger_account_id: null, is_default: isDefault, status: 'active', version: 1, created_at: '2026-10-01T00:00:00Z',
})

function signedIn(perms: string[]) {
  return {
    'POST /api/v1/auth/refresh': token('t1'),
    'GET /api/v1/auth/me': { user, memberships: [membership('t1', 'Rahim Store')], tenant: currentTenant('t1', 'Rahim Store', perms) },
    'GET /api/v1/payment-accounts': [account('a-cash', 'Cash drawer', 'cash', '15000.00', true), account('a-bkash', 'bKash', 'mobile_wallet', '2000.00')],
    'GET /api/v1/account-transfers': { data: [], next_cursor: null },
    'GET /api/v1/payment-methods': [],
  }
}

describe('money', () => {
  it('transfers money between accounts with an Idempotency-Key', async () => {
    const calls = mockApi({
      ...signedIn(['dashboard.view', 'accounts.view', 'transfers.create']),
      'POST /api/v1/account-transfers': () => Response.json({ id: 'tr1', transfer_no: 'TRF-0001' }, { status: 201 }),
    })
    renderRoute('/accounts')
    const table = await screen.findByRole('table', { name: 'Cash & bank accounts' })
    expect(within(table).getByText(/15,000\.00/)).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Transfer money' }))
    const dialog = await screen.findByRole('dialog')
    await userEvent.type(within(dialog).getByLabelText('Amount'), '5000')
    await userEvent.type(within(dialog).getByLabelText(/^Fee/), '10')
    await userEvent.click(within(dialog).getByRole('button', { name: 'Transfer money' }))
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument())
    const post = calls.find((c) => c.method === 'POST' && c.path === '/api/v1/account-transfers')
    expect(post?.body).toEqual({ from_account_id: 'a-cash', to_account_id: 'a-bkash', amount: '5000', fee_amount: '10' })
    expect(post?.headers.get('Idempotency-Key')).toBeTruthy()
  })

  it('hides balances without accounts.view', async () => {
    mockApi(signedIn(['dashboard.view', 'transfers.create']))
    renderRoute('/accounts')
    const table = await screen.findByRole('table', { name: 'Cash & bank accounts' })
    expect(within(table).queryByText(/15,000\.00/)).not.toBeInTheDocument()
    expect(screen.queryByText('Recent transfers')).not.toBeInTheDocument()
  })
})
