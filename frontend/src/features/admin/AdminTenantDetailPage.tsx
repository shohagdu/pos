import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, type ReactNode, useState } from 'react'
import type { TFunction } from 'i18next'
import { useTranslation } from 'react-i18next'
import { Link, useParams } from 'react-router'

import type { PaymentClaim, SubscriptionInvoice } from '../../api/billing'
import {
  type Override,
  type OverrideInput,
  type TenantDetail,
  canAdminister,
  canBill,
  platformApi,
  platformKeys,
  useAdminTenant,
} from '../../api/platform'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { ConfirmButton } from '../../components/ui/ConfirmButton'
import { DataTable } from '../../components/ui/DataTable'
import { Dialog } from '../../components/ui/Dialog'
import { Select } from '../../components/ui/Select'
import { Spinner } from '../../components/ui/Spinner'
import { TextField } from '../../components/ui/TextField'
import { formatDate, formatDateTime } from '../../lib/dates'
import { formatMoney } from '../../lib/format'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { LINK, cycleLabel, featureName, methodLabel, planLabel, stateLabel, useStaffRole } from './labels'
import { BusinessStatusBadge, ClaimStatusBadge, Fact, InvoiceStatusBadge, ReasonDialog, SubscriptionBadge } from './shared'

/** One business (ADM-R02…R06): subscription, usage, history, billing and overrides. */
export function AdminTenantDetailPage() {
  const { t } = useTranslation()
  const { id = '' } = useParams()
  const tenant = useAdminTenant(id)

  if (tenant.isPending) return <Spinner />
  if (tenant.isError) {
    return (
      <div className="flex flex-col gap-3">
        <Link to="/admin/tenants" className={LINK}>
          ← {t('admin.tenants.title')}
        </Link>
        <Alert tone="danger">{problemMessage(tenant.error, t)}</Alert>
      </div>
    )
  }
  const d = tenant.data
  return (
    <div className="flex max-w-6xl flex-col gap-6">
      <Link to="/admin/tenants" className={`${LINK} text-sm print:hidden`}>
        ← {t('admin.tenants.title')}
      </Link>
      <Header tenant={d} />
      <div className="grid gap-6 lg:grid-cols-2">
        <Contact tenant={d} />
        <Usage tenant={d} />
      </div>
      <Members tenant={d} />
      <Entitlements tenant={d} />
      <Overrides tenant={d} />
      <History tenant={d} />
      <Invoices invoices={d.invoices} />
      <Claims claims={d.claims} />
    </div>
  )
}

