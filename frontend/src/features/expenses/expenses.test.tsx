import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { currentTenant, membership, mockApi, token, user } from '../../test/api'
import { renderRoute } from '../../test/render'

afterEach(() => vi.restoreAllMocks())

const perms = ['dashboard.view', 'expenses.view', 'expenses.create', 'expenses.approve']

const rent = {
  id: 'e1', expense_no: 'EXP-0001', business_date: '2026-10-01', category_id: 'cat-rent', category_name: 'Shop rent', amount: '12000.00',
  tax_amount: '0.00', account_id: 'a-cash', account_name: 'Cash drawer', payment_id: 'pay1', payee: 'Landlord', reference: null, note: null,
  status: 'posted', void_reason: null, created_by_name: 'Rahim Uddin', created_at: '2026-10-01T05:00:00Z',
}

function signedIn() {
  return {
    'POST /api/v1/auth/refresh': token('t1'),
    'GET /api/v1/auth/me': { user, memberships: [membership('t1', 'Rahim Store')], tenant: currentTenant('t1', 'Rahim Store', perms) },
    'GET /api/v1/expenses': { data: [rent], next_cursor: null, total_amount: '12000.00' },
    'GET /api/v1/expense-categories': [
      { id: 'cat-rent', name: 'Shop rent', ledger_account_id: 'l1', ledger_code: '6100', is_system: true, status: 'active', expense_count: 1 },
      { id: 'cat-power', name: 'Electricity', ledger_account_id: 'l2', ledger_code: '6110', is_system: false, status: 'active', expense_count: 0 },
    ],
    'GET /api/v1/payment-accounts': [
      { id: 'a-cash', account_type: 'cash', name: 'Cash drawer', account_number: null, bank_name: null, opening_balance: '0', current_balance: '0', ledger_account_id: null, is_default: true, status: 'active', version: 1, created_at: '' },
    ],
  }
}

describe('expenses', () => {
  it('records an expense from the dialog', async () => {
    const calls = mockApi({ ...signedIn(), 'POST /api/v1/expenses': () => Response.json({ ...rent, id: 'e2' }, { status: 201 }) })
    renderRoute('/expenses')
    const table = await screen.findByRole('table')
    expect(within(table).getByText('EXP-0001')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'New expense' }))
    const dialog = await screen.findByRole('dialog')
    await userEvent.selectOptions(within(dialog).getByLabelText('Category'), 'cat-power')
    await userEvent.type(within(dialog).getByLabelText('Amount'), '1500')
    await userEvent.type(within(dialog).getByLabelText(/^VAT/), '75')
    await userEvent.type(within(dialog).getByLabelText(/^Paid to/), 'DESCO')
    await userEvent.click(within(dialog).getByRole('button', { name: 'Save expense' }))
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument())
    const post = calls.find((c) => c.method === 'POST' && c.path === '/api/v1/expenses')
    expect(post?.body).toEqual({ category_id: 'cat-power', amount: '1500', tax_amount: '75', payee: 'DESCO' })
    expect(post?.headers.get('Idempotency-Key')).toBeTruthy()
  })

  it('voids an expense with a reason', async () => {
    const calls = mockApi({ ...signedIn(), 'POST /api/v1/expenses/e1/void': { ...rent, status: 'voided', void_reason: 'Duplicate' } })
    renderRoute('/expenses')
    await userEvent.click(await screen.findByRole('button', { name: 'Void expense EXP-0001' }))
    const dialog = await screen.findByRole('dialog')
    await userEvent.type(within(dialog).getByLabelText('Reason'), 'Duplicate')
    await userEvent.click(within(dialog).getByRole('button', { name: 'Void' }))
    await waitFor(() => expect(calls.find((c) => c.path === '/api/v1/expenses/e1/void')?.body).toEqual({ reason: 'Duplicate' }))
  })
})
