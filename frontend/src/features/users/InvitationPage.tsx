import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { Link, useLocation, useNavigate } from 'react-router'

import { usersApi } from '../../api/users'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { Spinner } from '../../components/ui/Spinner'
import { problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'

/** Public `/invite#<token>`: the token sits in the fragment, so it never reaches a server log. */
export function InvitationPage() {
  const { t } = useTranslation()
  const location = useLocation()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const { status, me, switchTenant, signOut } = useSession()
  const token = location.hash.replace(/^#/, '')
  const here = `/invite${location.hash}`
  const preview = useQuery({
    queryKey: ['invitation', token],
    queryFn: () => usersApi.previewInvitation(token),
    enabled: token !== '',
    retry: false,
  })
  const accept = useMutation({
    mutationFn: async () => {
      const joined = await usersApi.acceptInvitation(token)
      await switchTenant(joined.tenant_id)
      await queryClient.invalidateQueries({ queryKey: ['auth', 'me'] })
    },
    onSuccess: () => navigate('/', { replace: true }),
  })

  if (!token) return <Card><Alert tone="danger">{t('invitation.missing')}</Alert></Card>
  if (preview.isPending || status === 'loading') return <Card><Spinner /></Card>
  if (preview.isError) return <Card><Alert tone="danger">{t('invitation.notFound')}</Alert></Card>
  const inv = preview.data
  const open = inv.status === 'pending'

  return (
    <Card className="flex flex-col gap-4">
      <h1 className="text-xl font-bold">{t('invitation.title', { business: inv.business_name })}</h1>
      <p className="text-sm text-ink-muted">
        {t('invitation.body', { inviter: inv.invited_by, business: inv.business_name, roles: inv.roles.join(', ') })}
      </p>
      <p className="text-sm">{t('invitation.sentTo', { destination: inv.destination })}</p>
      {!open && <Alert tone="warning">{t(`invitation.status.${inv.status}`)}</Alert>}
      {open && status !== 'authenticated' && (
        <div className="flex flex-col gap-2">
          <p className="text-sm">{t('invitation.signInFirst')}</p>
          <Link
            to="/login"
            state={{ from: here }}
            className="inline-flex min-h-11 items-center justify-center rounded-lg bg-brand-600 px-4 text-sm font-semibold text-white hover:bg-brand-700"
          >
            {t('invitation.haveAccount')}
          </Link>
          <Link
            to="/register"
            state={{ from: here }}
            className="inline-flex min-h-11 items-center justify-center rounded-lg border border-border px-4 text-sm font-semibold hover:bg-canvas"
          >
            {t('invitation.newAccount')}
          </Link>
        </div>
      )}
      {open && status === 'authenticated' && me && (
        <div className="flex flex-col gap-3">
          <p className="text-sm">{t('invitation.signedInAs', { name: me.user.full_name })}</p>
          {!me.user.phone_verified && (
            <Link to="/verify" state={{ next: here }} className="text-sm font-semibold text-brand-700 hover:underline">
              {t('invitation.verifyFirst')}
            </Link>
          )}
          {accept.isError && <Alert tone="danger">{problemMessage(accept.error, t)}</Alert>}
          <Button disabled={accept.isPending} onClick={() => accept.mutate()}>
            {accept.isPending ? t('common.pleaseWait') : t('invitation.accept', { business: inv.business_name })}
          </Button>
          <Button variant="ghost" onClick={() => void signOut()}>
            {t('invitation.notYou')}
          </Button>
        </div>
      )}
    </Card>
  )
}
