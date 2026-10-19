// Turns API errors into translated messages. Clients switch on `code`
// (API-05); the server's English `detail` is only a fallback.

import type { TFunction } from 'i18next'

import { ApiError, NetworkError } from '../api/client'

function retryParams(err: ApiError) {
  const seconds = err.problem.retry_after ?? 60
  return { minutes: Math.max(1, Math.ceil(seconds / 60)) }
}

/** One message for the whole form (top-level error). */
export function problemMessage(err: unknown, t: TFunction): string {
  if (err instanceof NetworkError) return t('errors.network')
  if (err instanceof ApiError) {
    if (err.status >= 500) return t('errors.unexpected')
    return t(`problems.${err.problem.code}`, {
      defaultValue: err.problem.detail || t('errors.unexpected'),
      ...retryParams(err),
      detail: err.problem.detail,
      max: err.problem.max,
      what: err.problem.limit ? t(`planLimits.${err.problem.limit.replace('.', '_')}`, { defaultValue: err.problem.limit }) : '',
    })
  }
  return t('errors.unexpected')
}

/** Field-level messages keyed by field name (`errors[]` of a 400). */
export function fieldErrors(err: unknown, t: TFunction): Record<string, string> {
  if (!(err instanceof ApiError)) return {}
  const out: Record<string, string> = {}
  for (const e of err.problem.errors ?? []) {
    out[e.field] = t(`problems.${e.code}`, { defaultValue: e.message })
  }
  return out
}

/** Whether a form-level alert is needed (field errors are shown inline). */
export function hasFormError(err: unknown): boolean {
  return !(err instanceof ApiError && (err.problem.errors?.length ?? 0) > 0)
}
