import type { HTMLAttributes } from 'react'

/**
 * A white panel. Default padding is `p-4` on phones, `p-6` from sm; a padding class in `className`
 * (`p-5`, `p-3.5`, `p-0`…) replaces it rather than competing with it in the
 * stylesheet.
 */
export function Card({ className = '', ...props }: HTMLAttributes<HTMLDivElement>) {
  const padded = /(^|\s)p-/.test(className)
  return <div className={`rounded-xl border border-border bg-surface shadow-sm ${padded ? '' : 'p-4 sm:p-6'} ${className}`} {...props} />
}
