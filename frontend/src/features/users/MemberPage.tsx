import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate, useParams } from 'react-router'

import { useBranches } from '../../api/tenancy'
import { type Member, type RoleAssignment, useMember, useMemberSessions, useRoles, usersApi, usersKeys } from '../../api/users'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { ConfirmButton } from '../../components/ui/ConfirmButton'
import { Spinner } from '../../components/ui/Spinner'
import { describeDevice } from '../../lib/device'
import { problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { roleName } from './labels'
import { BranchScopePicker, RolePicker } from './RolePicker'
import { dateLocale } from '../../lib/dates'

/** One member: roles (with branch scope), status changes and their sessions. */
export function MemberPage() {
  const { t } = useTranslation()
  const { id = '' } = useParams()
  const { me } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const member = useMember(tenantId, id)

  if (member.isPending) return <Spinner />
  if (member.isError) return <Alert tone="danger">{problemMessage(member.error, t)}</Alert>
  return <MemberDetail key={member.data.id} member={member.data} tenantId={tenantId} />
}

function MemberDetail({ member, tenantId }: { member: Member; tenantId: string }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const { me, can } = useSession()
  const isSelf = me?.tenant?.membership_id === member.id
  const isAdmin = member.roles.some((r) => r.code === 'admin')
  // The same protections as the API: not yourself, not the owner, admins only by the owner.
  const editable = can('users.manage_roles') && !isSelf && !member.is_owner && (!isAdmin || me?.tenant?.is_owner === true)
  const refresh = () => queryClient.invalidateQueries({ queryKey: usersKeys.all(tenantId) })

  const suspend = useMutation({ mutationFn: () => usersApi.suspend(member.id), onSuccess: refresh })
  const reactivate = useMutation({ mutationFn: () => usersApi.reactivate(member.id), onSuccess: refresh })
  const remove = useMutation({
    mutationFn: () => usersApi.remove(member.id),
    onSuccess: async () => {
      await refresh()
      navigate('/users', { replace: true })
    },
  })
  const failed = suspend.error ?? reactivate.error ?? remove.error

  return (
    <div className="flex max-w-2xl flex-col gap-6">
      <Link to="/users" className="text-sm font-medium text-brand-700 hover:underline">
        ← {t('users.title')}
      </Link>
      <div>
        <h1 className="text-2xl font-bold">{member.full_name}</h1>
        <p className="text-sm text-ink-muted">{[member.phone, member.email].filter(Boolean).join(' · ')}</p>
        <p className="mt-1 text-sm">
          {member.is_owner ? t('tenant.owner') : t(`users.status.${member.status}`)}
          {isSelf && ` · ${t('users.you')}`}
        </p>
      </div>
      {failed && <Alert tone="danger">{problemMessage(failed, t)}</Alert>}
      {editable ? (
        <RolesEditor member={member} tenantId={tenantId} />
      ) : (
        <Card>
          <h2 className="mb-3 font-semibold">{t('users.roles')}</h2>
          <ul className="flex flex-wrap gap-1">
            {member.roles.map((r) => (
              <li key={`${r.role_id}-${r.branch_id ?? 'all'}`} className="rounded bg-canvas px-2 py-0.5 text-sm">
                {roleName(t, r)}
              </li>
            ))}
          </ul>
          {(isSelf || member.is_owner) && <p className="mt-3 text-sm text-ink-muted">{t(isSelf ? 'users.selfProtected' : 'users.ownerProtected')}</p>}
        </Card>
      )}
      {editable && (
        <Card className="flex flex-wrap items-center gap-3">
          {member.status === 'active' && (
            <ConfirmButton disabled={suspend.isPending} onConfirm={() => suspend.mutate()}>
              {t('users.suspend')}
            </ConfirmButton>
          )}
          {member.status === 'suspended' && (
            <Button variant="secondary" disabled={reactivate.isPending} onClick={() => reactivate.mutate()}>
              {t('users.reactivate')}
            </Button>
          )}
          <ConfirmButton disabled={remove.isPending} onConfirm={() => remove.mutate()}>
            {t('users.remove')}
          </ConfirmButton>
        </Card>
      )}
      {can('users.revoke_sessions') && !isSelf && <MemberSessions member={member} tenantId={tenantId} />}
    </div>
  )
}

function RolesEditor({ member, tenantId }: { member: Member; tenantId: string }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const { me } = useSession()
  const roles = useRoles(tenantId)
  const branches = useBranches(tenantId)
  // The editor applies one branch scope to all selected roles.
  const [roleIds, setRoleIds] = useState(() => [...new Set(member.roles.map((r) => r.role_id))])
  const [branchIds, setBranchIds] = useState(() =>
    member.roles.some((r) => r.branch_id === null) ? [] : [...new Set(member.roles.map((r) => r.branch_id).filter((b): b is string => b !== null))],
  )
  const save = useMutation({
    mutationFn: () => {
      const assignments: RoleAssignment[] = roleIds.flatMap((role_id): RoleAssignment[] =>
        branchIds.length === 0 ? [{ role_id, branch_id: null }] : branchIds.map((branch_id) => ({ role_id, branch_id })),
      )
      return usersApi.setRoles(member.id, assignments)
    },
    onSuccess: (saved) => {
      queryClient.setQueryData(usersKeys.member(tenantId, member.id), saved)
      void queryClient.invalidateQueries({ queryKey: usersKeys.members(tenantId) })
    },
  })

  if (roles.isPending || branches.isPending) return <Spinner />
  return (
    <Card className="flex flex-col gap-5">
      <RolePicker roles={roles.data ?? []} selected={roleIds} onChange={setRoleIds} isOwner={me?.tenant?.is_owner ?? false} />
      <BranchScopePicker branches={branches.data ?? []} selected={branchIds} onChange={setBranchIds} />
      {save.isError && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}
      {save.isSuccess && <Alert tone="success">{t('users.rolesSaved')}</Alert>}
      <Button className="sm:self-end" disabled={save.isPending || roleIds.length === 0} onClick={() => save.mutate()}>
        {save.isPending ? t('common.saving') : t('common.save')}
      </Button>
    </Card>
  )
}

function MemberSessions({ member, tenantId }: { member: Member; tenantId: string }) {
  const { t, i18n } = useTranslation()
  const queryClient = useQueryClient()
  const sessions = useMemberSessions(tenantId, member.id, true)
  const revoke = useMutation({
    mutationFn: () => usersApi.revokeMemberSessions(member.id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: usersKeys.memberSessions(tenantId, member.id) }),
  })
  const when = (iso: string) =>
    new Intl.DateTimeFormat(dateLocale(i18n.language), { dateStyle: 'medium', timeStyle: 'short' }).format(new Date(iso))

  return (
    <Card className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h2 className="font-semibold">{t('users.memberSessions')}</h2>
        {(sessions.data?.length ?? 0) > 0 && (
          <ConfirmButton disabled={revoke.isPending} onConfirm={() => revoke.mutate()}>
            {t('users.signOutEverywhere')}
          </ConfirmButton>
        )}
      </div>
      {sessions.isPending && <Spinner />}
      {revoke.isSuccess && <Alert tone="success">{t('users.signedOut', { count: revoke.data.revoked })}</Alert>}
      {sessions.data?.length === 0 && <p className="text-sm text-ink-muted">{t('users.noSessions')}</p>}
      <ul className="flex flex-col divide-y divide-border">
        {sessions.data?.map((s) => (
          <li key={s.id} className="py-2 text-sm">
            <span className="font-medium">{describeDevice(s.user_agent)}</span>
            <span className="text-ink-muted"> · {t('sessions.lastActive', { when: when(s.last_used_at) })}</span>
          </li>
        ))}
      </ul>
    </Card>
  )
}