function Header({ tenant: d }: { tenant: TenantDetail }) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const role = useStaffRole()
  const queryClient = useQueryClient()
  const [action, setAction] = useState<'suspend' | 'reactivate' | null>(null)
  const change = useMutation({
    mutationFn: (reason: string) => (action === 'suspend' ? platformApi.suspend(d.id, reason) : platformApi.reactivate(d.id, reason)),
    onSuccess: async () => {
      setAction(null)
      await queryClient.invalidateQueries({ queryKey: platformKeys.all })
    },
  })
  const open = (a: 'suspend' | 'reactivate') => {
    change.reset()
    setAction(a)
  }
  const suspended = d.status === 'suspended'

  return (
    <Card className="flex flex-col gap-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h1 className="text-2xl font-bold">{d.name}</h1>
          <p className="font-mono text-sm text-ink-muted">{d.slug}</p>
          <div className="mt-2 flex flex-wrap gap-2">
            <BusinessStatusBadge status={d.status} />
            <SubscriptionBadge state={d.subscription_status} />
          </div>
        </div>
        {canAdminister(role) && (
          <div className="print:hidden">
            {suspended ? (
              <Button onClick={() => open('reactivate')}>{t('admin.tenant.reactivate')}</Button>
            ) : (
              <Button variant="secondary" className="text-danger" onClick={() => open('suspend')}>
                {t('admin.tenant.suspend')}
              </Button>
            )}
          </div>
        )}
      </div>
      {suspended && d.suspended_reason && (
        <Alert tone="danger">
          <span className="font-semibold">{t('admin.tenant.suspendedReason')}:</span> {d.suspended_reason}
        </Alert>
      )}
      <dl className="grid grid-cols-2 gap-4 sm:grid-cols-4">
        <Fact label={t('admin.tenants.plan')}>{planLabel(t, d.plan_code, d.plan_name)}</Fact>
        <Fact label={t('admin.tenant.cycle')}>{cycleLabel(t, d.billing_cycle)}</Fact>
        <Fact label={t('admin.tenant.price')}>{d.price_amount ? formatMoney(d.price_amount, lang) : '—'}</Fact>
        <Fact label={t('admin.tenant.daysLeft')}>{d.days_left === null ? '—' : t('plan.daysLeft', { count: d.days_left })}</Fact>
        <Fact label={t('admin.tenant.trialEnd')}>{formatDate(d.trial_ends_at, lang)}</Fact>
        <Fact label={t('admin.tenant.periodEnd')}>{formatDate(d.current_period_end, lang)}</Fact>
        <Fact label={t('admin.tenant.lastActivity')}>{formatDateTime(d.last_activity_at, lang)}</Fact>
        <Fact label={t('admin.tenants.created')}>{formatDate(d.created_at, lang)}</Fact>
      </dl>
      <ReasonDialog
        open={action !== null}
        onClose={() => setAction(null)}
        title={action === 'reactivate' ? t('admin.tenant.reactivateTitle', { name: d.name }) : t('admin.tenant.suspendTitle', { name: d.name })}
        hint={action === 'reactivate' ? t('admin.tenant.reactivateHint') : t('admin.tenant.suspendHint')}
        confirmLabel={action === 'reactivate' ? t('admin.tenant.reactivate') : t('admin.tenant.suspend')}
        pending={change.isPending}
        error={change.error}
        onSubmit={(reason) => change.mutate(reason)}
      />
    </Card>
  )
}

function Section({ title, children, actions }: { title: string; children: ReactNode; actions?: ReactNode }) {
  return (
    <section className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h2 className="text-lg font-semibold">{title}</h2>
        {actions}
      </div>
      {children}
    </section>
  )
}

function Contact({ tenant: d }: { tenant: TenantDetail }) {
  const { t } = useTranslation()
  return (
    <Card className="flex flex-col gap-3">
      <h2 className="text-lg font-semibold">{t('admin.tenant.contact')}</h2>
      <dl className="grid grid-cols-2 gap-4">
        <Fact label={t('admin.tenant.phone')}>{d.phone ?? '—'}</Fact>
        <Fact label={t('admin.tenant.email')}>{d.email ?? '—'}</Fact>
        <Fact label={t('admin.tenant.district')}>{d.district ?? '—'}</Fact>
        <Fact label={t('admin.tenants.owner')}>
          {d.owner_name ?? '—'}
          {d.owner_phone && <span className="block text-xs text-ink-muted">{d.owner_phone}</span>}
        </Fact>
      </dl>
    </Card>
  )
}

function Usage({ tenant: d }: { tenant: TenantDetail }) {
  const { t } = useTranslation()
  const names = new Map(d.entitlements.map((e) => [e.code, e.name]))
  return (
    <Card className="flex flex-col gap-3">
      <h2 className="text-lg font-semibold">{t('admin.tenant.usage')}</h2>
      {d.usage.length === 0 ? (
        <p className="text-sm text-ink-muted">{t('common.none')}</p>
      ) : (
        <ul className="flex flex-col gap-2 text-sm">
          {d.usage.map((u) => {
            const over = u.limit !== null && u.used >= u.limit
            return (
              <li key={u.code} className="flex items-center justify-between gap-3">
                <span>{featureName(t, { code: u.code, name: names.get(u.code) })}</span>
                <span className={`tabular-nums ${over ? 'font-semibold text-danger' : ''}`}>
                  {u.used} / {u.limit === null ? t('admin.tenant.unlimited') : u.limit}
                </span>
              </li>
            )
          })}
        </ul>
      )}
    </Card>
  )
}

