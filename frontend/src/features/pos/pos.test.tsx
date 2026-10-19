import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { currentTenant, membership, mockApi, token, user } from '../../test/api'
import { renderRoute } from '../../test/render'
import { posContext, rice, sale } from '../sales/fixtures'

beforeEach(() => {
  vi.spyOn(window, 'print').mockImplementation(() => {})
})
afterEach(() => vi.restoreAllMocks())

const cash = {
  id: 'm-cash', name: 'Cash', method_type: 'cash', default_account_id: 'a1', default_account_name: 'Cash drawer',
  requires_reference: false, sort_order: 0, is_active: true,
}

function routes(extra: Record<string, unknown> = {}) {
  return {
    'POST /api/v1/auth/refresh': token('t1'),
    'GET /api/v1/auth/me': {
      user,
      memberships: [membership('t1', 'Rahim Store')],
      tenant: currentTenant('t1', 'Rahim Store', ['dashboard.view', 'pos.sell', 'sales.view']),
    },
    'GET /api/v1/pos/context': posContext,
    'GET /api/v1/pos/products': [rice],
    'GET /api/v1/payment-methods': [cash],
    'GET /api/v1/held-carts': [],
    ...extra,
  } as Parameters<typeof mockApi>[0]
}

async function scanTwiceAndPay() {
  const search = await screen.findByRole('combobox', { name: 'Search products' })
  await userEvent.type(search, '4006381333931{Enter}')
  expect(await screen.findByLabelText('Quantity of Miniket Rice 5kg')).toHaveValue('1')
  expect(search).toHaveValue('')
  await userEvent.type(search, '4006381333931{Enter}')
  await waitFor(() => expect(screen.getByLabelText('Quantity of Miniket Rice 5kg')).toHaveValue('2'))
  expect(screen.getByTestId('grand-total')).toHaveTextContent('901.00')
  await userEvent.keyboard('{F8}')
  const dialog = await screen.findByRole('dialog', { name: 'Payment' })
  await userEvent.clear(within(dialog).getByLabelText('Amount 1'))
  await userEvent.type(within(dialog).getByLabelText('Amount 1'), '1000')
  expect(within(dialog).getByTestId('change')).toHaveTextContent('99.00')
  return dialog
}

describe('POS', () => {
  it('adds scanned items and posts the sale with an idempotency key and the expected total', async () => {
    const calls = mockApi(routes({ 'POST /api/v1/sales': () => Response.json(sale, { status: 201 }) }))
    renderRoute('/pos')
    const dialog = await scanTwiceAndPay()
    await userEvent.click(within(dialog).getByRole('button', { name: 'Confirm sale' }))

    expect(await screen.findByRole('dialog', { name: 'Sale INV-0001 completed' })).toBeInTheDocument()
    const post = calls.find((c) => c.method === 'POST' && c.path === '/api/v1/sales')
    expect(post?.headers.get('Idempotency-Key')).toMatch(/^[0-9a-f-]{36}$/)
    expect(post?.body).toEqual({
      lines: [{ variant_id: 'v1', quantity: '2' }],
      payments: [{ method_id: 'm-cash', amount: '1000' }],
      expected_total: '901.00',
    })
    await userEvent.click(screen.getByRole('button', { name: 'New sale' }))
    expect(await screen.findByText(/Scan a barcode or search/)).toBeInTheDocument()
  })

  it('reuses the idempotency key when retrying after a network error', async () => {
    let attempts = 0
    const calls = mockApi(
      routes({
        'POST /api/v1/sales': () => {
          attempts += 1
          if (attempts === 1) return Promise.reject(new TypeError('Failed to fetch'))
          return Response.json(sale, { status: 201 })
        },
      }),
    )
    renderRoute('/pos')
    const dialog = await scanTwiceAndPay()
    await userEvent.click(within(dialog).getByRole('button', { name: 'Confirm sale' }))
    expect(await within(dialog).findByText('Cannot reach the server. Check your connection.')).toBeInTheDocument()
    await userEvent.click(within(dialog).getByRole('button', { name: 'Confirm sale' }))
    expect(await screen.findByRole('dialog', { name: 'Sale INV-0001 completed' })).toBeInTheDocument()

    const keys = calls.filter((c) => c.method === 'POST' && c.path === '/api/v1/sales').map((c) => c.headers.get('Idempotency-Key'))
    expect(keys).toHaveLength(2)
    expect(keys[0]).toBe(keys[1])
  })
})
