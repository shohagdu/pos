import { type InputHTMLAttributes, useId } from 'react'

import { InfoTip } from './InfoTip'

export interface TextFieldProps extends Omit<InputHTMLAttributes<HTMLInputElement>, 'size'> {
  label: string
  error?: string
  /** Help text shown under the field while there is no error. */
  hint?: string
  /** Help text behind an ⓘ next to the label (read with the field while there is no error). */
  tip?: string
  /** `sm`: denser field for long forms that should fit on one screen. */
  size?: 'md' | 'sm'
  /** Label and input on one row: `true` = a narrow input, `'wide'` = the input takes the rest of the row. */
  inline?: boolean | 'wide'
  /** Fixed text inside the input on the left, e.g. the currency symbol. */
  prefix?: string
}

/** A red asterisk after the label (CSS only, so the accessible name stays the label). */
const REQUIRED_MARK = "after:ml-0.5 after:text-danger after:content-['*']"

export function TextField({ label, error, hint, tip, size = 'md', inline = false, prefix, id, className = '', ...props }: TextFieldProps) {
  const generated = useId()
  const inputId = id ?? generated
  const errorId = `${inputId}-error`
  const hintId = `${inputId}-hint`
  const tipId = `${inputId}-tip`
  const describedBy = error ? errorId : hint ? hintId : tip ? tipId : undefined
  const sm = size === 'sm'
  const labelRow = (
    <div className="flex min-w-0 items-center gap-0.5">
      <label htmlFor={inputId} className={`${sm ? 'text-[0.8125rem]' : 'text-sm'} font-medium text-ink ${props.required ? REQUIRED_MARK : ''}`}>
        {label}
      </label>
      {tip && <InfoTip id={tipId} text={tip} />}
    </div>
  )
  const field = (
    <input
      id={inputId}
      aria-invalid={error ? true : undefined}
      aria-describedby={describedBy}
      className={`${sm ? 'min-h-9' : 'min-h-11'} ${inline === 'wide' ? 'w-full' : inline ? 'w-24 shrink-0' : 'w-full'} ${prefix ? 'pr-3 pl-8' : 'px-3'} rounded-lg border border-border bg-surface text-sm outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100 aria-invalid:border-danger disabled:bg-canvas disabled:text-ink-muted`}
      {...props}
    />
  )
  const input = prefix ? (
    <div className="relative">
      <span aria-hidden="true" className="pointer-events-none absolute inset-y-0 left-3 flex items-center text-sm text-ink-muted">
        {prefix}
      </span>
      {field}
    </div>
  ) : (
    field
  )
  return (
    <div className={`flex flex-col ${sm ? 'gap-1' : 'gap-1.5'} ${className}`}>
      {inline ? (
        <div className="flex items-center justify-between gap-2">
          {inline === 'wide' ? <div className="w-24 shrink-0">{labelRow}</div> : labelRow}
          {inline === 'wide' ? <div className="min-w-0 flex-1">{input}</div> : input}
        </div>
      ) : (
        <>
          {labelRow}
          {input}
        </>
      )}
      {error ? (
        <p id={errorId} className={sm ? 'text-xs text-danger' : 'text-sm text-danger'}>
          {error}
        </p>
      ) : (
        hint && (
          <p id={hintId} className={sm ? 'text-xs text-ink-muted' : 'text-sm text-ink-muted'}>
            {hint}
          </p>
        )
      )}
    </div>
  )
}
