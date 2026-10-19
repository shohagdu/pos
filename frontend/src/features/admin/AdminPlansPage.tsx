import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { type AdminPlan, type PlanInput, canAdminister, platformApi, platformKeys, useAdminPlans } from '../../api/platform'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { Checkbox } from '../../components/ui/Checkbox'
import { Dialog } from '../../components/ui/Dialog'
import { PageHeader } from '../../components/ui/PageHeader'
import { Select } from '../../components/ui/Select'
import { Spinner } from '../../components/ui/Spinner'
import { TextField } from '../../components/ui/TextField'
import { formatMoney } from '../../lib/format'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { cycleLabel, featureName, useStaffRole } from './labels'

const CYCLES = ['monthly', 'quarterly', 'annual'] as const
type Cycle = (typeof CYCLES)[number]

/** Plans and pricing (ADM-R05). */
export function AdminPlansPage() {
  const { t } = useTranslation()
  const role = useStaffRole()
  const plans = useAdminPlans()
  // `null` = closed, `'new'` = creating, otherwise the plan being edited.
  const [editing, setEditing] = useState<AdminPlan | 'new' | null>(null)
  const editable = canAdminister(role)

  return (
    <div className="flex max-w-6xl flex-col gap-6">
      <PageHeader
        title={t('admin.plans.title')}
        actions={
          editable && plans.data && (
            <Button onClick={() => setEditing('new')}>{t('admin.plans.new')}</Button>
          )
        }
      />
      {plans.isPending ? (
        <Spinner />
      ) : plans.isError ? (
        <Alert tone="danger">{problemMessage(plans.error, t)}</Alert>
      ) : plans.data.length === 0 ? (
        <p className="text-sm text-ink-muted">{t('common.none')}</p>
      ) : (
        <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
          {plans.data.map((p) => (
            <PlanCard key={p.id} plan={p} onEdit={editable ? () => setEditing(p) : undefined} />
          ))}
        </div>
      )}
      {editing !== null && plans.data && (
        <PlanEditor
          plan={editing === 'new' ? null : editing}
          template={plans.data[0]?.features ?? []}
          onClose={() => setEditing(null)}
        />
      )}
    </div>
  )
}

function PlanCard({ plan: p, onEdit }: { plan: AdminPlan; onEdit?: () => void }) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const flags = p.features.filter((f) => f.kind === 'flag' && f.enabled)
  const limits = p.features.filter((f) => f.kind === 'limit' && f.enabled)
  return (
    <Card className="flex flex-col gap-3">
      <div className="flex items-start justify-between gap-2">
        <div>
          <h2 className="text-lg font-semibold">{p.name}</h2>
          <p className="font-mono text-xs text-ink-muted">{p.code}</p>
        </div>
        {onEdit && (
          <Button variant="secondary" onClick={onEdit} aria-label={`${t('common.edit')} ${p.name}`}>
            {t('common.edit')}
          </Button>
        )}
      </div>
      <div className="flex flex-wrap gap-1">
        <Badge tone={p.status === 'active' ? 'good' : 'neutral'}>{t(`admin.plans.statuses.${p.status}`)}</Badge>
        <Badge tone={p.is_public ? 'info' : 'neutral'}>{p.is_public ? t('admin.plans.public') : t('admin.plans.private')}</Badge>
        {p.is_trial && <Badge tone="warn">{t('admin.plans.trialBadge', { count: p.trial_days ?? 0 })}</Badge>}
      </div>
      {p.description && <p className="text-sm text-ink-muted">{p.description}</p>}
      <p className="text-sm">{t('admin.plans.subscribers', { count: p.subscribers })}</p>
      <dl className="grid grid-cols-3 gap-2 text-sm">
        {CYCLES.map((c) => {
          const price = p.prices.find((x) => x.billing_cycle === c)
          return (
            <div key={c}>
              <dt className="text-xs text-ink-muted">{cycleLabel(t, c)}</dt>
              <dd className="font-semibold tabular-nums">{price ? formatMoney(price.amount, lang) : '—'}</dd>
            </div>
          )
        })}
      </dl>
      <div className="text-sm">
        <p className="text-xs font-medium tracking-wide text-ink-muted uppercase">{t('admin.plans.features')}</p>
        <ul className="mt-1 flex flex-col gap-0.5">
          {limits.map((f) => (
            <li key={f.code} className="flex justify-between gap-2">
              <span>{featureName(t, f)}</span>
              <span className="tabular-nums">{f.limit_value === null ? t('admin.tenant.unlimited') : f.limit_value}</span>
            </li>
          ))}
        </ul>
        {flags.length > 0 && <p className="mt-1 text-ink-muted">{flags.map((f) => featureName(t, f)).join(' · ')}</p>}
      </div>
    </Card>
  )
}

