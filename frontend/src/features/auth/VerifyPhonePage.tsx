import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, Navigate, useLocation } from 'react-router'

import { authApi, type OtpChallenge } from '../../api/auth'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { TextField } from '../../components/ui/TextField'
import { useFields } from '../../lib/form'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { landingPath, useSession } from './session'

interface VerifyState {
  phone?: string
  challenge?: OtpChallenge
  /** Continue here after verification instead of the usual landing page. */
  next?: string
}

/** Confirms the mobile number with the SMS code (AUTH-R04) and signs in. */
export function VerifyPhonePage() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const { me, signIn } = useSession()
  const state = (useLocation().state as VerifyState | null) ?? {}
  const knownPhone = state.phone ?? me?.user.phone ?? ''
  const { values, bind } = useFields({ phone: knownPhone, code: '' })
  const [challenge, setChallenge] = useState<OtpChallenge | undefined>(state.challenge)
  const [verified, setVerified] = useState(false)

  const verify = useMutation({
    mutationFn: () => authApi.verifyPhone(values.phone, values.code),
    onSuccess: async (tokens) => {
      signIn(tokens)
      await queryClient.invalidateQueries({ queryKey: ['auth', 'me'] })
      setVerified(true)
    },
  })
  const resend = useMutation({ mutationFn: () => authApi.resendVerification(values.phone), onSuccess: setChallenge })
  const errors = fieldErrors(verify.error ?? resend.error, t)

  if (me?.user.phone_verified && (verified || !state.phone)) return <Navigate to={state.next ?? landingPath(me)} replace />

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    verify.mutate()
  }

  return (
    <Card>
      <p className="mb-1 text-sm font-medium text-brand-700">{t('onboarding.stepOf', { step: 1, total: 5 })}</p>
      <h1 className="mb-2 text-xl font-bold">{t('auth.verifyTitle')}</h1>
      <p className="mb-6 text-sm text-ink-muted">
        {challenge ? t('auth.codeSentTo', { destination: challenge.destination }) : t('auth.verifyIntro')}
      </p>
      {challenge?.dev_code && (
        <Alert tone="warning" className="mb-4">
          {t('auth.devCode', { code: challenge.dev_code })}
        </Alert>
      )}
      <form onSubmit={onSubmit} className="flex flex-col gap-4" noValidate>
        {!knownPhone && (
          <TextField label={t('auth.mobile')} type="tel" autoComplete="tel" required error={errors.phone} {...bind('phone')} />
        )}
        <TextField
          label={t('auth.code')}
          inputMode="numeric"
          autoComplete="one-time-code"
          maxLength={6}
          pattern="[0-9]*"
          required
          error={errors.code}
          {...bind('code')}
        />
        {verify.isError && hasFormError(verify.error) && <Alert tone="danger">{problemMessage(verify.error, t)}</Alert>}
        {resend.isError && hasFormError(resend.error) && <Alert tone="danger">{problemMessage(resend.error, t)}</Alert>}
        <Button type="submit" disabled={verify.isPending || verified}>
          {verify.isPending ? t('common.pleaseWait') : t('auth.verify')}
        </Button>
        <Button variant="ghost" disabled={resend.isPending || !values.phone} onClick={() => resend.mutate()}>
          {t('auth.resendCode')}
        </Button>
      </form>
      {!me && (
        <p className="mt-6 text-sm text-ink-muted">
          <Link to="/register" className="font-semibold text-brand-700 hover:underline">
            {t('auth.changeNumber')}
          </Link>
        </p>
      )}
    </Card>
  )
}
