import { useMutation } from '@tanstack/react-query'
import type { FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useLocation } from 'react-router'

import { authApi } from '../../api/auth'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { TextField } from '../../components/ui/TextField'
import { useFields } from '../../lib/form'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { useSession } from './session'

export function LoginPage() {
  const { t } = useTranslation()
  const { signIn } = useSession()
  const location = useLocation()
  const notice = (location.state as { notice?: string } | null)?.notice
  const { values, bind } = useFields({ identifier: '', password: '' })
  // GuestOnly redirects once the session is loaded.
  const login = useMutation({ mutationFn: () => authApi.login(values.identifier, values.password), onSuccess: signIn })
  const errors = fieldErrors(login.error, t)

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    login.mutate()
  }

  return (
    <Card>
      <h1 className="mb-6 text-xl font-bold">{t('auth.signIn')}</h1>
      {notice === 'passwordReset' && <Alert tone="success" className="mb-4">{t('auth.passwordResetDone')}</Alert>}
      <form onSubmit={onSubmit} className="flex flex-col gap-4" noValidate>
        <TextField label={t('auth.identifier')} autoComplete="username" required error={errors.identifier} {...bind('identifier')} />
        <TextField label={t('auth.password')} type="password" autoComplete="current-password" required {...bind('password')} />
        <div className="-mt-2 text-right text-sm">
          <Link to="/forgot-password" className="font-medium text-brand-700 hover:underline">
            {t('auth.forgotPassword')}
          </Link>
        </div>
        {login.isError && hasFormError(login.error) && <Alert tone="danger">{problemMessage(login.error, t)}</Alert>}
        <Button type="submit" disabled={login.isPending}>
          {login.isPending ? t('common.pleaseWait') : t('auth.signIn')}
        </Button>
      </form>
      <p className="mt-6 text-sm text-ink-muted">
        {t('auth.noAccount')}{' '}
        <Link to="/register" state={location.state} className="font-semibold text-brand-700 hover:underline">
          {t('auth.signUp')}
        </Link>
      </p>
    </Card>
  )
}
