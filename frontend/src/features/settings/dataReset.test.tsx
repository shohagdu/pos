import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { currentTenant, membership, mockApi, token, user } from '../../test/api'
import { renderRoute } from '../../test/render'

afterEach(() => vi.restoreAllMocks())

const archive = (id: string, kind: 'clear' | 'restore', status: 'held' | 'restored', documents: Record<string, number>) => ({
  id, kind, status, documents, total_rows: 10, created_by_name: 'Rahim Uddin', created_at: '2026-11-08T10:00:00Z',
  restored_by_name: null, restored_at: status === 'restored' ? '2026-11-08T11:00:00Z' : null,
})

function signedIn(isOwner = true, perms = ['settings.manage']) {
  return {
    'POST /api/v1/auth/refresh': token('t1'),
    'GET /api/v1/auth/me': {
      user,
      memberships: [membership('t1', 'Rahim Store')],
      tenant: { ...currentTenant('t1', 'Rahim Store', perms), is_owner: isOwner },
    },
    'GET /api/v1/settings': {},
  }
}

describe('clear all transactions', () => {
  it('lets the owner clear every transaction and restore them', async () => {
    let state: 'live' | 'cleared' | 'restored' = 'live'
    const calls = mockApi({
      ...signedIn(),
      'GET /api/v1/data-reset': () =>
        Response.json(
          state === 'live'
            ? { live: { sales: 12, purchases: 3 }, archives: [] }
            : state === 'cleared'
              ? { live: {}, archives: [archive('a1', 'clear', 'held', { sales: 12, purchases: 3 })] }
              : {
                  live: { sales: 12, purchases: 3 },
                  archives: [archive('a2', 'restore', 'held', {}), archive('a1', 'clear', 'restored', { sales: 12, purchases: 3 })],
                },
        ),
      'POST /api/v1/data-reset/clear': () => {
        state = 'cleared'
        return Response.json(archive('a1', 'clear', 'held', { sales: 12, purchases: 3 }))
      },
      'POST /api/v1/data-reset/archives/a1/restore': () => {
        state = 'restored'
        return Response.json(archive('a1', 'clear', 'restored', { sales: 12, purchases: 3 }))
      },
    })
    renderRoute('/settings/data')
    expect(await screen.findByText('Sales 12 · Purchases 3')).toBeInTheDocument()
    expect(screen.getByText('Nothing has been cleared yet.')).toBeInTheDocument()

    // One click, then confirm.
    await userEvent.click(screen.getByRole('button', { name: 'Clear all transactions' }))
    await userEvent.click(screen.getByRole('button', { name: 'Yes' }))
    expect(await screen.findByText(/All transactions are cleared/)).toBeInTheDocument()
    expect(await screen.findByText('no transactions')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Clear all transactions' })).toBeDisabled()

    const table = screen.getByRole('table', { name: 'Archives' })
    expect(within(table).getByText('Cleared')).toBeInTheDocument()
    await userEvent.click(within(table).getByRole('button', { name: 'Restore' }))
    await userEvent.click(within(table).getByRole('button', { name: 'Yes' }))
    expect(await screen.findByText('The archive is restored.')).toBeInTheDocument()
    await waitFor(() => expect(within(screen.getByRole('table', { name: 'Archives' })).getByText('Set aside by a restore')).toBeInTheDocument())
    expect(calls.filter((c) => c.method === 'POST' && c.path.startsWith('/api/v1/data-reset')).map((c) => c.path)).toEqual([
      '/api/v1/data-reset/clear',
      '/api/v1/data-reset/archives/a1/restore',
    ])
  })

  it('does not show the Data tab to staff who are not the owner', async () => {
    mockApi(signedIn(false))
    renderRoute('/settings/data')
    expect(await screen.findByText('Only the owner or an admin can change these settings.')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Clear all transactions' })).not.toBeInTheDocument()
  })
})
