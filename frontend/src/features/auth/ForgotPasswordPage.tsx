import { useMutation } from '@tanstack/react-query'
import type { FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate } from 'react-router'

import { authApi } from '../../api/auth'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { TextField } from '../../components/ui/TextField'
import { useFields } from '../../lib/form'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'

/** US-AUTH-03 step 1: request a reset code. */
export function ForgotPasswordPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { values, bind } = useFields({ identifier: '' })
  const forgot = useMutation({
    mutationFn: () => authApi.forgotPassword(values.identifier),
    onSuccess: (challenge) => navigate('/reset-password', { state: { identifier: values.identifier, challenge } }),
  })
  const errors = fieldErrors(forgot.error, t)

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    forgot.mutate()
  }

  return (
    <Card>
      <h1 className="mb-2 text-xl font-bold">{t('auth.forgotTitle')}</h1>
      <p className="mb-6 text-sm text-ink-muted">{t('auth.forgotIntro')}</p>
      <form onSubmit={onSubmit} className="flex flex-col gap-4" noValidate>
        <TextField label={t('auth.identifier')} autoComplete="username" required error={errors.identifier} {...bind('identifier')} />
        {forgot.isError && hasFormError(forgot.error) && <Alert tone="danger">{problemMessage(forgot.error, t)}</Alert>}
        <Button type="submit" disabled={forgot.isPending}>
          {forgot.isPending ? t('common.pleaseWait') : t('auth.sendCode')}
        </Button>
      </form>
      <p className="mt-6 text-sm">
        <Link to="/login" className="font-semibold text-brand-700 hover:underline">
          {t('auth.backToSignIn')}
        </Link>
      </p>
    </Card>
  )
}
