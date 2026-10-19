import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { currentTenant, membership, mockApi, token, user } from '../../test/api'
import { renderRoute } from '../../test/render'
import { dateRange } from './filters'

afterEach(() => vi.restoreAllMocks())

function signedIn(perms = ['dashboard.view', 'audit.view']) {
  return {
    'POST /api/v1/auth/refresh': token('t1'),
    'GET /api/v1/auth/me': { user, memberships: [membership('t1', 'Rahim Store')], tenant: currentTenant('t1', 'Rahim Store', perms) },
  }
}

const priceChange = {
  id: 'a1',
  created_at: '2026-10-18T10:15:00Z',
  actor_type: 'user',
  actor_id: 'u1',
  actor_name: 'Rahim Uddin',
  action: 'product.updated',
  entity_type: 'product',
  entity_id: '0192aabb-ccdd-7eee-8fff-000000000001',
  branch_id: null,
  before: { name: 'Rice', selling_price: '450.0000', barcode: 'X1' },
  after: { name: 'Rice', selling_price: '480.0000', unit: 'kg' },
  reason: 'Supplier price up',
  ip_address: '203.0.113.5',
  user_agent: null,
  request_id: null,
}

describe('audit log', () => {
  it('lists records and shows the before/after diff', async () => {
    mockApi({ ...signedIn(), 'GET /api/v1/audit-logs': { data: [priceChange], next_cursor: null } })
    renderRoute('/audit')
    const table = await screen.findByRole('table', { name: 'Audit log' })
    expect(within(table).getByText('product.updated')).toBeInTheDocument()
    expect(within(table).getByText('Supplier price up')).toBeInTheDocument()
    expect(within(table).getByText('0192aabb')).toBeInTheDocument()
    expect(within(table).getByText('User')).toBeInTheDocument()

    await userEvent.click(within(table).getByRole('button', { expanded: false }))
    const changes = await screen.findByRole('table', { name: 'Changes' })
    const price = within(changes).getByRole('rowheader', { name: /selling_price/ }).closest('tr')!
    expect(price).toHaveAttribute('data-change', 'changed')
    expect(within(price).getByText('450.0000')).toBeInTheDocument()
    expect(within(price).getByText('480.0000')).toBeInTheDocument()
    expect(within(changes).getByRole('rowheader', { name: /unit/ }).closest('tr')).toHaveAttribute('data-change', 'added')
    expect(within(changes).getByRole('rowheader', { name: /barcode/ }).closest('tr')).toHaveAttribute('data-change', 'removed')
    expect(within(changes).getByRole('rowheader', { name: /^name/ }).closest('tr')).toHaveAttribute('data-change', 'same')
  })

  it('filters by action, date range and actor', async () => {
    const calls = mockApi({ ...signedIn(), 'GET /api/v1/audit-logs': { data: [priceChange], next_cursor: null } })
    renderRoute('/audit')
    await screen.findByRole('table', { name: 'Audit log' })
    await userEvent.type(screen.getByLabelText('Action'), 'sale.*')
    await userEvent.click(screen.getByRole('button', { name: 'Apply' }))
    await waitFor(() => expect(calls.some((c) => c.path === '/api/v1/audit-logs' && c.search.includes('action=sale.*'))).toBe(true))

    await userEvent.type(screen.getByLabelText('From'), '2026-10-01')
    await userEvent.type(screen.getByLabelText('To'), '2026-10-31')
    await waitFor(() => {
      const last = new URLSearchParams(calls.filter((c) => c.path === '/api/v1/audit-logs').at(-1)!.search)
      expect(last.get('from')).toBe('2026-10-01T00:00:00+06:00')
      expect(last.get('to')).toBe('2026-11-01T00:00:00+06:00')
    })

    await userEvent.click(screen.getByRole('button', { name: 'Rahim Uddin' }))
    await waitFor(() => expect(calls.some((c) => c.search.includes('actor_id=u1'))).toBe(true))
    expect(screen.getByText('Showing actions by Rahim Uddin')).toBeInTheDocument()
  })

  it('converts business dates to an exclusive RFC 3339 range', () => {
    expect(dateRange('2026-12-01', '2026-12-31')).toEqual({ from: '2026-12-01T00:00:00+06:00', to: '2027-01-01T00:00:00+06:00' })
    expect(dateRange('', '')).toEqual({ from: undefined, to: undefined })
  })

  it('tells members without audit.view they cannot see it', async () => {
    const calls = mockApi(signedIn(['dashboard.view']))
    renderRoute('/audit')
    expect(await screen.findByText("You don't have permission to view the audit log.")).toBeInTheDocument()
    expect(calls.some((c) => c.path === '/api/v1/audit-logs')).toBe(false)
  })
})
