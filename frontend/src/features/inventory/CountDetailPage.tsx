import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useParams } from 'react-router'

import { type CountItem, type StockCount, inventoryApi, inventoryKeys, useCount } from '../../api/inventory'
import { newKey } from '../../api/query'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { Checkbox } from '../../components/ui/Checkbox'
import { ConfirmButton } from '../../components/ui/ConfirmButton'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { PageHeader } from '../../components/ui/PageHeader'
import { TextField } from '../../components/ui/TextField'
import { type Dec, dec, mul, sum, toFixed, toPlain, tryParse } from '../../lib/decimal'
import { formatDate, formatDateTime } from '../../lib/dates'
import { formatMoney, formatQuantity } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { QueryStatus } from '../money/shared'
import { useTenant } from '../money/utils'
import { COUNT_TONES } from './tones'

const INPUT =
  'min-h-10 w-full rounded-lg border border-border bg-surface px-2 text-sm outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100 aria-invalid:border-danger'

const entryOf = (i: CountItem) => (i.counted_qty === null ? '' : toPlain(dec(i.counted_qty)))

/** A stock count sheet (`/inventory/counts/:id`): enter, save, post or cancel. */
export function CountDetailPage() {
  const { t } = useTranslation()
  const { id = '' } = useParams()
  const { tenantId, can } = useTenant()
  const count = useCount(tenantId, id)
  if (!can('inventory.count')) return <Alert tone="warning">{t('inventory.noPermission')}</Alert>
  if (!count.data) return <QueryStatus query={count} />
  return <CountSheet key={count.data.id} count={count.data} tenantId={tenantId} />
}

