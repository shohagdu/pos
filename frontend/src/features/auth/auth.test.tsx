import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { currentTenant, membership, mockApi, problem, token, user } from '../../test/api'
import { renderRoute } from '../../test/render'

afterEach(() => vi.restoreAllMocks())

const anonymous = { 'POST /api/v1/auth/refresh': () => problem(401, 'SESSION_INVALID') }

describe('sign-in', () => {
  it('signs in and asks which business to open', async () => {
    let tenant: string | null = null
    const calls = mockApi({
      ...anonymous,
      'POST /api/v1/auth/login': token(),
      'POST /api/v1/auth/switch-tenant': (call) => {
        tenant = (call.body as { tenant_id: string }).tenant_id
        return Response.json(token(tenant))
      },
      'GET /api/v1/auth/me': () =>
        Response.json({
          user,
          memberships: [membership('t1', 'Rahim Store'), membership('t2', 'Rahim Fashion')],
          tenant: tenant ? currentTenant(tenant, tenant === 't2' ? 'Rahim Fashion' : 'Rahim Store') : null,
        }),
      'GET /api/v1/onboarding': { steps: [], dismissed: true, complete: false },
      'GET /readyz': { status: 'ready' },
      'GET /api/v1/meta': { name: 'pos-api', version: '0.1.0', environment: 'test' },
    })
    const { router } = renderRoute('/login')

    await userEvent.type(await screen.findByLabelText('Mobile number or email'), '01711000001')
    await userEvent.type(screen.getByLabelText('Password'), 'correct horse battery')
    await userEvent.click(screen.getByRole('button', { name: 'Sign in' }))

    expect(await screen.findByRole('heading', { name: 'Choose a business' })).toBeInTheDocument()
    expect(calls.find((c) => c.path === '/api/v1/auth/login')?.body).toEqual({ identifier: '01711000001', password: 'correct horse battery' })

    await userEvent.click(screen.getByRole('button', { name: /Rahim Fashion/ }))
    await waitFor(() => expect(router.state.location.pathname).toBe('/'))
    expect(await screen.findByRole('heading', { name: /^Good (morning|afternoon|evening), Rahim$/ })).toBeInTheDocument()
    const me = calls.filter((c) => c.path === '/api/v1/auth/me').at(-1)
    expect(me?.headers.get('Authorization')).toBe('Bearer token-t2')
  })

  it('shows the lockout message with the wait time', async () => {
    mockApi({ ...anonymous, 'POST /api/v1/auth/login': () => problem(429, 'TOO_MANY_ATTEMPTS', { retry_after: 840 }) })
    renderRoute('/login')
    await userEvent.type(await screen.findByLabelText('Mobile number or email'), '01711000001')
    await userEvent.type(screen.getByLabelText('Password'), 'whatever')
    await userEvent.click(screen.getByRole('button', { name: 'Sign in' }))
    expect(await screen.findByText('Too many failed attempts. Try again in 14 minutes.')).toBeInTheDocument()
  })
})

