import type { ReactNode } from 'react'

type Tone = 'neutral' | 'good' | 'warn' | 'bad' | 'info'

const TONES: Record<Tone, string> = {
  neutral: 'bg-canvas text-ink-muted',
  good: 'bg-brand-50 text-brand-700',
  warn: 'bg-warning/15 text-ink',
  bad: 'bg-danger/10 text-danger',
  info: 'bg-brand-100 text-brand-700',
}

export function Badge({ tone = 'neutral', children }: { tone?: Tone; children: ReactNode }) {
  return <span className={`inline-block rounded px-2 py-0.5 text-xs font-semibold whitespace-nowrap ${TONES[tone]}`}>{children}</span>
}
