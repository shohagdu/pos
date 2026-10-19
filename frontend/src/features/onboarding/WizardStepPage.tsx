import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Navigate, useNavigate, useParams } from 'react-router'

import { ApiError } from '../../api/client'
import {
  type Branch,
  type BusinessSettings,
  RECEIPT_PAPERS,
  type SettingsPatch,
  TIMEZONES,
  tenancyApi,
  tenancyKeys,
  useBranches,
  useSettings,
} from '../../api/tenancy'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { Checkbox } from '../../components/ui/Checkbox'
import { Select } from '../../components/ui/Select'
import { Spinner } from '../../components/ui/Spinner'
import { TextField } from '../../components/ui/TextField'
import { LANGUAGES } from '../../i18n'
import { useFields } from '../../lib/form'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { isWizardStep, nextStep } from './steps'

/** Wizard steps 3–5 (regional, branch, invoice & tax). Defaults are already
 * set by provisioning, so every step can be skipped and revisited later. */
export function WizardStepPage() {
  const { t } = useTranslation()
  const { step } = useParams()
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id
  if (!isWizardStep(step) || !tenantId) return <Navigate to="/" replace />
  if (!can('settings.manage')) {
    return (
      <Card>
        <Alert tone="warning">{t('onboarding.ownerOnly')}</Alert>
      </Card>
    )
  }
  if (step === 'branch') return <BranchStep tenantId={tenantId} mode="wizard" />
  return <SettingsStep key={step} step={step} tenantId={tenantId} mode="wizard" />
}

/** `wizard`: skip / save & continue to the next step. `settings`: save in place. */
export type FormMode = 'wizard' | 'settings'

function StepCard({ title, intro, children }: { title: string; intro: string; children: ReactNode }) {
  return (
    <Card>
      <h1 className="mb-2 text-xl font-bold">{title}</h1>
      <p className="mb-6 text-sm text-ink-muted">{intro}</p>
      {children}
    </Card>
  )
}

function StepActions({
  pending,
  onSkip,
  last,
  mode,
  saved,
}: {
  pending: boolean
  onSkip: () => void
  last: boolean
  mode: FormMode
  saved: boolean
}) {
  const { t } = useTranslation()
  if (mode === 'settings') {
    return (
      <div className="flex flex-col gap-3">
        {saved && <Alert tone="success">{t('settings.saved')}</Alert>}
        <Button type="submit" disabled={pending} className="sm:self-end">
          {pending ? t('common.saving') : t('common.save')}
        </Button>
      </div>
    )
  }
  return (
    <div className="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
      <Button variant="ghost" onClick={onSkip} disabled={pending}>
        {t('onboarding.skip')}
      </Button>
      <Button type="submit" disabled={pending}>
        {pending ? t('common.saving') : last ? t('onboarding.finish') : t('onboarding.saveContinue')}
      </Button>
    </div>
  )
}

/** Saving a section also ticks it off the setup checklist. */
function useStepDone(tenantId: string, step: 'regional' | 'branch' | 'invoice', mode: FormMode) {
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  return async () => {
    await tenancyApi.completeStep(step)
    await queryClient.invalidateQueries({ queryKey: tenancyKeys.onboarding(tenantId) })
    if (mode === 'wizard') navigate(nextStep(step))
  }
}

export function SettingsStep({ step, tenantId, mode }: { step: 'regional' | 'invoice'; tenantId: string; mode: FormMode }) {
  const { t } = useTranslation()
  const settings = useSettings(tenantId)
  if (settings.isPending) return <Spinner />
  if (settings.isError) return <Alert tone="danger">{problemMessage(settings.error, t)}</Alert>
  return step === 'regional' ? (
    <RegionalForm settings={settings.data} tenantId={tenantId} mode={mode} />
  ) : (
    <InvoiceForm settings={settings.data} tenantId={tenantId} mode={mode} />
  )
}

function useSaveSettings(tenantId: string, step: 'regional' | 'invoice', version: number, mode: FormMode) {
  const queryClient = useQueryClient()
  const done = useStepDone(tenantId, step, mode)
  return useMutation({
    mutationFn: async (patch: SettingsPatch) => {
      const saved = await tenancyApi.updateSettings(version, patch)
      queryClient.setQueryData(tenancyKeys.settings(tenantId), saved)
      await done()
    },
    onError: (err) => {
      // Someone else saved first: reload so the next attempt has the new version.
      if (err instanceof ApiError && err.status === 412) void queryClient.invalidateQueries({ queryKey: tenancyKeys.settings(tenantId) })
    },
  })
}

const MONTHS = Array.from({ length: 12 }, (_, i) => i + 1)

