import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate } from 'react-router'

import {
  type PromotionAction,
  type PromotionFilter,
  type PromotionState,
  type PromotionSummary,
  type PromotionType,
  promotionKeys,
  promotionsApi,
  usePromotions,
} from '../../api/promotions'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { PageHeader } from '../../components/ui/PageHeader'
import { Select } from '../../components/ui/Select'
import { Spinner } from '../../components/ui/Spinner'
import { TextField } from '../../components/ui/TextField'
import { dec, toPlain } from '../../lib/decimal'
import { problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { benefitText, localDateTime } from './format'
import { StateBadge } from './labels'

const TYPES: PromotionType[] = ['product_discount', 'flash', 'bxgy', 'coupon']
const STATES: PromotionState[] = ['running', 'scheduled', 'draft', 'paused', 'expired', 'ended']

const linkButton = 'inline-flex min-h-11 items-center rounded-lg px-4 text-sm font-semibold'

/** Promotions and coupons (§7): what runs, when, where and how much it was used. */
export function PromotionsPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [text, setText] = useState('')
  const [filter, setFilter] = useState<PromotionFilter>({ state: '' })
  const list = usePromotions(tenantId, filter)
  const canManage = can('promotions.manage')

  const act = useMutation({
    mutationFn: ({ id, action }: { id: string; action: PromotionAction }) => promotionsApi.action(id, action),
    onSuccess: (detail, { action }) => {
      void queryClient.invalidateQueries({ queryKey: promotionKeys.all(tenantId) })
      if (action === 'duplicate') void navigate(`/promotions/${detail.id}`)
    },
  })

  function onSearch(e: FormEvent) {
    e.preventDefault()
    setFilter((f) => ({ ...f, q: text.trim() || undefined }))
  }

  const usage = (p: PromotionSummary) => {
    if (p.type === 'flash' && p.quantity_limit) return t('promotions.unitsUsed', { used: toPlain(dec(p.quantity_used)), limit: toPlain(dec(p.quantity_limit)) })
    return p.total_usage_limit ? t('promotions.usesOf', { used: p.usage_count, limit: p.total_usage_limit }) : t('promotions.uses', { count: p.usage_count })
  }

  const actions = (p: PromotionSummary) => {
    if (!canManage) return null
    const button = (action: PromotionAction, label: string, danger = false) => (
      <Button
        key={action}
        variant="ghost"
        size="sm"
        className={danger ? 'text-danger' : ''}
        disabled={act.isPending}
        onClick={(e) => {
          e.stopPropagation()
          act.mutate({ id: p.id, action })
        }}
      >
        {label}
      </Button>
    )
    return (
      <span className="flex flex-wrap justify-end gap-1">
        {p.status === 'draft' && button('activate', t('promotions.actions.activate'))}
        {p.status === 'active' && button('pause', t('promotions.actions.pause'))}
        {p.status === 'paused' && button('resume', t('promotions.actions.resume'))}
        {button('duplicate', t('promotions.actions.duplicate'))}
        {p.status !== 'ended' && button('end', t('promotions.actions.end'), true)}
      </span>
    )
  }

  const columns: Column<PromotionSummary>[] = [
    {
      key: 'name',
      header: t('promotions.name'),
      card: 'title',
      render: (p) => (
        <>
          <Link to={`/promotions/${p.id}`} className="font-medium text-brand-700 hover:underline" onClick={(e) => e.stopPropagation()}>
            {lang === 'bn' && p.name_bn ? p.name_bn : p.name}
          </Link>
          <span className="block text-xs text-ink-muted">{t(`promotions.types.${p.type}`)}</span>
        </>
      ),
    },
    {
      key: 'benefit',
      header: t('promotions.benefit.title'),
      render: (p) =>
        benefitText(
          {
            ...p,
            bxgy:
              p.bxgy_buy_quantity && p.bxgy_get_quantity
                ? {
                    buy_quantity: p.bxgy_buy_quantity,
                    get_quantity: p.bxgy_get_quantity,
                    get_discount_type: p.bxgy_get_discount_type ?? 'free',
                    get_discount_value: p.bxgy_get_discount_value,
                  }
                : null,
          },
          t,
          lang,
        ),
    },
    { key: 'state', header: t('promotions.state'), render: (p) => <StateBadge state={p.state} /> },
    {
      key: 'schedule',
      header: t('promotions.schedule'),
      render: (p) =>
        p.starts_at || p.ends_at ? (
          <span className="text-xs whitespace-nowrap">
            {localDateTime(p.starts_at, lang) || '…'} → {localDateTime(p.ends_at, lang) || '…'}
          </span>
        ) : (
          <span className="text-xs text-ink-muted">{t('promotions.always')}</span>
        ),
    },
    {
      key: 'branches',
      header: t('promotions.branches'),
      showFrom: 'xl',
      render: (p) => (p.branches.length ? p.branches.join(', ') : t('promotions.allBranches')),
    },
    { key: 'usage', header: t('promotions.usage'), render: usage },
    { key: 'actions', header: <span className="sr-only">{t('common.actions')}</span>, align: 'right', render: actions },
  ]

  return (
    <div className="flex max-w-6xl flex-col gap-5">
      <PageHeader
        title={t('promotions.title')}
        subtitle={t('promotions.subtitle')}
        actions={
          canManage && (
            <Link to="/promotions/new" className={`${linkButton} bg-brand-600 text-white hover:bg-brand-700`}>
              {t('promotions.new')}
            </Link>
          )
        }
      />
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-[1fr_1fr_2fr]">
        <Select
          label={t('promotions.type')}
          value={filter.type ?? ''}
          onChange={(e) => setFilter((f) => ({ ...f, type: e.target.value as PromotionType | '' }))}
          options={[{ value: '', label: t('common.all') }, ...TYPES.map((v) => ({ value: v, label: t(`promotions.types.${v}`) }))]}
        />
        <Select
          label={t('promotions.state')}
          value={filter.state ?? ''}
          onChange={(e) => setFilter((f) => ({ ...f, state: e.target.value as PromotionState | '' }))}
          options={[{ value: '', label: t('common.all') }, ...STATES.map((v) => ({ value: v, label: t(`promotions.states.${v}`) }))]}
        />
        <form onSubmit={onSearch} role="search" className="flex items-end gap-2 sm:col-span-2 lg:col-span-1">
          <TextField
            className="min-w-0 flex-1"
            label={t('common.search')}
            type="search"
            value={text}
            placeholder={t('promotions.searchHint')}
            onChange={(e) => setText(e.target.value)}
          />
          <Button type="submit">{t('common.search')}</Button>
        </form>
      </div>
      {act.isError && <Alert tone="danger">{problemMessage(act.error, t)}</Alert>}
      {list.isPending ? (
        <Spinner />
      ) : list.isError ? (
        <Alert tone="danger">{problemMessage(list.error, t)}</Alert>
      ) : (
        <DataTable
          caption={t('promotions.title')}
          columns={columns}
          rows={list.data.data}
          rowKey={(p) => p.id}
          onRowClick={(p) => void navigate(`/promotions/${p.id}`)}
          empty={filter.q || filter.type || filter.state ? t('promotions.noMatches') : t('promotions.none')}
        />
      )}
    </div>
  )
}
