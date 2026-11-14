import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { type ReactNode, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate, useParams } from 'react-router'

import { catalogApi, categoryTree, useBrands, useCategories } from '../../api/catalog'
import {
  type BxgyInput,
  type PromotionAction,
  type PromotionDetail,
  type PromotionInput,
  type PromotionType,
  type TargetInput,
  type Weekday,
  promotionKeys,
  promotionsApi,
  usePromotion,
} from '../../api/promotions'
import { useBranches } from '../../api/tenancy'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { Checkbox } from '../../components/ui/Checkbox'
import { PageHeader } from '../../components/ui/PageHeader'
import { Select } from '../../components/ui/Select'
import { Spinner } from '../../components/ui/Spinner'
import { TextField } from '../../components/ui/TextField'
import { ZERO, dec, mul, round, toFixed, toPlain, tryParse } from '../../lib/decimal'
import { formatMoney } from '../../lib/format'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { type Promotion as EnginePromotion, applyPromotions } from '../../lib/promotions'
import { useSession } from '../auth/session'
import { useDebounced } from '../pos/hooks'
import { categoryLabel } from '../products/labels'
import { CouponsPanel } from './CouponsPanel'
import { StateBadge } from './labels'

const TYPES: PromotionType[] = ['product_discount', 'flash', 'bxgy', 'coupon']
const DAYS: Weekday[] = ['sat', 'sun', 'mon', 'tue', 'wed', 'thu', 'fri']

interface Target extends TargetInput {
  /** Shown in the list. */
  label: string
}

interface Form {
  name: string
  name_bn: string
  type: PromotionType
  discount_type: string
  discount_value: string
  max_discount_amount: string
  min_purchase_amount: string
  starts_at: string
  ends_at: string
  active_days: Weekday[]
  active_time_from: string
  active_time_to: string
  priority: string
  stackable: boolean
  registered_only: boolean
  total_usage_limit: string
  per_customer_limit: string
  quantity_limit: string
  bxgy: BxgyInput
  targets: Target[]
  branch_ids: string[]
}

const blank: Form = {
  name: '',
  name_bn: '',
  type: 'product_discount',
  discount_type: 'percent',
  discount_value: '',
  max_discount_amount: '',
  min_purchase_amount: '',
  starts_at: '',
  ends_at: '',
  active_days: [],
  active_time_from: '',
  active_time_to: '',
  priority: '0',
  stackable: false,
  registered_only: false,
  total_usage_limit: '',
  per_customer_limit: '',
  quantity_limit: '',
  bxgy: {
    buy_quantity: '1',
    get_quantity: '1',
    get_target_type: 'same_product',
    get_target_id: null,
    get_discount_type: 'free',
    get_discount_value: '',
    repeat: true,
    auto_add_free_item: true,
  },
  targets: [],
  branch_ids: [],
}

function fromDetail(d: PromotionDetail): Form {
  return {
    name: d.name,
    name_bn: d.name_bn ?? '',
    type: d.type,
    discount_type: d.type === 'bxgy' ? 'percent' : d.discount_type,
    discount_value: d.type === 'bxgy' ? '' : toPlain(dec(d.discount_value)),
    max_discount_amount: d.max_discount_amount ? toPlain(dec(d.max_discount_amount)) : '',
    min_purchase_amount: d.min_purchase_amount ? toPlain(dec(d.min_purchase_amount)) : '',
    starts_at: d.starts_at?.slice(0, 16) ?? '',
    ends_at: d.ends_at?.slice(0, 16) ?? '',
    active_days: d.active_days ?? [],
    active_time_from: d.active_time_from?.slice(0, 5) ?? '',
    active_time_to: d.active_time_to?.slice(0, 5) ?? '',
    priority: String(d.priority),
    stackable: d.stackable,
    registered_only: d.applies_to_customer === 'registered_only',
    total_usage_limit: d.total_usage_limit?.toString() ?? '',
    per_customer_limit: d.per_customer_limit?.toString() ?? '',
    quantity_limit: d.quantity_limit ? toPlain(dec(d.quantity_limit)) : '',
    bxgy: d.bxgy
      ? {
          buy_quantity: toPlain(dec(d.bxgy.buy_quantity)),
          get_quantity: toPlain(dec(d.bxgy.get_quantity)),
          get_target_type: d.bxgy.get_target_type,
          get_target_id: d.bxgy.get_target_id,
          get_discount_type: d.bxgy.get_discount_type,
          get_discount_value: d.bxgy.get_discount_type === 'percent' ? toPlain(dec(d.bxgy.get_discount_value)) : '',
          repeat: d.bxgy.repeat,
          auto_add_free_item: d.bxgy.auto_add_free_item,
        }
      : blank.bxgy,
    targets: d.targets.map((t) => ({ target_type: t.target_type, target_id: t.target_id, is_exclusion: t.is_exclusion, label: t.name ?? '' })),
    branch_ids: d.branch_ids,
  }
}

