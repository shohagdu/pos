import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest'

import i18n from '../../i18n'
import accountingEn from '../accounting/i18n/en.json'
import reportsEn from '../reports/i18n/en.json'

import { currentTenant, membership, mockApi, problem, token, user } from '../../test/api'
import { renderRoute } from '../../test/render'

afterEach(() => vi.restoreAllMocks())

// Register this feature's strings explicitly (deep-merged with the base ones).
beforeAll(() => {
  i18n.addResourceBundle('en', 'translation', accountingEn, true, true)
  i18n.addResourceBundle('en', 'translation', reportsEn, true, true)
})

const accounts = [
  { id: 'a-assets', code: '1000', name: 'Assets', account_type: 'asset', normal_side: 'debit', parent_id: null, system_key: null, is_postable: false, status: 'active', version: 1, has_entries: false },
  { id: 'a-cash', code: '1010', name: 'Cash in hand', account_type: 'asset', normal_side: 'debit', parent_id: 'a-assets', system_key: 'cash', is_postable: true, status: 'active', version: 1, has_entries: true },
  { id: 'a-rent', code: '6100', name: 'Rent', account_type: 'expense', normal_side: 'debit', parent_id: null, system_key: null, is_postable: true, status: 'active', version: 3, has_entries: true },
]

const journal = {
  id: 'j1',
  entry_no: 'JV-0001',
  entry_date: '2026-10-18',
  source_type: 'manual',
  source_id: null,
  source_no: null,
  memo: 'October rent',
  total_debit: '500.25',
  total_credit: '500.25',
  status: 'posted',
  reversal_of_id: null,
  created_at: '2026-10-18T05:00:00Z',
  lines: [
    { account_id: 'a-rent', account_code: '6100', account_name: 'Rent', debit: '500.25', credit: '0', party_type: null, party_id: null, memo: null },
    { account_id: 'a-cash', account_code: '1010', account_name: 'Cash in hand', debit: '0', credit: '500.25', party_type: null, party_id: null, memo: null },
  ],
}

function signedIn(perms = ['accounting.view', 'accounting.journal_create']) {
  return {
    'POST /api/v1/auth/refresh': token('t1'),
    'GET /api/v1/auth/me': { user, memberships: [membership('t1', 'Rahim Store')], tenant: currentTenant('t1', 'Rahim Store', perms) },
    'GET /api/v1/ledger-accounts': accounts,
    'GET /api/v1/journals': { data: [], next_cursor: null },
  }
}

