import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { currentTenant, membership, mockApi, problem, token, user } from '../../test/api'
import { renderRoute } from '../../test/render'

afterEach(() => vi.restoreAllMocks())

const ownerPerms = ['dashboard.view', 'users.view', 'users.invite', 'users.manage_roles', 'users.revoke_sessions', 'pos.sell', 'sales.view', 'settings.manage']

const roles = [
  { id: 'r-owner', code: 'owner', name: 'Owner', description: 'x', is_system: true, max_discount_pct: '100', permissions: ownerPerms, member_count: 1, version: 1 },
  { id: 'r-admin', code: 'admin', name: 'Admin', description: 'x', is_system: true, max_discount_pct: '100', permissions: ['users.view'], member_count: 0, version: 1 },
  { id: 'r-cashier', code: 'cashier', name: 'Cashier', description: 'x', is_system: true, max_discount_pct: null, permissions: ['pos.sell', 'sales.view'], member_count: 0, version: 1 },
]
const catalogue = [
  { code: 'pos.sell', module: 'pos', description: 'Use the POS', is_sensitive: false },
  { code: 'sales.view', module: 'sales', description: 'View own sales', is_sensitive: false },
  { code: 'settings.manage', module: 'settings', description: 'Change settings', is_sensitive: true },
  { code: 'subscription.manage', module: 'settings', description: 'Subscription', is_sensitive: true },
]

function signedIn(perms = ownerPerms, isOwner = true) {
  const tenant = { ...currentTenant('t1', 'Rahim Store', perms), is_owner: isOwner }
  return {
    'POST /api/v1/auth/refresh': token('t1'),
    'GET /api/v1/auth/me': { user, memberships: [membership('t1', 'Rahim Store')], tenant },
    'GET /api/v1/roles': roles,
    'GET /api/v1/permissions': catalogue,
    'GET /api/v1/branches': [{ id: 'b1', code: 'MAIN', name: 'Main Branch', address: null, phone: null, receipt_header: null, is_default: true, status: 'active', version: 1 }],
  }
}

describe('users & roles', () => {
  it('invites staff and shows the development link', async () => {
    const calls = mockApi({
      ...signedIn(),
      'POST /api/v1/invitations': () =>
        Response.json(
          {
            invitation: { id: 'i1', destination: '+8801711000202', roles: [{ id: 'r-cashier', name: 'Cashier' }], branch_ids: [], status: 'pending', expires_at: '2026-10-24T00:00:00Z', created_at: '2026-10-17T00:00:00Z', invited_by: 'Rahim' },
            dev_link: 'http://localhost:5173/invite#tok123',
          },
          { status: 201 },
        ),
    })
    renderRoute('/users/invite')
    await userEvent.type(await screen.findByLabelText('Mobile number or email'), '01711000202')
    expect(screen.queryByLabelText('Owner')).not.toBeInTheDocument()
    await userEvent.click(screen.getByLabelText('Cashier'))
    await userEvent.click(screen.getByRole('button', { name: 'Send invitation' }))
    expect(await screen.findByText('Invitation sent')).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'http://localhost:5173/invite#tok123' })).toBeInTheDocument()
    expect(calls.find((c) => c.path === '/api/v1/invitations' && c.method === 'POST')?.body).toEqual({
      destination: '01711000202',
      role_ids: ['r-cashier'],
      branch_ids: [],
    })
  })

  it('does not offer the admin role to non-owners', async () => {
    mockApi(signedIn(ownerPerms, false))
    renderRoute('/users/invite')
    expect(await screen.findByLabelText('Cashier')).toBeInTheDocument()
    expect(screen.queryByLabelText('Admin')).not.toBeInTheDocument()
  })

  it('shows the plan limit with its maximum', async () => {
    mockApi({ ...signedIn(), 'POST /api/v1/invitations': () => problem(402, 'PLAN_LIMIT_REACHED', { limit: 'users.max', max: 3 }) })
    renderRoute('/users/invite')
    await userEvent.type(await screen.findByLabelText('Mobile number or email'), '01711000202')
    await userEvent.click(screen.getByLabelText('Cashier'))
    await userEvent.click(screen.getByRole('button', { name: 'Send invitation' }))
    expect(await screen.findByText('Your plan allows at most 3 users. Upgrade the plan or remove some first.')).toBeInTheDocument()
  })

  it('keeps system roles read-only and creates custom roles from a copy', async () => {
    const calls = mockApi({
      ...signedIn(['users.view', 'users.manage_roles', 'pos.sell', 'sales.view'], false),
      'POST /api/v1/roles': (call) => Response.json({ ...roles[2], ...(call.body as object), id: 'r-new', code: 'c_x', is_system: false }, { status: 201 }),
    })
    const { router } = renderRoute('/users/roles/r-cashier')
    expect(await screen.findByText(/System roles cannot be changed/)).toBeInTheDocument()
    expect(screen.getByLabelText('Role name')).toBeDisabled()

    await userEvent.click(screen.getByRole('link', { name: 'Duplicate' }))
    const name = await screen.findByLabelText('Role name')
    expect(name).toHaveValue('Cashier (copy)')
    // A non-owner cannot grant what they do not hold, and nobody gets subscription.manage.
    expect(screen.getByRole('checkbox', { name: /Change business settings/ })).toBeDisabled()
    expect(screen.getByRole('checkbox', { name: /Manage subscription/ })).toBeDisabled()
    await userEvent.click(screen.getByRole('checkbox', { name: /View own sales/ }))
    await userEvent.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() => expect(router.state.location.pathname).toBe('/users/roles/r-new'))
    expect(calls.find((c) => c.method === 'POST' && c.path === '/api/v1/roles')?.body).toMatchObject({
      name: 'Cashier (copy)',
      permissions: ['pos.sell'],
    })
  })

  it('suspends a member after an inline confirmation', async () => {
    const member = {
      id: 'm2', user_id: 'u2', full_name: 'Karim', phone: '+8801711000202', email: null, status: 'active', is_owner: false,
      roles: [{ role_id: 'r-cashier', code: 'cashier', name: 'Cashier', branch_id: null }], joined_at: null, last_login_at: null,
    }
    const calls = mockApi({
      ...signedIn(),
      'GET /api/v1/members/m2': member,
      'GET /api/v1/members/m2/sessions': [],
      'POST /api/v1/members/m2/suspend': { ...member, status: 'suspended' },
    })
    renderRoute('/users/members/m2')
    await userEvent.click(await screen.findByRole('button', { name: 'Suspend' }))
    expect(calls.some((c) => c.path.endsWith('/suspend'))).toBe(false)
    await userEvent.click(screen.getByRole('button', { name: 'Yes' }))
    await waitFor(() => expect(calls.some((c) => c.path === '/api/v1/members/m2/suspend')).toBe(true))
  })

  it('hides the Users menu without users.view', async () => {
    mockApi({
      ...signedIn(['pos.sell']),
      'GET /readyz': { status: 'ready' },
      'GET /api/v1/meta': { name: 'pos-api', version: '0.1.0', environment: 'test' },
    })
    renderRoute('/')
    const nav = await screen.findByRole('navigation', { name: 'Main' })
    expect(within(nav).queryByRole('link', { name: 'Users' })).not.toBeInTheDocument()
  })
})

