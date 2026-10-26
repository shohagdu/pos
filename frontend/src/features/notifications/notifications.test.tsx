import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { currentTenant, membership, mockApi, token, user } from '../../test/api'
import { renderRoute } from '../../test/render'

afterEach(() => vi.restoreAllMocks())

function signedIn() {
  return {
    'POST /api/v1/auth/refresh': token('t1'),
    'GET /api/v1/auth/me': { user, memberships: [membership('t1', 'Rahim Store')], tenant: currentTenant('t1', 'Rahim Store', ['dashboard.view', 'inventory.view']) },
  }
}

const note = (id: string, read: boolean) => ({
  id, notif_type: 'stock.low', title: '3 products are low on stock', body: '3 products are at or below their reorder level; 1 are out of stock.',
  link: '/inventory?status=low', data: {}, read_at: read ? '2026-10-20T05:00:00Z' : null, created_at: '2026-10-20T04:00:00Z',
})

describe('notifications', () => {
  it('shows the unread count, then opens a notification and marks it read', async () => {
    let read = false
    const calls = mockApi({
      ...signedIn(),
      'GET /api/v1/notifications/unread-count': () => Response.json({ unread_count: read ? 0 : 1 }),
      'GET /api/v1/notifications': () => Response.json({ data: [note('n1', read)], next_cursor: null, unread_count: read ? 0 : 1 }),
      'POST /api/v1/notifications/n1/read': () => {
        read = true
        return new Response(null, { status: 204 })
      },
      'GET /api/v1/stock': { data: [], next_cursor: null, summary: { items: 0, quantity: '0', value: null, low_count: 0, out_count: 0 } },
      'GET /api/v1/stock-locations': [],
      'GET /api/v1/categories': [],
    })
    const { router } = renderRoute('/notifications')
    expect(await screen.findByRole('link', { name: 'Notifications, 1 unread' })).toHaveAttribute('href', '/notifications')
    await userEvent.click(await screen.findByRole('button', { name: /3 products are low on stock/ }))
    await waitFor(() => expect(router.state.location.pathname).toBe('/inventory'))
    expect(router.state.location.search).toBe('?status=low')
    expect(calls.some((c) => c.method === 'POST' && c.path === '/api/v1/notifications/n1/read')).toBe(true)
  })

  it('saves only the changed channel choices', async () => {
    const pref = (notif_type: string, channels: string[]) => ({
      notif_type, channels, default_channels: ['in_app'], available_channels: ['in_app', 'email'],
    })
    const calls = mockApi({
      ...signedIn(),
      'GET /api/v1/notifications/unread-count': { unread_count: 0 },
      'GET /api/v1/notification-preferences': [pref('stock.low', ['in_app']), pref('security.alert', ['in_app', 'email'])],
      'PUT /api/v1/notification-preferences': [pref('stock.low', ['in_app', 'email']), pref('security.alert', ['in_app', 'email'])],
    })
    renderRoute('/account/notifications')
    const table = await screen.findByRole('table', { name: 'Notification settings' })
    expect(within(table).getByRole('checkbox', { name: 'SMS for Low stock' })).toBeDisabled()
    expect(screen.getByText(/SMS notifications are available on plans/)).toBeInTheDocument()
    await userEvent.click(within(table).getByRole('checkbox', { name: 'Email for Low stock' }))
    await userEvent.click(screen.getByRole('button', { name: 'Save' }))
    expect(await screen.findByText('Saved.', { exact: false })).toBeInTheDocument()
    const put = calls.find((c) => c.method === 'PUT')
    expect(put?.body).toEqual([{ notif_type: 'stock.low', channels: ['in_app', 'email'] }])
  })
})