const opt = (v: string) => (v.trim() === '' ? null : v.trim())
const optInt = (v: string) => (v.trim() === '' ? null : Number.parseInt(v, 10))

function toInput(f: Form, status?: 'draft' | 'active'): PromotionInput {
  return {
    name: f.name.trim(),
    name_bn: opt(f.name_bn),
    type: f.type,
    discount_type: f.type === 'bxgy' ? null : f.discount_type,
    discount_value: f.type === 'bxgy' ? null : opt(f.discount_value),
    max_discount_amount: opt(f.max_discount_amount),
    min_purchase_amount: f.type === 'coupon' ? opt(f.min_purchase_amount) : null,
    starts_at: f.starts_at ? `${f.starts_at}:00` : null,
    ends_at: f.ends_at ? `${f.ends_at}:00` : null,
    active_days: f.active_days.length ? f.active_days : null,
    active_time_from: f.active_time_from ? `${f.active_time_from}:00` : null,
    active_time_to: f.active_time_to ? `${f.active_time_to}:00` : null,
    priority: Number.parseInt(f.priority || '0', 10) || 0,
    stackable: f.stackable,
    applies_to_customer: f.registered_only ? 'registered_only' : 'all',
    total_usage_limit: optInt(f.total_usage_limit),
    per_customer_limit: optInt(f.per_customer_limit),
    quantity_limit: f.type === 'flash' ? opt(f.quantity_limit) : null,
    bxgy:
      f.type === 'bxgy'
        ? {
            ...f.bxgy,
            get_target_id: f.bxgy.get_target_type === 'same_product' ? null : f.bxgy.get_target_id,
            get_discount_value: f.bxgy.get_discount_type === 'percent' ? opt(f.bxgy.get_discount_value ?? '') : null,
          }
        : null,
    targets: f.targets.map(({ target_type, target_id, is_exclusion }) => ({
      target_type,
      target_id: target_type === 'all_products' ? null : target_id,
      is_exclusion,
    })),
    branch_ids: f.branch_ids,
    ...(status ? { status } : {}),
  }
}

/** `/promotions/new` and `/promotions/:id`. */
export function PromotionFormPage() {
  const { id } = useParams()
  const { t } = useTranslation()
  const { me } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const editing = id !== undefined && id !== 'new'
  const detail = usePromotion(tenantId, id ?? '', editing)
  if (!editing) return <PromotionForm tenantId={tenantId} detail={null} />
  if (detail.isPending) return <Spinner />
  if (detail.isError) return <Alert tone="danger">{problemMessage(detail.error, t)}</Alert>
  // Re-mount on a new version so the form starts from what was saved.
  return <PromotionForm key={`${detail.data.id}-${detail.data.version}`} tenantId={tenantId} detail={detail.data} />
}

function Section({ title, children, hint }: { title: string; children: ReactNode; hint?: ReactNode }) {
  return (
    <Card className="flex flex-col gap-3 p-4">
      <div>
        <h2 className="font-semibold">{title}</h2>
        {hint && <p className="text-xs text-ink-muted">{hint}</p>}
      </div>
      {children}
    </Card>
  )
}

