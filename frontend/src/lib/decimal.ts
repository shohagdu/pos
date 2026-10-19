// Exact decimal arithmetic for money in the browser (Requirement Analysis
// §8.1: never floating point). Values are BigInt fixed-point numbers with 9
// decimals; rounding is half-up away from zero like the server
// (`MidpointAwayFromZero`).

const SCALE = 9
const ONE = 10n ** BigInt(SCALE)

/** A decimal value: an integer count of 10⁻⁹ units. */
export type Dec = bigint

export const ZERO: Dec = 0n

/** Parses "12.5", "-0.25", "1e3"-free decimal strings; blank or invalid = null. */
export function tryParse(text: string | number | null | undefined): Dec | null {
  if (text === null || text === undefined) return null
  const s = String(text).trim()
  const m = /^([+-])?(\d*)(?:\.(\d*))?$/.exec(s)
  if (!m || (!m[2] && !m[3])) return null
  const sign = m[1] === '-' ? -1n : 1n
  const whole = BigInt(m[2] || '0')
  const frac = (m[3] ?? '').padEnd(SCALE, '0')
  if (frac.length > SCALE) return null
  return sign * (whole * ONE + BigInt(frac || '0'))
}

/** Like [`tryParse`], 0 for blank or invalid input. */
export function dec(text: string | number | null | undefined): Dec {
  return tryParse(text) ?? ZERO
}

function abs(a: Dec): Dec {
  return a < 0n ? -a : a
}

/** Rounds to `dp` decimals, half away from zero. */
export function round(a: Dec, dp: number): Dec {
  const unit = 10n ** BigInt(SCALE - dp)
  const q = abs(a) / unit
  const r = abs(a) % unit
  const rounded = (r * 2n >= unit ? q + 1n : q) * unit
  return a < 0n ? -rounded : rounded
}

export const add = (a: Dec, b: Dec): Dec => a + b
export const sub = (a: Dec, b: Dec): Dec => a - b

export function mul(a: Dec, b: Dec): Dec {
  const p = a * b
  const q = abs(p) / ONE
  const r = abs(p) % ONE
  const v = r * 2n >= ONE ? q + 1n : q
  return p < 0n ? -v : v
}

export function div(a: Dec, b: Dec): Dec {
  if (b === 0n) return 0n
  const n = a * ONE
  const q = abs(n) / abs(b)
  const r = abs(n) % abs(b)
  const v = r * 2n >= abs(b) ? q + 1n : q
  return (n < 0n) !== (b < 0n) ? -v : v
}

export const min = (a: Dec, b: Dec): Dec => (a < b ? a : b)
export const max = (a: Dec, b: Dec): Dec => (a > b ? a : b)
export const sum = (values: Dec[]): Dec => values.reduce((s, v) => s + v, 0n)

/** Formats with exactly `dp` decimals (rounded half-up): `toFixed(dec("1.005"), 2) === "1.01"`. */
export function toFixed(a: Dec, dp = 2): string {
  const r = round(a, dp)
  const sign = r < 0n ? '-' : ''
  const v = abs(r)
  const whole = v / ONE
  if (dp === 0) return `${sign}${whole}`
  const frac = (v % ONE).toString().padStart(SCALE, '0').slice(0, dp)
  return `${sign}${whole}.${frac}`
}

/** Shortest form without trailing zeros: "2.5", "3". */
export function toPlain(a: Dec): string {
  const s = toFixed(a, SCALE)
  return s.includes('.') ? s.replace(/0+$/, '').replace(/\.$/, '') : s
}

export const isZero = (a: Dec) => a === 0n
