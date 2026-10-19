// Shared helpers for list endpoints: query strings and cursor paging.

/** `?a=1&b=x` from an object, skipping empty values. */
export function queryString(params: Record<string, string | number | boolean | null | undefined>): string {
  const q = new URLSearchParams()
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== null && value !== '') q.set(key, String(value))
  }
  const s = q.toString()
  return s ? `?${s}` : ''
}

export interface Page<T> {
  data: T[]
  next_cursor: string | null
}

export const PAGE_SIZE = 50

/** A UUID for an `Idempotency-Key` (one per cart / form submission). */
export function newKey(): string {
  return crypto.randomUUID()
}
