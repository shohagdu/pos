/**
 * Expiry dates as spreadsheets write them, to `YYYY-MM-DD`.
 *
 * - `2027-03-31`, `2027/3/31`
 * - `31/03/2027`, `31-03-27`, `31.03.2027`: day first, as in Bangladesh and
 *   Italy; `03/31/2027` is read month first because 31 cannot be a month.
 * - `03/2027`, `2027-03`, `Mar 2027`, `MAR-27`: month and year only, as
 *   printed on packs, mean the last day of that month.
 * - `31 Mar 2027`, `31-Mar-27`.
 */

const MONTHS = ['jan', 'feb', 'mar', 'apr', 'may', 'jun', 'jul', 'aug', 'sep', 'oct', 'nov', 'dec']

const pad = (n: number) => String(n).padStart(2, '0')

function fullYear(y: number, digits: number): number {
  return digits <= 2 ? 2000 + y : y
}

function lastDay(year: number, month: number): number {
  return new Date(Date.UTC(year, month, 0)).getUTCDate()
}

function iso(year: number, month: number, day: number): string | null {
  if (year < 1900 || year > 2200 || month < 1 || month > 12 || day < 1 || day > lastDay(year, month)) return null
  return `${year}-${pad(month)}-${pad(day)}`
}

function monthOf(name: string): number | null {
  const i = MONTHS.indexOf(name.slice(0, 3).toLowerCase())
  return i < 0 ? null : i + 1
}

/** `''` for a blank cell, the ISO date, or `null` when the text is not a date. */
export function parseExpiry(text: string): string | null {
  const t = text.trim()
  if (!t) return ''
  let m = /^(\d{4})[-/.](\d{1,2})[-/.](\d{1,2})$/.exec(t)
  if (m) return iso(Number(m[1]), Number(m[2]), Number(m[3]))
  m = /^(\d{4})[-/.](\d{1,2})$/.exec(t)
  if (m) {
    const [y, mo] = [Number(m[1]), Number(m[2])]
    return mo >= 1 && mo <= 12 ? iso(y, mo, lastDay(y, mo)) : null
  }
  m = /^(\d{1,2})[-/.](\d{1,2})[-/.](\d{2}|\d{4})$/.exec(t)
  if (m) {
    const [a, b] = [Number(m[1]), Number(m[2])]
    const y = fullYear(Number(m[3]), m[3]!.length)
    // Day first unless the second part cannot be a month.
    return b > 12 && a <= 12 ? iso(y, a, b) : iso(y, b, a)
  }
  m = /^(\d{1,2})[-/.](\d{4})$/.exec(t)
  if (m) {
    const [mo, y] = [Number(m[1]), Number(m[2])]
    return mo >= 1 && mo <= 12 ? iso(y, mo, lastDay(y, mo)) : null
  }
  m = /^(\d{1,2})[\s\-/.]+([a-z]{3,9})[\s\-/.,]+(\d{2}|\d{4})$/i.exec(t)
  if (m) {
    const mo = monthOf(m[2]!)
    return mo ? iso(fullYear(Number(m[3]), m[3]!.length), mo, Number(m[1])) : null
  }
  m = /^([a-z]{3,9})[\s\-/.,]+(\d{2}|\d{4})$/i.exec(t)
  if (m) {
    const mo = monthOf(m[1]!)
    if (!mo) return null
    const y = fullYear(Number(m[2]), m[2]!.length)
    return iso(y, mo, lastDay(y, mo))
  }
  return null
}
