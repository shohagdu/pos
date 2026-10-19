import type { ReactNode } from 'react'

type Tone = 'info' | 'success' | 'warning' | 'danger'

const TONES: Record<Tone, string> = {
  info: 'bg-brand-50 text-brand-700 border-brand-100',
  success: 'bg-brand-50 text-brand-700 border-brand-100',
  warning: 'bg-warning/10 text-ink border-warning/40',
  danger: 'bg-danger/10 text-danger border-danger/30',
}

export function Alert({ tone = 'info', children, className = '' }: { tone?: Tone; children: ReactNode; className?: string }) {
  return (
    <div role={tone === 'danger' ? 'alert' : 'status'} className={`rounded-lg border p-3 text-sm ${TONES[tone]} ${className}`}>
      {children}
    </div>
  )
}
