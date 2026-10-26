import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { mockApi, problem, token, user } from '../../test/api'
import { renderRoute } from '../../test/render'

afterEach(() => vi.restoreAllMocks())

const staff = (role = 'platform_owner') => ({ user_id: 'u1', full_name: 'Rahim Uddin', role, created_at: '2026-01-01T00:00:00Z' })

const metrics = {
  businesses: 12,
  by_status: { trialing: 5, active: 6, grace: 1, business_suspended: 1 },
  mrr: '12450.00',
  arr: '149400.00',
  signups_30d: 7,
  signups_7d: 2,
  revenue_30d: '9900.00',
  pending_claims: 1,
  by_plan: { trial: 5, starter: 7 },
}

function signedIn(role: string | null = 'platform_owner') {
  return {
    // Platform staff sign in without a business.
    'POST /api/v1/auth/refresh': token(null),
    'GET /api/v1/auth/me': { user, memberships: [], tenant: null },
    'GET /api/v1/admin/me': role ? staff(role) : () => problem(403, 'PLATFORM_STAFF_ONLY'),
    'GET /api/v1/admin/metrics': metrics,
  }
}

const claim = {
  id: 'c1', tenant_id: 't1', tenant_name: 'Rahim Store', invoice_id: 'i1', invoice_no: 'SUB-000001', plan_name: 'Starter',
  billing_cycle: 'monthly', invoice_total: '990.00', method: 'bkash', amount: '900.00', transaction_ref: 'TX123',
  payer_account: '01711000001', status: 'pending', rejection_reason: null, submitted_by_name: 'Karim', reviewed_by_name: null,
  reviewed_at: null, created_at: '2026-10-18T10:00:00Z',
}

const tenant = {
  id: 't1', name: 'Rahim Store', slug: 'rahim-store', business_type: 'grocery', status: 'active', phone: '+8801711000001',
  email: null, district: 'Dhaka', plan_code: 'starter', plan_name: 'Starter', stored_subscription_status: 'active',
  subscription_status: 'active', owner_name: 'Rahim', owner_phone: '+8801711000001', trial_ends_at: null,
  current_period_end: '2026-11-18T00:00:00Z', grace_ends_at: null, read_only_ends_at: null, created_at: '2026-09-01T00:00:00Z',
  suspended_reason: null, billing_cycle: 'monthly', price_amount: '990.00', days_left: 30, last_activity_at: null,
  members: [{ user_id: 'u9', full_name: 'Rahim', phone: '+8801711000001', email: null, is_owner: true, status: 'active' }],
  usage: [{ code: 'users.max', used: 2, limit: 3 }],
  events: [],
  invoices: [],
  claims: [],
  overrides: [],
  entitlements: [
    { code: 'reports.export', name: 'Report export', kind: 'flag', enabled: false, limit: null, overridden: false },
    { code: 'users.max', name: 'Users', kind: 'limit', enabled: true, limit: 3, overridden: false },
  ],
}

