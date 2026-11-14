import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { useSubscription } from '../../api/billing'
import { useBranches } from '../../api/tenancy'
import { useRoles, usersApi, usersKeys } from '../../api/users'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { Spinner } from '../../components/ui/Spinner'
import { TextField } from '../../components/ui/TextField'
import { useFields } from '../../lib/form'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { BranchScopePicker, RolePicker } from './RolePicker'

const EMPTY = { full_name: '', phone: '', email: '', password: '' }

/**
 * Adds staff directly: name, mobile number, a first password and roles. The
 * account signs in at once, without an invitation link. A number that already
 * has an account joins with its own password.
 */
export function InvitePage() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const roles = useRoles(tenantId)
  const branches = useBranches(tenantId)
  const seats = useSubscription(tenantId).data?.usage.find((m) => m.feature === 'users.max')
  const full = seats?.used != null && seats.limit !== null && seats.used >= seats.limit
  const { values, bind, setValues } = useFields(EMPTY)
  const [showPassword, setShowPassword] = useState(false)
  const [roleIds, setRoleIds] = useState<string[]>([])
  const [branchIds, setBranchIds] = useState<string[]>([])
  const add = useMutation({
    mutationFn: () =>
      usersApi.createMember({
        full_name: values.full_name.trim(),
        phone: values.phone.trim(),
        email: values.email.trim() || undefined,
        password: values.password || undefined,
        role_ids: roleIds,
        branch_ids: branchIds,
      }),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: usersKeys.all(tenantId) })
      void queryClient.invalidateQueries({ queryKey: ['tenancy', tenantId, 'onboarding'] })
      void queryClient.invalidateQueries({ queryKey: ['billing', tenantId] })
    },
  })
  const errors = fieldErrors(add.error, t)

  if (!can('users.invite')) return <Alert tone="warning">{t('users.noPermission')}</Alert>
  if (roles.isPending || branches.isPending) return <Spinner />

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    add.mutate()
  }

  if (add.isSuccess) {
    const { member } = add.data
    return (
      <Card className="flex max-w-xl flex-col gap-4">
        {member ? (
          <>
            <h1 className="text-xl font-bold">{t('addMember.doneTitle', { name: member.full_name })}</h1>
            <p className="text-sm">{t('addMember.doneNew', { phone: member.phone ?? values.phone })}</p>
          </>
        ) : (
          // The number already had an account: it was invited, not added.
          <>
            <h1 className="text-xl font-bold">{t('addMember.invitedTitle', { name: values.full_name })}</h1>
            <p className="text-sm">{t('addMember.doneExisting', { phone: values.phone })}</p>
          </>
        )}
        <div className="flex flex-wrap gap-2">
          <Button
            variant="secondary"
            onClick={() => {
              add.reset()
              setValues(EMPTY)
              setRoleIds([])
              setBranchIds([])
            }}
          >
            {t('addMember.another')}
          </Button>
          <Link
            to={member ? `/users/members/${member.id}` : '/users/invitations'}
            className="inline-flex min-h-11 items-center px-4 text-sm font-semibold text-brand-700 hover:underline"
          >
            {member ? t('addMember.viewMember') : t('addMember.viewInvitations')}
          </Link>
        </div>
      </Card>
    )
  }

  return (
    <Card className="max-w-xl">
      <h1 className="mb-2 text-xl font-bold">{t('addMember.title')}</h1>
      <p className="mb-4 text-sm text-ink-muted">{t('addMember.intro')}</p>
      {seats?.used != null && seats.limit !== null && (
        <Alert tone={full ? 'warning' : 'info'} className="mb-6">
          {t('addMember.seats', { used: seats.used, max: seats.limit })}
          {full && <> {t('addMember.seatsFull')}</>}
        </Alert>
      )}
      <form onSubmit={onSubmit} className="flex flex-col gap-5" noValidate>
        <TextField label={t('addMember.fullName')} autoComplete="off" required error={errors.full_name} {...bind('full_name')} />
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <TextField
            label={t('addMember.phone')}
            type="tel"
            inputMode="tel"
            autoComplete="off"
            placeholder="01XXXXXXXXX"
            required
            hint={t('addMember.phoneHint')}
            error={errors.phone}
            {...bind('phone')}
          />
          <TextField label={t('addMember.email')} type="email" autoComplete="off" error={errors.email} {...bind('email')} />
        </div>
        <div className="flex flex-col gap-1">
          <TextField
            label={t('addMember.password')}
            type={showPassword ? 'text' : 'password'}
            autoComplete="new-password"
            required
            hint={t('addMember.passwordHint')}
            error={errors.password}
            {...bind('password')}
          />
          <label className="flex items-center gap-2 text-sm text-ink-muted">
            <input type="checkbox" className="size-4 accent-brand-600" checked={showPassword} onChange={(e) => setShowPassword(e.target.checked)} />
            {t('addMember.showPassword')}
          </label>
        </div>
        <RolePicker roles={roles.data ?? []} selected={roleIds} onChange={setRoleIds} isOwner={me?.tenant?.is_owner ?? false} error={errors.role_ids} />
        <BranchScopePicker branches={branches.data ?? []} selected={branchIds} onChange={setBranchIds} />
        {add.isError && hasFormError(add.error) && <Alert tone="danger">{problemMessage(add.error, t)}</Alert>}
        <div className="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
          <Link to="/users" className="inline-flex min-h-11 items-center justify-center px-4 text-sm font-semibold text-ink-muted hover:text-ink">
            {t('common.cancel')}
          </Link>
          <Button type="submit" disabled={add.isPending || full}>
            {add.isPending ? t('common.pleaseWait') : t('addMember.submit')}
          </Button>
        </div>
      </form>
    </Card>
  )
}
