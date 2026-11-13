import { type InputHTMLAttributes, useId } from 'react'

import { InfoTip } from './InfoTip'

export interface CheckboxProps extends Omit<InputHTMLAttributes<HTMLInputElement>, 'type'> {
  label: string
  hint?: string
  /** Help text behind an ⓘ next to the label instead of a line under it. */
  tip?: string
  tipAlign?: 'start' | 'end'
}

export function Checkbox({ label, hint, tip, tipAlign, id, className = '', ...props }: CheckboxProps) {
  const generated = useId()
  const inputId = id ?? generated
  return (
    <div className={`flex items-start gap-3 ${className}`}>
      <input
        id={inputId}
        type="checkbox"
        className="mt-0.5 size-5 shrink-0 rounded border-border accent-brand-600 disabled:cursor-not-allowed disabled:opacity-50"
        aria-describedby={hint ? `${inputId}-hint` : tip ? `${inputId}-tip` : undefined}
        {...props}
      />
      <div className="flex flex-col">
        <div className="flex items-center gap-0.5">
          <label htmlFor={inputId} className={`text-sm font-medium ${props.disabled ? 'text-ink-muted' : 'text-ink'}`}>
            {label}
          </label>
          {tip && <InfoTip id={`${inputId}-tip`} text={tip} align={tipAlign} />}
        </div>
        {hint && (
          <p id={`${inputId}-hint`} className="text-sm text-ink-muted">
            {hint}
          </p>
        )}
      </div>
    </div>
  )
}