function Members({ tenant: d }: { tenant: TenantDetail }) {
  const { t } = useTranslation()
  return (
    <Section title={t('admin.tenant.members')}>
      <DataTable
        caption={t('admin.tenant.members')}
        rows={d.members}
        rowKey={(m) => m.user_id}
        empty={t('common.none')}
        columns={[
          {
            key: 'name',
            header: t('admin.tenants.name'),
            render: (m) => (
              <span className="inline-flex flex-wrap items-center gap-2">
                {m.full_name}
                {m.is_owner && <Badge tone="info">{t('admin.tenant.owner')}</Badge>}
              </span>
            ),
          },
          { key: 'phone', header: t('admin.tenant.phone'), render: (m) => m.phone ?? '—' },
          { key: 'email', header: t('admin.tenant.email'), render: (m) => m.email ?? '—' },
          {
            key: 'status',
            header: t('common.status'),
            render: (m) => <Badge tone={m.status === 'active' ? 'good' : 'neutral'}>{t(`admin.memberStatus.${m.status}`, { defaultValue: m.status })}</Badge>,
          },
        ]}
      />
    </Section>
  )
}

function Entitlements({ tenant: d }: { tenant: TenantDetail }) {
  const { t } = useTranslation()
  return (
    <Section title={t('admin.tenant.entitlements')}>
      <ul className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
        {d.entitlements.map((e) => (
          <li key={e.code} className="flex items-center justify-between gap-2 rounded-lg border border-border bg-surface px-3 py-2 text-sm">
            <span>
              {featureName(t, e)}
              <span className="block font-mono text-xs text-ink-muted">{e.code}</span>
            </span>
            <span className="flex flex-wrap items-center justify-end gap-1">
              {e.overridden && <Badge tone="warn">{t('admin.tenant.overridden')}</Badge>}
              {e.kind === 'flag' || !e.enabled ? (
                <Badge tone={e.enabled ? 'good' : 'neutral'}>{e.enabled ? t('admin.on') : t('admin.off')}</Badge>
              ) : (
                <span className="font-semibold tabular-nums">{e.limit === null ? t('admin.tenant.unlimited') : e.limit}</span>
              )}
            </span>
          </li>
        ))}
      </ul>
    </Section>
  )
}

function overrideValue(t: TFunction, o: Override): string {
  if (o.kind === 'flag' || o.enabled === false) return o.enabled ? t('admin.on') : t('admin.off')
  return o.limit_value === null ? t('admin.tenant.unlimited') : String(o.limit_value)
}

function Overrides({ tenant: d }: { tenant: TenantDetail }) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const role = useStaffRole()
  const queryClient = useQueryClient()
  const [adding, setAdding] = useState(false)
  const remove = useMutation({
    mutationFn: (overrideId: string) => platformApi.removeOverride(d.id, overrideId),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: platformKeys.all }),
  })
  const editable = canBill(role)

  return (
    <Section
      title={t('admin.tenant.overrides')}
      actions={
        editable && (
          <Button variant="secondary" className="print:hidden" onClick={() => setAdding(true)}>
            {t('admin.tenant.addOverride')}
          </Button>
        )
      }
    >
      {remove.isError && <Alert tone="danger">{problemMessage(remove.error, t)}</Alert>}
      <DataTable<Override>
        caption={t('admin.tenant.overrides')}
        rows={d.overrides}
        rowKey={(o) => o.id}
        empty={t('admin.tenant.noOverrides')}
        columns={[
          { key: 'feature', header: t('admin.tenant.feature'), render: (o) => featureName(t, { code: o.feature_code, name: o.feature_name }) },
          { key: 'value', header: t('admin.tenant.value'), render: (o) => overrideValue(t, o) },
          { key: 'expires', header: t('admin.tenant.expires'), className: 'whitespace-nowrap', render: (o) => (o.expires_at ? formatDate(o.expires_at, lang) : t('admin.tenant.never')) },
          { key: 'reason', header: t('common.reason'), render: (o) => o.reason },
          {
            key: 'by',
            header: t('admin.tenant.createdBy'),
            render: (o) => (
              <>
                {o.created_by_name ?? '—'}
                <span className="block text-xs text-ink-muted">{formatDateTime(o.created_at, lang)}</span>
              </>
            ),
          },
          ...(editable
            ? [
                {
                  key: 'actions',
                  header: <span className="sr-only">{t('common.actions')}</span>,
                  render: (o: Override) => (
                    <ConfirmButton disabled={remove.isPending} onConfirm={() => remove.mutate(o.id)}>
                      {t('common.remove')}
                    </ConfirmButton>
                  ),
                },
              ]
            : []),
        ]}
      />
      {adding && <OverrideDialog tenant={d} onClose={() => setAdding(false)} />}
    </Section>
  )
}

