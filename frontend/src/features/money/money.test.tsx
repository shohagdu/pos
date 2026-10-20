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

  it('reconciles a wallet: ticks lines, then completes at a zero difference', async () => {
    const line = (id: string, amount: string, txn_type: string, cleared = false) => ({
      id, business_date: '2026-10-10', txn_type, amount, reference_type: 'account_transfer', reference_id: 'tr1', reference_no: `REF-${id}`, note: null, cleared,
    })
    const rec = (cleared: string[], status = 'in_progress') => {
      const lines = [line('l1', '1000.00', 'transfer_in', cleared.includes('l1')), line('l2', '-15.00', 'expense', cleared.includes('l2'))]
      const ticked = lines.filter((l) => l.cleared).reduce((s, l) => s + Number(l.amount), 0)
      return {
        id: 'rec1', account_id: 'a-bkash', account_name: 'bKash', statement_date: '2026-10-15', statement_balance: '985.00', opening_balance: '0.00',
        cleared_balance: ticked.toFixed(2), status, cleared_count: cleared.length, note: null, created_by_name: 'Rahim Uddin',
        created_at: '2026-10-15T05:00:00Z', completed_by_name: status === 'completed' ? 'Rahim Uddin' : null,
        completed_at: status === 'completed' ? '2026-10-15T06:00:00Z' : null, difference: (985 - ticked).toFixed(2), cleared_in: '0.00',
        cleared_out: '0.00', book_balance: '985.00', lines,
      }
    }
    let current = rec([])
    const calls = mockApi({
      ...signedIn(['dashboard.view', 'accounts.view', 'accounts.manage']),
      'GET /api/v1/bank-reconciliations': [],
      'POST /api/v1/bank-reconciliations': () => Response.json((current = rec([])), { status: 201 }),
      'GET /api/v1/bank-reconciliations/rec1': () => Response.json(current),
      'PUT /api/v1/bank-reconciliations/rec1/lines': (call) => {
        const body = call.body as { clear?: string[] }
        return Response.json((current = rec([...current.lines.filter((l) => l.cleared).map((l) => l.id), ...(body.clear ?? [])])))
      },
      'POST /api/v1/bank-reconciliations/rec1/complete': () => Response.json((current = rec(['l1', 'l2'], 'completed'))),
    })
    renderRoute('/accounts/reconciliations')
    await userEvent.click(await screen.findByRole('button', { name: 'Start reconciliation' }))
    const dialog = await screen.findByRole('dialog')
    await userEvent.selectOptions(within(dialog).getByLabelText('Account'), 'a-bkash')
    await userEvent.type(within(dialog).getByLabelText('Statement closing balance'), '985')
    await userEvent.click(within(dialog).getByRole('button', { name: 'Start reconciliation' }))

    const complete = await screen.findByRole('button', { name: 'Complete' })
    expect(complete).toBeDisabled()
    const start = calls.find((c) => c.method === 'POST' && c.path === '/api/v1/bank-reconciliations')
    expect(start?.body).toMatchObject({ account_id: 'a-bkash', statement_balance: '985' })
    await userEvent.click(screen.getByRole('checkbox', { name: /REF-l1/ }))
    await waitFor(() => expect(screen.getByRole('checkbox', { name: /REF-l1/ })).toBeChecked())
    expect(complete).toBeDisabled()
    await userEvent.click(screen.getByRole('checkbox', { name: /REF-l2/ }))
    await waitFor(() => expect(screen.getByRole('button', { name: 'Complete' })).toBeEnabled())
    expect(calls.filter((c) => c.method === 'PUT').map((c) => c.body)).toEqual([{ clear: ['l1'] }, { clear: ['l2'] }])
    await userEvent.click(screen.getByRole('button', { name: 'Complete' }))
    expect(await screen.findByText('Completed')).toBeInTheDocument()
    expect(screen.queryByRole('checkbox')).not.toBeInTheDocument()
  })
})
