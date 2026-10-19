import { type InputHTMLAttributes, useId } from 'react'

export interface TextFieldProps extends InputHTMLAttributes<HTMLInputElement> {
  label: string
  error?: string
  /** Help text shown under the field while there is no error. */
  hint?: string
}

export function TextField({ label, error, hint, id, className = '', ...props }: TextFieldProps) {
  const generated = useId()
  const inputId = id ?? generated
  const errorId = `${inputId}-error`
  const hintId = `${inputId}-hint`
  const describedBy = error ? errorId : hint ? hintId : undefined
  return (
    <div className={`flex flex-col gap-1.5 ${className}`}>
      <label htmlFor={inputId} className="text-sm font-medium text-ink">
        {label}
      </label>
      <input
        id={inputId}
        aria-invalid={error ? true : undefined}
        aria-describedby={describedBy}
        className="min-h-11 rounded-lg border border-border bg-surface px-3 text-sm outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100 aria-invalid:border-danger disabled:bg-canvas disabled:text-ink-muted"
        {...props}
      />
      {error ? (
        <p id={errorId} className="text-sm text-danger">
          {error}
        </p>
      ) : (
        hint && (
          <p id={hintId} className="text-sm text-ink-muted">
            {hint}
          </p>
        )
      )}
    </div>
  )
}