function OverrideDialog({ tenant: d, onClose }: { tenant: TenantDetail; onClose: () => void }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const [code, setCode] = useState(d.entitlements[0]?.code ?? '')
  const [enabled, setEnabled] = useState('true')
  const [limit, setLimit] = useState('')
  const [expires, setExpires] = useState('')
  const [reason, setReason] = useState('')
  const [missing, setMissing] = useState(false)
  const feature = d.entitlements.find((e) => e.code === code)
  const save = useMutation({
    mutationFn: (input: OverrideInput) => platformApi.setOverride(d.id, input),
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
    const input: OverrideInput = { feature_code: code, reason: reason.trim() }
    if (feature?.kind === 'flag') input.enabled = enabled === 'true'
    else if (limit.trim() !== '') input.limit_value = Number(limit)
    if (expires) input.expires_at = `${expires}T23:59:59Z`
    save.mutate(input)
  }

  return (
    <Dialog
      open
      onClose={onClose}
      title={t('admin.tenant.addOverride')}
      footer={
        <>
          <Button variant="secondary" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button type="submit" form="override-form" disabled={save.isPending || !code}>
            {save.isPending ? t('common.saving') : t('common.save')}
          </Button>
        </>
      }
    >
      <form id="override-form" onSubmit={submit} className="flex flex-col gap-3" noValidate>
        {save.isError && hasFormError(save.error) && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}
        <Select
          label={t('admin.tenant.feature')}
          value={code}
          onChange={(e) => setCode(e.target.value)}
          error={errors.feature_code}
          options={d.entitlements.map((e) => ({ value: e.code, label: `${featureName(t, e)} (${e.code})` }))}
        />
        {feature?.kind === 'flag' ? (
          <Select
            label={t('admin.tenant.value')}
            value={enabled}
            onChange={(e) => setEnabled(e.target.value)}
            error={errors.enabled}
            options={[
              { value: 'true', label: t('admin.on') },
              { value: 'false', label: t('admin.off') },
            ]}
          />
        ) : (
          <TextField
            label={t('admin.tenant.limit')}
            type="number"
            min={0}
            step={1}
            inputMode="numeric"
            value={limit}
            onChange={(e) => setLimit(e.target.value)}
            hint={t('admin.blankUnlimited')}
            error={errors.limit_value}
          />
        )}
        <TextField
          label={`${t('admin.tenant.expires')} (${t('common.optional')})`}
          type="date"
          value={expires}
          onChange={(e) => setExpires(e.target.value)}
          hint={t('admin.tenant.expiresHint')}
          error={errors.expires_at}
        />
        <TextField
          label={t('common.reason')}
          value={reason}
          required
          maxLength={500}
          onChange={(e) => {
            setReason(e.target.value)
            setMissing(false)
          }}
          error={missing ? t('problems.REQUIRED') : errors.reason}
        />
      </form>
    </Dialog>
  )
}

function eventLabel(t: TFunction, type: string): string {
  const reminder = /^reminder_t(\d+)$/.exec(type)
  if (reminder) return t('admin.events.reminder', { days: reminder[1] })
  return t(`admin.events.${type}`, { defaultValue: type.replace(/_/g, ' ') })
}

function History({ tenant: d }: { tenant: TenantDetail }) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const arrow = (from: string | null, to: string | null, label: (v: string) => string) =>
    from || to ? `${from ? label(from) : '—'} → ${to ? label(to) : '—'}` : '—'
  return (
    <Section title={t('admin.tenant.history')}>
      <DataTable
        caption={t('admin.tenant.history')}
        rows={d.events}
        rowKey={(e) => e.id}
        empty={t('common.none')}
        columns={[
          { key: 'date', header: t('common.date'), className: 'whitespace-nowrap', render: (e) => formatDateTime(e.created_at, lang) },
          { key: 'type', header: t('admin.tenant.event'), render: (e) => eventLabel(t, e.event_type) },
          { key: 'status', header: t('common.status'), render: (e) => arrow(e.from_status, e.to_status, (s) => stateLabel(t, s)) },
          { key: 'plan', header: t('admin.tenants.plan'), render: (e) => arrow(e.from_plan, e.to_plan, (p) => planLabel(t, p)) },
          {
            key: 'actor',
            header: t('admin.tenant.actor'),
            render: (e) => e.actor_name ?? t(`admin.actors.${e.actor_type}`, { defaultValue: e.actor_type }),
          },
          { key: 'reason', header: t('common.reason'), render: (e) => e.reason ?? '—' },
        ]}
      />
    </Section>
  )
}

