import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { currentTenant, membership, mockApi, token, user } from '../../test/api'
import { renderRoute } from '../../test/render'

afterEach(() => vi.restoreAllMocks())

function signedIn(perms: string[]) {
  return {
    'POST /api/v1/auth/refresh': token('t1'),
    'GET /api/v1/auth/me': { user, memberships: [membership('t1', 'Rahim Store')], tenant: currentTenant('t1', 'Rahim Store', perms) },
  }
}

const zero = { current: '0.00', days_31_60: '0.00', days_61_90: '0.00', over_90: '0.00', other: '0.00', total: '0.00' }

describe('analysis reports', () => {
  it('shows customer aging buckets and switches to suppliers', async () => {
    const calls = mockApi({
      ...signedIn(['reports.parties']),
      'GET /api/v1/reports/aging': (call) => {
        const suppliers = call.search.includes('party=suppliers')
        const row = suppliers
          ? { ...zero, party_id: 'sup1', code: 'S-01', name: 'Akij Foods', phone: null, oldest_date: '2026-06-01', over_90: '5000.00', total: '5000.00' }
          : { ...zero, party_id: 'c1', code: 'C-01', name: 'Karim Mia', phone: '01711000002', oldest_date: '2026-10-01', current: '700.00', other: '-200.00', total: '500.00' }
        const { party_id: _id, code: _c, name: _n, phone: _p, oldest_date: _o, ...totals } = row
        return Response.json({ party: suppliers ? 'suppliers' : 'customers', as_of: '2026-10-19', rows: [row], totals })
      },
    })
    renderRoute('/reports/aging')
    const table = await screen.findByRole('table')
    expect(within(table).getByRole('link', { name: 'Karim Mia' })).toHaveAttribute('href', '/customers/c1')
    expect(within(table).getAllByText('৳700.00').length).toBeGreaterThan(0)
    expect(screen.getByText(/Other = balance not tied/)).toBeInTheDocument()
    expect(calls.find((c) => c.path === '/api/v1/reports/aging')?.search).toContain('party=customers')

    await userEvent.click(screen.getByRole('tab', { name: 'Supplier payables' }))
    const link = await screen.findByRole('link', { name: 'Akij Foods' })
    expect(link).toHaveAttribute('href', '/suppliers/sup1')
  })

  it('lists unpaid sales per customer, oldest first, and searches', async () => {
    const row = {
      id: 's1', doc_no: 'INV-0007', reference: null, doc_date: '2026-07-01', days_open: 129, party_id: 'c1', party_code: 'C-01',
      party_name: 'Karim Mia', party_phone: '01711000002', grand_total: '1000.00', paid_total: '300.00', returned_total: '0.00', due_total: '700.00',
    }
    const calls = mockApi({
      ...signedIn(['reports.parties']),
      'GET /api/v1/reports/dues': (call) =>
        Response.json({
          party: 'customers',
          as_of: '2026-11-07',
          rows: call.search.includes('q=nobody') ? [] : [row],
          totals: { grand_total: '1000.00', paid_total: '300.00', returned_total: '0.00', due_total: '700.00', parties: 1 },
        }),
    })
    renderRoute('/reports/customer-dues')
    expect(await screen.findByRole('heading', { name: 'Due customer records' })).toBeInTheDocument()
    const table = await screen.findByRole('table')
    expect(within(table).getByRole('link', { name: 'Karim Mia' })).toHaveAttribute('href', '/customers/c1')
    expect(within(table).getByRole('link', { name: 'INV-0007' })).toHaveAttribute('href', '/sales/s1')
    expect(within(table).getByText('129 days')).toHaveClass('text-danger')
    expect(calls.find((c) => c.path === '/api/v1/reports/dues')?.search).toBe('?party=customers')

    await userEvent.type(screen.getByLabelText('Customer, phone or invoice'), 'nobody')
    expect(await screen.findByText('No customer owes anything.')).toBeInTheDocument()
    expect(calls.at(-1)?.search).toBe('?party=customers&q=nobody')
  })

  it('lists purchases still to pay with the supplier bill number', async () => {
    mockApi({
      ...signedIn(['reports.parties']),
      'GET /api/v1/reports/dues': {
        party: 'suppliers',
        as_of: '2026-11-07',
        rows: [{
          id: 'p1', doc_no: 'PUR-0003', reference: 'KT-77', doc_date: '2026-10-20', days_open: 18, party_id: 'sup1', party_code: null,
          party_name: 'Karim Traders', party_phone: null, grand_total: '1050.00', paid_total: '400.00', returned_total: '0.00', due_total: '650.00',
        }],
        totals: { grand_total: '1050.00', paid_total: '400.00', returned_total: '0.00', due_total: '650.00', parties: 1 },
      },
    })
    renderRoute('/reports/supplier-dues')
    const table = await screen.findByRole('table')
    expect(within(table).getByRole('link', { name: 'PUR-0003' })).toHaveAttribute('href', '/purchases/p1')
    expect(within(table).getByText('Bill KT-77')).toBeInTheDocument()
    expect(screen.getByText('Due to pay', { selector: 'dt' })).toBeInTheDocument()
  })

  it('shows output and input VAT with the net payable and the expenses row', async () => {
    mockApi({
      ...signedIn(['reports.financial']),
      'GET /api/v1/reports/vat-summary': {
        from: '2026-10-01', to: '2026-10-19', is_vat_registered: false,
        output: [{ rate_pct: '15.00', taxable: '10000.00', vat: '1500.00' }],
        input: [{ rate_pct: '15.00', taxable: '4000.00', vat: '600.00' }, { rate_pct: null, taxable: '1000.00', vat: '50.00' }],
        output_vat: '1500.00', input_vat: '650.00', net_vat: '850.00',
      },
    })
    renderRoute('/reports/vat')
    expect(await screen.findByText('Net VAT payable')).toBeInTheDocument()
    expect(screen.getByText('৳850.00')).toBeInTheDocument()
    expect(screen.getByText(/not marked as VAT-registered/)).toBeInTheDocument()
    const input = screen.getByRole('table', { name: 'Input VAT (purchases less returns and expenses)' })
    expect(within(input).getByText('Expenses')).toBeInTheDocument()
    expect(within(input).getByText('15.00%')).toBeInTheDocument()
  })

  it('refuses the VAT report without the financial permission', async () => {
    const calls = mockApi(signedIn(['reports.sales']))
    renderRoute('/reports/vat')
    expect(await screen.findByText("You don't have permission to view this.")).toBeInTheDocument()
    await waitFor(() => expect(calls.some((c) => c.path === '/api/v1/reports/vat-summary')).toBe(false))
  })

  it('lists the new reports on the hub by permission', async () => {
    mockApi(signedIn(['reports.parties', 'reports.inventory']))
    renderRoute('/reports')
    expect(await screen.findByRole('link', { name: /Receivable & payable aging/ })).toHaveAttribute('href', '/reports/aging')
    expect(screen.getByRole('link', { name: /Due customers/ })).toHaveAttribute('href', '/reports/customer-dues')
    expect(screen.getByRole('link', { name: /Suppliers waiting for payment/ })).toHaveAttribute('href', '/reports/supplier-dues')
    expect(screen.getByRole('link', { name: /Fast & slow movers/ })).toHaveAttribute('href', '/reports/product-movement')
    expect(screen.queryByRole('link', { name: /VAT summary/ })).not.toBeInTheDocument()
  })
})
