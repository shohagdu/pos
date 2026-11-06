import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { currentTenant, membership, mockApi, problem, token, user } from '../../test/api'
import { renderRoute } from '../../test/render'

afterEach(() => vi.restoreAllMocks())

function signedIn() {
  return {
    'POST /api/v1/auth/refresh': token('t1'),
    'GET /api/v1/auth/me': { user, memberships: [membership('t1', 'Rahim Store')], tenant: currentTenant('t1', 'Rahim Store', ['dashboard.view']) },
  }
}

async function fill(current: string, next: string, again = next) {
  await userEvent.type(await screen.findByLabelText(/Current password/), current)
  await userEvent.type(screen.getByLabelText(/^New password/), next)
  await userEvent.type(screen.getByLabelText(/Type the new password again/), again)
  await userEvent.click(screen.getByRole('button', { name: 'Change password' }))
}

describe('change password', () => {
  it('is in the account menu and changes the password', async () => {
    const calls = mockApi({ ...signedIn(), 'POST /api/v1/auth/password/change': { signed_out_sessions: 2 } })
    const { router } = renderRoute('/')
    await userEvent.click(await screen.findByRole('button', { name: 'Account menu' }))
    await userEvent.click(screen.getByRole('link', { name: 'Change password' }))
    await waitFor(() => expect(router.state.location.pathname).toBe('/account/password'))

    await fill('old-pass-123', 'new-pass-456')
    expect(await screen.findByText('Your password was changed and 2 other devices were signed out.')).toBeInTheDocument()
    const post = calls.find((c) => c.path === '/api/v1/auth/password/change')
    expect(post?.body).toEqual({ current_password: 'old-pass-123', new_password: 'new-pass-456' })
    expect(post?.headers.get('Authorization')).toMatch(/^Bearer /)
    expect(screen.getByLabelText(/Current password/)).toHaveValue('')
  })

  it('stops a mistyped new password before sending it', async () => {
    const calls = mockApi(signedIn())
    renderRoute('/account/password')
    await fill('old-pass-123', 'new-pass-456', 'new-pass-457')
    expect(await screen.findByText('The two new passwords do not match.')).toBeInTheDocument()
    expect(calls.some((c) => c.path === '/api/v1/auth/password/change')).toBe(false)
  })

  it('shows a wrong current password on its field', async () => {
    mockApi({
      ...signedIn(),
      'POST /api/v1/auth/password/change': () =>
        problem(400, 'VALIDATION_FAILED', { errors: [{ field: 'current_password', code: 'WRONG_PASSWORD', message: 'wrong' }] }),
    })
    renderRoute('/account/password')
    await fill('nope', 'new-pass-456')
    expect(await screen.findByText('The current password is wrong.')).toBeInTheDocument()
    expect(screen.getByLabelText(/Current password/)).toHaveAttribute('aria-invalid', 'true')
  })
})
