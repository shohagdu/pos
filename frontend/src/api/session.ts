// Access token store. The access token lives in memory only; the refresh
// token is an httpOnly cookie the browser sends to /api/v1/auth (D-19).

import { ApiError, apiFetch, configureAuth } from './client'

export interface TokenResponse {
  access_token: string
  token_type: 'Bearer'
  expires_in: number
  tenant_id: string | null
}

export interface SessionSnapshot {
  token: string | null
  tenantId: string | null
}

let snapshot: SessionSnapshot = { token: null, tenantId: null }
const listeners = new Set<() => void>()

export function getSession(): SessionSnapshot {
  return snapshot
}

export function subscribeSession(listener: () => void): () => void {
  listeners.add(listener)
  return () => listeners.delete(listener)
}

export function setTokens(tokens: TokenResponse | null) {
  snapshot = tokens ? { token: tokens.access_token, tenantId: tokens.tenant_id } : { token: null, tenantId: null }
  listeners.forEach((l) => l())
}

let inflight: Promise<boolean> | null = null

/**
 * Exchanges the refresh cookie for a new access token. Concurrent callers
 * share one request; across tabs a Web Lock serialises refreshes, because
 * the cookie is rotated on every use and presenting an old one signs the
 * user out everywhere (reuse detection, AC-AUTH-02).
 */
export function refreshSession(): Promise<boolean> {
  inflight ??= runRefresh().finally(() => {
    inflight = null
  })
  return inflight
}

async function runRefresh(): Promise<boolean> {
  const attempt = async () => {
    try {
      setTokens(await apiFetch<TokenResponse>('/api/v1/auth/refresh', { method: 'POST', auth: false }))
      return true
    } catch (err) {
      if (err instanceof ApiError && (err.status === 401 || err.status === 403)) {
        setTokens(null)
        return false
      }
      throw err
    }
  }
  const locks = typeof navigator !== 'undefined' ? navigator.locks : undefined
  return locks ? locks.request('pos-auth-refresh', attempt) : attempt()
}

// The branch the member works in, remembered per business on this device.
// `null` = no `X-Branch-Id`: the default branch for postings and every
// allowed branch for lists (BRN-R02).

const branchKey = (tenantId: string) => `pos.branch.${tenantId}`
const branchListeners = new Set<() => void>()

export function getBranchId(): string | null {
  if (!snapshot.tenantId) return null
  try {
    return localStorage.getItem(branchKey(snapshot.tenantId))
  } catch {
    return null
  }
}

export function setBranchId(branchId: string | null) {
  if (!snapshot.tenantId) return
  try {
    if (branchId) localStorage.setItem(branchKey(snapshot.tenantId), branchId)
    else localStorage.removeItem(branchKey(snapshot.tenantId))
  } catch {
    // storage unavailable (private mode): the default branch is used
  }
  branchListeners.forEach((l) => l())
}

export function subscribeBranch(listener: () => void): () => void {
  branchListeners.add(listener)
  listeners.add(listener)
  return () => {
    branchListeners.delete(listener)
    listeners.delete(listener)
  }
}

configureAuth({
  getToken: () => snapshot.token,
  refresh: refreshSession,
  getBranch: getBranchId,
  dropBranch: () => setBranchId(null),
})
