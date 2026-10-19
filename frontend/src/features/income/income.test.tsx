import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { currentTenant, membership, mockApi, token, user } from '../../test/api'
import { renderRoute } from '../../test/render'

afterEach(() => vi.restoreAllMocks())

const perms = ['dashboard.view', 'income.create', 'payments.void']

const interest = {
  id: 'i1', income_no: 'INC-0001', business_date: '2026-10-05', category_id: 'cat-int', category_name: 'Bank interest', amount: '250.00',
  account_id: 'a-cash', account_name: 'Cash drawer', payment_id: 'pay1', source: 'City Bank', reference: null, note: null,
  status: 'posted', void_reason: null, created_by_name: 'Rahim Uddin', created_at: '2026-10-05T05:00:00Z',
}

function signedIn() {
  return {
    'POST /api/v1/auth/refresh': token('t1'),
    'GET /api/v1/auth/me': { user, memberships: [membership('t1', 'Rahim Store')], tenant: currentTenant('t1', 'Rahim Store', perms) },
    'GET /api/v1/incomes': { data: [interest], next_cursor: null, total_amount: '250.00' },
    'GET /api/v1/income-categories': [
      { id: 'cat-int', name: 'Bank interest', is_system: true, status: 'active', income_count: 1 },
      { id: 'cat-scrap', name: 'Scrap sales', is_system: false, status: 'active', income_count: 0 },
    ],
    'GET /api/v1/payment-accounts': [
      { id: 'a-cash', account_type: 'cash', name: 'Cash drawer', account_number: null, bank_name: null, opening_balance: '0', current_balance: '0', ledger_account_id: null, is_default: true, status: 'active', version: 1, created_at: '' },
    ],
  }
}

describe('other income', () => {
  it('lists income with its total and records new income', async () => {
    const calls = mockApi({ ...signedIn(), 'POST /api/v1/incomes': () => Response.json({ ...interest, id: 'i2' }, { status: 201 }) })
    renderRoute('/incomes')
    const table = await screen.findByRole('table')
    expect(within(table).getByText('INC-0001')).toBeInTheDocument()
    expect(screen.getByText('Total income')).toBeInTheDocument()
    expect(calls.find((c) => c.path === '/api/v1/incomes')?.search).toContain('status=posted')

    await userEvent.click(screen.getByRole('button', { name: 'Record income' }))
    const dialog = await screen.findByRole('dialog')
    expect(within(dialog).getByLabelText('Received into')).toHaveDisplayValue('Cash drawer (default)')
    await userEvent.click(within(dialog).getByRole('button', { name: 'Save income' }))
    expect(within(dialog).getByText('Choose a category.')).toBeInTheDocument()
    await userEvent.selectOptions(within(dialog).getByLabelText('Category'), 'cat-scrap')
    await userEvent.type(within(dialog).getByLabelText('Amount'), '800')
    await userEvent.type(within(dialog).getByLabelText(/^Received from/), 'Bhangari shop')
    await userEvent.click(within(dialog).getByRole('button', { name: 'Save income' }))
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument())
    const post = calls.find((c) => c.method === 'POST' && c.path === '/api/v1/incomes')
    expect(post?.body).toEqual({ category_id: 'cat-scrap', amount: '800', source: 'Bhangari shop' })
    expect(post?.headers.get('Idempotency-Key')).toBeTruthy()
  })

  it('voids income with a reason', async () => {
    const calls = mockApi({ ...signedIn(), 'POST /api/v1/incomes/i1/void': { ...interest, status: 'voided', void_reason: 'Entered twice' } })
    renderRoute('/incomes')
    await userEvent.click(await screen.findByRole('button', { name: 'Void income INC-0001' }))
    const dialog = await screen.findByRole('dialog')
    await userEvent.type(within(dialog).getByLabelText('Reason'), 'Entered twice')
    await userEvent.click(within(dialog).getByRole('button', { name: 'Void' }))
    await waitFor(() => expect(calls.find((c) => c.path === '/api/v1/incomes/i1/void')?.body).toEqual({ reason: 'Entered twice' }))
  })

  it('hides recording and voiding without the permissions', async () => {
    mockApi({
      ...signedIn(),
      'GET /api/v1/auth/me': { user, memberships: [membership('t1', 'Rahim Store')], tenant: currentTenant('t1', 'Rahim Store', ['expenses.view']) },
    })
    renderRoute('/incomes')
    await screen.findByRole('table')
    expect(screen.queryByRole('button', { name: 'Record income' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /Void income/ })).not.toBeInTheDocument()
  })
})
