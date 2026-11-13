import { type SelectHTMLAttributes, useId } from 'react'

export interface SelectProps extends Omit<SelectHTMLAttributes<HTMLSelectElement>, 'size'> {
  label: string
  options: { value: string; label: string }[]
  error?: string
  /** `sm`: denser field for long forms that should fit on one screen. */
  size?: 'md' | 'sm'
}

/** A red asterisk after the label (CSS only, so the accessible name stays the label). */
const REQUIRED_MARK = "after:ml-0.5 after:text-danger after:content-['*']"

export function Select({ label, options, error, size = 'md', id, className = '', ...props }: SelectProps) {
  const generated = useId()
  const selectId = id ?? generated
  const errorId = `${selectId}-error`
  const sm = size === 'sm'
  return (
    <div className={`flex flex-col ${sm ? 'gap-1' : 'gap-1.5'} ${className}`}>
      <label htmlFor={selectId} className={`${sm ? 'text-[0.8125rem]' : 'text-sm'} font-medium text-ink ${props.required ? REQUIRED_MARK : ''}`}>
        {label}
      </label>
      <select
        id={selectId}
        aria-invalid={error ? true : undefined}
        aria-describedby={error ? errorId : undefined}
        className={`${sm ? 'min-h-9' : 'min-h-11'} min-w-0 rounded-lg border border-border bg-surface px-3 text-sm outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100 aria-invalid:border-danger`}
        {...props}
      >
        {options.map((o) => (
          <option key={o.value} value={o.value}>
            {o.label}
          </option>
        ))}
      </select>
      {error && (
        <p id={errorId} className={sm ? 'text-xs text-danger' : 'text-sm text-danger'}>
          {error}
        </p>
      )}
    </div>
  )
}