function PromotionForm({ tenantId, detail }: { tenantId: string; detail: PromotionDetail | null }) {
  const { t } = useTranslation()
  const { can } = useSession()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const canManage = can('promotions.manage')
  const [form, setForm] = useState<Form>(() => (detail ? fromDetail(detail) : blank))
  const set = <K extends keyof Form>(key: K, value: Form[K]) => setForm((f) => ({ ...f, [key]: value }))
  const locked = detail?.locked ?? false
  const ended = detail?.status === 'ended'
  const readOnly = !canManage || ended

  const save = useMutation({
    mutationFn: (status?: 'draft' | 'active') =>
      detail ? promotionsApi.update(detail.id, detail.version, toInput(form)) : promotionsApi.create(toInput(form, status)),
    onSuccess: (saved) => {
      void queryClient.invalidateQueries({ queryKey: promotionKeys.all(tenantId) })
      if (!detail) void navigate(`/promotions/${saved.id}`, { replace: true })
    },
  })
  const act = useMutation({
    mutationFn: async (action: PromotionAction | 'delete'): Promise<PromotionDetail | null> => {
      if (action === 'delete') {
        await promotionsApi.remove(detail!.id)
        return null
      }
      return promotionsApi.action(detail!.id, action)
    },
    onSuccess: (result, action) => {
      void queryClient.invalidateQueries({ queryKey: promotionKeys.all(tenantId) })
      if (action === 'delete') void navigate('/promotions')
      else if (action === 'duplicate' && result) void navigate(`/promotions/${result.id}`)
    },
  })
  const errors = fieldErrors(save.error, t)

  // Live count of matched products (§7 step 3).
  const targetKey = useDebounced(JSON.stringify(toInput(form).targets), 400)
  const count = useQuery({
    queryKey: ['promotions', tenantId, 'target-count', targetKey],
    queryFn: () => promotionsApi.targetCount(JSON.parse(targetKey) as TargetInput[]),
    enabled: form.targets.some((x) => !x.is_exclusion),
  })

  const locks = locked ? t('promotions.lockedHint') : undefined
  return (
    <div className="flex max-w-6xl flex-col gap-5">
      <Link to="/promotions" className="inline-flex min-h-11 items-center self-start text-sm font-medium text-brand-700 hover:underline md:min-h-0">
        ← {t('promotions.title')}
      </Link>
      <PageHeader
        title={detail ? detail.name : t('promotions.new')}
        subtitle={detail ? <StateBadge state={detail.state} /> : undefined}
        actions={
          detail &&
          canManage && (
            <>
              {detail.status === 'draft' && (
                <Button variant="secondary" disabled={act.isPending} onClick={() => act.mutate('activate')}>
                  {t('promotions.actions.activate')}
                </Button>
              )}
              {detail.status === 'active' && (
                <Button variant="secondary" disabled={act.isPending} onClick={() => act.mutate('pause')}>
                  {t('promotions.actions.pause')}
                </Button>
              )}
              {detail.status === 'paused' && (
                <Button variant="secondary" disabled={act.isPending} onClick={() => act.mutate('resume')}>
                  {t('promotions.actions.resume')}
                </Button>
              )}
              <Button variant="secondary" disabled={act.isPending} onClick={() => act.mutate('duplicate')}>
                {t('promotions.actions.duplicate')}
              </Button>
              {!ended && (
                <Button variant="secondary" className="text-danger" disabled={act.isPending} onClick={() => act.mutate(locked ? 'end' : 'delete')}>
                  {locked ? t('promotions.actions.end') : t('promotions.actions.delete')}
                </Button>
              )}
            </>
          )
        }
      />
      {act.isError && <Alert tone="danger">{problemMessage(act.error, t)}</Alert>}
      {ended && <Alert tone="info">{t('promotions.endedHint')}</Alert>}
      {locked && !ended && <Alert tone="info">{t('promotions.lockedHint')}</Alert>}

      <fieldset disabled={readOnly} className="grid min-w-0 grid-cols-1 gap-4 xl:grid-cols-[minmax(0,1fr)_22rem] xl:items-start">
        <div className="flex min-w-0 flex-col gap-4">
          <Section title={t('promotions.steps.type')}>
            <div role="radiogroup" aria-label={t('promotions.type')} className="grid grid-cols-2 gap-2 lg:grid-cols-4">
              {TYPES.map((v) => (
                <button
                  key={v}
                  type="button"
                  role="radio"
                  aria-checked={form.type === v}
                  disabled={locked || readOnly}
                  onClick={() =>
                    setForm((f) => ({ ...f, type: v, discount_type: v === 'coupon' && f.discount_type === 'fixed_price' ? 'percent' : f.discount_type }))
                  }
                  className={`flex min-h-16 flex-col items-start justify-center rounded-lg border px-3 py-2 text-left text-sm disabled:cursor-not-allowed ${
                    form.type === v ? 'border-brand-600 bg-brand-50 ring-1 ring-brand-600' : 'border-border hover:border-brand-500'
                  }`}
                >
                  <span className="font-semibold">{t(`promotions.types.${v}`)}</span>
                  <span className="text-xs text-ink-muted">{t(`promotions.typeHints.${v}`)}</span>
                </button>
              ))}
            </div>
            <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
              <TextField
                label={t('promotions.name')}
                required
                value={form.name}
                maxLength={120}
                onChange={(e) => set('name', e.target.value)}
                error={errors.name}
              />
              <TextField
                label={t('promotions.nameBn')}
                value={form.name_bn}
                maxLength={120}
                onChange={(e) => set('name_bn', e.target.value)}
                error={errors.name_bn}
              />
            </div>
          </Section>

          <Section title={t('promotions.steps.benefit')} hint={locks}>
            {form.type === 'bxgy' ? (
              <BxgyFields tenantId={tenantId} value={form.bxgy} onChange={(b) => set('bxgy', b)} disabled={locked} errors={errors} />
            ) : (
              <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
                <Select
                  label={t('promotions.discountType')}
                  value={form.discount_type}
                  disabled={locked}
                  onChange={(e) => set('discount_type', e.target.value)}
                  options={(form.type === 'coupon' ? ['percent', 'fixed_amount'] : ['percent', 'fixed_amount', 'fixed_price']).map((v) => ({
                    value: v,
                    label: t(`promotions.discountTypes.${form.type === 'coupon' && v === 'fixed_amount' ? 'fixed_order' : v}`),
                  }))}
                />
                <TextField
                  label={
                    form.discount_type === 'percent'
                      ? t('promotions.percent')
                      : form.discount_type === 'fixed_price'
                        ? t('promotions.price')
                        : t('promotions.amount')
                  }
                  required
                  inputMode="decimal"
                  value={form.discount_value}
                  disabled={locked}
                  onChange={(e) => set('discount_value', e.target.value)}
                  error={errors.discount_value}
                />
                <TextField
                  label={t('promotions.maxDiscount')}
                  hint={t('promotions.maxDiscountHint')}
                  inputMode="decimal"
                  value={form.max_discount_amount}
                  disabled={locked}
                  onChange={(e) => set('max_discount_amount', e.target.value)}
                  error={errors.max_discount_amount}
                />
              </div>
            )}
          </Section>

          <Section title={t('promotions.steps.products')} hint={locks}>
            <TargetPicker tenantId={tenantId} targets={form.targets} onChange={(v) => set('targets', v)} disabled={locked || readOnly} />
            {errors.targets && <p className="text-sm text-danger">{errors.targets}</p>}
            {form.targets.some((x) => !x.is_exclusion) && (
              <p className="text-sm text-ink-muted" role="status">
                {count.isPending ? t('common.loading') : t('promotions.matched', { count: count.data?.matched_products ?? 0 })}
              </p>
            )}
          </Section>

          <Section title={t('promotions.steps.schedule')} hint={form.type === 'flash' ? t('promotions.flashScheduleHint') : t('promotions.scheduleHint')}>
            <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
              <TextField
                label={t('promotions.startsAt')}
                type="datetime-local"
                value={form.starts_at}
                onChange={(e) => set('starts_at', e.target.value)}
                error={errors.starts_at}
                required={form.type === 'flash'}
              />
              <TextField
                label={t('promotions.endsAt')}
                type="datetime-local"
                value={form.ends_at}
                onChange={(e) => set('ends_at', e.target.value)}
                error={errors.ends_at}
                required={form.type === 'flash'}
              />
            </div>
            <div role="group" aria-label={t('promotions.days')} className="flex flex-col gap-1.5">
              <span className="text-sm font-medium">{t('promotions.days')}</span>
              <div className="flex flex-wrap gap-1.5">
                {DAYS.map((d) => {
                  const on = form.active_days.includes(d)
                  return (
                    <button
                      key={d}
                      type="button"
                      aria-pressed={on}
                      onClick={() => set('active_days', on ? form.active_days.filter((x) => x !== d) : [...form.active_days, d])}
                      className={`min-h-11 min-w-11 rounded-lg border px-2 text-sm font-medium lg:min-h-9 ${on ? 'border-brand-600 bg-brand-600 text-white' : 'border-border hover:border-brand-500'}`}
                    >
                      {t(`promotions.weekdays.${d}`)}
                    </button>
                  )
                })}
              </div>
              <span className="text-xs text-ink-muted">{t('promotions.daysHint')}</span>
            </div>
            <div className="grid grid-cols-2 gap-3">
              <TextField label={t('promotions.timeFrom')} type="time" value={form.active_time_from} onChange={(e) => set('active_time_from', e.target.value)} />
              <TextField
                label={t('promotions.timeTo')}
                type="time"
                value={form.active_time_to}
                onChange={(e) => set('active_time_to', e.target.value)}
                error={errors.active_time_to}
              />
            </div>
          </Section>
        </div>

        <div className="flex min-w-0 flex-col gap-4">
          <Section title={t('promotions.steps.limits')}>
            {form.type === 'flash' && (
              <TextField
                label={t('promotions.quantityLimit')}
                hint={t('promotions.quantityLimitHint')}
                inputMode="decimal"
                value={form.quantity_limit}
                onChange={(e) => set('quantity_limit', e.target.value)}
                error={errors.quantity_limit}
              />
            )}
            {form.type === 'coupon' && (
              <TextField
                label={t('promotions.minPurchase')}
                inputMode="decimal"
                value={form.min_purchase_amount}
                onChange={(e) => set('min_purchase_amount', e.target.value)}
                error={errors.min_purchase_amount}
              />
            )}
            <div className="grid grid-cols-2 gap-3">
              <TextField
                label={t('promotions.totalLimit')}
                inputMode="numeric"
                value={form.total_usage_limit}
                onChange={(e) => set('total_usage_limit', e.target.value)}
                error={errors.total_usage_limit}
              />
              <TextField
                label={t('promotions.perCustomerLimit')}
                inputMode="numeric"
                value={form.per_customer_limit}
                onChange={(e) => set('per_customer_limit', e.target.value)}
                error={errors.per_customer_limit}
              />
            </div>
            {detail && (
              <p className="text-xs text-ink-muted">
                {t('promotions.usedSoFar', { count: detail.usage_count })}
                {detail.quantity_limit &&
                  ` · ${t('promotions.unitsUsed', { used: toPlain(dec(detail.quantity_used)), limit: toPlain(dec(detail.quantity_limit)) })}`}
              </p>
            )}
          </Section>

          <Section title={t('promotions.steps.who')}>
            <BranchPicker tenantId={tenantId} value={form.branch_ids} onChange={(v) => set('branch_ids', v)} />
            <Checkbox label={t('promotions.registeredOnly')} checked={form.registered_only} onChange={(e) => set('registered_only', e.target.checked)} />
            <Checkbox
              label={t('promotions.stackable')}
              hint={t('promotions.stackableHint')}
              checked={form.stackable}
              onChange={(e) => set('stackable', e.target.checked)}
            />
            <TextField
              label={t('promotions.priority')}
              hint={t('promotions.priorityHint')}
              inputMode="numeric"
              value={form.priority}
              onChange={(e) => set('priority', e.target.value)}
              error={errors.priority}
            />
          </Section>

          <Preview form={form} />

          {!readOnly && (
            <Card className="sticky bottom-0 z-10 flex flex-col gap-2 p-4 xl:static">
              {save.isError && hasFormError(save.error) && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}
              {save.isSuccess && detail && <Alert tone="success">{t('promotions.saved')}</Alert>}
              {detail ? (
                <Button disabled={save.isPending} onClick={() => save.mutate(undefined)}>
                  {save.isPending ? t('common.loading') : t('common.save')}
                </Button>
              ) : (
                <div className="flex flex-wrap gap-2">
                  <Button className="flex-1" disabled={save.isPending} onClick={() => save.mutate('active')}>
                    {t('promotions.saveActivate')}
                  </Button>
                  <Button variant="secondary" disabled={save.isPending} onClick={() => save.mutate('draft')}>
                    {t('promotions.saveDraft')}
                  </Button>
                </div>
              )}
            </Card>
          )}
        </div>
      </fieldset>

      {detail?.type === 'coupon' && <CouponsPanel tenantId={tenantId} promotion={detail} canManage={canManage && !ended} />}
    </div>
  )
}

