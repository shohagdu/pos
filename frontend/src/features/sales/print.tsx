import type { ReactNode } from 'react'

/** Only visible when printing (the receipt). */
export function PrintOnly({ children }: { children: ReactNode }) {
  return <div className="hidden print:block">{children}</div>
}

/** Page content that must not print. */
export function ScreenOnly({ children, className = '' }: { children: ReactNode; className?: string }) {
  return <div className={`print:hidden ${className}`}>{children}</div>
}
