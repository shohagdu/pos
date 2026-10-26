import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { type LicenceInput, type LicenceKind, type TenantDetail, platformApi, platformKeys, useAdminPlans } from '../../api/platform'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Dialog } from '../../components/ui/Dialog'
import { Select } from '../../components/ui/Select'
import { StateTabs } from '../../components/ui/Tabs'
import { TextField } from '../../components/ui/TextField'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { planLabel } from './labels'

const KINDS: LicenceKind[] = ['annual', 'lifetime', 'trial']

/** Grants a licence sold offline: a yearly or one-time licence, or more trial days. */
export function LicenceDialog({ tenant: d, onClose }: { tenant: TenantDetail; onClose: () => void }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const plans = (useAdminPlans().data ?? []).filter((p) => !p.is_trial && p.status === 'active')
  const [kind, setKind] = useState<LicenceKind>('annual')
  const [plan, setPlan] = useState(d.plan_code && d.plan_code !== 'trial' ? d.plan_code : '')
  const [years, setYears] = useState('1')
  const [days, setDays] = useState('7')
  const [price, setPrice] = useState('')
  const [reason, setReason] = useState('')
  const [missing, setMissing] = useState(false)
  const chosen = plan || plans[0]?.code || ''
  const listPrice = plans.find((p) => p.code === chosen)?.prices.find((p) => p.billing_cycle === kind)?.amount
  const save = useMutation({
    mutationFn: (input: LicenceInput) => platformApi.grantLicence(d.id, input),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: platformKeys.all })
      onClose()
    },
  })
  const errors = fieldErrors(save.error, t)

  function submit(e: FormEvent) {
    e.preventDefault()
    if (!reason.trim()) {
      setMissing(true)
      return
    }
    const input: LicenceInput = { kind, reason: reason.trim() }
    if (kind === 'trial') input.days = Number(days)
    else {
      input.plan_code = chosen
      input.price_amount = price.trim() || listPrice || '0'
      if (kind === 'annual') input.years = Number(years)
    }
    save.mutate(input)
  }

  return (
    <Dialog
      open
      onClose={onClose}
      title={t('admin.licence.title', { name: d.name })}
      footer={
        <>
          <Button variant="secondary" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button type="submit" form="licence-form" disabled={save.isPending || (kind !== 'trial' && !chosen)}>
            {save.isPending ? t('common.saving') : t(`admin.licence.confirm.${kind}`)}
          </Button>
        </>
      }
    >
      <form id="licence-form" onSubmit={submit} className="flex flex-col gap-3" noValidate>
        {save.isError && hasFormError(save.error) && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}
        <StateTabs
          label={t('admin.licence.kind')}
          value={kind}
          onChange={(k) => {
            setKind(k)
            setPrice('')
          }}
          items={KINDS.map((k) => ({ value: k, label: t(`admin.licence.kinds.${k}`) }))}
        />
        <p className="text-sm text-ink-muted">{t(`admin.licence.hints.${kind}`)}</p>
        {kind === 'trial' ? (
          <TextField
            label={t('admin.licence.days')}
            type="number"
            min={1}
            max={365}
            step={1}
            inputMode="numeric"
            value={days}
            onChange={(e) => setDays(e.target.value)}
            error={errors.days}
          />
        ) : (
          <>
            <Select
              label={t('admin.tenants.plan')}
              value={chosen}
              onChange={(e) => setPlan(e.target.value)}
              error={errors.plan_code}
              options={plans.map((p) => ({ value: p.code, label: planLabel(t, p.code, p.name) }))}
            />
            {kind === 'annual' && (
              <TextField
                label={t('admin.licence.years')}
                type="number"
                min={1}
                max={10}
                step={1}
                inputMode="numeric"
                value={years}
                onChange={(e) => setYears(e.target.value)}
                error={errors.years}
              />
            )}
            <TextField
              label={t('admin.licence.price')}
              type="number"
              min={0}
              step="0.01"
              inputMode="decimal"
              value={price}
              placeholder={listPrice ?? '0'}
              onChange={(e) => setPrice(e.target.value)}
              hint={listPrice ? t('admin.licence.priceHint', { amount: listPrice }) : undefined}
              error={errors.price_amount}
            />
          </>
        )}
        <TextField
          label={t('admin.licence.reason')}
          value={reason}
          required
          maxLength={500}
          onChange={(e) => {
            setReason(e.target.value)
            setMissing(false)
          }}
          hint={t('admin.licence.reasonHint')}
          error={missing ? t('problems.REQUIRED') : errors.reason}
        />
      </form>
    </Dialog>
  )
}