function BxgyFields({
  tenantId,
  value,
  onChange,
  disabled,
  errors,
}: {
  tenantId: string
  value: BxgyInput
  onChange: (b: BxgyInput) => void
  disabled: boolean
  errors: Record<string, string>
}) {
  const { t } = useTranslation()
  const categories = useCategories(tenantId)
  const set = <K extends keyof BxgyInput>(k: K, v: BxgyInput[K]) => onChange({ ...value, [k]: v })
  return (
    <div className="flex flex-col gap-3">
      <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
        <TextField
          label={t('promotions.bxgy.buy')}
          inputMode="decimal"
          value={value.buy_quantity}
          disabled={disabled}
          onChange={(e) => set('buy_quantity', e.target.value)}
          error={errors['bxgy.buy_quantity']}
        />
        <TextField
          label={t('promotions.bxgy.get')}
          inputMode="decimal"
          value={value.get_quantity}
          disabled={disabled}
          onChange={(e) => set('get_quantity', e.target.value)}
          error={errors['bxgy.get_quantity']}
        />
        <Select
          label={t('promotions.bxgy.getWhat')}
          value={value.get_discount_type}
          disabled={disabled}
          onChange={(e) => set('get_discount_type', e.target.value as BxgyInput['get_discount_type'])}
          options={[
            { value: 'free', label: t('promotions.bxgy.free') },
            { value: 'percent', label: t('promotions.bxgy.percentOff') },
          ]}
        />
        {value.get_discount_type === 'percent' && (
          <TextField
            label={t('promotions.percent')}
            inputMode="decimal"
            value={value.get_discount_value ?? ''}
            disabled={disabled}
            onChange={(e) => set('get_discount_value', e.target.value)}
            error={errors['bxgy.get_discount_value']}
          />
        )}
      </div>
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <Select
          label={t('promotions.bxgy.getTarget')}
          value={value.get_target_type}
          disabled={disabled}
          onChange={(e) => onChange({ ...value, get_target_type: e.target.value as BxgyInput['get_target_type'], get_target_id: null })}
          options={(['same_product', 'product', 'category'] as const).map((v) => ({ value: v, label: t(`promotions.bxgy.targets.${v}`) }))}
        />
        {value.get_target_type === 'category' && (
          <Select
            label={t('promotions.category')}
            value={value.get_target_id ?? ''}
            disabled={disabled}
            onChange={(e) => set('get_target_id', e.target.value || null)}
            options={[
              { value: '', label: t('promotions.choose') },
              ...categoryTree(categories.data ?? []).map((c) => ({ value: c.id, label: categoryLabel(c) })),
            ]}
            error={errors['bxgy.get_target_id']}
          />
        )}
        {value.get_target_type === 'product' && (
          <ProductSearch
            tenantId={tenantId}
            disabled={disabled}
            label={t('promotions.bxgy.freeProduct')}
            chosen={value.get_target_id ?? null}
            onPick={(id) => set('get_target_id', id)}
            error={errors['bxgy.get_target_id']}
          />
        )}
      </div>
      <div className="flex flex-col gap-2">
        <Checkbox
          label={t('promotions.bxgy.repeat')}
          hint={t('promotions.bxgy.repeatHint')}
          checked={value.repeat}
          disabled={disabled}
          onChange={(e) => set('repeat', e.target.checked)}
        />
        <Checkbox
          label={t('promotions.bxgy.autoAdd')}
          hint={t('promotions.bxgy.autoAddHint')}
          checked={value.auto_add_free_item}
          disabled={disabled}
          onChange={(e) => set('auto_add_free_item', e.target.checked)}
        />
      </div>
    </div>
  )
}

