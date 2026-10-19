import { useMutation } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate } from 'react-router'

import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { problemMessage } from '../../lib/problem'
import { useSession } from './session'

/** R-TEN-01: a user in several businesses picks the active one. */
export function ChooseBusinessPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { me, switchTenant, signOut } = useSession()
  const choose = useMutation({
    mutationFn: switchTenant,
    onSuccess: () => navigate('/', { replace: true }),
  })
  const memberships = me?.memberships ?? []

  return (
    <Card>
      <h1 className="mb-2 text-xl font-bold">{t('tenant.chooseTitle')}</h1>
      <p className="mb-6 text-sm text-ink-muted">{t('tenant.chooseIntro', { name: me?.user.full_name ?? '' })}</p>
      <ul className="flex flex-col gap-2">
        {memberships.map((m) => (
          <li key={m.tenant_id}>
            <button
              type="button"
              disabled={choose.isPending}
              onClick={() => choose.mutate(m.tenant_id)}
              className={`flex w-full items-center justify-between gap-3 rounded-lg border px-4 py-3 text-left hover:border-brand-500 hover:bg-brand-50 disabled:opacity-60 ${
                me?.tenant?.tenant_id === m.tenant_id ? 'border-brand-500' : 'border-border'
              }`}
            >
              <span className="flex flex-col">
                <span className="font-semibold">{m.tenant_name}</span>
                <span className="text-xs text-ink-muted">{m.is_owner ? t('tenant.owner') : t('tenant.member')}</span>
              </span>
              {m.tenant_status === 'suspended' && <span className="text-xs font-medium text-danger">{t('tenant.suspended')}</span>}
            </button>
          </li>
        ))}
      </ul>
      {choose.isError && <Alert tone="danger" className="mt-4">{problemMessage(choose.error, t)}</Alert>}
      <div className="mt-6 flex flex-wrap items-center justify-between gap-2 text-sm">
        <Link to="/onboarding/business" className="font-semibold text-brand-700 hover:underline">
          {t('tenant.createAnother')}
        </Link>
        <Button variant="ghost" onClick={() => void signOut()}>
          {t('auth.signOut')}
        </Button>
      </div>
    </Card>
  )
}