describe('accepting an invitation', () => {
  const preview = {
    business_name: 'Rahim Store', invited_by: 'Rahim', destination: '+880*******202', destination_kind: 'phone',
    roles: ['Cashier'], status: 'pending', expires_at: '2026-10-24T00:00:00Z',
  }

  it('asks visitors to sign in and keeps the link for afterwards', async () => {
    mockApi({ 'POST /api/v1/auth/refresh': () => problem(401, 'SESSION_INVALID'), 'POST /api/v1/invitations/preview': preview })
    const { router } = renderRoute('/invite#tok123')
    expect(await screen.findByRole('heading', { name: 'Join Rahim Store' })).toBeInTheDocument()
    await userEvent.click(screen.getByRole('link', { name: 'I have an account: sign in' }))
    await waitFor(() => expect(router.state.location.pathname).toBe('/login'))
    expect(router.state.location.state).toEqual({ from: '/invite#tok123' })
  })

  it('joins the business and opens it', async () => {
    let tenant: string | null = null
    const calls = mockApi({
      'POST /api/v1/auth/refresh': token(),
      'GET /api/v1/auth/me': () =>
        Response.json({ user, memberships: tenant ? [membership('t1', 'Rahim Store')] : [], tenant: tenant ? currentTenant('t1', 'Rahim Store') : null }),
      'POST /api/v1/invitations/preview': preview,
      'POST /api/v1/invitations/accept': { tenant_id: 't1', membership_id: 'm2' },
      'POST /api/v1/auth/switch-tenant': () => {
        tenant = 't1'
        return Response.json(token('t1'))
      },
      'GET /api/v1/onboarding': { steps: [], dismissed: true, complete: false },
      'GET /readyz': { status: 'ready' },
      'GET /api/v1/meta': { name: 'pos-api', version: '0.1.0', environment: 'test' },
    })
    const { router } = renderRoute('/invite#tok123')
    await userEvent.click(await screen.findByRole('button', { name: 'Join Rahim Store' }))
    await waitFor(() => expect(router.state.location.pathname).toBe('/'))
    expect(calls.find((c) => c.path === '/api/v1/invitations/preview')?.body).toEqual({ token: 'tok123' })
    expect(calls.find((c) => c.path === '/api/v1/invitations/accept')?.body).toEqual({ token: 'tok123' })
  })

  it('explains links that were already used', async () => {
    mockApi({ 'POST /api/v1/auth/refresh': () => problem(401, 'SESSION_INVALID'), 'POST /api/v1/invitations/preview': { ...preview, status: 'accepted' } })
    renderRoute('/invite#tok123')
    expect(await screen.findByText('This invitation was already used.')).toBeInTheDocument()
  })
})
