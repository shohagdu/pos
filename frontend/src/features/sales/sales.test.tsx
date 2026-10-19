import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { currentTenant, membership, mockApi, token, user } from '../../test/api'
import { renderRoute } from '../../test/render'
import { posContext, receipt, sale } from './fixtures'

afterEach(() => vi.restoreAllMocks())

function signedIn(perms = ['dashboard.view', 'sales.view', 'sales.return', 'pos.sell']) {
  return {
    'POST /api/v1/auth/refresh': token('t1'),
    'GET /api/v1/auth/me': { user, memberships: [membership('t1', 'Rahim Store')], tenant: currentTenant('t1', 'Rahim Store', perms) },
    'GET /api/v1/pos/context': posContext,
  }
}

describe('sales', () => {
  it('lists sales with the summary', async () => {
    mockApi({
      ...signedIn(),
      'GET /api/v1/sales': {
        data: [sale],
        next_cursor: null,
        summary: { count: 1, grand_total: '901.0000', paid_total: '901.0000', due_total: '0.0000', returned_total: '0.0000' },
      },
    })
    renderRoute('/sales')
    const table = await screen.findByRole('table')
    expect(within(table).getByRole('link', { name: 'INV-0001' })).toHaveAttribute('href', '/sales/s1')
    expect(within(table).getByText('Karim')).toBeInTheDocument()
    expect(within(table).getByText('Completed')).toBeInTheDocument()
    expect(screen.getByText('Invoices').nextSibling).toHaveTextContent('1')
    expect(screen.getByRole('link', { name: 'New sale' })).toHaveAttribute('href', '/pos')
  })

  it('limits the return quantity and posts the return', async () => {
    const withReturned = { ...sale, items: [{ ...sale.items[0]!, returned_quantity: '1.0000' }] }
    const calls = mockApi({
      ...signedIn(),
      'GET /api/v1/sales/s1': withReturned,
      'POST /api/v1/sale-returns': () =>
        Response.json(
          {
            id: 'r1', return_no: 'RET-1', sale_id: 's1', invoice_no: 'INV-0001', customer_id: 'c1', customer_name: 'Karim',
            business_date: '2026-10-18', reason: 'Torn bag', refund_mode: 'cash', subtotal: '450.5000', tax_total: '0.0000',
            total: '450.5000', applied_to_due: '0.0000', refunded_amount: '450.5000', credited_amount: '0.0000', cost_total: null,
            created_by_name: 'Rahim Uddin', created_at: '2026-10-18T06:00:00Z', items: [], receipt,
          },
          { status: 201 },
        ),
      'GET /api/v1/sale-returns/r1': { id: 'r1' },
    })
    const { router } = renderRoute('/sales/s1/return')
    const qty = await screen.findByLabelText('Return quantity of Miniket Rice 5kg')
    await userEvent.type(qty, '2')
    expect(screen.getByText('At most 1 can be returned.')).toBeInTheDocument()
    await userEvent.type(screen.getByLabelText('Reason'), 'Torn bag')
    await userEvent.click(screen.getByRole('button', { name: 'Save return' }))
    expect(calls.some((c) => c.path === '/api/v1/sale-returns')).toBe(false)

    await userEvent.clear(qty)
    await userEvent.type(qty, '1')
    expect(screen.getByText('Estimated refund').nextSibling).toHaveTextContent('450.50')
    await userEvent.click(screen.getByRole('button', { name: 'Save return' }))
    await waitFor(() => expect(router.state.location.pathname).toBe('/sales/returns/r1'))
    const post = calls.find((c) => c.method === 'POST' && c.path === '/api/v1/sale-returns')
    expect(post?.headers.get('Idempotency-Key')).toBeTruthy()
    expect(post?.body).toEqual({
      sale_id: 's1',
      reason: 'Torn bag',
      refund_mode: 'cash',
      lines: [{ sale_item_id: 'si1', quantity: '1', condition: 'restock' }],
    })
  })
})
