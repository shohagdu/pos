import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { type Coupon, type PromotionDetail, type Redemption, promotionKeys, promotionsApi, useCoupons, useRedemptions } from '../../api/promotions'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { Select } from '../../components/ui/Select'
import { StateTabs } from '../../components/ui/Tabs'
import { TextField } from '../../components/ui/TextField'
import { downloadCsv } from '../../lib/csv'
import { formatDateTime } from '../../lib/dates'
import { formatMoney } from '../../lib/format'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { localDateTime } from './format'

/** Coupon codes of a coupon promotion (§7): shared codes, generated single-use codes, redemptions. */
export function CouponsPanel({ tenantId, promotion, canManage }: { tenantId: string; promotion: PromotionDetail; canManage: boolean }) {
  const { t } = useTranslation()
  const [tab, setTab] = useState<'codes' | 'redemptions'>('codes')
  return (
    <Card className="flex flex-col gap-4 p-4">
      <h2 className="text-lg font-semibold">{t('promotions.coupons.title')}</h2>
      <StateTabs
        label={t('promotions.coupons.title')}
        value={tab}
        onChange={setTab}
        items={[
          { value: 'codes', label: t('promotions.coupons.codes', { count: promotion.coupon_count }) },
          { value: 'redemptions', label: t('promotions.coupons.redemptions') },
        ]}
      />
      {tab === 'codes' ? <Codes tenantId={tenantId} promotion={promotion} canManage={canManage} /> : <Redemptions tenantId={tenantId} id={promotion.id} />}
    </Card>
  )
}

