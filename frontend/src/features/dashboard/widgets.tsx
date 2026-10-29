import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { Icon, type IconName } from '../../app/layouts/Icon'
import { dateLocale } from '../../lib/dates'

// Dashboard-only outline icons (24×24, current colour), kept here so the
// shared icon set stays untouched.
const LOCAL_ICONS = {
  alert: 'M12 4 2.5 20h19zM12 10v4M12 17h.01',
  check: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM8 12.5l2.5 2.5L16 9.5',
  empty: 'M3 7l9-4 9 4v10l-9 4-9-4zM3 7l9 4 9-4M12 11v10M7.5 5l9 4',
  arrowRight: 'M5 12h14M13 6l6 6-6 6',
  receivable: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM12 7v10M9.5 9.5c0-1 1-1.8 2.5-1.8s2.5.8 2.5 1.8-1 1.6-2.5 1.9-2.5.8-2.5 1.9 1 1.9 2.5 1.9 2.5-.8 2.5-1.9',
  profit: 'M3 17l6-6 4 4 8-8M15 7h6v6',
  invoice: 'M7 3h10v18l-2.5-1.5L12 21l-2.5-1.5L7 21zM10 8h4M10 12h4M10 16h2',
} as const

export type DashIcon = IconName | keyof typeof LOCAL_ICONS

export function DIcon({ name, className = 'size-5' }: { name: DashIcon; className?: string }) {
  if (name in LOCAL_ICONS) {
    return (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.8} strokeLinecap="round" strokeLinejoin="round" className={`shrink-0 ${className}`} aria-hidden="true">
        <path d={LOCAL_ICONS[name as keyof typeof LOCAL_ICONS]} />
      </svg>
    )
  }
  return <Icon name={name as IconName} className={className} />
}

export type Tone = 'default' | 'good' | 'warn' | 'bad'

const VALUE_TONE: Record<Tone, string> = {
  default: 'text-ink',
  good: 'text-brand-700',
  warn: 'text-ink',
  bad: 'text-danger',
}

const ICON_TONE: Record<Tone, string> = {
  default: 'bg-brand-50 text-brand-700',
  good: 'bg-success/15 text-success',
  warn: 'bg-warning/20 text-ink',
  bad: 'bg-danger/10 text-danger',
}

/** A headline number with an icon chip (dashboard KPI). */
export function KpiCard({ icon, label, value, hint, tone = 'default' }: { icon: DashIcon; label: string; value: ReactNode; hint?: ReactNode; tone?: Tone }) {
  return (
    <div className="flex h-full items-start gap-3 rounded-xl border border-border bg-surface p-4 shadow-sm">
      <span className={`flex size-10 shrink-0 items-center justify-center rounded-lg ${ICON_TONE[tone]}`}>
        <DIcon name={icon} />
      </span>
      <div className="min-w-0">
        <div className="text-xs font-medium text-ink-muted">{label}</div>
        <div className={`mt-0.5 truncate text-lg font-bold tabular-nums sm:text-xl ${VALUE_TONE[tone]}`}>{value}</div>
        {hint && <div className="mt-0.5 text-xs text-ink-muted">{hint}</div>}
      </div>
    </div>
  )
}

/** One scannable, clickable row in an attention/balances list. */
export function ListLink({ to, icon, label, detail, value, tone = 'default' }: { to: string; icon: DashIcon; label: string; detail?: string; value: ReactNode; tone?: Tone }) {
  return (
    <li>
      <Link
        to={to}
        className="flex min-h-14 items-center gap-3 rounded-lg px-2 py-2 hover:bg-canvas focus-visible:outline-2 focus-visible:outline-brand-600"
      >
        <span className={`flex size-9 shrink-0 items-center justify-center rounded-lg ${ICON_TONE[tone]}`}>
          <DIcon name={icon} className="size-[18px]" />
        </span>
        <span className="flex min-w-0 flex-1 flex-col">
          <span className="text-sm font-medium">{label}</span>
          {detail && <span className="text-xs text-ink-muted">{detail}</span>}
        </span>
        <span className={`text-sm font-bold tabular-nums ${VALUE_TONE[tone]}`}>{value}</span>
        <DIcon name="expand" className="size-4 text-ink-muted" />
      </Link>
    </li>
  )
}

export interface QuickAction {
  key: string
  to: string
  icon: DashIcon
  primary?: boolean
}

/** Large tap targets for the day's most common tasks. */
export function QuickActions({ actions }: { actions: QuickAction[] }) {
  const { t } = useTranslation()
  if (actions.length === 0) return null
  return (
    <nav aria-label={t('dashboard.quickActions')}>
      <ul className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-6">
        {actions.map((a) => (
          <li key={a.key}>
            <Link
              to={a.to}
              className={`flex min-h-20 flex-col justify-between gap-2 rounded-xl p-3 text-sm font-semibold shadow-sm focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand-600 ${
                a.primary ? 'bg-brand-600 text-white hover:bg-brand-700' : 'border border-border bg-surface text-ink hover:border-brand-500 hover:bg-brand-50'
              }`}
            >
              <span className={`flex size-9 items-center justify-center rounded-lg ${a.primary ? 'bg-white/15' : 'bg-brand-50 text-brand-700'}`}>
                <DIcon name={a.icon} />
              </span>
              <span>{t(`dashboard.actions.${a.key}`)}</span>
            </Link>
          </li>
        ))}
      </ul>
    </nav>
  )
}

export interface StartStep {
  key: string
  to: string
}

/** Shown to a shop that has not made a sale yet. */
export function GettingStarted({ steps }: { steps: StartStep[] }) {
  const { t, i18n } = useTranslation()
  const locale = dateLocale(i18n.language)
  if (steps.length === 0) return null
  return (
    <section aria-labelledby="dash-start" className="rounded-xl border border-brand-100 bg-brand-50 p-4 sm:p-5">
      <h2 id="dash-start" className="font-semibold">
        {t('dashboard.start.title')}
      </h2>
      <p className="mt-1 text-sm text-ink-muted">{t('dashboard.start.subtitle')}</p>
      <ol className="mt-4 grid gap-3 md:grid-cols-3">
        {steps.map((s, i) => (
          <li key={s.key} className="flex flex-col gap-2 rounded-lg border border-border bg-surface p-3">
            <span className="flex items-center gap-2">
              <span aria-hidden className="flex size-6 items-center justify-center rounded-full bg-brand-600 text-xs font-bold text-white">
                {(i + 1).toLocaleString(locale)}
              </span>
              <span className="text-sm font-semibold">{t(`dashboard.start.steps.${s.key}.title`)}</span>
            </span>
            <span className="text-xs text-ink-muted">{t(`dashboard.start.steps.${s.key}.body`)}</span>
            <Link to={s.to} className="mt-auto inline-flex items-center gap-1 text-sm font-semibold text-brand-700 hover:underline">
              {t(`dashboard.start.steps.${s.key}.cta`)}
              <DIcon name="arrowRight" className="size-4" />
            </Link>
          </li>
        ))}
      </ol>
    </section>
  )
}