function Invoices({ invoices }: { invoices: SubscriptionInvoice[] }) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  return (
    <Section title={t('admin.tenant.invoices')}>
      <DataTable<SubscriptionInvoice>
        caption={t('admin.tenant.invoices')}
        rows={invoices}
        rowKey={(i) => i.id}
        empty={t('common.none')}
        columns={[
          { key: 'no', header: t('admin.claims.invoice'), render: (i) => <span className="font-mono text-xs">{i.invoice_no}</span> },
          { key: 'date', header: t('common.date'), className: 'whitespace-nowrap', render: (i) => formatDate(i.created_at, lang) },
          { key: 'plan', header: t('admin.tenants.plan'), render: (i) => `${planLabel(t, i.plan_code, i.plan_name)} · ${cycleLabel(t, i.billing_cycle)}` },
          {
            key: 'period',
            header: t('common.period'),
            className: 'whitespace-nowrap',
            render: (i) => `${formatDate(i.period_start, lang)} – ${formatDate(i.period_end, lang)}`,
          },
          { key: 'total', header: t('common.total'), align: 'right', render: (i) => formatMoney(i.total, lang) },
          { key: 'status', header: t('common.status'), render: (i) => <InvoiceStatusBadge status={i.status} /> },
          { key: 'paid', header: t('admin.tenant.paidAt'), className: 'whitespace-nowrap', render: (i) => formatDateTime(i.paid_at, lang) },
        ]}
      />
    </Section>
  )
}

function Claims({ claims }: { claims: PaymentClaim[] }) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  return (
    <Section title={t('admin.tenant.claims')}>
      <DataTable<PaymentClaim>
        caption={t('admin.tenant.claims')}
        rows={claims}
        rowKey={(c) => c.id}
        empty={t('common.none')}
        columns={[
          { key: 'date', header: t('admin.claims.submitted'), className: 'whitespace-nowrap', render: (c) => formatDateTime(c.created_at, lang) },
          { key: 'invoice', header: t('admin.claims.invoice'), render: (c) => <span className="font-mono text-xs">{c.invoice_no}</span> },
          { key: 'method', header: t('admin.claims.method'), render: (c) => methodLabel(t, c.method) },
          { key: 'amount', header: t('common.amount'), align: 'right', render: (c) => formatMoney(c.amount, lang) },
          { key: 'ref', header: t('admin.claims.reference'), render: (c) => <span className="font-mono text-xs">{c.transaction_ref}</span> },
          { key: 'status', header: t('common.status'), render: (c) => <ClaimStatusBadge status={c.status} /> },
          { key: 'reason', header: t('admin.claims.rejectionReason'), render: (c) => c.rejection_reason ?? '—' },
        ]}
      />
      {claims.some((c) => c.status === 'pending') && (
        <Link to="/admin/claims" className={`${LINK} text-sm print:hidden`}>
          {t('admin.tenant.reviewClaims')}
        </Link>
      )}
    </Section>
  )
}