function ProductSearch({
  tenantId,
  label,
  chosen,
  onPick,
  disabled,
  error,
}: {
  tenantId: string
  label: string
  chosen: string | null
  onPick: (id: string, name: string) => void
  disabled?: boolean
  error?: string
}) {
  const { t } = useTranslation()
  const [q, setQ] = useState('')
  const term = useDebounced(q.trim(), 300)
  const found = useQuery({
    queryKey: ['catalog', tenantId, 'promo-picker', term],
    queryFn: () => catalogApi.products({ q: term, status: 'active' }, null),
    enabled: term.length >= 2,
  })
  const picked = useQuery({ queryKey: ['catalog', tenantId, 'product', chosen], queryFn: () => catalogApi.product(chosen!), enabled: !!chosen })
  return (
    <div className="flex min-w-0 flex-col gap-1">
      <TextField
        label={label}
        type="search"
        value={q}
        disabled={disabled}
        placeholder={t('promotions.searchProduct')}
        onChange={(e) => setQ(e.target.value)}
        error={error}
      />
      {chosen && picked.data && <span className="text-xs text-brand-700">{t('promotions.chosen', { name: picked.data.name })}</span>}
      {term.length >= 2 && (
        <ul className="max-h-48 overflow-y-auto rounded-lg border border-border bg-surface text-sm">
          {(found.data?.data ?? []).slice(0, 20).map((p) => (
            <li key={p.id}>
              <button
                type="button"
                className="flex min-h-11 w-full items-center justify-between gap-2 px-3 text-left hover:bg-canvas lg:min-h-9"
                onClick={() => {
                  onPick(p.id, p.name)
                  setQ('')
                }}
              >
                <span className="truncate">{p.name}</span>
                <span className="shrink-0 font-mono text-xs text-ink-muted">{p.sku}</span>
              </button>
            </li>
          ))}
          {found.isSuccess && found.data.data.length === 0 && <li className="px-3 py-2 text-ink-muted">{t('common.noResults')}</li>}
        </ul>
      )}
    </div>
  )
}

