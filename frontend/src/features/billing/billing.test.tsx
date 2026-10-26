import { screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { currentTenant, membership, mockApi, problem, token, user } from '../../test/api'
import { renderRoute } from '../../test/render'

afterEach(() => vi.restoreAllMocks())

function session(status: string, daysLeft: number | null) {
  const tenant = currentTenant('t1', 'Rahim Store')
  return {
    'POST /api/v1/auth/refresh': token('t1'),
    'GET /api/v1/auth/me': {
      user,
      memberships: [membership('t1', 'Rahim Store')],
      tenant: { ...tenant, subscription: { ...tenant.subscription, status, days_left: daysLeft } },
    },
    'GET /api/v1/onboarding': { steps: [], dismissed: true, complete: false },
    'GET /readyz': { status: 'ready' },
    'GET /api/v1/meta': { name: 'pos-api', version: '0.1.0', environment: 'test' },
  }
}

const overview = {
  subscription: { status: 'trialing', stored_status: 'trialing', plan_code: 'trial', plan_name: 'Free Trial', trial_ends_at: '2026-10-31T00:00:00Z', current_period_end: '2026-10-31T00:00:00Z', days_left: 14 },
  features: [
    { code: 'pos.core', name: 'POS', kind: 'flag', enabled: true, limit: null, overridden: false },
    { code: 'inventory.transfers', name: 'Stock transfers', kind: 'flag', enabled: false, limit: null, overridden: false },
  ],
  usage: [
    { feature: 'users.max', name: 'Users', used: 3, limit: 3 },
    { feature: 'products.max', name: 'Products', used: null, limit: 500 },
    { feature: 'storage.mb', name: 'File storage (MB)', used: null, limit: null },
  ],
}

describe('plan and subscription state', () => {
  it('shows the plan, usage meters and features', async () => {
    mockApi({ ...session('trialing', 14), 'GET /api/v1/subscription': overview })
    renderRoute('/plan')
    expect(await screen.findByRole('heading', { name: 'Plan & usage' })).toBeInTheDocument()
    expect(screen.getAllByText('Free Trial').length).toBeGreaterThan(0)
    expect(screen.getByText('14 days')).toBeInTheDocument()
    expect(screen.getByText('3 of 3')).toBeInTheDocument()
    expect(screen.getByRole('meter', { name: 'Users' })).toHaveAttribute('aria-valuenow', '3')
    expect(screen.getByText('Up to 500')).toBeInTheDocument()
    expect(screen.getByText('Unlimited')).toBeInTheDocument()
    expect(screen.getByText('Point of sale')).toBeInTheDocument()
  })

  it('warns when the business is read-only', async () => {
    mockApi(session('read_only', 27))
    renderRoute('/')
    expect(await screen.findByText('Read-only: renew to record sales. Suspension in 27 days.')).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'See plan' })).toHaveAttribute('href', '/plan')
  })

  it('shows only the renewal screen when suspended', async () => {
    mockApi({ ...session('suspended', null), 'GET /api/v1/subscription': { ...overview, subscription: { ...overview.subscription, status: 'suspended' } } })
    renderRoute('/')
    expect(await screen.findByRole('heading', { name: 'Rahim Store is suspended' })).toBeInTheDocument()
    expect(screen.queryByRole('heading', { name: /^Good (morning|afternoon|evening)/ })).not.toBeInTheDocument()
  })

  function inviteSession(used: number) {
    const base = session('read_only', 27)
    const me = base['GET /api/v1/auth/me']
    return {
      ...base,
      'GET /api/v1/auth/me': { ...me, tenant: { ...me.tenant, permissions: ['users.view', 'users.invite'] } },
      'GET /api/v1/subscription': { ...overview, usage: [{ feature: 'users.max', name: 'Users', used, limit: 3 }] },
      'GET /api/v1/roles': [{ id: 'r1', code: 'cashier', name: 'Cashier', description: null, is_system: true, max_discount_pct: null, permissions: [], member_count: 0, version: 1 }],
      'GET /api/v1/branches': [],
    }
  }

  it('blocks inviting when every seat is used', async () => {
    mockApi(inviteSession(3))
    renderRoute('/users/invite')
    expect(await screen.findByText(/3 of 3 user seats used/)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Send invitation' })).toBeDisabled()
  })

  it('explains read-only errors from the API', async () => {
    mockApi({ ...inviteSession(1), 'POST /api/v1/invitations': () => problem(402, 'SUBSCRIPTION_READ_ONLY') })
    renderRoute('/users/invite')
    await userEvent.type(await screen.findByLabelText('Mobile number or email'), '01711000202')
    await userEvent.click(screen.getByLabelText('Cashier'))
    await userEvent.click(screen.getByRole('button', { name: 'Send invitation' }))
    expect(await screen.findByText('The subscription has ended, so the business is read-only. Renew to make changes.')).toBeInTheDocument()
  })
})

