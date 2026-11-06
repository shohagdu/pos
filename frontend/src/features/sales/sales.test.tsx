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
    const calls = mockApi({
      ...signedIn(),
      'GET /api/v1/sales': {
        data: [sale, { ...sale, id: 's2', invoice_no: 'INV-0002', warranty_items: 2, warranty_until: '2099-02-06' }],
        next_cursor: null,
        summary: { count: 1, grand_total: '901.0000', paid_total: '901.0000', due_total: '0.0000', returned_total: '0.0000' },
      },
    })
    renderRoute('/sales')
    const table = await screen.findByRole('table')
    expect(within(table).getByRole('link', { name: 'INV-0001' })).toHaveAttribute('href', '/sales/s1')
    expect(within(table).getAllByText('Karim')).toHaveLength(2)
    expect(within(table).getAllByText('Completed')).toHaveLength(2)
    expect(screen.getByText('Invoices').nextSibling).toHaveTextContent('1')
    expect(screen.getByRole('link', { name: 'New sale' })).toHaveAttribute('href', '/pos')
    // Warranty: the latest expiry and how many items carry one.
    const warranted = within(table).getByRole('link', { name: 'INV-0002' }).closest('tr')!
    expect(within(warranted).getByText('Until 06 Feb 2099')).toBeInTheDocument()
    expect(within(warranted).getByText('2 items')).toBeInTheDocument()
    await userEvent.click(screen.getByLabelText('Only sales with a warranty'))
    await waitFor(() => expect(calls.some((c) => c.path === '/api/v1/sales' && c.search.includes('with_warranty=true'))).toBe(true))
  })

  it('shows when each warranty runs out', async () => {
    const line = sale.items[0]!
    const items = [
      { ...line, product_name: 'Power Bank', warranty_period: 3, warranty_unit: 'month' as const, warranty_expires_on: '2099-01-18' },
      { ...line, id: 'si2', line_no: 2, product_name: 'Cable', warranty_period: 7, warranty_unit: 'day' as const, warranty_expires_on: '2026-10-25' },
    ]
    mockApi({ ...signedIn(), 'GET /api/v1/sales/s1': { ...sale, items } })
    renderRoute('/sales/s1')
    const table = await screen.findByRole('table', { name: /items/i })
    expect(within(table).getByText(/Warranty until 18 Jan 2099/)).toHaveTextContent('· 3 months')
    expect(within(table).getByText(/Warranty expired 25 Oct 2026/)).toHaveTextContent('· 7 days')
    const printed = screen.getByTestId('receipt')
    expect(within(printed).getByText('Warranty 3 months, until 18 Jan 2099')).toBeInTheDocument()
  })

  it('looks up a warranty by invoice, phone or barcode', async () => {
    const item = {
      id: 'si1', sale_id: 's1', invoice_no: 'INV-0001', business_date: '2026-10-18', customer_id: 'c1', customer_name: 'Karim',
      customer_phone: '01711000002', is_walk_in: false, variant_id: 'v1', product_name: 'Power Bank', sku: 'P000009',
      unit_short_name: 'pcs', quantity: '1.000', returned_quantity: '0.000', warranty_period: 3, warranty_unit: 'month' as const,
      warranty_expires_on: '2027-01-18',
    }
    const calls = mockApi({
      ...signedIn(),
      'GET /api/v1/sale-warranties': (call) =>
        Response.json({
          data: call.search.includes('q=nothing') ? [] : [item, { ...item, id: 'si2', customer_id: 'walk', customer_name: 'Walk-in', is_walk_in: true, customer_phone: null, invoice_no: 'INV-0002', sale_id: 's2', warranty_expires_on: '2026-11-01' }],
          next_cursor: null,
          today: '2026-11-07',
        }),
    })
    renderRoute('/sales/warranties')
    const table = await screen.findByRole('table', { name: 'Warranty lookup' })
    // The sidebar marks the most specific entry only.
    const nav = screen.getAllByRole('link', { name: 'Warranty lookup' }).find((l) => l.closest('aside'))!
    expect(nav).toHaveAttribute('aria-current', 'page')
    expect(within(nav.closest('aside')!).getByRole('link', { name: 'Sales' })).not.toHaveAttribute('aria-current')
    const row = (no: string) => within(table).getByRole('link', { name: no }).closest('tr')!
    expect(within(row('INV-0001')).getByRole('link', { name: 'Karim' })).toHaveAttribute('href', '/customers/c1')
    expect(within(row('INV-0001')).getByText('01711000002')).toBeInTheDocument()
    expect(within(row('INV-0001')).getByText('72 days left')).toBeInTheDocument()
    expect(within(row('INV-0002')).getByText('Walk-in customer')).toBeInTheDocument()
    expect(within(row('INV-0002')).getByText('Expired')).toBeInTheDocument()

    const search = screen.getByLabelText('Search')
    expect(search).toHaveFocus()
    await userEvent.type(search, '2000000000015{Enter}')
    await waitFor(() => expect(calls.some((c) => c.path === '/api/v1/sale-warranties' && c.search.includes('q=2000000000015'))).toBe(true))
    await userEvent.selectOptions(screen.getByLabelText('Status'), 'active')
    await waitFor(() => expect(calls.some((c) => c.search.includes('status=active') && c.search.includes('q=2000000000015'))).toBe(true))
    await userEvent.clear(search)
    await userEvent.type(search, 'nothing{Enter}')
    expect(await screen.findByText('Nothing sold with a warranty matches “nothing”.')).toBeInTheDocument()
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
