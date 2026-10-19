import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { currentTenant, membership, mockApi, token, user } from '../../test/api'
import { renderRoute } from '../../test/render'

afterEach(() => vi.restoreAllMocks())

const perms = ['dashboard.view', 'pos.sell']

const branch = (id: string, name: string, isDefault: boolean) => ({
  id, code: id.toUpperCase(), name, address: null, phone: null, receipt_header: null, is_default: isDefault, status: 'active', version: 1,
})

function signedIn(extra: object = {}) {
  return {
    'POST /api/v1/auth/refresh': token('t1'),
    'GET /api/v1/auth/me': { user, memberships: [membership('t1', 'Rahim Store')], tenant: currentTenant('t1', 'Rahim Store', perms) },
    'GET /api/v1/branches': [branch('b1', 'Mirpur', true)],
    'GET /api/v1/registers': [
      { id: 'r1', branch_id: 'b1', branch_name: 'Mirpur', name: 'Counter 1', cash_account_id: 'a1', cash_account_name: 'Cash', status: 'active', open_shift_id: null, open_shift_user: null },
    ],
    'GET /api/v1/shifts': [],
    'GET /api/v1/shifts/current': () => Response.json(null),
    ...extra,
  }
}

const shift = (over: object = {}) => ({
  id: 's1', shift_no: 3, register_id: 'r1', register_name: 'Counter 1', branch_id: 'b1', status: 'open', opened_by: user.id,
  opened_by_name: 'Rahim Uddin', opened_at: '2026-10-19T03:00:00Z', opening_cash: '500.00', closed_at: null, closed_by_name: null,
  expected_cash: null, counted_cash: null, difference: null, note: null, ...over,
})

const report = (kind: 'X' | 'Z', over: object = {}) => ({
  kind, shift: shift(over), sales_count: 4, sales_total: '1200.00', discount_total: '20.00', tax_total: '0.00', due_total: '0.00',
  cancelled_count: 0, returns_count: 0, returns_total: '0.00', payments: [{ method_name: 'Cash', count: 4, money_in: '1200.00', money_out: '0.00' }],
  cash_in: '1200.00', cash_out: '0.00', expected_cash: '1700.00',
})

describe('shifts', () => {
  it('opens a shift on a free register with the opening cash', async () => {
    let opened = false
    const calls = mockApi(
      signedIn({
        'POST /api/v1/shifts': () => {
          opened = true
          return Response.json(shift(), { status: 201 })
        },
        'GET /api/v1/shifts/current': () => Response.json(opened ? shift() : null),
      }),
    )
    renderRoute('/shifts')
    await userEvent.click(await screen.findByRole('button', { name: 'Open shift' }))
    const dialog = await screen.findByRole('dialog', { name: 'Open shift' })
    const cash = within(dialog).getByLabelText('Opening cash')
    await userEvent.clear(cash)
    await userEvent.type(cash, '500')
    await userEvent.click(within(dialog).getByRole('button', { name: 'Open shift' }))
    await waitFor(() => expect(calls.some((c) => c.method === 'POST' && c.path === '/api/v1/shifts')).toBe(true))
    expect(calls.find((c) => c.method === 'POST' && c.path === '/api/v1/shifts')?.body).toEqual({ register_id: 'r1', opening_cash: '500' })
    expect(await screen.findByRole('link', { name: 'X report / close' })).toHaveAttribute('href', '/shifts/s1')
  })

  it('closes a shift with the counted cash and shows the Z report with the shortage', async () => {
    const calls = mockApi(
      signedIn({
        'GET /api/v1/shifts/s1/report': report('X'),
        'POST /api/v1/shifts/s1/close': report('Z', { status: 'closed', closed_at: '2026-10-19T12:00:00Z', expected_cash: '1700.00', counted_cash: '1680.00', difference: '-20.00' }),
      }),
    )
    renderRoute('/shifts/s1')
    expect(await screen.findByRole('heading', { name: /X report/ })).toBeInTheDocument()
    await userEvent.type(screen.getByLabelText(/Counted cash/), '1680')
    expect(screen.getByText(/20\.00 short/)).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Close shift and print Z' }))
    expect(await screen.findByRole('heading', { name: /Z report/ })).toBeInTheDocument()
    expect(calls.find((c) => c.method === 'POST' && c.path === '/api/v1/shifts/s1/close')?.body).toEqual({ counted_cash: '1680' })
    expect(screen.getByText(/20\.00 short/)).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Close shift and print Z' })).not.toBeInTheDocument()
  })
})

describe('branch switcher', () => {
  it('sends X-Branch-Id once another branch is picked', async () => {
    const calls = mockApi(signedIn({ 'GET /api/v1/branches': [branch('b1', 'Mirpur', true), branch('b2', 'Dhanmondi', false)] }))
    renderRoute('/shifts')
    const picker = await screen.findByLabelText('Working in')
    expect(picker).toHaveValue('')
    const before = calls.length
    expect(calls.some((c) => c.headers.get('X-Branch-Id'))).toBe(false)
    await userEvent.selectOptions(picker, 'b2')
    await waitFor(() =>
      expect(calls.slice(before).some((c) => c.path === '/api/v1/shifts/current' && c.headers.get('X-Branch-Id') === 'b2')).toBe(true),
    )
    // Session calls never carry the branch.
    expect(calls.filter((c) => c.path.startsWith('/api/v1/auth/')).every((c) => !c.headers.get('X-Branch-Id'))).toBe(true)
    expect(screen.getByText(/Shifts of Dhanmondi/)).toBeInTheDocument()
  })
})
