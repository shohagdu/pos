import { useMutation } from '@tanstack/react-query'
import type { FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useLocation, useNavigate } from 'react-router'

import { authApi } from '../../api/auth'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { TextField } from '../../components/ui/TextField'
import { optional, useFields } from '../../lib/form'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'

/** Onboarding step 1 (Requirement Analysis §7.2): the owner's account. */
export function RegisterPage() {
  const { t, i18n } = useTranslation()
  const navigate = useNavigate()
  // Where to continue after verification (e.g. back to an invitation).
  const next = (useLocation().state as { from?: string } | null)?.from
  const { values, bind } = useFields({ full_name: '', phone: '', email: '', password: '' })
  const register = useMutation({
    mutationFn: () =>
      authApi.register({
        full_name: values.full_name,
        phone: values.phone,
        email: optional(values.email),
        password: values.password,
        locale: i18n.language,
      }),
    onSuccess: (challenge) => navigate('/verify', { state: { phone: values.phone, challenge, next } }),
  })
  const errors = fieldErrors(register.error, t)

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    register.mutate()
  }

  return (
    <Card>
      <p className="mb-1 text-sm font-medium text-brand-700">{t('onboarding.stepOf', { step: 1, total: 5 })}</p>
      <h1 className="mb-6 text-xl font-bold">{t('auth.signUp')}</h1>
      <form onSubmit={onSubmit} className="flex flex-col gap-4" noValidate>
        <TextField label={t('auth.fullName')} autoComplete="name" required error={errors.full_name} {...bind('full_name')} />
        <TextField
          label={t('auth.mobile')}
          type="tel"
          inputMode="tel"
          autoComplete="tel"
          placeholder="01XXXXXXXXX"
          required
          error={errors.phone}
          {...bind('phone')}
        />
        <TextField label={t('auth.emailOptional')} type="email" autoComplete="email" error={errors.email} {...bind('email')} />
        <TextField
          label={t('auth.password')}
          type="password"
          autoComplete="new-password"
          required
          error={errors.password}
          hint={t('auth.passwordHint')}
          {...bind('password')}
        />
        {register.isError && hasFormError(register.error) && <Alert tone="danger">{problemMessage(register.error, t)}</Alert>}
        <Button type="submit" disabled={register.isPending}>
          {register.isPending ? t('common.pleaseWait') : t('auth.continue')}
        </Button>
      </form>
      <p className="mt-6 text-sm text-ink-muted">
        {t('auth.haveAccount')}{' '}
        <Link to="/login" state={next ? { from: next } : undefined} className="font-semibold text-brand-700 hover:underline">
          {t('auth.signIn')}
        </Link>
      </p>
    </Card>
  )
}