function Codes({ tenantId, promotion, canManage }: { tenantId: string; promotion: PromotionDetail; canManage: boolean }) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const queryClient = useQueryClient()
  const [q, setQ] = useState('')
  const [status, setStatus] = useState('')
  const list = useCoupons(tenantId, promotion.id, { q: q.trim() || undefined, status: status || undefined })
  const refresh = () => void queryClient.invalidateQueries({ queryKey: promotionKeys.all(tenantId) })

  // One shared code.
  const [code, setCode] = useState('')
  const [limit, setLimit] = useState('')
  const [expires, setExpires] = useState('')
  const add = useMutation({
    mutationFn: () =>
      promotionsApi.addCoupon(promotion.id, {
        code: code.trim(),
        usage_limit: limit.trim() ? Number.parseInt(limit, 10) : null,
        expires_at: expires ? `${expires}:00` : null,
      }),
    onSuccess: () => {
      setCode('')
      setLimit('')
      setExpires('')
      refresh()
    },
  })
  // Generated single-use codes.
  const [count, setCount] = useState('50')
  const [prefix, setPrefix] = useState('')
  const [generated, setGenerated] = useState<Coupon[] | null>(null)
  const generate = useMutation({
    mutationFn: () => promotionsApi.generate(promotion.id, { count: Number.parseInt(count, 10) || 0, prefix: prefix.trim() || undefined, usage_limit: 1 }),
    onSuccess: (made) => {
      setGenerated(made)
      refresh()
    },
  })
  const toggle = useMutation({
    mutationFn: (c: Coupon) => promotionsApi.couponAction(c.id, c.status === 'disabled' ? 'enable' : 'disable'),
    onSuccess: refresh,
  })
  const addErrors = fieldErrors(add.error, t)
  const genErrors = fieldErrors(generate.error, t)

  const exportCodes = (rows: Coupon[]) =>
    downloadCsv(`coupons-${promotion.name}`, rows, [
      { header: t('promotions.coupons.code'), value: (c) => c.code },
      { header: t('promotions.coupons.uses'), value: (c) => `${c.used_count}${c.usage_limit ? ` / ${c.usage_limit}` : ''}` },
      { header: t('promotions.coupons.expires'), value: (c) => c.expires_at ?? '' },
      { header: t('common.status'), value: (c) => t(`promotions.coupons.statuses.${c.status}`) },
    ])

  const columns: Column<Coupon>[] = [
    {
      key: 'code',
      header: t('promotions.coupons.code'),
      card: 'title',
      render: (c) => <span className="font-mono font-semibold tracking-wide">{c.code}</span>,
    },
    { key: 'type', header: t('promotions.coupons.type'), render: (c) => t(`promotions.coupons.types.${c.type}`) },
    { key: 'uses', header: t('promotions.coupons.uses'), render: (c) => `${c.used_count}${c.usage_limit ? ` / ${c.usage_limit}` : ''}` },
    { key: 'customer', header: t('promotions.coupons.customer'), render: (c) => c.customer_name ?? '—' },
    { key: 'expires', header: t('promotions.coupons.expires'), render: (c) => (c.expires_at ? localDateTime(c.expires_at, lang) : '—') },
    {
      key: 'status',
      header: t('common.status'),
      render: (c) => (
        <Badge tone={c.status === 'active' ? 'good' : c.status === 'used' ? 'neutral' : 'bad'}>{t(`promotions.coupons.statuses.${c.status}`)}</Badge>
      ),
    },
    {
      key: 'actions',
      header: <span className="sr-only">{t('common.actions')}</span>,
      align: 'right',
      render: (c) =>
        canManage ? (
          <Button variant="ghost" size="sm" disabled={toggle.isPending} onClick={() => toggle.mutate(c)}>
            {c.status === 'disabled' ? t('promotions.coupons.enable') : t('promotions.coupons.disable')}
          </Button>
        ) : null,
    },
  ]

  return (
    <div className="flex flex-col gap-4">
      {canManage && (
        <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
          <form
            className="flex flex-col gap-3 rounded-lg border border-border p-3"
            onSubmit={(e: FormEvent) => {
              e.preventDefault()
              add.mutate()
            }}
          >
            <h3 className="font-semibold">{t('promotions.coupons.addPublic')}</h3>
            <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
              <TextField
                label={t('promotions.coupons.code')}
                required
                value={code}
                onChange={(e) => setCode(e.target.value.toUpperCase())}
                error={addErrors.code}
              />
              <TextField
                label={t('promotions.coupons.usageLimit')}
                inputMode="numeric"
                value={limit}
                onChange={(e) => setLimit(e.target.value)}
                error={addErrors.usage_limit}
              />
              <TextField label={t('promotions.coupons.expires')} type="datetime-local" value={expires} onChange={(e) => setExpires(e.target.value)} />
            </div>
            {add.isError && hasFormError(add.error) && <Alert tone="danger">{problemMessage(add.error, t)}</Alert>}
            <Button type="submit" className="self-start" disabled={!code.trim() || add.isPending}>
              {t('promotions.coupons.add')}
            </Button>
          </form>
          <form
            className="flex flex-col gap-3 rounded-lg border border-border p-3"
            onSubmit={(e: FormEvent) => {
              e.preventDefault()
              generate.mutate()
            }}
          >
            <h3 className="font-semibold">{t('promotions.coupons.generate')}</h3>
            <div className="grid grid-cols-2 gap-3">
              <TextField
                label={t('promotions.coupons.count')}
                inputMode="numeric"
                value={count}
                onChange={(e) => setCount(e.target.value)}
                error={genErrors.count}
              />
              <TextField
                label={t('promotions.coupons.prefix')}
                value={prefix}
                onChange={(e) => setPrefix(e.target.value.toUpperCase())}
                error={genErrors.prefix}
              />
            </div>
            <p className="text-xs text-ink-muted">{t('promotions.coupons.generateHint')}</p>
            {generate.isError && hasFormError(generate.error) && <Alert tone="danger">{problemMessage(generate.error, t)}</Alert>}
            <div className="flex flex-wrap gap-2">
              <Button type="submit" disabled={generate.isPending}>
                {generate.isPending ? t('common.loading') : t('promotions.coupons.generateButton')}
              </Button>
              {generated && generated.length > 0 && (
                <>
                  <Button variant="secondary" onClick={() => exportCodes(generated)}>
                    {t('promotions.coupons.downloadNew', { count: generated.length })}
                  </Button>
                  <Button variant="secondary" onClick={() => window.print()}>
                    {t('common.print')}
                  </Button>
                </>
              )}
            </div>
          </form>
        </div>
      )}
      {generated && generated.length > 0 && (
        // Printable sheet of the codes just generated.
        <ul className="hidden grid-cols-3 gap-2 font-mono text-sm print:grid">
          {generated.map((c) => (
            <li key={c.id} className="rounded border border-ink p-2 text-center">
              <div className="text-xs">{promotion.name}</div>
              <div className="text-base font-bold tracking-wider">{c.code}</div>
            </li>
          ))}
        </ul>
      )}
      <div className="flex flex-wrap items-end gap-3 print:hidden">
        <TextField className="min-w-0 flex-1" label={t('common.search')} type="search" value={q} onChange={(e) => setQ(e.target.value)} />
        <Select
          label={t('common.status')}
          value={status}
          onChange={(e) => setStatus(e.target.value)}
          options={[
            { value: '', label: t('common.all') },
            ...(['active', 'used', 'disabled'] as const).map((s) => ({ value: s, label: t(`promotions.coupons.statuses.${s}`) })),
          ]}
        />
        <Button variant="secondary" disabled={!list.data?.data.length} onClick={() => list.data && exportCodes(list.data.data)}>
          {t('common.exportCsv')}
        </Button>
      </div>
      {toggle.isError && <Alert tone="danger">{problemMessage(toggle.error, t)}</Alert>}
      <div className="print:hidden">
        {list.isError ? (
          <Alert tone="danger">{problemMessage(list.error, t)}</Alert>
        ) : (
          <DataTable
            caption={t('promotions.coupons.codes', { count: list.data?.total ?? 0 })}
            columns={columns}
            rows={list.data?.data ?? []}
            rowKey={(c) => c.id}
            empty={t('promotions.coupons.none')}
          />
        )}
      </div>
    </div>
  )
}

