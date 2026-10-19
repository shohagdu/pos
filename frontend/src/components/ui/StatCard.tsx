import type { ReactNode } from 'react'

/** A headline number (dashboard, report summaries). */
export function StatCard({ label, value, hint, tone = 'default' }: { label: string; value: ReactNode; hint?: ReactNode; tone?: 'default' | 'good' | 'bad' }) {
  const color = tone === 'good' ? 'text-brand-700' : tone === 'bad' ? 'text-danger' : 'text-ink'
  return (
    <div className="rounded-xl border border-border bg-surface p-4 shadow-sm">
      <div className="text-xs font-medium tracking-wide text-ink-muted uppercase">{label}</div>
      <div className={`mt-1 text-xl font-bold tabular-nums ${color}`}>{value}</div>
      {hint && <div className="mt-1 text-xs text-ink-muted">{hint}</div>}
    </div>
  )
}