describe('super admin console', () => {
  it('tells non-staff users the area is for platform staff', async () => {
    mockApi(signedIn(null))
    renderRoute('/admin')
    expect(await screen.findByText('This area is for platform staff')).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'Back to app' })).toHaveAttribute('href', '/')
  })

  it('shows platform metrics with links to filtered businesses', async () => {
    mockApi(signedIn())
    renderRoute('/admin')
    expect(await screen.findByText('MRR')).toBeInTheDocument()
    expect(screen.getByText(/12,450\.00/)).toBeInTheDocument()
    expect(screen.getByRole('link', { name: /Payment overdue/ })).toHaveAttribute('href', '/admin/tenants?subscription_status=grace')
    const nav = screen.getByRole('navigation', { name: 'Console' })
    expect(within(nav).getByLabelText('1 pending claim')).toBeInTheDocument()
  })

  it('approves a pending claim and flags an amount that differs', async () => {
    const calls = mockApi({
      ...signedIn('billing'),
      'GET /api/v1/admin/claims': { data: [claim], next_cursor: null, pending_count: 1 },
      'POST /api/v1/admin/claims/c1/approve': { ...claim, status: 'approved' },
    })
    renderRoute('/admin/claims')
    const table = await screen.findByRole('table')
    expect(within(table).getByText('Differs from invoice')).toBeInTheDocument()
    expect(within(table).getByRole('link', { name: 'Rahim Store' })).toHaveAttribute('href', '/admin/tenants/t1')
    await userEvent.click(within(table).getByRole('button', { name: 'Approve' }))
    await userEvent.click(within(table).getByRole('button', { name: 'Yes' }))
    await waitFor(() => expect(calls.some((c) => c.method === 'POST' && c.path === '/api/v1/admin/claims/c1/approve')).toBe(true))
    expect(calls.find((c) => c.path === '/api/v1/admin/claims')?.search).toContain('status=pending')
  })

  it('hides suspend and override actions from support staff', async () => {
    mockApi({ ...signedIn('support'), 'GET /api/v1/admin/tenants/t1': tenant })
    renderRoute('/admin/tenants/t1')
    expect(await screen.findByRole('heading', { name: 'Rahim Store' })).toBeInTheDocument()
    expect(screen.getByText('2 / 3')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Suspend business' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Add override' })).not.toBeInTheDocument()
  })

  it('adds a limit override with an expiry and a reason', async () => {
    const calls = mockApi({
      ...signedIn('billing'),
      'GET /api/v1/admin/tenants/t1': tenant,
      'PUT /api/v1/admin/tenants/t1/overrides': (call) => Response.json({ id: 'o1', ...(call.body as object) }),
    })
    renderRoute('/admin/tenants/t1')
    await userEvent.click(await screen.findByRole('button', { name: 'Add override' }))
    expect(screen.queryByRole('button', { name: 'Suspend business' })).not.toBeInTheDocument()
    const dialog = screen.getByRole('dialog', { name: 'Add override' })
    await userEvent.selectOptions(within(dialog).getByLabelText('Feature'), 'users.max')
    await userEvent.type(within(dialog).getByLabelText('Limit'), '10')
    await userEvent.type(within(dialog).getByLabelText('Expires (optional)'), '2026-12-31')
    await userEvent.click(screen.getByRole('button', { name: 'Save' }))
    expect(await within(dialog).findByText('This field is required.')).toBeInTheDocument()
    await userEvent.type(within(dialog).getByLabelText('Reason'), 'Festival promotion')
    await userEvent.click(screen.getByRole('button', { name: 'Save' }))
    await waitFor(() =>
      expect(calls.find((c) => c.method === 'PUT')?.body).toEqual({
        feature_code: 'users.max',
        limit_value: 10,
        expires_at: '2026-12-31T23:59:59Z',
        reason: 'Festival promotion',
      }),
    )
  })

  it('activates a one-time licence at the plan price, and extends a trial', async () => {
    const plan = (code: string, isTrial: boolean, prices: object[]) => ({
      id: code, code, name: code === 'trial' ? 'Free Trial' : 'Starter', description: null, is_public: true, is_trial: isTrial,
      trial_days: isTrial ? 14 : null, sort_order: 0, status: 'active', subscribers: 1, prices, features: [], updated_at: '2026-10-01T00:00:00Z',
    })
    const calls = mockApi({
      ...signedIn('billing'),
      'GET /api/v1/admin/tenants/t1': tenant,
      'GET /api/v1/admin/plans': [plan('trial', true, []), plan('starter', false, [{ billing_cycle: 'lifetime', amount: '25000.00' }])],
      'POST /api/v1/admin/tenants/t1/licence': { ...tenant, billing_cycle: 'lifetime', current_period_end: null },
    })
    renderRoute('/admin/tenants/t1')
    await userEvent.click(await screen.findByRole('button', { name: 'Set licence' }))
    const dialog = screen.getByRole('dialog', { name: 'Set licence for Rahim Store' })
    await userEvent.click(within(dialog).getByRole('tab', { name: 'One-time' }))
    expect(within(dialog).getByText('Blank = the plan price (25000.00).')).toBeInTheDocument()
    await userEvent.type(within(dialog).getByLabelText('Receipt or note'), 'MR-1042 cash')
    await userEvent.click(screen.getByRole('button', { name: 'Activate one-time licence' }))
    await waitFor(() =>
      expect(calls.find((c) => c.path === '/api/v1/admin/tenants/t1/licence')?.body).toEqual({
        kind: 'lifetime', plan_code: 'starter', price_amount: '25000.00', reason: 'MR-1042 cash',
      }),
    )
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument())

    await userEvent.click(screen.getByRole('button', { name: 'Set licence' }))
    const again = screen.getByRole('dialog', { name: 'Set licence for Rahim Store' })
    await userEvent.click(within(again).getByRole('tab', { name: 'Extend trial' }))
    await userEvent.clear(within(again).getByLabelText('Days to add'))
    await userEvent.type(within(again).getByLabelText('Days to add'), '10')
    await userEvent.type(within(again).getByLabelText('Receipt or note'), 'Demo for owner')
    await userEvent.click(screen.getByRole('button', { name: 'Extend trial' }))
    await waitFor(() =>
      expect(calls.filter((c) => c.path === '/api/v1/admin/tenants/t1/licence')[1]?.body).toEqual({ kind: 'trial', days: 10, reason: 'Demo for owner' }),
    )
  })
})