function TargetPicker({ tenantId, targets, onChange, disabled }: { tenantId: string; targets: Target[]; onChange: (t: Target[]) => void; disabled: boolean }) {
  const { t } = useTranslation()
  const categories = useCategories(tenantId)
  const brands = useBrands(tenantId)
  const [kind, setKind] = useState<TargetInput['target_type']>('category')
  const [exclude, setExclude] = useState(false)
  const add = (target: Target) => {
    if (targets.some((x) => x.target_type === target.target_type && x.target_id === target.target_id && !!x.is_exclusion === !!target.is_exclusion)) return
    onChange([...targets, target])
  }
  return (
    <div className="flex flex-col gap-3">
      {targets.length > 0 && (
        <ul className="flex flex-wrap gap-1.5">
          {targets.map((x, i) => (
            <li
              key={`${x.target_type}-${x.target_id}-${x.is_exclusion}`}
              className={`inline-flex items-center gap-1 rounded-full border px-2.5 py-1 text-sm ${x.is_exclusion ? 'border-danger/40 bg-danger/5 text-danger' : 'border-brand-500 bg-brand-50 text-brand-700'}`}
            >
              {x.is_exclusion ? t('promotions.except') : ''} {t(`promotions.targetTypes.${x.target_type}`)}
              {x.label && `: ${x.label}`}
              {!disabled && (
                <button
                  type="button"
                  aria-label={t('promotions.removeTarget')}
                  className="ml-1 flex size-6 items-center justify-center rounded-full hover:bg-ink/10"
                  onClick={() => onChange(targets.filter((_, j) => j !== i))}
                >
                  ✕
                </button>
              )}
            </li>
          ))}
        </ul>
      )}
      {!disabled && (
        <div className="grid grid-cols-1 items-end gap-3 sm:grid-cols-[12rem_minmax(0,1fr)]">
          <Select
            label={t('promotions.addTarget')}
            value={kind}
            onChange={(e) => setKind(e.target.value as TargetInput['target_type'])}
            options={(['category', 'brand', 'product', 'all_products'] as const).map((v) => ({ value: v, label: t(`promotions.targetTypes.${v}`) }))}
          />
          {kind === 'category' && (
            <Select
              label={t('promotions.category')}
              value=""
              onChange={(e) => {
                const c = (categories.data ?? []).find((x) => x.id === e.target.value)
                if (c) add({ target_type: 'category', target_id: c.id, is_exclusion: exclude, label: c.name })
              }}
              options={[
                { value: '', label: t('promotions.choose') },
                ...categoryTree(categories.data ?? []).map((c) => ({ value: c.id, label: categoryLabel(c) })),
              ]}
            />
          )}
          {kind === 'brand' && (
            <Select
              label={t('promotions.brand')}
              value=""
              onChange={(e) => {
                const b = (brands.data ?? []).find((x) => x.id === e.target.value)
                if (b) add({ target_type: 'brand', target_id: b.id, is_exclusion: exclude, label: b.name })
              }}
              options={[{ value: '', label: t('promotions.choose') }, ...(brands.data ?? []).map((b) => ({ value: b.id, label: b.name }))]}
            />
          )}
          {kind === 'product' && (
            <ProductSearch
              tenantId={tenantId}
              label={t('promotions.product')}
              chosen={null}
              onPick={(id, name) => add({ target_type: 'product', target_id: id, is_exclusion: exclude, label: name })}
            />
          )}
          {kind === 'all_products' && (
            <Button
              variant="secondary"
              disabled={exclude}
              onClick={() => add({ target_type: 'all_products', target_id: null, is_exclusion: false, label: '' })}
            >
              {t('promotions.addAll')}
            </Button>
          )}
        </div>
      )}
      {!disabled && (
        <Checkbox label={t('promotions.asExclusion')} hint={t('promotions.asExclusionHint')} checked={exclude} onChange={(e) => setExclude(e.target.checked)} />
      )}
    </div>
  )
}