function Redemptions({ tenantId, id }: { tenantId: string; id: string }) {
  const { t, i18n } = useTranslation()
  const list = useRedemptions(tenantId, id)
  const columns: Column<Redemption>[] = [
    { key: 'code', header: t('promotions.coupons.code'), render: (r) => <span className="font-mono">{r.code}</span> },
    {
      key: 'invoice',
      header: t('promotions.coupons.invoice'),
      card: 'title',
      render: (r) => (
        <Link to={`/sales/${r.sale_id}`} className="font-medium text-brand-700 hover:underline">
          {r.invoice_no}
        </Link>
      ),
    },
    { key: 'customer', header: t('promotions.coupons.customer'), render: (r) => r.customer_name },
    { key: 'cashier', header: t('promotions.coupons.cashier'), render: (r) => r.cashier_name ?? '—' },
    { key: 'branch', header: t('promotions.branches'), showFrom: 'xl', render: (r) => r.branch_name },
    { key: 'date', header: t('common.date'), render: (r) => formatDateTime(r.created_at, i18n.language) },
    {
      key: 'status',
      header: t('common.status'),
      render: (r) => <Badge tone={r.status === 'applied' ? 'good' : 'neutral'}>{t(`promotions.coupons.redemptionStatuses.${r.status}`)}</Badge>,
    },
    { key: 'amount', header: t('common.amount'), align: 'right', render: (r) => formatMoney(r.discount_amount, i18n.language) },
  ]
  if (list.isError) return <Alert tone="danger">{problemMessage(list.error, t)}</Alert>
  return (
    <DataTable
      caption={t('promotions.coupons.redemptions')}
      columns={columns}
      rows={list.data ?? []}
      rowKey={(r) => r.id}
      empty={t('promotions.coupons.noRedemptions')}
    />
  )
}
