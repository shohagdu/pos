import { type ChangeEvent, useState } from 'react'

type Values = Record<string, string | boolean>

/** Minimal controlled-form state: `bind('name')` for inputs/selects, `bindCheck` for checkboxes. */
export function useFields<T extends Values>(initial: T) {
  const [values, setValues] = useState<T>(initial)
  const bind = (name: keyof T & string) => ({
    name,
    value: String(values[name] ?? ''),
    onChange: (e: ChangeEvent<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>) =>
      setValues((v) => ({ ...v, [name]: e.target.value })),
  })
  const bindCheck = (name: keyof T & string) => ({
    name,
    checked: Boolean(values[name]),
    onChange: (e: ChangeEvent<HTMLInputElement>) => setValues((v) => ({ ...v, [name]: e.target.checked })),
  })
  return { values, setValues, bind, bindCheck }
}

/** Empty strings become `undefined` so optional fields are left out of the request. */
export function optional(value: string): string | undefined {
  const trimmed = value.trim()
  return trimmed === '' ? undefined : trimmed
}
