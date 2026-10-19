import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'

import { useCategories } from '../../api/catalog'
import { COUNT_STATUSES, type CountSummary, inventoryApi, inventoryKeys, useCounts, useStockLocations } from '../../api/inventory'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { Dialog } from '../../components/ui/Dialog'
import { PageHeader } from '../../components/ui/PageHeader'
import { Select } from '../../components/ui/Select'
import { TextField } from '../../components/ui/TextField'
import { dec } from '../../lib/decimal'
import { formatDate } from '../../lib/dates'
import { formatMoney } from '../../lib/format'
import { optional } from '../../lib/form'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { useBranch } from '../branches/useBranch'
import { LoadMore, QueryStatus, TextLink } from '../money/shared'
import { useTenant } from '../money/utils'
import { InventoryNav } from './InventoryNav'
import { COUNT_TONES } from './tones'

/** Stock counts (`/inventory/counts`): list and start a new count. */
export function CountsPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { tenantId, can } = useTenant()
  const [status, setStatus] = useState('')
  const [creating, setCreating] = useState(false)
  const list = useCounts(tenantId, status || undefined)
  const rows = list.data?.pages.flatMap((p) => p.data) ?? []

  if (!can('inventory.count')) return <Alert tone="warning">{t('inventory.noPermission')}</Alert>

  const columns: Column<CountSummary>[] = [
    { key: 'no', header: t('inventory.countNo'), render: (c) => <TextLink to={`/inventory/counts/${c.id}`}>{c.count_no ?? t('common.view')}</TextLink> },
    { key: 'date', header: t('common.date'), render: (c) => <span className="whitespace-nowrap">{formatDate(c.business_date, lang)}</span> },
    { key: 'location', header: t('inventory.location'), render: (c) => c.location_name },
    { key: 'category', header: t('inventory.countCategory'), render: (c) => c.category_name ?? t('inventory.allCategories') },
    { key: 'progress', header: t('inventory.counted'), align: 'right', render: (c) => t('inventory.countedProgress', { counted: c.counted_count, total: c.item_count }) },
    {
      key: 'variance',
      header: t('inventory.varianceValue'),
      align: 'right',
      render: (c) =>
        c.variance_value === null ? '—' : <span className={dec(c.variance_value) < 0n ? 'text-danger' : ''}>{formatMoney(c.variance_value, lang)}</span>,
    },
    { key: 'status', header: t('common.status'), render: (c) => <Badge tone={COUNT_TONES[c.status]}>{t(`inventory.countStatuses.${c.status}`, { defaultValue: c.status })}</Badge> },
    { key: 'by', header: t('inventory.createdBy'), render: (c) => c.created_by_name ?? '—' },
  ]

  return (
    <div className="flex flex-col gap-6">
      <PageHeader title={t('inventory.counts')} subtitle={t('inventory.countsSubtitle')} actions={<Button onClick={() => setCreating(true)}>{t('inventory.newCount')}</Button>} />
      <InventoryNav />
      <div className="flex flex-wrap items-end gap-3 print:hidden">
        <Select
          label={t('common.status')}
          value={status}
          onChange={(e) => setStatus(e.target.value)}
          options={[{ value: '', label: t('common.all') }, ...COUNT_STATUSES.map((s) => ({ value: s, label: t(`inventory.countStatuses.${s}`) }))]}
        />
      </div>
      {list.isSuccess ? (
        <>
          <DataTable caption={t('inventory.counts')} columns={columns} rows={rows} rowKey={(c) => c.id} empty={t('inventory.noCounts')} />
          <LoadMore query={list} />
        </>
      ) : (
        <QueryStatus query={list} />
      )}
      {creating && <NewCountDialog tenantId={tenantId} onClose={() => setCreating(false)} />}
    </div>
  )
}

/** Starts a count: the system quantities are snapshotted on the server. */
function NewCountDialog({ tenantId, onClose }: { tenantId: string; onClose: () => void }) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const { branchId, choices } = useBranch()
  const locations = useStockLocations(tenantId)
  const categories = useCategories(tenantId)
  const branch = branchId ?? choices.find((b) => b.is_default)?.id ?? choices[0]?.id
  const branchLocations = (locations.data ?? []).filter((l) => l.status === 'active' && l.branch_id === branch)
  const [locationId, setLocationId] = useState('')
  const [categoryId, setCategoryId] = useState('')
  const [note, setNote] = useState('')

  const save = useMutation({
    mutationFn: () => inventoryApi.createCount({ location_id: locationId || undefined, category_id: categoryId || undefined, note: optional(note) }),
    onSuccess: async (saved) => {
      queryClient.setQueryData(inventoryKeys.count(tenantId, saved.id), saved)
      await queryClient.invalidateQueries({ queryKey: inventoryKeys.all(tenantId) })
      navigate(`/inventory/counts/${saved.id}`)
    },
  })

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    save.mutate()
  }

  const errors = fieldErrors(save.error, t)
  const indent = (depth: number) => '  '.repeat(depth)
  return (
    <Dialog open onClose={onClose} title={t('inventory.newCount')}>
      <form onSubmit={onSubmit} noValidate className="flex flex-col gap-4">
        <p className="text-sm text-ink-muted">{t('inventory.countHint')}</p>
        {save.isError && hasFormError(save.error) && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}
        {branchLocations.length > 1 && (
          <Select
            label={t('inventory.location')}
            value={locationId}
            onChange={(e) => setLocationId(e.target.value)}
            error={errors.location_id}
            options={[{ value: '', label: t('inventory.defaultLocation') }, ...branchLocations.map((l) => ({ value: l.id, label: l.name }))]}
          />
        )}
        <Select
          label={t('inventory.countCategory')}
          value={categoryId}
          onChange={(e) => setCategoryId(e.target.value)}
          error={errors.category_id}
          options={[
            { value: '', label: t('inventory.allCategories') },
            ...(categories.data ?? []).filter((c) => c.status === 'active').map((c) => ({ value: c.id, label: `${indent(c.depth)}${c.name}` })),
          ]}
        />
        <TextField label={`${t('common.note')} (${t('common.optional')})`} value={note} onChange={(e) => setNote(e.target.value)} maxLength={2000} error={errors.note} />
        <div className="flex flex-wrap justify-end gap-2">
          <Button variant="ghost" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button type="submit" disabled={save.isPending}>
            {save.isPending ? t('common.saving') : t('inventory.startCount')}
          </Button>
        </div>
      </form>
    </Dialog>
  )
}
