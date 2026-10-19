import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useSearchParams } from 'react-router'

import { type TenantFilter, type TenantRow, useAdminPlans, useAdminTenants } from '../../api/platform'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { DataTable } from '../../components/ui/DataTable'
import { PageHeader } from '../../components/ui/PageHeader'
import { Select } from '../../components/ui/Select'
import { Spinner } from '../../components/ui/Spinner'
import { TextField } from '../../components/ui/TextField'
import { formatDate } from '../../lib/dates'
import { problemMessage } from '../../lib/problem'
import { LINK, planLabel, stateLabel } from './labels'
import { BusinessStatusBadge, SubscriptionBadge } from './shared'

const STATES = ['trialing', 'active', 'grace', 'read_only', 'suspended', 'cancelled', 'terminated']
const FILTER_KEYS = ['q', 'subscription_status', 'status', 'plan_code'] as const

/** Businesses (ADM-R02): search and filters kept in the URL. */
export function AdminTenantsPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const [params, setParams] = useSearchParams()
  const filter: TenantFilter = {}
  for (const key of FILTER_KEYS) {
    const value = params.get(key)
    if (value) filter[key] = value
  }
  const [text, setText] = useState(filter.q ?? '')
  const tenants = useAdminTenants(filter)
  const plans = useAdminPlans()

  function set(key: (typeof FILTER_KEYS)[number], value: string) {
    const next = new URLSearchParams(params)
    if (value) next.set(key, value)
    else next.delete(key)
    setParams(next, { replace: true })
  }
  function onSearch(e: FormEvent) {
    e.preventDefault()
    set('q', text.trim())
  }

  const rows = tenants.data?.pages.flatMap((p) => p.data) ?? []
  const first = tenants.data?.pages[0]
  const counts = Object.entries(first?.counts ?? {}).filter(([, n]) => n > 0)

  return (
    <div className="flex max-w-7xl flex-col gap-6">
      <PageHeader title={t('admin.tenants.title')} />
      <div className="flex flex-col gap-3">
        <form onSubmit={onSearch} className="flex flex-wrap items-end gap-2" role="search">
          <TextField
            className="min-w-60 flex-1"
            label={t('common.search')}
            type="search"
            value={text}
            onChange={(e) => setText(e.target.value)}
            placeholder={t('admin.tenants.searchHint')}
          />
          <Button type="submit">{t('common.search')}</Button>
        </form>
        <div className="grid gap-3 sm:grid-cols-3">
          <Select
            label={t('admin.tenants.subscriptionState')}
            value={filter.subscription_status ?? ''}
            onChange={(e) => set('subscription_status', e.target.value)}
            options={[{ value: '', label: t('common.all') }, ...STATES.map((s) => ({ value: s, label: stateLabel(t, s) }))]}
          />
          <Select
            label={t('admin.tenants.businessStatus')}
            value={filter.status ?? ''}
            onChange={(e) => set('status', e.target.value)}
            options={[
              { value: '', label: t('common.all') },
              { value: 'active', label: t('admin.businessStatus.active') },
              { value: 'suspended', label: t('admin.businessStatus.suspended') },
            ]}
          />
          <Select
            label={t('admin.tenants.plan')}
            value={filter.plan_code ?? ''}
            onChange={(e) => set('plan_code', e.target.value)}
            options={[
              { value: '', label: t('common.all') },
              ...(plans.data ?? []).map((p) => ({ value: p.code, label: planLabel(t, p.code, p.name) })),
              // Keep a plan code from a link visible even before plans load.
              ...(filter.plan_code && !plans.data?.some((p) => p.code === filter.plan_code)
                ? [{ value: filter.plan_code, label: filter.plan_code }]
                : []),
            ]}
          />
        </div>
      </div>

      {first && (
        <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-sm text-ink-muted" aria-live="polite">
          <span className="font-semibold text-ink">{t('admin.tenants.total', { count: first.total })}</span>
          {counts.map(([key, n]) => (
            <span key={key}>
              {stateLabel(t, key)}: <span className="font-semibold text-ink tabular-nums">{n}</span>
            </span>
          ))}
        </div>
      )}

      {tenants.isPending ? (
        <Spinner />
      ) : tenants.isError ? (
        <Alert tone="danger">{problemMessage(tenants.error, t)}</Alert>
      ) : (
        <DataTable<TenantRow>
          caption={t('admin.tenants.title')}
          rows={rows}
          rowKey={(r) => r.id}
          empty={Object.keys(filter).length ? t('common.noResults') : t('common.none')}
          columns={[
            {
              key: 'name',
              header: t('admin.tenants.name'),
              render: (r) => (
                <>
                  <Link to={`/admin/tenants/${r.id}`} className={LINK}>
                    {r.name}
                  </Link>
                  <span className="block font-mono text-xs text-ink-muted">{r.slug}</span>
                </>
              ),
            },
            {
              key: 'owner',
              header: t('admin.tenants.owner'),
              render: (r) => (
                <>
                  {r.owner_name ?? '—'}
                  {r.owner_phone && <span className="block text-xs text-ink-muted">{r.owner_phone}</span>}
                </>
              ),
            },
            { key: 'plan', header: t('admin.tenants.plan'), render: (r) => planLabel(t, r.plan_code, r.plan_name) },
            { key: 'state', header: t('admin.tenants.subscriptionState'), render: (r) => <SubscriptionBadge state={r.subscription_status} /> },
            {
              key: 'ends',
              header: t('admin.tenants.ends'),
              className: 'whitespace-nowrap',
              render: (r) => formatDate(r.subscription_status === 'trialing' ? r.trial_ends_at : (r.current_period_end ?? r.trial_ends_at), lang),
            },
            { key: 'status', header: t('admin.tenants.businessStatus'), render: (r) => <BusinessStatusBadge status={r.status} /> },
            { key: 'created', header: t('admin.tenants.created'), className: 'whitespace-nowrap', render: (r) => formatDate(r.created_at, lang) },
          ]}
        />
      )}
      {tenants.hasNextPage && (
        <Button variant="secondary" className="self-center" disabled={tenants.isFetchingNextPage} onClick={() => void tenants.fetchNextPage()}>
          {tenants.isFetchingNextPage ? t('common.loading') : t('common.loadMore')}
        </Button>
      )}
    </div>
  )
}
