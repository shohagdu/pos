import { vi } from 'vitest'

export interface Call {
  method: string
  path: string
  /** Query string, e.g. `?q=rice&limit=50`. */
  search: string
  body: unknown
  headers: Headers
}

type Handler = (call: Call) => Response | Promise<Response>

/** JSON problem response (RFC 9457) like the API returns. */
export function problem(status: number, code: string, extra: Record<string, unknown> = {}): Response {
  return new Response(JSON.stringify({ type: 'about:blank', title: 'Error', status, code, detail: code, ...extra }), {
    status,
    headers: { 'content-type': 'application/problem+json' },
  })
}

/**
 * Routes `fetch` to handlers keyed by `"METHOD /path"`. A plain value is
 * returned as JSON; unknown routes get a 404 problem. Returns the call log.
 */
export function mockApi(routes: Record<string, Handler | object>): Call[] {
  const calls: Call[] = []
  vi.spyOn(globalThis, 'fetch').mockImplementation(async (input, init) => {
    const url = new URL(String(input), 'http://localhost')
    const call: Call = {
      method: (init?.method ?? 'GET').toUpperCase(),
      path: url.pathname,
      search: url.search,
      body: init?.body ? JSON.parse(String(init.body)) : undefined,
      headers: new Headers(init?.headers),
    }
    calls.push(call)
    const handler = routes[`${call.method} ${call.path}`]
    if (handler === undefined) return problem(404, 'NOT_FOUND')
    return typeof handler === 'function' ? (handler as Handler)(call) : Response.json(handler)
  })
  return calls
}

export const token = (tenantId: string | null = null) => ({
  access_token: `token-${tenantId ?? 'none'}`,
  token_type: 'Bearer',
  expires_in: 900,
  tenant_id: tenantId,
})

export const user = { id: 'u1', full_name: 'Rahim Uddin', phone: '+8801711000001', email: null, phone_verified: true, locale: 'en' }

export const membership = (id: string, name: string) => ({
  tenant_id: id,
  membership_id: `m-${id}`,
  tenant_name: name,
  tenant_slug: name.toLowerCase().replace(/ /g, '-'),
  tenant_status: 'active',
  is_owner: true,
})

export const currentTenant = (id: string, name: string, permissions = ['dashboard.view', 'settings.view', 'settings.manage']) => ({
  tenant_id: id,
  membership_id: `m-${id}`,
  name,
  slug: name.toLowerCase(),
  is_owner: true,
  permissions,
  subscription: {
    status: 'trialing',
    stored_status: 'trialing',
    plan_code: 'trial',
    plan_name: 'Free Trial',
    trial_ends_at: new Date(Date.now() + 10 * 864e5).toISOString(),
    current_period_end: null,
    days_left: 10,
  },
  features: ['pos.core', 'reports.export'],
})