function CountSheet({ count: c, tenantId }: { count: StockCount; tenantId: string }) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const queryClient = useQueryClient()
  const { can } = useTenant()
  const open = c.status === 'in_progress'
  const [entries, setEntries] = useState<Record<string, string>>(() => Object.fromEntries(c.items.map((i) => [i.id, entryOf(i)])))
  const [filter, setFilter] = useState('')
  const [onlyUncounted, setOnlyUncounted] = useState(false)
  const [onlyDifferences, setOnlyDifferences] = useState(false)
  const [postKey, setPostKey] = useState(newKey)

  const entry = (i: CountItem) => entries[i.id] ?? ''
  const invalid = (i: CountItem) => {
    const text = entry(i).trim()
    if (!text) return false
    const q = tryParse(text)
    return q === null || q < 0n || (!i.allow_fraction && q % 10n ** 9n !== 0n)
  }
  const changed = c.items.filter((i) => entry(i).trim() !== entryOf(i))
  const hasInvalid = changed.some(invalid)
  /** Counted quantity from the sheet (open) or the server (closed). */
  const countedOf = (i: CountItem): Dec | null => {
    if (!open) return i.counted_qty === null ? null : dec(i.counted_qty)
    return invalid(i) || !entry(i).trim() ? null : dec(entry(i))
  }
  const varianceOf = (i: CountItem): Dec | null => {
    const counted = countedOf(i)
    return counted === null ? null : counted - dec(i.system_qty)
  }
  const hasCost = c.items.some((i) => i.unit_cost !== null)
  const valueOf = (i: CountItem): Dec | null => {
    const v = varianceOf(i)
    return v === null || i.unit_cost === null ? null : mul(v, dec(i.unit_cost))
  }

  const refresh = async (saved: StockCount) => {
    queryClient.setQueryData(inventoryKeys.count(tenantId, saved.id), saved)
    setEntries(Object.fromEntries(saved.items.map((i) => [i.id, entryOf(i)])))
    await queryClient.invalidateQueries({ queryKey: inventoryKeys.all(tenantId) })
  }
  const saveEntries = useMutation({
    mutationFn: () =>
      inventoryApi.enterCount(
        c.id,
        changed.map((i) => ({ item_id: i.id, counted_qty: entry(i).trim() || null })),
      ),
    onSuccess: refresh,
  })
  const post = useMutation({
    mutationFn: () => inventoryApi.postCount(c.id, postKey),
    onSuccess: async (saved) => {
      setPostKey(newKey())
      await refresh(saved)
      await queryClient.invalidateQueries({ queryKey: ['reports', tenantId] })
    },
  })
  const cancel = useMutation({ mutationFn: () => inventoryApi.cancelCount(c.id), onSuccess: refresh })
  const actionError = saveEntries.error ?? post.error ?? cancel.error

  const needle = filter.trim().toLowerCase()
  const rows = c.items.filter((i) => {
    if (needle && ![i.product_name, i.sku, i.barcode ?? ''].some((s) => s.toLowerCase().includes(needle))) return false
    if (onlyUncounted && countedOf(i) !== null) return false
    if (onlyDifferences && (varianceOf(i) ?? 0n) === 0n) return false
    return true
  })
  const countedCount = c.items.filter((i) => countedOf(i) !== null).length
  const totalValue = hasCost ? sum(c.items.map((i) => valueOf(i) ?? 0n)) : 0n

  const columns: Column<CountItem>[] = [
    {
      key: 'product',
      header: t('inventory.product'),
      render: (i) => (
        <div className="flex flex-col">
          <span className="font-medium">{i.product_name}</span>
          <span className="font-mono text-xs text-ink-muted">
            {i.sku}
            {i.barcode && ` · ${i.barcode}`}
          </span>
        </div>
      ),
    },
    { key: 'system', header: t('inventory.systemQty'), align: 'right', render: (i) => `${formatQuantity(i.system_qty, lang, true)} ${i.unit_short_name}` },
    {
      key: 'counted',
      header: t('inventory.counted'),
      align: 'right',
      render: (i) =>
        open ? (
          <div className="flex flex-col items-end">
            <input
              aria-label={t('inventory.countedFor', { name: i.product_name })}
              aria-invalid={invalid(i) ? true : undefined}
              className={`${INPUT} max-w-28 text-right`}
              inputMode={i.allow_fraction ? 'decimal' : 'numeric'}
              value={entry(i)}
              onChange={(e) => setEntries((x) => ({ ...x, [i.id]: e.target.value }))}
            />
            {invalid(i) && <p className="text-sm text-danger">{t(i.allow_fraction ? 'inventory.errors.counted' : 'inventory.errors.whole')}</p>}
          </div>
        ) : i.counted_qty === null ? (
          '—'
        ) : (
          formatQuantity(i.counted_qty, lang, true)
        ),
    },
    {
      key: 'variance',
      header: t('inventory.variance'),
      align: 'right',
      render: (i) => {
        const v = varianceOf(i)
        if (v === null) return '—'
        return (
          <span className={v < 0n ? 'text-danger' : v > 0n ? 'text-brand-700' : ''}>
            {v > 0n ? '+' : ''}
            {formatQuantity(toPlain(v), lang, true)}
          </span>
        )
      },
    },
    ...(hasCost
      ? ([
          {
            key: 'value',
            header: t('inventory.varianceValue'),
            align: 'right',
            render: (i) => {
              const v = valueOf(i)
              return v === null ? '—' : formatMoney(toFixed(v), lang)
            },
          },
        ] satisfies Column<CountItem>[])
      : []),
  ]

  const title = `${t('inventory.count')} ${c.count_no ?? ''}`.trim()
  return (
    <div className="flex max-w-5xl flex-col gap-6">
      <Link to="/inventory/counts" className="text-sm font-medium text-brand-700 hover:underline print:hidden">
        ← {t('inventory.counts')}
      </Link>
      <PageHeader
        title={title}
        subtitle={formatDate(c.business_date, lang)}
        actions={
          <div className="flex flex-wrap items-center gap-2 print:hidden">
            {open && (
              <>
                <Button disabled={saveEntries.isPending || changed.length === 0 || hasInvalid} onClick={() => saveEntries.mutate()}>
                  {saveEntries.isPending ? t('common.saving') : t('inventory.saveEntries')}
                </Button>
                {can('inventory.adjust') && (
                  <ConfirmButton disabled={post.isPending || changed.length > 0} onConfirm={() => post.mutate()}>
                    {t('inventory.postCount')}
                  </ConfirmButton>
                )}
                <ConfirmButton disabled={cancel.isPending} onConfirm={() => cancel.mutate()}>
                  {t('inventory.cancelCount')}
                </ConfirmButton>
              </>
            )}
            <Button variant="secondary" onClick={() => window.print()}>
              {t('common.print')}
            </Button>
          </div>
        }
      />
      {actionError && <Alert tone="danger">{problemMessage(actionError, t)}</Alert>}
      {saveEntries.isSuccess && changed.length === 0 && <Alert tone="success">{t('inventory.entriesSaved')}</Alert>}
      {open && changed.length > 0 && <Alert tone="info">{t('inventory.unsavedEntries')}</Alert>}
      {open && !can('inventory.adjust') && <Alert tone="warning">{t('inventory.postNeedsAdjust')}</Alert>}
      <Card>
        <dl className="grid gap-4 text-sm sm:grid-cols-3">
          <div>
            <dt className="text-ink-muted">{t('common.status')}</dt>
            <dd>
              <Badge tone={COUNT_TONES[c.status]}>{t(`inventory.countStatuses.${c.status}`, { defaultValue: c.status })}</Badge>
            </dd>
          </div>
          <div>
            <dt className="text-ink-muted">{t('inventory.location')}</dt>
            <dd className="font-medium">{c.location_name}</dd>
          </div>
          <div>
            <dt className="text-ink-muted">{t('inventory.countCategory')}</dt>
            <dd className="font-medium">{c.category_name ?? t('inventory.allCategories')}</dd>
          </div>
          <div>
            <dt className="text-ink-muted">{t('inventory.counted')}</dt>
            <dd className="font-medium">{t('inventory.countedProgress', { counted: countedCount, total: c.item_count })}</dd>
          </div>
          {hasCost && (
            <div>
              <dt className="text-ink-muted">{t('inventory.varianceValue')}</dt>
              <dd className={`font-medium ${totalValue < 0n ? 'text-danger' : ''}`}>
                {formatMoney(open || c.variance_value === null ? toFixed(totalValue) : c.variance_value, lang)}
              </dd>
            </div>
          )}
          <div>
            <dt className="text-ink-muted">{t('inventory.createdBy')}</dt>
            <dd className="font-medium">
              {c.created_by_name ?? '—'} · {formatDateTime(c.created_at, lang)}
            </dd>
          </div>
          {c.snapshot_at && (
            <div>
              <dt className="text-ink-muted">{t('inventory.snapshotAt')}</dt>
              <dd className="font-medium">{formatDateTime(c.snapshot_at, lang)}</dd>
            </div>
          )}
          {c.posted_at && (
            <div>
              <dt className="text-ink-muted">{t('inventory.postedAt')}</dt>
              <dd className="font-medium">{formatDateTime(c.posted_at, lang)}</dd>
            </div>
          )}
          {c.note && (
            <div className="sm:col-span-3">
              <dt className="text-ink-muted">{t('common.note')}</dt>
              <dd className="whitespace-pre-line">{c.note}</dd>
            </div>
          )}
        </dl>
      </Card>
      <div className="flex flex-wrap items-end gap-4 print:hidden">
        <TextField
          className="min-w-60 flex-1"
          label={t('inventory.filterItems')}
          type="search"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          placeholder={t('inventory.filterItemsHint')}
        />
        <Checkbox label={t('inventory.onlyUncounted')} checked={onlyUncounted} onChange={(e) => setOnlyUncounted(e.target.checked)} />
        <Checkbox label={t('inventory.onlyDifferences')} checked={onlyDifferences} onChange={(e) => setOnlyDifferences(e.target.checked)} />
      </div>
      <DataTable caption={t('inventory.lines')} columns={columns} rows={rows} rowKey={(i) => i.id} empty={t('common.noResults')} />
    </div>
  )
}
