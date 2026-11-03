import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'

import { BUSINESS_TYPES, type BusinessType, type NewBusiness, tenancyApi } from '../../api/tenancy'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { Select } from '../../components/ui/Select'
import { TextField } from '../../components/ui/TextField'
import { optional, useFields } from '../../lib/form'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'

/** Wizard step 2: creates the business with its defaults, then signs in to it. */
export function CreateBusinessPage() {
  const { t, i18n } = useTranslation()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const { me, switchTenant } = useSession()
  const [more, setMore] = useState(false)
  const { values, bind } = useFields({
    name: '',
    business_type: 'retail',
    address: '',
    district: '',
    phone: '',
    email: '',
    bin_number: '',
    trade_license_no: '',
  })

  async function createAndEnter(input: NewBusiness) {
    const tenant = await tenancyApi.createTenant(input)
    await switchTenant(tenant.tenant_id)
    await queryClient.invalidateQueries({ queryKey: ['auth', 'me'] })
    return tenant
  }

  const create = useMutation({
    mutationFn: () =>
      createAndEnter({
        name: values.name,
        business_type: values.business_type as BusinessType,
        address: optional(values.address),
        district: optional(values.district),
        phone: optional(values.phone),
        email: optional(values.email),
        bin_number: optional(values.bin_number),
        trade_license_no: optional(values.trade_license_no),
        locale: i18n.language,
      }),
    onSuccess: () => navigate('/onboarding/regional', { replace: true }),
  })
  // Skip: a placeholder business with all defaults; the owner renames it and
  // fills in the rest later in Settings.
  const skip = useMutation({
    mutationFn: () =>
      createAndEnter({
        name: t('onboarding.business.defaultName', { name: me?.user.full_name ?? '' }).trim(),
        business_type: 'retail',
        locale: i18n.language,
      }),
    onSuccess: () => navigate('/', { replace: true }),
  })
  const pending = create.isPending || skip.isPending
  const errors = fieldErrors(create.error, t)

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    create.mutate()
  }

  return (
    <Card>
      <h1 className="mb-2 text-xl font-bold">{t('onboarding.business.title')}</h1>
      <p className="mb-6 text-sm text-ink-muted">{t('onboarding.business.intro')}</p>
      <form onSubmit={onSubmit} className="flex flex-col gap-4" noValidate>
        <TextField label={t('onboarding.business.name')} autoComplete="organization" required error={errors.name} {...bind('name')} />
        <Select
          label={t('onboarding.business.type')}
          options={BUSINESS_TYPES.map((v) => ({ value: v, label: t(`businessTypes.${v}`) }))}
          error={errors.business_type}
          {...bind('business_type')}
        />
        <TextField label={t('onboarding.business.address')} autoComplete="street-address" error={errors.address} {...bind('address')} />
        <TextField label={t('onboarding.business.district')} error={errors.district} {...bind('district')} />
        {more ? (
          <>
            <TextField label={t('onboarding.business.phone')} type="tel" error={errors.phone} {...bind('phone')} />
            <TextField label={t('onboarding.business.email')} type="email" error={errors.email} {...bind('email')} />
            <TextField label={t('onboarding.business.bin')} error={errors.bin_number} {...bind('bin_number')} />
            <TextField label={t('onboarding.business.tradeLicense')} error={errors.trade_license_no} {...bind('trade_license_no')} />
          </>
        ) : (
          <Button variant="ghost" className="self-start" onClick={() => setMore(true)}>
            {t('onboarding.business.moreDetails')}
          </Button>
        )}
        {create.isError && hasFormError(create.error) && <Alert tone="danger">{problemMessage(create.error, t)}</Alert>}
        {skip.isError && <Alert tone="danger">{problemMessage(skip.error, t)}</Alert>}
        <Button type="submit" disabled={pending}>
          {create.isPending ? t('onboarding.business.creating') : t('onboarding.business.create')}
        </Button>
        <Button variant="ghost" disabled={pending} onClick={() => skip.mutate()}>
          {skip.isPending ? t('onboarding.business.creating') : t('onboarding.business.skip')}
        </Button>
        <p className="text-center text-xs text-ink-muted">{t('onboarding.business.skipHint')}</p>
      </form>
    </Card>
  )
}
