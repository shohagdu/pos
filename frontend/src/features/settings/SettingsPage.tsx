import { useMutation, useQueryClient } from '@tanstack/react-query'
import type { FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { Navigate, NavLink, useParams } from 'react-router'

import { ApiError } from '../../api/client'
import { BUSINESS_TYPES, type BusinessSettings, type BusinessType, tenancyApi, tenancyKeys, useSettings } from '../../api/tenancy'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { Select } from '../../components/ui/Select'
import { Spinner } from '../../components/ui/Spinner'
import { TextField } from '../../components/ui/TextField'
import { useFields } from '../../lib/form'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { BranchesSection } from '../branches/BranchesSection'
import { RegistersSection } from '../branches/RegistersSection'
import { BranchStep, SettingsStep } from '../onboarding/WizardStepPage'

const SECTIONS = ['profile', 'regional', 'branch', 'invoice', 'branches', 'registers'] as const
type Section = (typeof SECTIONS)[number]
/** Sections for `branches.manage`; the rest need `settings.manage`. */
const BRANCH_SECTIONS: readonly Section[] = ['branches', 'registers']

/** Business settings; the same forms as the onboarding wizard, saved in place. */
export function SettingsPage() {
  const { t } = useTranslation()
  const { section = 'profile' } = useParams()
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id
  const visible = SECTIONS.filter((s) => (BRANCH_SECTIONS.includes(s) ? can('branches.manage') : can('settings.manage')))
  if (!tenantId || !SECTIONS.includes(section as Section)) return <Navigate to="/settings" replace />
  if (!visible.includes(section as Section)) {
    // Branch managers without `settings.manage` land on their first section.
    if (section === 'profile' && visible.length > 0) return <Navigate to={`/settings/${visible[0]}`} replace />
    return <Alert tone="warning">{t('onboarding.ownerOnly')}</Alert>
  }

  return (
    <div className={`flex flex-col gap-6 ${BRANCH_SECTIONS.includes(section as Section) ? 'max-w-5xl' : 'max-w-2xl'}`}>
      <h1 className="text-2xl font-bold">{t('settings.title')}</h1>
      <nav aria-label={t('settings.title')} className="flex flex-wrap gap-1 border-b border-border">
        {visible.map((s) => (
          <NavLink
            key={s}
            to={s === 'profile' ? '/settings' : `/settings/${s}`}
            end
            className={({ isActive }) =>
              `-mb-px border-b-2 px-3 py-2 text-sm font-medium ${isActive ? 'border-brand-600 text-brand-700' : 'border-transparent text-ink-muted hover:text-ink'}`
            }
          >
            {t(`settings.sections.${s}`)}
          </NavLink>
        ))}
      </nav>
      {section === 'profile' && <ProfileSection tenantId={tenantId} />}
      {section === 'regional' && <SettingsStep step="regional" tenantId={tenantId} mode="settings" />}
      {section === 'invoice' && <SettingsStep step="invoice" tenantId={tenantId} mode="settings" />}
      {section === 'branch' && <BranchStep tenantId={tenantId} mode="settings" />}
      {section === 'branches' && <BranchesSection tenantId={tenantId} />}
      {section === 'registers' && <RegistersSection tenantId={tenantId} />}
    </div>
  )
}

function ProfileSection({ tenantId }: { tenantId: string }) {
  const { t } = useTranslation()
  const settings = useSettings(tenantId)
  if (settings.isPending) return <Spinner />
  if (settings.isError) return <Alert tone="danger">{problemMessage(settings.error, t)}</Alert>
  return <ProfileForm settings={settings.data} tenantId={tenantId} />
}

function ProfileForm({ settings, tenantId }: { settings: BusinessSettings; tenantId: string }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const { values, bind } = useFields({
    name: settings.name,
    legal_name: settings.legal_name ?? '',
    business_type: settings.business_type as string,
    phone: settings.phone ?? '',
    email: settings.email ?? '',
    address: settings.address ?? '',
    district: settings.district ?? '',
    bin_number: settings.bin_number ?? '',
    trade_license_no: settings.trade_license_no ?? '',
  })
  const save = useMutation({
    mutationFn: () => tenancyApi.updateSettings(settings.version, { ...values, business_type: values.business_type as BusinessType }),
    onSuccess: async (saved) => {
      queryClient.setQueryData(tenancyKeys.settings(tenantId), saved)
      // The business name is shown in the shell.
      await queryClient.invalidateQueries({ queryKey: ['auth', 'me'] })
    },
    onError: (err) => {
      if (err instanceof ApiError && err.status === 412) void queryClient.invalidateQueries({ queryKey: tenancyKeys.settings(tenantId) })
    },
  })
  const errors = fieldErrors(save.error, t)

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    save.mutate()
  }

  return (
    <Card>
      <form onSubmit={onSubmit} className="flex flex-col gap-4" noValidate>
        <TextField label={t('onboarding.business.name')} required error={errors.name} {...bind('name')} />
        <TextField label={t('settings.legalName')} error={errors.legal_name} {...bind('legal_name')} />
        <Select
          label={t('onboarding.business.type')}
          options={BUSINESS_TYPES.map((v) => ({ value: v, label: t(`businessTypes.${v}`) }))}
          error={errors.business_type}
          {...bind('business_type')}
        />
        <TextField label={t('onboarding.business.address')} error={errors.address} {...bind('address')} />
        <TextField label={t('onboarding.business.district')} error={errors.district} {...bind('district')} />
        <TextField label={t('onboarding.business.phone')} type="tel" error={errors.phone} {...bind('phone')} />
        <TextField label={t('onboarding.business.email')} type="email" error={errors.email} {...bind('email')} />
        <TextField label={t('onboarding.business.bin')} error={errors.bin_number} {...bind('bin_number')} />
        <TextField label={t('onboarding.business.tradeLicense')} error={errors.trade_license_no} {...bind('trade_license_no')} />
        <p className="text-sm text-ink-muted">{t('settings.address', { slug: settings.slug })}</p>
        {save.isError && hasFormError(save.error) && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}
        {save.isSuccess && <Alert tone="success">{t('settings.saved')}</Alert>}
        <Button type="submit" disabled={save.isPending} className="sm:self-end">
          {save.isPending ? t('common.saving') : t('common.save')}
        </Button>
      </form>
    </Card>
  )
}
