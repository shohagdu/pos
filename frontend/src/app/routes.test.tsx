import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'

import i18n from '../i18n'
import { currentTenant, membership, mockApi, problem, token, user } from '../test/api'
import { renderRoute } from '../test/render'

afterEach(async () => {
  vi.restoreAllMocks()
  await i18n.changeLanguage('en')
})

const signedInWithTenant = {
  'POST /api/v1/auth/refresh': token('t1'),
  'GET /api/v1/auth/me': { user, memberships: [membership('t1', 'Rahim Store')], tenant: currentTenant('t1', 'Rahim Store') },
  'GET /api/v1/onboarding': { steps: [{ key: 'business', done: true }, { key: 'regional', done: false }], dismissed: false, complete: false },
  'GET /readyz': { status: 'ready' },
  'GET /api/v1/meta': { name: 'pos-api', version: '0.1.0', environment: 'test' },
}

describe('routes', () => {
  it('sends visitors without a session to sign-in', async () => {
    mockApi({ 'POST /api/v1/auth/refresh': () => problem(401, 'SESSION_INVALID') })
    const { router } = renderRoute('/')
    expect(await screen.findByRole('heading', { name: 'Sign in' })).toBeInTheDocument()
    expect(router.state.location.pathname).toBe('/login')
  })

  it('restores the session from the refresh cookie and shows the dashboard', async () => {
    mockApi(signedInWithTenant)
    renderRoute('/')
    expect(await screen.findByRole('heading', { name: /^Good (morning|afternoon|evening), Rahim$/ })).toBeInTheDocument()
    expect(await screen.findByText('0.1.0')).toBeInTheDocument()
    expect(screen.getAllByText('Rahim Store').length).toBeGreaterThan(0)
    expect(screen.queryByText('Free trial: 10 days left')).not.toBeInTheDocument()
    expect(await screen.findByText('Finish setting up')).toBeInTheDocument()
  })

  it('reports the database as unavailable when readiness fails', async () => {
    mockApi({ ...signedInWithTenant, 'GET /readyz': () => problem(503, 'SERVICE_UNAVAILABLE') })
    renderRoute('/')
    expect(await screen.findByText('Unavailable')).toBeInTheDocument()
  })

  it('hides menu entries the member may not use', async () => {
    mockApi({
      ...signedInWithTenant,
      'GET /api/v1/auth/me': { user, memberships: [membership('t1', 'Rahim Store')], tenant: currentTenant('t1', 'Rahim Store', ['pos.sell']) },
    })
    renderRoute('/')
    expect(await screen.findByRole('link', { name: 'Dashboard' })).toBeInTheDocument()
    expect(screen.queryByRole('link', { name: 'Settings' })).not.toBeInTheDocument()
    expect(screen.queryByText('Finish setting up')).not.toBeInTheDocument()
  })

  it('switches the interface to Bengali', async () => {
    mockApi({ 'POST /api/v1/auth/refresh': () => problem(401, 'SESSION_INVALID') })
    renderRoute('/login')
    await userEvent.selectOptions(await screen.findByRole('combobox'), 'bn')
    expect(await screen.findByRole('heading', { name: 'সাইন ইন' })).toBeInTheDocument()
    expect(document.documentElement.lang).toBe('bn')
  })

  it('renders the not-found screen for unknown paths', async () => {
    mockApi({ 'POST /api/v1/auth/refresh': () => problem(401, 'SESSION_INVALID') })
    renderRoute('/does-not-exist')
    expect(await screen.findByText('Page not found')).toBeInTheDocument()
  })

  it('sends signed-in users without a business to onboarding', async () => {
    mockApi({
      'POST /api/v1/auth/refresh': token(),
      'GET /api/v1/auth/me': { user, memberships: [], tenant: null },
    })
    const { router } = renderRoute('/')
    expect(await screen.findByRole('heading', { name: 'Tell us about your business' })).toBeInTheDocument()
    await waitFor(() => expect(router.state.location.pathname).toBe('/onboarding/business'))
  })
})