function RegionalForm({ settings, tenantId, mode }: { settings: BusinessSettings; tenantId: string; mode: FormMode }) {
  const { t, i18n } = useTranslation()
  const navigate = useNavigate()
  const { values, bind } = useFields({
    timezone: settings.timezone,
    locale: settings.locale,
    fiscal_year_start_month: String(settings.fiscal_year_start_month),
  })
  const save = useSaveSettings(tenantId, 'regional', settings.version, mode)
  const errors = fieldErrors(save.error, t)
  const monthName = (m: number) => new Intl.DateTimeFormat(i18n.language === 'bn' ? 'bn-BD' : 'en-GB', { month: 'long' }).format(new Date(2026, m - 1, 1))

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    save.mutate(
      { timezone: values.timezone, locale: values.locale, fiscal_year_start_month: Number(values.fiscal_year_start_month) },
      { onSuccess: () => void i18n.changeLanguage(values.locale) },
    )
  }

  return (
    <StepCard title={t('onboarding.regional.title')} intro={t('onboarding.regional.intro')}>
      <form onSubmit={onSubmit} className="flex flex-col gap-4" noValidate>
        <Select label={t('onboarding.regional.currency')} options={[{ value: 'BDT', label: t('onboarding.regional.bdt') }]} value="BDT" disabled onChange={() => {}} />
        <Select label={t('onboarding.regional.timezone')} options={TIMEZONES.map((z) => ({ value: z, label: z }))} error={errors.timezone} {...bind('timezone')} />
        <Select label={t('onboarding.regional.language')} options={LANGUAGES.map((l) => ({ value: l.code, label: l.label }))} error={errors.locale} {...bind('locale')} />
        <Select
          label={t('onboarding.regional.fiscalYear')}
          options={MONTHS.map((m) => ({ value: String(m), label: monthName(m) }))}
          error={errors.fiscal_year_start_month}
          {...bind('fiscal_year_start_month')}
        />
        {save.isError && hasFormError(save.error) && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}
        <StepActions pending={save.isPending} onSkip={() => navigate(nextStep('regional'))} last={false} mode={mode} saved={save.isSuccess} />
      </form>
    </StepCard>
  )
}

function InvoiceForm({ settings, tenantId, mode }: { settings: BusinessSettings; tenantId: string; mode: FormMode }) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { values, bind, bindCheck } = useFields({
    invoice_prefix: settings.invoice_prefix,
    receipt_paper: settings.receipt_paper as string,
    receipt_footer: settings.receipt_footer ?? '',
    auto_print_receipt: settings.auto_print_receipt,
    is_vat_registered: settings.is_vat_registered,
    prices_include_tax: settings.prices_include_tax,
  })
  const save = useSaveSettings(tenantId, 'invoice', settings.version, mode)
  const errors = fieldErrors(save.error, t)

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    save.mutate({
      invoice_prefix: values.invoice_prefix,
      receipt_paper: values.receipt_paper as BusinessSettings['receipt_paper'],
      receipt_footer: values.receipt_footer,
      auto_print_receipt: values.auto_print_receipt,
      is_vat_registered: values.is_vat_registered,
      prices_include_tax: values.prices_include_tax,
    })
  }

  return (
    <StepCard title={t('onboarding.invoice.title')} intro={t('onboarding.invoice.intro')}>
      <form onSubmit={onSubmit} className="flex flex-col gap-4" noValidate>
        <TextField
          label={t('onboarding.invoice.prefix')}
          maxLength={10}
          error={errors.invoice_prefix}
          hint={t('onboarding.invoice.prefixHint', { example: `${values.invoice_prefix.toUpperCase() || 'INV'}-2610-00001` })}
          {...bind('invoice_prefix')}
        />
        <Select
          label={t('onboarding.invoice.paper')}
          options={RECEIPT_PAPERS.map((p) => ({ value: p, label: t(`onboarding.invoice.papers.${p}`) }))}
          error={errors.receipt_paper}
          {...bind('receipt_paper')}
        />
        <TextField label={t('onboarding.invoice.footer')} maxLength={500} error={errors.receipt_footer} hint={t('onboarding.invoice.footerHint')} {...bind('receipt_footer')} />
        <Checkbox label={t('onboarding.invoice.autoPrint')} {...bindCheck('auto_print_receipt')} />
        <Checkbox label={t('onboarding.invoice.vatRegistered')} hint={t('onboarding.invoice.vatHint')} {...bindCheck('is_vat_registered')} />
        <Checkbox label={t('onboarding.invoice.pricesIncludeTax')} hint={t('onboarding.invoice.pricesIncludeTaxHint')} {...bindCheck('prices_include_tax')} />
        {save.isError && hasFormError(save.error) && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}
        <StepActions pending={save.isPending} onSkip={() => navigate(nextStep('invoice'))} last mode={mode} saved={save.isSuccess} />
      </form>
    </StepCard>
  )
}

export function BranchStep({ tenantId, mode }: { tenantId: string; mode: FormMode }) {
  const { t } = useTranslation()
  const branches = useBranches(tenantId)
  const branch = branches.data?.find((b) => b.is_default) ?? branches.data?.[0]
  if (branches.isPending) return <Spinner />
  if (branches.isError || !branch) return <Alert tone="danger">{problemMessage(branches.error, t)}</Alert>
  return <BranchForm key={branch.id} branch={branch} tenantId={tenantId} mode={mode} />
}

function BranchForm({ branch, tenantId, mode }: { branch: Branch; tenantId: string; mode: FormMode }) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const done = useStepDone(tenantId, 'branch', mode)
  const { values, bind } = useFields({ name: branch.name, address: branch.address ?? '', phone: branch.phone ?? '' })

  const save = useMutation({
    mutationFn: async () => {
      await tenancyApi.updateBranch(branch.id, branch.version, values)
      await queryClient.invalidateQueries({ queryKey: tenancyKeys.branches(tenantId) })
      await done()
    },
  })
  const errors = fieldErrors(save.error, t)

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    save.mutate()
  }

  return (
    <StepCard title={t('onboarding.branch.title')} intro={t('onboarding.branch.intro')}>
      <form onSubmit={onSubmit} className="flex flex-col gap-4" noValidate>
        <TextField label={t('onboarding.branch.name')} required error={errors.name} {...bind('name')} />
        <TextField label={t('onboarding.branch.address')} autoComplete="street-address" error={errors.address} {...bind('address')} />
        <TextField label={t('onboarding.branch.phone')} type="tel" error={errors.phone} {...bind('phone')} />
        {save.isError && hasFormError(save.error) && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}
        <StepActions pending={save.isPending} onSkip={() => navigate(nextStep('branch'))} last={false} mode={mode} saved={save.isSuccess} />
      </form>
    </StepCard>
  )
}
