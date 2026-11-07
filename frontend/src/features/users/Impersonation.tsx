import { useMutation } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'

import type { Member } from '../../api/users'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { useCanImpersonate } from './canImpersonate'

/** "Sign in as" for the owner, like switching business but to another person. */
export function ImpersonateButton({ member, compact = false }: { member: Member; compact?: boolean }) {
  const { t } = useTranslation()
  const { impersonate } = useSession()
  const navigate = useNavigate()
  const allowed = useCanImpersonate(member)
  const start = useMutation({
    mutationFn: () => impersonate(member.id),
    onSuccess: () => navigate('/', { replace: true }),
  })
  if (!allowed) return null
  return (
    <div className="flex flex-col gap-2">
      <Button
        variant="secondary"
        disabled={start.isPending}
        onClick={() => start.mutate()}
        title={t('users.impersonateHint', { name: member.full_name })}
        className={compact ? 'min-h-9 px-3' : ''}
      >
        {compact ? t('users.impersonateShort') : t('users.impersonate')}
      </Button>
      {start.isError && <Alert tone="danger">{problemMessage(start.error, t)}</Alert>}
    </div>
  )
}

/** Shown on every page while the owner is signed in as a staff member. */
export function ImpersonationBanner() {
  const { t } = useTranslation()
  const { me, stopImpersonation } = useSession()
  const navigate = useNavigate()
  const support = Boolean(me?.impersonator?.platform_support)
  const stop = useMutation({
    mutationFn: stopImpersonation,
    // Platform support goes back to the console it came from.
    onSuccess: () => navigate(support ? '/admin' : '/users', { replace: true }),
  })
  if (!me?.impersonator) return null
  return (
    <div role="status" className="flex flex-wrap items-center justify-center gap-x-3 gap-y-1 bg-warning/20 px-4 py-2 text-center text-sm text-ink sm:px-8">
      <span>
        <strong>
          {support ? t('users.supportSession', { business: me.tenant?.name ?? '' }) : t('users.impersonating', { name: me.user.full_name })}
        </strong>
        <span className="hidden sm:inline">
          {' · '}
          {support
            ? t('users.supportSessionBy', { staff: me.impersonator.full_name, name: me.user.full_name })
            : t('users.impersonatingBy', { owner: me.impersonator.full_name, name: me.user.full_name })}
        </span>
      </span>
      <button
        type="button"
        disabled={stop.isPending}
        onClick={() => stop.mutate()}
        className="rounded-md bg-ink px-3 py-1 text-xs font-semibold text-surface hover:opacity-90 disabled:opacity-60"
      >
        {support ? t('users.returnToAdmin') : t('users.stopImpersonating')}
      </button>
      {stop.isError && <span className="w-full text-danger">{problemMessage(stop.error, t)}</span>}
    </div>
  )
}
