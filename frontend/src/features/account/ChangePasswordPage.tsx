import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { ApiError } from '../../api/client'
import { authApi, authKeys } from '../../api/auth'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { TextField } from '../../components/ui/TextField'
import { useFields } from '../../lib/form'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'

const EMPTY = { current_password: '', new_password: '', confirm: '' }

/** The signed-in user changes their own password; other devices are signed out. */
export function ChangePasswordPage() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const { values, bind, setValues } = useFields(EMPTY)
  const [show, setShow] = useState(false)
  const change = useMutation({
    mutationFn: () => {
      // Checked here so a typo never reaches the server as the new password.
      if (values.new_password !== values.confirm) {
        throw new ApiError({
          type: 'urn:pos:problem:validation_failed',
          title: 'Bad Request',
          status: 400,
          code: 'VALIDATION_FAILED',
          detail: '',
          errors: [{ field: 'confirm', code: 'PASSWORD_MISMATCH', message: '' }],
        })
      }
      return authApi.changePassword(values.current_password, values.new_password)
    },
    onSuccess: () => {
      setValues(EMPTY)
      void queryClient.invalidateQueries({ queryKey: authKeys.sessions })
    },
  })
  const errors = fieldErrors(change.error, t)

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    change.mutate()
  }

  const type = show ? 'text' : 'password'
  return (
    <div className="flex max-w-xl flex-col gap-6">
      <div>
        <h1 className="text-2xl font-bold">{t('password.title')}</h1>
        <p className="text-sm text-ink-muted">{t('password.intro')}</p>
      </div>
      {change.isSuccess && (
        <Alert tone="success">
          {change.data.signed_out_sessions > 0 ? t('password.doneSignedOut', { count: change.data.signed_out_sessions }) : t('password.done')}
        </Alert>
      )}
      <Card>
        <form onSubmit={onSubmit} className="flex flex-col gap-5" noValidate>
          <TextField
            label={t('password.current')}
            type={type}
            autoComplete="current-password"
            required
            error={errors.current_password}
            {...bind('current_password')}
          />
          <TextField
            label={t('password.new')}
            type={type}
            autoComplete="new-password"
            required
            hint={t('auth.passwordHint')}
            error={errors.new_password}
            {...bind('new_password')}
          />
          <TextField label={t('password.confirm')} type={type} autoComplete="new-password" required error={errors.confirm} {...bind('confirm')} />
          <label className="flex items-center gap-2 text-sm text-ink-muted">
            <input type="checkbox" className="size-4 accent-brand-600" checked={show} onChange={(e) => setShow(e.target.checked)} />
            {t('password.show')}
          </label>
          {change.isError && hasFormError(change.error) && <Alert tone="danger">{problemMessage(change.error, t)}</Alert>}
          <div className="flex flex-col-reverse gap-2 sm:flex-row sm:items-center sm:justify-between">
            <Link to="/account/sessions" className="text-sm font-semibold text-brand-700 hover:underline">
              {t('sessions.title')}
            </Link>
            <Button type="submit" disabled={change.isPending || !values.current_password || !values.new_password || !values.confirm}>
              {change.isPending ? t('common.pleaseWait') : t('password.submit')}
            </Button>
          </div>
        </form>
      </Card>
    </div>
  )
}
