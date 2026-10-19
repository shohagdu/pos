import { useMutation } from '@tanstack/react-query'
import type { FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useLocation, useNavigate } from 'react-router'

import { authApi, type OtpChallenge } from '../../api/auth'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { TextField } from '../../components/ui/TextField'
import { useFields } from '../../lib/form'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'

/** US-AUTH-03 step 2: set a new password with the code. Signs out every device. */
export function ResetPasswordPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const state = (useLocation().state as { identifier?: string; challenge?: OtpChallenge } | null) ?? {}
  const { values, bind } = useFields({ identifier: state.identifier ?? '', code: '', new_password: '' })
  const reset = useMutation({
    mutationFn: () => authApi.resetPassword(values.identifier, values.code, values.new_password),
    onSuccess: () => navigate('/login', { replace: true, state: { notice: 'passwordReset' } }),
  })
  const errors = fieldErrors(reset.error, t)

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    reset.mutate()
  }

  return (
    <Card>
      <h1 className="mb-2 text-xl font-bold">{t('auth.resetTitle')}</h1>
      <p className="mb-6 text-sm text-ink-muted">
        {state.challenge ? t('auth.codeSentTo', { destination: state.challenge.destination }) : t('auth.resetIntro')}
      </p>
      {state.challenge?.dev_code && (
        <Alert tone="warning" className="mb-4">
          {t('auth.devCode', { code: state.challenge.dev_code })}
        </Alert>
      )}
      <form onSubmit={onSubmit} className="flex flex-col gap-4" noValidate>
        {!state.identifier && (
          <TextField label={t('auth.identifier')} autoComplete="username" required error={errors.identifier} {...bind('identifier')} />
        )}
        <TextField label={t('auth.code')} inputMode="numeric" autoComplete="one-time-code" maxLength={6} required error={errors.code} {...bind('code')} />
        <TextField
          label={t('auth.newPassword')}
          type="password"
          autoComplete="new-password"
          required
          error={errors.new_password}
          hint={t('auth.passwordHint')}
          {...bind('new_password')}
        />
        {reset.isError && hasFormError(reset.error) && <Alert tone="danger">{problemMessage(reset.error, t)}</Alert>}
        <Button type="submit" disabled={reset.isPending}>
          {reset.isPending ? t('common.pleaseWait') : t('auth.setPassword')}
        </Button>
      </form>
      <p className="mt-6 text-sm">
        <Link to="/forgot-password" className="font-semibold text-brand-700 hover:underline">
          {t('auth.resendCode')}
        </Link>
      </p>
    </Card>
  )
}