function BranchPicker({ tenantId, value, onChange }: { tenantId: string; value: string[]; onChange: (v: string[]) => void }) {
  const { t } = useTranslation()
  const branches = useBranches(tenantId)
  const list = branches.data ?? []
  if (list.length <= 1) return null
  return (
    <div role="group" aria-label={t('promotions.branches')} className="flex flex-col gap-1.5">
      <span className="text-sm font-medium">{t('promotions.branches')}</span>
      {list.map((b) => (
        <Checkbox
          key={b.id}
          label={b.name}
          checked={value.length === 0 || value.includes(b.id)}
          onChange={(e) => {
            const all = value.length === 0 ? list.map((x) => x.id) : value
            const next = e.target.checked ? [...all, b.id] : all.filter((x) => x !== b.id)
            onChange(next.length === list.length ? [] : next)
          }}
        />
      ))}
      <span className="text-xs text-ink-muted">{value.length === 0 ? t('promotions.allBranches') : t('promotions.someBranches', { count: value.length })}</span>
    </div>
  )
}

/** §7 step 7: a sample cart priced with the same engine as the POS. */
function Preview({ form }: { form: Form }) {
  const { t, i18n } = useTranslation()
  const [price, setPrice] = useState('100')
  const [qty, setQty] = useState('2')
  const result = useMemo(() => {
    const p = tryParse(price)
    const q = tryParse(qty)
    if (p === null || q === null || p <= ZERO || q <= ZERO) return null
    const input = toInput(form)
    const promo: EnginePromotion = {
      id: 'preview',
      name: input.name || t('promotions.preview'),
      kind: input.type,
      discount_type: (input.type === 'bxgy' ? 'free' : (input.discount_type ?? 'percent')) as EnginePromotion['discount_type'],
      discount_value: input.discount_value ?? '0',
      max_discount_amount: input.max_discount_amount ?? null,
      min_purchase_amount: input.min_purchase_amount ?? null,
      // The sample item stands for a product the promotion covers.
      targets: [{ target_type: 'all_products', target_id: null }],
      quantity_remaining: input.quantity_limit ?? null,
      bxgy: input.bxgy ? { ...input.bxgy, get_target_type: 'same_product', get_discount_value: input.bxgy.get_discount_value ?? undefined } : null,
      coupon_code: input.type === 'coupon' ? 'PREVIEW' : null,
    }
    if (!(tryParse(promo.discount_value) !== null || input.type === 'bxgy')) return null
    const line = { variantId: 'v', productId: 'p', categoryIds: [], brandId: null, quantity: q, unitPrice: p, minUnitPrice: null, promotionsOff: false }
    const out =
      input.type === 'coupon'
        ? applyPromotions([line], [], promo, { promotionsStack: false, couponStacks: false })
        : applyPromotions([line], [promo], null, { promotionsStack: false, couponStacks: false })
    const gross = round(mul(p, q), 2)
    return { gross, saved: out.lines[0]!.amount, free: out.lines[0]!.freeQuantity, coupon: out.coupon }
  }, [form, price, qty, t])
  const money = (v: bigint) => formatMoney(toFixed(v, 2), i18n.language)
  return (
    <Section title={t('promotions.steps.preview')} hint={t('promotions.previewHint')}>
      <div className="grid grid-cols-2 gap-3">
        <TextField label={t('promotions.samplePrice')} inputMode="decimal" value={price} onChange={(e) => setPrice(e.target.value)} />
        <TextField label={t('promotions.sampleQty')} inputMode="decimal" value={qty} onChange={(e) => setQty(e.target.value)} />
      </div>
      {result && (
        <dl className="flex flex-col gap-1 text-sm" aria-live="polite">
          <div className="flex justify-between gap-3">
            <dt className="text-ink-muted">{t('promotions.previewGross')}</dt>
            <dd className="tabular-nums">{money(result.gross)}</dd>
          </div>
          <div className="flex justify-between gap-3 text-brand-700">
            <dt>{result.free > ZERO ? t('promotions.previewFree', { qty: toPlain(result.free) }) : t('promotions.previewSaved')}</dt>
            <dd className="tabular-nums">−{money(result.saved)}</dd>
          </div>
          <div className="flex justify-between gap-3 border-t border-border pt-1 font-semibold">
            <dt>{t('promotions.previewPays')}</dt>
            <dd className="tabular-nums">{money(result.gross - result.saved)}</dd>
          </div>
          {result.coupon && result.coupon !== 'applied' && (
            <p className="text-xs text-warning">
              {t(`pos.coupon.${result.coupon === 'min_purchase' ? 'minPurchase' : result.coupon === 'not_better' ? 'notBetter' : 'noItems'}`, {
                amount: form.min_purchase_amount,
              })}
            </p>
          )}
        </dl>
      )}
    </Section>
  )
}