describe('accounting', () => {
  it('keeps Post disabled until the manual journal balances and posts the lines', async () => {
    const calls = mockApi({
      ...signedIn(),
      'POST /api/v1/journals': () => Response.json(journal, { status: 201 }),
      'GET /api/v1/journals/j1': journal,
    })
    const { router } = renderRoute('/accounting/journals')
    await userEvent.click(await screen.findByRole('button', { name: 'New journal' }))
    const dialog = await screen.findByRole('dialog', { name: 'New journal' })
    const post = within(dialog).getByRole('button', { name: 'Post journal' })
    expect(post).toBeDisabled()

    // Only postable accounts are offered.
    const account1 = within(dialog).getByLabelText('Account, line 1')
    await waitFor(() => expect(within(account1).getAllByRole('option')).toHaveLength(3))
    expect(within(account1).queryByRole('option', { name: /Assets/ })).not.toBeInTheDocument()

    await userEvent.type(within(dialog).getByLabelText('Memo'), 'October rent')
    await userEvent.selectOptions(account1, 'a-rent')
    await userEvent.type(within(dialog).getByLabelText('Debit, line 1'), '500.25')
    await userEvent.selectOptions(within(dialog).getByLabelText('Account, line 2'), 'a-cash')
    await userEvent.type(within(dialog).getByLabelText('Credit, line 2'), '500.2')
    expect(within(dialog).getByText(/Difference: .*0\.05/)).toBeInTheDocument()
    expect(post).toBeDisabled()

    await userEvent.type(within(dialog).getByLabelText('Credit, line 2'), '5')
    expect(within(dialog).getByText(/Balanced/)).toBeInTheDocument()
    expect(post).toBeEnabled()
    await userEvent.click(post)

    await waitFor(() => expect(router.state.location.pathname).toBe('/accounting/journals/j1'))
    const body = calls.find((c) => c.method === 'POST' && c.path === '/api/v1/journals')?.body as Record<string, unknown>
    expect(body).toMatchObject({
      memo: 'October rent',
      lines: [
        { account_id: 'a-rent', debit: '500.25' },
        { account_id: 'a-cash', credit: '500.25' },
      ],
    })
    expect(body.date).toMatch(/^\d{4}-\d{2}-\d{2}$/)
    expect(await screen.findByRole('heading', { name: 'Journal JV-0001' })).toBeInTheDocument()
  })

  it('shows a locked-period error from the server', async () => {
    mockApi({ ...signedIn(['accounting.view', 'accounting.period_lock']), 'GET /api/v1/fiscal-periods': [], 'POST /api/v1/fiscal-periods/lock': () => problem(422, 'PERIOD_NOT_ENDED') })
    renderRoute('/accounting/periods')
    await userEvent.click(await screen.findByRole('button', { name: 'Lock month' }))
    expect(await screen.findByText('Only months that have ended can be locked.')).toBeInTheDocument()
  })
  it('adds a sub-account under a group with a suggested code', async () => {
    const calls = mockApi({
      ...signedIn(['accounting.view', 'accounting.coa_manage']),
      'POST /api/v1/ledger-accounts': (call: { body: unknown }) =>
        Response.json({ id: 'a-new', system_key: null, status: 'active', version: 1, has_entries: false, ...(call.body as object) }, { status: 201 }),
    })
    renderRoute('/accounting/journals')
    await userEvent.click(await screen.findByRole('tab', { name: 'Chart of accounts' }))
    await userEvent.click(await screen.findByRole('button', { name: 'Add a sub-account under Assets' }))
    const dialog = screen.getByRole('dialog', { name: 'Add account' })
    expect(within(dialog).getByLabelText('Code')).toHaveValue('1020')
    expect(within(dialog).getByLabelText('Type')).toBeDisabled()
    await userEvent.type(within(dialog).getByLabelText('Name'), 'Petty cash')
    await userEvent.click(within(dialog).getByRole('button', { name: 'Save' }))
    await waitFor(() => expect(calls.some((c) => c.method === 'POST' && c.path === '/api/v1/ledger-accounts')).toBe(true))
    expect(calls.find((c) => c.method === 'POST' && c.path === '/api/v1/ledger-accounts')?.body).toEqual({
      code: '1020', name: 'Petty cash', account_type: 'asset', parent_id: 'a-assets', normal_side: 'debit', is_postable: true,
    })
  })

  it('renames a system account, with its code and type locked, and shows a stale-version error', async () => {
    const calls = mockApi({
      ...signedIn(['accounting.view', 'accounting.coa_manage']),
      'PUT /api/v1/ledger-accounts/a-cash': () => problem(412, 'VERSION_MISMATCH'),
    })
    renderRoute('/accounting/journals')
    await userEvent.click(await screen.findByRole('tab', { name: 'Chart of accounts' }))
    await userEvent.click(await screen.findByRole('button', { name: 'Edit Cash in hand' }))
    const dialog = screen.getByRole('dialog', { name: 'Edit account' })
    expect(within(dialog).getByText(/Only its name can be changed/)).toBeInTheDocument()
    expect(within(dialog).getByLabelText('Code')).toBeDisabled()
    expect(within(dialog).getByLabelText('Normal side')).toBeDisabled()
    expect(within(dialog).queryByLabelText('Active')).not.toBeInTheDocument()
    const name = within(dialog).getByLabelText('Name')
    await userEvent.clear(name)
    await userEvent.type(name, 'Cash box')
    await userEvent.click(within(dialog).getByRole('button', { name: 'Save' }))
    expect(await within(dialog).findByText(/Someone else changed this/)).toBeInTheDocument()
    const put = calls.find((c) => c.method === 'PUT')
    expect(put?.headers.get('If-Match')).toBe('"1"')
    expect(put?.body).toMatchObject({ code: '1010', name: 'Cash box', parent_id: 'a-assets', status: 'active' })
  })

  it('hides chart editing without accounting.coa_manage', async () => {
    mockApi(signedIn(['accounting.view']))
    renderRoute('/accounting/journals')
    await userEvent.click(await screen.findByRole('tab', { name: 'Chart of accounts' }))
    expect(await screen.findByText('Cash in hand')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /Add account/ })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Edit Cash in hand' })).not.toBeInTheDocument()
  })
})
