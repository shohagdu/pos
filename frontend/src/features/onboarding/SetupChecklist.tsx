import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { type OnboardingStep, tenancyApi, tenancyKeys, useOnboarding } from '../../api/tenancy'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { useSession } from '../auth/session'
import { CHECKLIST } from './steps'

/** ONB-R03: stays on the dashboard until complete or dismissed. */
export function SetupChecklist() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const manage = can('settings.manage')
  const status = useOnboarding(tenantId, manage && tenantId !== '')
  const update = (data: unknown) => queryClient.setQueryData(tenancyKeys.onboarding(tenantId), data)
  const skip = useMutation({ mutationFn: (step: OnboardingStep) => tenancyApi.completeStep(step), onSuccess: update })
  const dismiss = useMutation({ mutationFn: tenancyApi.dismissOnboarding, onSuccess: update })

  if (!manage || !status.data || status.data.dismissed || status.data.complete) return null
  const done = new Set(status.data.steps.filter((s) => s.done).map((s) => s.key))

  return (
    <Card className="max-w-xl">
      <div className="mb-4 flex items-start justify-between gap-4">
        <div>
          <h2 className="font-semibold">{t('checklist.title')}</h2>
          <p className="text-sm text-ink-muted">{t('checklist.progress', { done: done.size, total: CHECKLIST.length })}</p>
        </div>
        <Button variant="ghost" onClick={() => dismiss.mutate()} disabled={dismiss.isPending}>
          {t('checklist.dismiss')}
        </Button>
      </div>
      <div className="mb-4 h-2 overflow-hidden rounded-full bg-border" aria-hidden>
        <div className="h-full bg-brand-600" style={{ width: `${(done.size / CHECKLIST.length) * 100}%` }} />
      </div>
      <ul className="flex flex-col divide-y divide-border">
        {CHECKLIST.map((item) => {
          const isDone = done.has(item.key)
          return (
            <li key={item.key} className="flex items-center justify-between gap-3 py-3">
              <span className="flex items-center gap-3">
                <span
                  aria-hidden
                  className={`flex size-6 items-center justify-center rounded-full text-xs font-bold ${isDone ? 'bg-brand-600 text-white' : 'border border-border text-ink-muted'}`}
                >
                  {isDone ? '✓' : ''}
                </span>
                <span className="flex flex-col">
                  <span className={`text-sm font-medium ${isDone ? 'text-ink-muted line-through' : ''}`}>{t(`checklist.items.${item.key}`)}</span>
                  {item.comingSoon && !isDone && <span className="text-xs text-ink-muted">{t('checklist.comingSoon')}</span>}
                  <span className="sr-only">{isDone ? t('checklist.done') : t('checklist.todo')}</span>
                </span>
              </span>
              {!isDone && item.to && (
                <Link to={item.to} className="text-sm font-semibold text-brand-700 hover:underline">
                  {t('checklist.start')}
                </Link>
              )}
              {!isDone && item.skippable && (
                <Button variant="ghost" onClick={() => skip.mutate(item.key)} disabled={skip.isPending}>
                  {t('checklist.skip')}
                </Button>
              )}
            </li>
          )
        })}
      </ul>
    </Card>
  )
}
