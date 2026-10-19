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
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { BranchScopePicker, RolePicker } from './RolePicker'

/** US-AUTH-04: invite staff by phone or email with roles. */
export function InvitePage() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const roles = useRoles(tenantId)
  const branches = useBranches(tenantId)
  const seats = useSubscription(tenantId).data?.usage.find((m) => m.feature === 'users.max')
  const full = seats?.used != null && seats.limit !== null && seats.used >= seats.limit
  const [destination, setDestination] = useState('')
  const [roleIds, setRoleIds] = useState<string[]>([])
  const [branchIds, setBranchIds] = useState<string[]>([])
  const invite = useMutation({
    mutationFn: () => usersApi.invite(destination, roleIds, branchIds),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: usersKeys.all(tenantId) })
      void queryClient.invalidateQueries({ queryKey: ['tenancy', tenantId, 'onboarding'] })
    },
  })
  const errors = fieldErrors(invite.error, t)

  if (!can('users.invite')) return <Alert tone="warning">{t('users.noPermission')}</Alert>
  if (roles.isPending || branches.isPending) return <Spinner />

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    invite.mutate()
  }

  if (invite.isSuccess) {
    const sent = invite.data.invitation
    return (
      <Card className="flex max-w-xl flex-col gap-4">
        <h1 className="text-xl font-bold">{t('invitations.sentTitle')}</h1>
        <p className="text-sm">{t('invitations.sentBody', { destination: sent.destination })}</p>
        {invite.data.dev_link && (
          <Alert tone="warning">
            {t('invitations.devLinkIntro')}{' '}
            <a href={invite.data.dev_link} className="break-all font-mono underline">
              {invite.data.dev_link}
            </a>
          </Alert>
        )}
        <div className="flex flex-wrap gap-2">
          <Button
            variant="secondary"
            onClick={() => {
              invite.reset()
              setDestination('')
              setRoleIds([])
            }}
          >
            {t('invitations.inviteAnother')}
          </Button>
          <Link to="/users/invitations" className="inline-flex min-h-11 items-center px-4 text-sm font-semibold text-brand-700 hover:underline">
            {t('invitations.viewAll')}
          </Link>
        </div>
      </Card>
    )
  }

  return (
    <Card className="max-w-xl">
      <h1 className="mb-2 text-xl font-bold">{t('invitations.title')}</h1>
      <p className="mb-4 text-sm text-ink-muted">{t('invitations.intro')}</p>
      {seats?.used != null && seats.limit !== null && (
        <Alert tone={full ? 'warning' : 'info'} className="mb-6">
          {t('invitations.seats', { used: seats.used, max: seats.limit })}
          {full && <> {t('invitations.seatsFull')}</>}
        </Alert>
      )}
      <form onSubmit={onSubmit} className="flex flex-col gap-5" noValidate>
        <TextField
          label={t('invitations.destination')}
          value={destination}
          onChange={(e) => setDestination(e.target.value)}
          placeholder="01XXXXXXXXX"
          required
          error={errors.destination}
        />
        <RolePicker roles={roles.data ?? []} selected={roleIds} onChange={setRoleIds} isOwner={me?.tenant?.is_owner ?? false} error={errors.role_ids} />
        <BranchScopePicker branches={branches.data ?? []} selected={branchIds} onChange={setBranchIds} />
        {invite.isError && hasFormError(invite.error) && <Alert tone="danger">{problemMessage(invite.error, t)}</Alert>}
        <div className="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
          <Link to="/users" className="inline-flex min-h-11 items-center justify-center px-4 text-sm font-semibold text-ink-muted hover:text-ink">
            {t('common.cancel')}
          </Link>
          <Button type="submit" disabled={invite.isPending || full}>
            {invite.isPending ? t('common.pleaseWait') : t('invitations.send')}
          </Button>
        </div>
      </form>
    </Card>
  )
}