interface FeatureRow {
  code: string
  name: string
  kind: 'flag' | 'limit'
  enabled: boolean
  limit: string
}

function PlanEditor({
  plan,
  template,
  onClose,
}: {
  plan: AdminPlan | null
  template: AdminPlan['features']
  onClose: () => void
}) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const creating = plan === null
  const [code, setCode] = useState('')
  const [name, setName] = useState(plan?.name ?? '')
  const [description, setDescription] = useState(plan?.description ?? '')
  const [isPublic, setIsPublic] = useState(plan?.is_public ?? true)
  const [sortOrder, setSortOrder] = useState(String(plan?.sort_order ?? 0))
  const [status, setStatus] = useState<'active' | 'retired'>(plan?.status ?? 'active')
  const [trialDays, setTrialDays] = useState(plan?.trial_days ? String(plan.trial_days) : '')
  const [prices, setPrices] = useState<Record<Cycle, string>>(() => {
    const out = { monthly: '', quarterly: '', annual: '' }
    for (const p of plan?.prices ?? []) out[p.billing_cycle] = p.amount
    return out
  })
  const [features, setFeatures] = useState<FeatureRow[]>(() =>
    (plan?.features ?? template).map((f) => ({
      code: f.code,
      name: f.name,
      kind: f.kind,
      enabled: plan ? f.enabled : false,
      limit: plan && f.limit_value !== null ? String(f.limit_value) : '',
    })),
  )

  const save = useMutation({
    mutationFn: (input: PlanInput) => (plan ? platformApi.updatePlan(plan.id, input) : platformApi.createPlan(input)),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: platformKeys.all })
      onClose()
    },
  })
  const errors = fieldErrors(save.error, t)

  function setFeature(code: string, patch: Partial<FeatureRow>) {
    setFeatures((rows) => rows.map((r) => (r.code === code ? { ...r, ...patch } : r)))
  }

  function submit(e: FormEvent) {
    e.preventDefault()
    const input: PlanInput = {
      name: name.trim(),
      is_public: isPublic,
      prices: {},
      features: features.map((f) => ({
        code: f.code,
        enabled: f.enabled,
        limit_value: f.kind === 'limit' && f.limit.trim() !== '' ? Number(f.limit) : null,
      })),
    }
    if (creating) input.code = code.trim()
    else input.status = status
    if (description.trim()) input.description = description.trim()
    if (sortOrder.trim() !== '') input.sort_order = Number(sortOrder)
    if (plan?.is_trial && trialDays.trim() !== '') input.trial_days = Number(trialDays)
    for (const c of CYCLES) {
      const value = prices[c].trim()
      if (value) input.prices[c] = value
    }
    save.mutate(input)
  }

  return (
    <Dialog
      open
      wide
      onClose={onClose}
      title={creating ? t('admin.plans.createTitle') : t('admin.plans.editTitle', { name: plan.name })}
      footer={
        <>
          <Button variant="secondary" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button type="submit" form="plan-form" disabled={save.isPending}>
            {save.isPending ? t('common.saving') : t('common.save')}
          </Button>
        </>
      }
    >
      <form id="plan-form" onSubmit={submit} className="flex flex-col gap-4" noValidate>
        {save.isError && hasFormError(save.error) && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}
        <div className="grid gap-3 sm:grid-cols-2">
          {creating ? (
            <TextField
              label={t('admin.plans.code')}
              value={code}
              required
              pattern="[a-z0-9_]{2,40}"
              onChange={(e) => setCode(e.target.value.toLowerCase())}
              hint={t('admin.plans.codeHint')}
              error={errors.code}
            />
          ) : (
            <TextField label={t('admin.plans.code')} value={plan.code} disabled />
          )}
          <TextField label={t('admin.plans.name')} value={name} required maxLength={100} onChange={(e) => setName(e.target.value)} error={errors.name} />
          <TextField
            className="sm:col-span-2"
            label={`${t('admin.plans.description')} (${t('common.optional')})`}
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            error={errors.description}
          />
          <TextField
            label={t('admin.plans.sortOrder')}
            type="number"
            step={1}
            value={sortOrder}
            onChange={(e) => setSortOrder(e.target.value)}
            error={errors.sort_order}
          />
          {!creating && (
            <Select
              label={t('common.status')}
              value={status}
              onChange={(e) => setStatus(e.target.value as 'active' | 'retired')}
              error={errors.status}
              options={[
                { value: 'active', label: t('admin.plans.statuses.active') },
                { value: 'retired', label: t('admin.plans.statuses.retired') },
              ]}
            />
          )}
          {plan?.is_trial && (
            <TextField
              label={t('admin.plans.trialDays')}
              type="number"
              min={1}
              max={365}
              value={trialDays}
              onChange={(e) => setTrialDays(e.target.value)}
              error={errors.trial_days}
            />
          )}
          <Checkbox
            className="self-end sm:col-span-2"
            label={t('admin.plans.publicLabel')}
            hint={t('admin.plans.publicHint')}
            checked={isPublic}
            onChange={(e) => setIsPublic(e.target.checked)}
          />
        </div>

        <fieldset className="flex flex-col gap-2">
          <legend className="text-sm font-semibold">{t('admin.plans.prices')}</legend>
          <p className="text-sm text-ink-muted">{t('admin.plans.pricesHint')}</p>
          {errors.prices && <p className="text-sm text-danger">{errors.prices}</p>}
          <div className="grid gap-3 sm:grid-cols-3">
            {CYCLES.map((c) => (
              <TextField
                key={c}
                label={cycleLabel(t, c)}
                inputMode="decimal"
                value={prices[c]}
                onChange={(e) => setPrices((p) => ({ ...p, [c]: e.target.value }))}
                error={errors[`prices.${c}`]}
              />
            ))}
          </div>
        </fieldset>

        <fieldset className="flex flex-col gap-2">
          <legend className="text-sm font-semibold">{t('admin.plans.features')}</legend>
          {errors.features && <p className="text-sm text-danger">{errors.features}</p>}
          <ul className="flex flex-col divide-y divide-border rounded-lg border border-border">
            {features.map((f) => (
              <li key={f.code} className="flex flex-wrap items-center justify-between gap-3 px-3 py-2">
                <Checkbox
                  label={featureName(t, f)}
                  hint={f.code}
                  checked={f.enabled}
                  onChange={(e) => setFeature(f.code, { enabled: e.target.checked })}
                />
                {f.kind === 'limit' && (
                  <TextField
                    className="w-40"
                    label={t('admin.plans.limit')}
                    aria-label={t('admin.plans.limitFor', { name: featureName(t, f) })}
                    type="number"
                    min={0}
                    step={1}
                    placeholder={t('admin.tenant.unlimited')}
                    value={f.limit}
                    disabled={!f.enabled}
                    onChange={(e) => setFeature(f.code, { limit: e.target.value })}
                  />
                )}
              </li>
            ))}
          </ul>
          <p className="text-sm text-ink-muted">{t('admin.blankUnlimited')}</p>
        </fieldset>
      </form>
    </Dialog>
  )
}
