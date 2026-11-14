import type { ReactNode } from 'react'

/** A labelled value in a detail header. */
export function Info({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div className="min-w-0 break-words">
      <div className="text-xs font-medium text-ink-muted uppercase">{label}</div>
      <div>{value}</div>
    </div>
  )
}