describe('manual subscription billing', () => {
  const ownerPerms = ['dashboard.view', 'settings.view', 'settings.manage', 'subscription.manage']
  const starter = {
    id: 'pl1',
    code: 'starter',
    name: 'Starter',
    description: 'For a single shop',
    prices: [
      { billing_cycle: 'monthly', amount: '500.0000' },
      { billing_cycle: 'quarterly', amount: '1350.0000' },
      { billing_cycle: 'annual', amount: '5000.0000' },
    ],
    features: [
      { code: 'users.max', name: 'Users', kind: 'limit', enabled: true, limit_value: 3 },
      { code: 'branches.max', name: 'Branches', kind: 'limit', enabled: true, limit_value: 1 },
      { code: 'products.max', name: 'Products', kind: 'limit', enabled: true, limit_value: null },
      { code: 'pos.core', name: 'POS', kind: 'flag', enabled: true, limit_value: null },
    ],
  }
  const invoice = {
    id: 'inv1', tenant_id: 't1', invoice_no: 'SUB-000001', plan_code: 'starter', plan_name: 'Starter', billing_cycle: 'annual',
    period_start: '2026-10-31', period_end: '2027-10-31', subtotal: '5000.0000', discount_amount: '0.0000', total: '5000.0000',
    status: 'open', paid_at: null, created_at: '2026-10-19T00:00:00Z',
  }
  const billing = {
    plans: [starter],
    invoices: [],
    claims: [],
    payment_instructions: { bkash: 'Send money to 01700000000 (merchant).', note: 'Use the invoice number as reference.' },
  }

  function owner(perms = ownerPerms) {
    const base = session('trialing', 14)
    const me = base['GET /api/v1/auth/me']
    return {
      ...base,
      'GET /api/v1/auth/me': { ...me, tenant: { ...me.tenant, permissions: perms } },
      'GET /api/v1/subscription': overview,
      'GET /api/v1/subscription/billing': billing,
    }
  }

  it('requests an invoice and reports the payment', async () => {
    const calls = mockApi({
      ...owner(),
      'POST /api/v1/subscription/invoices': invoice,
      'POST /api/v1/subscription/invoices/inv1/claims': (call) => Response.json({ id: 'c1', ...(call.body as object) }, { status: 201 }),
    })
    renderRoute('/plan')
    const card = await screen.findByRole('listitem', { name: 'Starter' })
    expect(within(card).getByText('Unlimited')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('tab', { name: 'Annual' }))
    expect(within(card).getByText(/5,000\.00/)).toBeInTheDocument()
    expect(within(card).getByText(/416\.67 per month/)).toBeInTheDocument()
    expect(within(card).getByText(/Save .*1,000\.00 \(17%\)/)).toBeInTheDocument()
    await userEvent.click(within(card).getByRole('button', { name: 'Pay for Starter' }))

    const dialog = await screen.findByRole('dialog', { name: 'Pay invoice' })
    expect(calls.find((c) => c.method === 'POST' && c.path === '/api/v1/subscription/invoices')?.body).toEqual({ plan_code: 'starter', billing_cycle: 'annual' })
    expect(within(dialog).getByText('SUB-000001')).toBeInTheDocument()
    expect(within(dialog).getByText('Send money to 01700000000 (merchant).')).toBeInTheDocument()
    expect(within(dialog).getByText('Use the invoice number as reference.')).toBeInTheDocument()
    expect(within(dialog).getByLabelText('Amount paid')).toHaveValue('5000.00')
    await userEvent.type(within(dialog).getByLabelText('Transaction ID'), 'ABC')
    await userEvent.click(within(dialog).getByRole('button', { name: 'I have paid' }))
    expect(await within(dialog).findByText('Enter the transaction ID (4–60 characters).')).toBeInTheDocument()

    await userEvent.type(within(dialog).getByLabelText('Transaction ID'), 'X12345')
    await userEvent.type(within(dialog).getByLabelText(/Paid from/), '01711000001')
    await userEvent.click(within(dialog).getByRole('button', { name: 'I have paid' }))
    expect(await screen.findByText(/We'll activate your plan as soon as the payment is verified/)).toBeInTheDocument()
    expect(calls.find((c) => c.method === 'POST' && c.path.endsWith('/claims'))?.body).toEqual({
      method: 'bkash',
      amount: '5000.00',
      transaction_ref: 'ABCX12345',
      payer_account: '01711000001',
    })
  })

  it('explains a payment already waiting for verification', async () => {
    mockApi({
      ...owner(),
      'GET /api/v1/subscription/billing': { ...billing, invoices: [invoice] },
      'POST /api/v1/subscription/invoices/inv1/claims': () => problem(409, 'CLAIM_PENDING'),
    })
    renderRoute('/plan')
    await userEvent.click(await screen.findByRole('button', { name: 'Report payment for SUB-000001' }))
    const dialog = await screen.findByRole('dialog', { name: 'Pay invoice' })
    await userEvent.type(within(dialog).getByLabelText('Transaction ID'), 'TRX9999')
    await userEvent.click(within(dialog).getByRole('button', { name: 'I have paid' }))
    expect(await within(dialog).findByText('A payment for this invoice is already waiting for verification.')).toBeInTheDocument()
  })

  it('hides renewal and export from members without subscription.manage', async () => {
    const calls = mockApi(owner(['dashboard.view']))
    renderRoute('/plan')
    expect(await screen.findByRole('heading', { name: 'Plan & usage' })).toBeInTheDocument()
    expect(screen.queryByText('Choose a plan / Renew')).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Download all my data (ZIP)' })).not.toBeInTheDocument()
    expect(calls.some((c) => c.path === '/api/v1/subscription/billing')).toBe(false)
  })
})
