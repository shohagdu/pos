import type { ButtonHTMLAttributes } from 'react'

type Variant = 'primary' | 'secondary' | 'dark' | 'ghost'

const VARIANTS: Record<Variant, string> = {
  primary: 'bg-brand-600 text-white hover:bg-brand-700 disabled:bg-brand-600/50',
  secondary: 'bg-surface text-ink border border-border hover:bg-canvas',
  /** A second main action next to a primary one (e.g. Save & Add Product). */
  dark: 'bg-ink text-white hover:bg-ink/85 disabled:bg-ink/50',
  ghost: 'text-ink-muted hover:bg-canvas hover:text-ink',
}

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant
  /** `sm`: 36px tall, for toolbars and dense headers (default 44px). */
  size?: 'md' | 'sm'
}

export function Button({ variant = 'primary', size = 'md', className = '', type = 'button', ...props }: ButtonProps) {
  return (
    <button
      type={type}
      className={`inline-flex ${size === 'sm' ? 'min-h-9 px-3' : 'min-h-11 px-4'} items-center justify-center gap-2 rounded-lg text-sm font-semibold transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand-600 disabled:cursor-not-allowed ${VARIANTS[variant]} ${className}`}
      {...props}
    />
  )
}
