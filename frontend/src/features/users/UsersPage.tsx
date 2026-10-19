import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { Link, NavLink, useParams } from 'react-router'

import { type Invitation, useInvitations, useMembers, useRoles, usersApi, usersKeys } from '../../api/users'
import { Alert } from '../../components/ui/Alert'
import { Card } from '../../components/ui/Card'
import { ConfirmButton } from '../../components/ui/ConfirmButton'
import { Spinner } from '../../components/ui/Spinner'
import { Button } from '../../components/ui/Button'
import { problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { roleName } from './labels'

const TABS = ['members', 'invitations', 'roles'] as const
type Tab = (typeof TABS)[number]

/** Users & roles (E1.10): members, invitations and roles. */
export function UsersPage() {
  const { t } = useTranslation()
  const { tab = 'members' } = useParams()
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const current = (TABS.includes(tab as Tab) ? tab : 'members') as Tab

  return (
    <div className="flex max-w-4xl flex-col gap-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h1 className="text-2xl font-bold">{t('users.title')}</h1>
        <div className="flex gap-2">
          {current === 'roles' && can('users.manage_roles') && (
            <Link to="/users/roles/new" className="inline-flex min-h-11 items-center rounded-lg border border-border bg-surface px-4 text-sm font-semibold hover:bg-canvas">
              {t('roles.new')}
            </Link>
          )}
          {can('users.invite') && (
            <Link to="/users/invite" className="inline-flex min-h-11 items-center rounded-lg bg-brand-600 px-4 text-sm font-semibold text-white hover:bg-brand-700">
              {t('users.invite')}
            </Link>
          )}
        </div>
      </div>
      <nav aria-label={t('users.title')} className="flex flex-wrap gap-1 border-b border-border">
        {TABS.map((s) => (
          <NavLink
            key={s}
            to={s === 'members' ? '/users' : `/users/${s}`}
            end
            className={({ isActive }) =>
              `-mb-px border-b-2 px-3 py-2 text-sm font-medium ${isActive ? 'border-brand-600 text-brand-700' : 'border-transparent text-ink-muted hover:text-ink'}`
            }
          >
            {t(`users.tabs.${s}`)}
          </NavLink>
        ))}
      </nav>
      {current === 'members' && <MembersList tenantId={tenantId} />}
      {current === 'invitations' && <InvitationsList tenantId={tenantId} />}
      {current === 'roles' && <RolesList tenantId={tenantId} />}
    </div>
  )
}

function StatusBadge({ status }: { status: string }) {
  const { t } = useTranslation()
  const tone =
    status === 'active' || status === 'accepted'
      ? 'bg-brand-50 text-brand-700'
      : status === 'pending'
        ? 'bg-warning/15 text-ink'
        : 'bg-canvas text-ink-muted'
  return <span className={`rounded px-2 py-0.5 text-xs font-semibold ${tone}`}>{t(`users.status.${status}`)}</span>
}

function MembersList({ tenantId }: { tenantId: string }) {
  const { t } = useTranslation()
  const members = useMembers(tenantId)
  if (members.isPending) return <Spinner />
  if (members.isError) return <Alert tone="danger">{problemMessage(members.error, t)}</Alert>
  return (
    <ul className="flex flex-col gap-2">
      {members.data.map((m) => (
        <li key={m.id}>
          <Link to={`/users/members/${m.id}`} className="block rounded-xl border border-border bg-surface p-4 hover:border-brand-500">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <span className="font-semibold">
                {m.full_name}
                {m.is_owner && <span className="ml-2 text-xs font-medium text-brand-700">{t('tenant.owner')}</span>}
              </span>
              <StatusBadge status={m.status} />
            </div>
            <div className="mt-1 text-sm text-ink-muted">{[m.phone, m.email].filter(Boolean).join(' · ')}</div>
            <div className="mt-2 flex flex-wrap gap-1">
              {m.roles.map((r) => (
                <span key={`${r.role_id}-${r.branch_id ?? 'all'}`} className="rounded bg-canvas px-2 py-0.5 text-xs">
                  {roleName(t, r)}
                </span>
              ))}
            </div>
          </Link>
        </li>
      ))}
    </ul>
  )
}

function InvitationsList({ tenantId }: { tenantId: string }) {
  const { t, i18n } = useTranslation()
  const queryClient = useQueryClient()
  const { can } = useSession()
  const invitations = useInvitations(tenantId)
  const refresh = () => queryClient.invalidateQueries({ queryKey: usersKeys.invitations(tenantId) })
  const resend = useMutation({ mutationFn: usersApi.resendInvitation, onSuccess: refresh })
  const revoke = useMutation({ mutationFn: usersApi.revokeInvitation, onSuccess: refresh })
  const date = (iso: string) => new Intl.DateTimeFormat(i18n.language === 'bn' ? 'bn-BD' : 'en-GB', { dateStyle: 'medium' }).format(new Date(iso))

  if (invitations.isPending) return <Spinner />
  if (invitations.isError) return <Alert tone="danger">{problemMessage(invitations.error, t)}</Alert>
  if (invitations.data.length === 0) return <p className="text-sm text-ink-muted">{t('invitations.none')}</p>
  const open = (i: Invitation) => i.status === 'pending' || i.status === 'expired'
  return (
    <div className="flex flex-col gap-3">
      {(resend.isError || revoke.isError) && <Alert tone="danger">{problemMessage(resend.error ?? revoke.error, t)}</Alert>}
      {resend.data?.dev_link && <Alert tone="warning">{t('invitations.devLink', { link: resend.data.dev_link })}</Alert>}
      <ul className="flex flex-col gap-2">
        {invitations.data.map((i) => (
          <li key={i.id}>
            <Card className="flex flex-wrap items-center justify-between gap-3 p-4">
              <div className="flex flex-col gap-0.5">
                <span className="font-medium">
                  {i.destination} <StatusBadge status={i.status} />
                </span>
                <span className="text-sm text-ink-muted">{i.roles.map((r) => r.name).join(', ')}</span>
                <span className="text-xs text-ink-muted">
                  {i.status === 'pending'
                    ? t('invitations.expires', { date: date(i.expires_at) })
                    : t('invitations.invitedBy', { name: i.invited_by, date: date(i.created_at) })}
                </span>
              </div>
              {open(i) && can('users.invite') && (
                <div className="flex flex-wrap gap-2">
                  <Button variant="secondary" disabled={resend.isPending} onClick={() => resend.mutate(i.id)}>
                    {t('invitations.resend')}
                  </Button>
                  <ConfirmButton disabled={revoke.isPending} onConfirm={() => revoke.mutate(i.id)}>
                    {t('invitations.revoke')}
                  </ConfirmButton>
                </div>
              )}
            </Card>
          </li>
        ))}
      </ul>
    </div>
  )
}

function RolesList({ tenantId }: { tenantId: string }) {
  const { t } = useTranslation()
  const roles = useRoles(tenantId)
  if (roles.isPending) return <Spinner />
  if (roles.isError) return <Alert tone="danger">{problemMessage(roles.error, t)}</Alert>
  return (
    <ul className="grid gap-2 sm:grid-cols-2">
      {roles.data.map((r) => (
        <li key={r.id}>
          <Link to={`/users/roles/${r.id}`} className="flex h-full flex-col gap-1 rounded-xl border border-border bg-surface p-4 hover:border-brand-500">
            <span className="flex items-center justify-between gap-2 font-semibold">
              {roleName(t, r)}
              <span className="text-xs font-medium text-ink-muted">{r.is_system ? t('roles.system') : t('roles.custom')}</span>
            </span>
            <span className="text-sm text-ink-muted">
              {t('roles.summary', { permissions: r.permissions.length, members: r.member_count })}
            </span>
          </Link>
        </li>
      ))}
    </ul>
  )
}