describe('sign-up and onboarding', () => {
  it('registers, verifies the phone, creates the business and saves the regional step', async () => {
    let session: 'none' | 'user' | 'tenant' = 'none'
    const calls = mockApi({
      ...anonymous,
      'POST /api/v1/auth/register': () => Response.json({ destination: '+880*******001', expires_in: 300, dev_code: '123456' }, { status: 201 }),
      'POST /api/v1/auth/verify-phone': () => {
        session = 'user'
        return Response.json(token())
      },
      'POST /api/v1/tenants': () =>
        Response.json({ tenant_id: 't1', slug: 'rahim-store', name: 'Rahim Store', branch_id: 'b1', membership_id: 'm-t1' }, { status: 201 }),
      'POST /api/v1/auth/switch-tenant': () => {
        session = 'tenant'
        return Response.json(token('t1'))
      },
      'GET /api/v1/auth/me': () =>
        Response.json(
          session === 'tenant'
            ? { user, memberships: [membership('t1', 'Rahim Store')], tenant: currentTenant('t1', 'Rahim Store') }
            : { user, memberships: [], tenant: null },
        ),
      'GET /api/v1/settings': {
        name: 'Rahim Store', slug: 'rahim-store', legal_name: null, business_type: 'grocery', phone: null, email: null,
        address: null, district: null, bin_number: null, trade_license_no: null, currency_code: 'BDT', timezone: 'Asia/Dhaka',
        locale: 'en', fiscal_year_start_month: 7, invoice_prefix: 'INV', receipt_paper: '80mm', receipt_footer: null,
        auto_print_receipt: true, is_vat_registered: false, prices_include_tax: false, version: 1,
      },
      'PATCH /api/v1/settings': (call) => Response.json({ ...(call.body as object), version: 2 }),
      'POST /api/v1/onboarding/steps/regional': { steps: [], dismissed: false, complete: false },
      'GET /api/v1/branches': [{ id: 'b1', code: 'MAIN', name: 'Main Branch', address: null, phone: null, receipt_header: null, is_default: true, status: 'active', version: 1 }],
    })
    const { router } = renderRoute('/register')

    await userEvent.type(await screen.findByLabelText('Full name'), 'Rahim Uddin')
    await userEvent.type(screen.getByLabelText('Mobile number'), '01711000001')
    await userEvent.type(screen.getByLabelText('Password'), 'correct horse battery')
    await userEvent.click(screen.getByRole('button', { name: 'Continue' }))

    expect(await screen.findByText('Development mode: your code is 123456')).toBeInTheDocument()
    expect(screen.getByText(/\+880\*{7}001/)).toBeInTheDocument()
    await userEvent.type(screen.getByLabelText('6-digit code'), '123456')
    await userEvent.click(screen.getByRole('button', { name: 'Verify' }))

    expect(await screen.findByRole('heading', { name: 'Tell us about your business' })).toBeInTheDocument()
    await userEvent.type(screen.getByLabelText('Business name'), 'Rahim Store')
    await userEvent.selectOptions(screen.getByLabelText('Business type'), 'grocery')
    await userEvent.click(screen.getByRole('button', { name: 'Create business' }))

    expect(await screen.findByRole('heading', { name: 'Regional settings' })).toBeInTheDocument()
    expect(calls.find((c) => c.path === '/api/v1/tenants')?.body).toMatchObject({ name: 'Rahim Store', business_type: 'grocery' })
    const currency = screen.getByLabelText('Currency')
    expect(within(currency).getByRole('option', { name: 'Euro (EUR) — €' })).toBeInTheDocument()
    await userEvent.selectOptions(currency, 'EUR')
    await userEvent.selectOptions(screen.getByLabelText('Fiscal year starts in'), '1')
    await userEvent.click(screen.getByRole('button', { name: 'Save and continue' }))

    expect(await screen.findByRole('heading', { name: 'Your shop' })).toBeInTheDocument()
    expect(router.state.location.pathname).toBe('/onboarding/branch')
    const patch = calls.find((c) => c.method === 'PATCH' && c.path === '/api/v1/settings')
    expect(patch?.headers.get('If-Match')).toBe('"1"')
    expect(patch?.body).toMatchObject({ currency_code: 'EUR', fiscal_year_start_month: 1, timezone: 'Asia/Dhaka' })
  })

  it('shows field errors from the API next to the field', async () => {
    mockApi({
      ...anonymous,
      'POST /api/v1/auth/register': () =>
        problem(400, 'VALIDATION_FAILED', { errors: [{ field: 'password', code: 'PASSWORD_TOO_COMMON', message: 'too common' }] }),
    })
    renderRoute('/register')
    await userEvent.type(await screen.findByLabelText('Full name'), 'Rahim Uddin')
    await userEvent.type(screen.getByLabelText('Mobile number'), '01711000001')
    await userEvent.type(screen.getByLabelText('Password'), 'password123')
    await userEvent.click(screen.getByRole('button', { name: 'Continue' }))
    expect(await screen.findByText('This password is too easy to guess.')).toBeInTheDocument()
    expect(screen.getByLabelText('Password')).toHaveAttribute('aria-invalid', 'true')
  })
})
