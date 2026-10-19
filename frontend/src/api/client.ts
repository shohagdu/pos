// Thin fetch wrapper. Errors follow RFC 9457 Problem Details (Requirement
// Analysis API-05); callers switch on `problem.code`.

export interface FieldError {
  field: string
  code: string
  message: string
}

export interface Problem {
  type: string
  title: string
  status: number
  code: string
  detail: string
  errors?: FieldError[]
  request_id?: string
  retry_after?: number
  /** For `402 PLAN_LIMIT_REACHED`. */
  limit?: string
  max?: number
}

export class ApiError extends Error {
  readonly problem: Problem

  constructor(problem: Problem) {
    super(problem.detail || problem.title)
    this.name = 'ApiError'
    this.problem = problem
  }

  get status(): number {
    return this.problem.status
  }
}

/** The request never reached the server (offline, DNS, CORS, proxy down). */
export class NetworkError extends Error {
  constructor(cause: unknown) {
    super('network error', { cause })
    this.name = 'NetworkError'
  }
}

export interface RequestOptions extends Omit<RequestInit, 'body'> {
  body?: unknown
  /** Required by the API for posting endpoints (sales, payments, …). */
  idempotencyKey?: string
  /** Send the access token and refresh it once on 401 (default true). */
  auth?: boolean
  /** Entity version for optimistic concurrency (`If-Match`, API-07). */
  ifMatch?: number
}

interface AuthHooks {
  getToken: () => string | null
  /** Resolves true when a new access token is available. */
  refresh: () => Promise<boolean>
  /** The branch picked in the shell, sent as `X-Branch-Id` (BRN-R02). */
  getBranch?: () => string | null
  /** The picked branch was refused (deactivated, or out of the member's scope). */
  dropBranch?: () => void
}

let authHooks: AuthHooks = { getToken: () => null, refresh: async () => false }

/** Wired by the session store; keeps this module free of session state. */
export function configureAuth(hooks: AuthHooks) {
  authHooks = hooks
}

// 401 codes that a fresh access token can fix.
const REFRESHABLE = new Set(['UNAUTHORIZED', 'TOKEN_EXPIRED', 'TOKEN_STALE', 'TOKEN_INVALID'])

export async function apiFetch<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const { auth = true } = options
  try {
    return await send<T>(path, options)
  } catch (err) {
    const retry = auth && err instanceof ApiError && err.status === 401 && REFRESHABLE.has(err.problem.code)
    if (!retry || !(await authHooks.refresh())) throw err
    return send<T>(path, options)
  }
}

async function send<T>(path: string, options: RequestOptions): Promise<T> {
  const { body, idempotencyKey, headers, auth = true, ifMatch, ...init } = options
  const finalHeaders = new Headers(headers)
  finalHeaders.set('Accept', 'application/json')
  if (body !== undefined) finalHeaders.set('Content-Type', 'application/json')
  if (idempotencyKey) finalHeaders.set('Idempotency-Key', idempotencyKey)
  if (ifMatch !== undefined) finalHeaders.set('If-Match', `"${ifMatch}"`)
  const token = auth ? authHooks.getToken() : null
  if (token) finalHeaders.set('Authorization', `Bearer ${token}`)
  // Sign-in and `/auth/me` never depend on the branch, so a stale one cannot lock the user out.
  const branch = token && !path.startsWith('/api/v1/auth/') ? authHooks.getBranch?.() : null
  if (branch) finalHeaders.set('X-Branch-Id', branch)

  let response: Response
  try {
    response = await fetch(path, {
      ...init,
      headers: finalHeaders,
      body: body === undefined ? undefined : JSON.stringify(body),
      credentials: 'same-origin',
    })
  } catch (cause) {
    throw new NetworkError(cause)
  }

  if (!response.ok) {
    const problem = await readProblem(response)
    // Never retried: a posting must not land in another branch than the one shown.
    if (problem.code === 'BRANCH_NOT_ALLOWED' && branch) authHooks.dropBranch?.()
    throw new ApiError(problem)
  }
  if (response.status === 204) return undefined as T
  return (await response.json()) as T
}

async function readProblem(response: Response): Promise<Problem> {
  const fallback: Problem = {
    type: 'about:blank',
    title: response.statusText || 'Error',
    status: response.status,
    code: response.status >= 500 ? 'INTERNAL_ERROR' : 'HTTP_ERROR',
    detail: response.statusText,
    request_id: response.headers.get('x-request-id') ?? undefined,
  }
  const type = response.headers.get('content-type') ?? ''
  if (!type.includes('json')) return fallback
  try {
    return { ...fallback, ...((await response.json()) as Partial<Problem>) }
  } catch {
    return fallback
  }
}
