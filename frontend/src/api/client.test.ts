import { afterEach, describe, expect, it, vi } from 'vitest'

import { ApiError, apiFetch, configureAuth, NetworkError } from './client'

function mockFetch(response: Response | Error) {
  return vi.spyOn(globalThis, 'fetch').mockImplementation(async () => {
    if (response instanceof Error) throw response
    return response
  })
}

afterEach(() => vi.restoreAllMocks())

describe('apiFetch', () => {
  it('returns parsed JSON on success', async () => {
    mockFetch(Response.json({ status: 'ok' }))
    await expect(apiFetch('/healthz')).resolves.toEqual({ status: 'ok' })
  })

  it('sends JSON bodies and the idempotency key', async () => {
    const spy = mockFetch(Response.json({ id: '1' }, { status: 201 }))
    await apiFetch('/api/v1/sales', { method: 'POST', body: { a: 1 }, idempotencyKey: 'key-1' })
    const init = spy.mock.calls[0]?.[1]
    const headers = new Headers(init?.headers)
    expect(headers.get('Idempotency-Key')).toBe('key-1')
    expect(headers.get('Content-Type')).toBe('application/json')
    expect(init?.body).toBe('{"a":1}')
  })

  it('throws ApiError with the problem details', async () => {
    mockFetch(
      new Response(JSON.stringify({ status: 409, code: 'DUPLICATE', title: 'Conflict', detail: 'exists', request_id: 'r1' }), {
        status: 409,
        headers: { 'content-type': 'application/problem+json' },
      }),
    )
    const err = await apiFetch('/api/v1/x').catch((e: unknown) => e)
    expect(err).toBeInstanceOf(ApiError)
    expect((err as ApiError).problem).toMatchObject({ code: 'DUPLICATE', status: 409, request_id: 'r1' })
  })

  it('builds a problem for non-JSON errors (e.g. proxy 502)', async () => {
    mockFetch(new Response('Bad Gateway', { status: 502, statusText: 'Bad Gateway' }))
    const err = (await apiFetch('/api/v1/x').catch((e: unknown) => e)) as ApiError
    expect(err.problem).toMatchObject({ status: 502, code: 'INTERNAL_ERROR' })
  })

  it('wraps transport failures in NetworkError', async () => {
    mockFetch(new TypeError('Failed to fetch'))
    await expect(apiFetch('/healthz')).rejects.toBeInstanceOf(NetworkError)
  })
})

describe('apiFetch authentication', () => {
  afterEach(() => configureAuth({ getToken: () => null, refresh: async () => false }))

  it('sends the bearer token and If-Match', async () => {
    configureAuth({ getToken: () => 'tok', refresh: async () => false })
    const spy = mockFetch(Response.json({}))
    await apiFetch('/api/v1/settings', { method: 'PATCH', body: {}, ifMatch: 4 })
    const headers = new Headers(spy.mock.calls[0]?.[1]?.headers)
    expect(headers.get('Authorization')).toBe('Bearer tok')
    expect(headers.get('If-Match')).toBe('"4"')
  })

  it('refreshes once on an expired token and retries', async () => {
    let current = 'old'
    const refresh = vi.fn(async () => {
      current = 'new'
      return true
    })
    configureAuth({ getToken: () => current, refresh })
    const spy = vi.spyOn(globalThis, 'fetch').mockImplementation(async (_input, init) => {
      const auth = new Headers(init?.headers).get('Authorization')
      if (auth === 'Bearer old') {
        return new Response(JSON.stringify({ status: 401, code: 'TOKEN_EXPIRED', title: 'x', detail: 'x' }), {
          status: 401,
          headers: { 'content-type': 'application/problem+json' },
        })
      }
      return Response.json({ ok: true })
    })
    await expect(apiFetch('/api/v1/auth/me')).resolves.toEqual({ ok: true })
    expect(refresh).toHaveBeenCalledTimes(1)
    expect(spy).toHaveBeenCalledTimes(2)
  })

  it('does not retry credential errors or unauthenticated calls', async () => {
    const refresh = vi.fn(async () => true)
    configureAuth({ getToken: () => 'tok', refresh })
    mockFetch(
      new Response(JSON.stringify({ status: 401, code: 'INVALID_CREDENTIALS', title: 'x', detail: 'x' }), {
        status: 401,
        headers: { 'content-type': 'application/problem+json' },
      }),
    )
    await expect(apiFetch('/api/v1/auth/login', { method: 'POST', body: {}, auth: false })).rejects.toBeInstanceOf(ApiError)
    expect(refresh).not.toHaveBeenCalled()
  })
})
