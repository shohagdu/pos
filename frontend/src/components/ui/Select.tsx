import { type SelectHTMLAttributes, useId } from 'react'

export interface SelectProps extends SelectHTMLAttributes<HTMLSelectElement> {
  label: string
  options: { value: string; label: string }[]
  error?: string
}

/** A red asterisk after the label (CSS only, so the accessible name stays the label). */
const REQUIRED_MARK = "after:ml-0.5 after:text-danger after:content-['*']"

export function Select({ label, options, error, id, className = '', ...props }: SelectProps) {
  const generated = useId()
  const selectId = id ?? generated
  const errorId = `${selectId}-error`
  return (
    <div className={`flex flex-col gap-1.5 ${className}`}>
      <label htmlFor={selectId} className={`text-sm font-medium text-ink ${props.required ? REQUIRED_MARK : ''}`}>
        {label}
      </label>
      <select
        id={selectId}
        aria-invalid={error ? true : undefined}
        aria-describedby={error ? errorId : undefined}
        className="min-h-11 rounded-lg border border-border bg-surface px-3 text-sm outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100 aria-invalid:border-danger"
        {...props}
      >
        {options.map((o) => (
          <option key={o.value} value={o.value}>
            {o.label}
          </option>
        ))}
      </select>
      {error && (
        <p id={errorId} className="text-sm text-danger">
          {error}
        </p>
      )}
    </div>
  )
}
