// Before/after comparison of audit record payloads.

export type ChangeKind = 'added' | 'removed' | 'changed' | 'same'

export interface DiffRow {
  key: string
  before: unknown
  after: unknown
  kind: ChangeKind
}

export function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

/**
 * Key-by-key diff when both sides are objects (or one side is missing);
 * `null` when the values are not objects and need a plain JSON view.
 */
export function diffObjects(before: unknown, after: unknown): DiffRow[] | null {
  const b = before ?? null
  const a = after ?? null
  if ((b !== null && !isPlainObject(b)) || (a !== null && !isPlainObject(a)) || (b === null && a === null)) return null
  const left = (b ?? {}) as Record<string, unknown>
  const right = (a ?? {}) as Record<string, unknown>
  const keys = [...new Set([...Object.keys(left), ...Object.keys(right)])].sort()
  return keys.map((key) => {
    const inLeft = key in left
    const inRight = key in right
    const kind: ChangeKind = !inLeft ? 'added' : !inRight ? 'removed' : JSON.stringify(left[key]) === JSON.stringify(right[key]) ? 'same' : 'changed'
    return { key, before: left[key], after: right[key], kind }
  })
}

/** Readable text for one value: strings as they are, everything else as JSON. */
export function showValue(value: unknown): string {
  if (value === undefined) return ''
  if (typeof value === 'string') return value
  return JSON.stringify(value, null, 2)
}
