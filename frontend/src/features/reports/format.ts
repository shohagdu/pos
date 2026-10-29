// Report helpers: default range, money/percent formatting with exact decimals.

import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { formatDate, startOfMonth, today, dateLocale } from '../../lib/dates'
import { dec, div, isZero, mul, sum, toFixed } from '../../lib/decimal'
import { formatMoney } from '../../lib/format'

export interface Range {
  from: string
  to: string
}

/** The report range: start of this month to today, shown as explicit dates. */
export function useRange() {
  return useState<Range>(() => ({ from: startOfMonth(), to: today() }))
}

/** Money or "—" when the value is hidden (no cost permission). */
export function money(value: string | null | undefined, lang: string): string {
  return value === null || value === undefined ? '—' : formatMoney(value, lang)
}

/** "12.50%" from a decimal string, "—" when missing. */
export function pct(value: string | null | undefined): string {
  return value === null || value === undefined ? '—' : `${toFixed(dec(value), 2)}%`
}

/** Share of `part` in `total` as a percentage string (exact decimals). */
export function share(part: string, total: string): string | null {
  const t = dec(total)
  return isZero(t) ? null : toFixed(div(mul(dec(part), dec('100')), t), 2)
}

/** Sum of decimal strings with 2 decimals; `null` when any value is hidden. */
export function total(values: (string | null)[]): string | null {
  if (values.some((v) => v === null)) return null
  return toFixed(sum(values.map((v) => dec(v))), 2)
}

/** Difference `a − b` with 2 decimals. */
export function minus(a: string, b: string): string {
  return toFixed(dec(a) - dec(b), 2)
}

/** "2026-10-01" or "2026-10" (month groups) for display. */
export function formatPeriod(period: string, lang: string): string {
  if (period.length === 7) {
    return new Intl.DateTimeFormat(dateLocale(lang), { month: 'short', year: 'numeric' }).format(new Date(`${period}-01T00:00:00`))
  }
  return formatDate(period, lang)
}

export function usePeriodLabel() {
  const { t, i18n } = useTranslation()
  return (from: string | null | undefined, to: string) =>
    from
      ? t('reports.periodRange', { from: formatDate(from, i18n.language), to: formatDate(to, i18n.language) })
      : t('reports.upTo', { to: formatDate(to, i18n.language) })
}
