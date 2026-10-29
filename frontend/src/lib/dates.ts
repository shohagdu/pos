// Business dates are plain `YYYY-MM-DD` strings in the tenant's timezone
// (REP-R02); these helpers build the usual report ranges from the browser's
// local date, which matches the shop's day for Bangladesh businesses.

function pad(n: number): string {
  return String(n).padStart(2, '0')
}

export function isoDate(d: Date): string {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`
}

export function today(): string {
  return isoDate(new Date())
}

export function daysAgo(n: number): string {
  const d = new Date()
  d.setDate(d.getDate() - n)
  return isoDate(d)
}

export function startOfMonth(): string {
  const d = new Date()
  return isoDate(new Date(d.getFullYear(), d.getMonth(), 1))
}

export function startOfYear(): string {
  return `${new Date().getFullYear()}-01-01`
}

export type RangePreset = 'today' | 'yesterday' | 'last7' | 'thisMonth' | 'lastMonth' | 'thisYear'

export function presetRange(preset: RangePreset): { from: string; to: string } {
  const now = new Date()
  switch (preset) {
    case 'today':
      return { from: today(), to: today() }
    case 'yesterday':
      return { from: daysAgo(1), to: daysAgo(1) }
    case 'last7':
      return { from: daysAgo(6), to: today() }
    case 'thisMonth':
      return { from: startOfMonth(), to: today() }
    case 'lastMonth': {
      const first = new Date(now.getFullYear(), now.getMonth() - 1, 1)
      const last = new Date(now.getFullYear(), now.getMonth(), 0)
      return { from: isoDate(first), to: isoDate(last) }
    }
    case 'thisYear':
      return { from: startOfYear(), to: today() }
  }
}

const DATE_LOCALES: Record<string, string> = { en: 'en-GB', bn: 'bn-BD', it: 'it-IT' }

/** The Intl locale for dates in a UI language. */
export function dateLocale(lang: string): string {
  return DATE_LOCALES[lang] ?? 'en-GB'
}

/** Formats an ISO date or timestamp for display in the UI language. */
export function formatDate(value: string | null | undefined, lang = 'en'): string {
  if (!value) return '—'
  const date = value.length === 10 ? new Date(`${value}T00:00:00`) : new Date(value)
  return new Intl.DateTimeFormat(dateLocale(lang), { day: '2-digit', month: 'short', year: 'numeric' }).format(date)
}

export function formatDateTime(value: string | null | undefined, lang = 'en'): string {
  if (!value) return '—'
  return new Intl.DateTimeFormat(dateLocale(lang), {
    day: '2-digit',
    month: 'short',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  }).format(new Date(value))
}
