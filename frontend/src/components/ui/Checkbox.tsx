import { type InputHTMLAttributes, useId } from 'react'

export interface CheckboxProps extends Omit<InputHTMLAttributes<HTMLInputElement>, 'type'> {
  label: string
  hint?: string
}

export function Checkbox({ label, hint, id, className = '', ...props }: CheckboxProps) {
  const generated = useId()
  const inputId = id ?? generated
  return (
    <div className={`flex items-start gap-3 ${className}`}>
      <input
        id={inputId}
        type="checkbox"
        className="mt-0.5 size-5 shrink-0 rounded border-border accent-brand-600"
        aria-describedby={hint ? `${inputId}-hint` : undefined}
        {...props}
      />
      <div className="flex flex-col">
        <label htmlFor={inputId} className="text-sm font-medium text-ink">
          {label}
        </label>
        {hint && (
          <p id={`${inputId}-hint`} className="text-sm text-ink-muted">
            {hint}
          </p>
        )}
      </div>
    </div>
  )
}
