import { useQuery } from '@tanstack/react-query'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useSearchParams } from 'react-router'

import { MOVEMENT_TYPES, type Movement, type MovementFilter, inventoryApi, inventoryKeys, useMovements, useStockLocations } from '../../api/inventory'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { DateRangeFilter } from '../../components/ui/DateRangeFilter'
import { PageHeader } from '../../components/ui/PageHeader'
import { Select } from '../../components/ui/Select'
import { downloadCsv } from '../../lib/csv'
import { dec, toPlain } from '../../lib/decimal'
import { formatDateTime } from '../../lib/dates'
import { formatMoney, formatQuantity } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { LoadMore, QueryStatus, TextLink } from '../money/shared'
import { absText, fetchAll, stamp, useTenant, referencePath } from '../money/utils'
import { InventoryNav } from './InventoryNav'

const TYPE_TONE: Record<string, 'good' | 'bad' | 'info' | 'warn' | 'neutral'> = {
  opening: 'info',
  purchase_receipt: 'good',
  purchase_return: 'warn',
  sale: 'neutral',
  sale_cancel: 'info',
  sale_return: 'info',
  adjustment: 'warn',
}

/** Stock movement ledger (`/inventory/movements`), newest first. */
export function MovementsPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { tenantId } = useTenant()
  const [params, setParams] = useSearchParams()
  const variantId = params.get('variant_id') ?? ''
  const [range, setRange] = useState({ from: '', to: '' })
  const [type, setType] = useState('')
  const [locationId, setLocationId] = useState('')
  const [exportError, setExportError] = useState<unknown>(null)
  const filter: MovementFilter = {
    variant_id: variantId || undefined,
    movement_type: type || undefined,
    location_id: locationId || undefined,
    from: range.from || undefined,
    to: range.to || undefined,
  }
  const movements = useMovements(tenantId, filter)
  const locations = useStockLocations(tenantId)
  // The product's name for the filter chip (the stock row of that variant).
  const product = useQuery({
    queryKey: [...inventoryKeys.all(tenantId), 'stock-row', variantId],
    queryFn: () => inventoryApi.stock({ variant_id: variantId }, null, 1),
    enabled: Boolean(variantId),
  })
  const productRow = variantId ? product.data?.data[0] : undefined
  const rows = movements.data?.pages.flatMap((p) => p.data) ?? []

  const clearProduct = () =>
    setParams(
      (prev) => {
        const next = new URLSearchParams(prev)
        next.delete('variant_id')
        return next
      },
      { replace: true },
    )

  const qtyIn = (m: Movement) => (dec(m.quantity) > 0n ? formatQuantity(m.quantity, lang, true) : '')
  const qtyOut = (m: Movement) => (dec(m.quantity) < 0n ? formatQuantity(absText(m.quantity), lang, true) : '')
  const typeLabel = (type: string) => t(`inventory.movementTypes.${type}`, { defaultValue: type })

  const columns: Column<Movement>[] = [
    { key: 'date', header: t('inventory.dateTime'), render: (m) => <span className="whitespace-nowrap">{formatDateTime(m.created_at, lang)}</span> },
    {
      key: 'product',
      header: t('inventory.product'),
      render: (m) => (
        <div className="flex flex-col">
          <span className="font-medium">{m.product_name}</span>
          <span className="font-mono text-xs text-ink-muted">{m.sku}</span>
        </div>
      ),
    },
    { key: 'type', header: t('inventory.movementType'), render: (m) => <Badge tone={TYPE_TONE[m.movement_type] ?? 'neutral'}>{typeLabel(m.movement_type)}</Badge> },
    { key: 'in', header: t('inventory.in'), align: 'right', render: (m) => <span className="text-brand-700">{qtyIn(m)}</span> },
    { key: 'out', header: t('inventory.outQty'), align: 'right', render: (m) => <span className="text-danger">{qtyOut(m)}</span> },
    {
      key: 'balance',
      header: t('inventory.balanceAfter'),
      align: 'right',
      render: (m) => `${formatQuantity(m.balance_qty_after, lang, true)} ${m.unit_short_name}`,
    },
    { key: 'cost', header: t('inventory.unitCost'), align: 'right', render: (m) => (m.unit_cost === null ? '—' : formatMoney(m.unit_cost, lang)) },
    {
      key: 'ref',
      header: t('inventory.referenceNo'),
      render: (m) => {
        const path = referencePath(m.reference_type, m.reference_id)
        const label = m.reference_no ?? '—'
        return path && m.reference_no ? <TextLink to={path}>{label}</TextLink> : label
      },
    },
    { key: 'reason', header: t('common.reason'), render: (m) => (m.reason ? t(`inventory.reasons.${m.reason}`, { defaultValue: m.reason }) : '—') },
  ]

  async function exportCsv() {
    setExportError(null)
    try {
      const all = await fetchAll((cursor) => inventoryApi.movements(filter, cursor))
      downloadCsv(`stock-movements-${stamp()}`, all, [
        { header: t('inventory.dateTime'), value: (m) => m.created_at },
        { header: t('inventory.product'), value: (m) => m.product_name },
        { header: t('inventory.sku'), value: (m) => m.sku },
        { header: t('inventory.location'), value: (m) => m.location_name },
        { header: t('inventory.movementType'), value: (m) => typeLabel(m.movement_type) },
        { header: t('inventory.in'), value: (m) => (dec(m.quantity) > 0n ? toPlain(dec(m.quantity)) : '') },
        { header: t('inventory.outQty'), value: (m) => (dec(m.quantity) < 0n ? toPlain(-dec(m.quantity)) : '') },
        { header: t('inventory.balanceAfter'), value: (m) => m.balance_qty_after },
        { header: t('inventory.unitCost'), value: (m) => m.unit_cost },
        { header: t('inventory.referenceNo'), value: (m) => m.reference_no },
        { header: t('common.reason'), value: (m) => m.reason },
      ])
    } catch (err) {
      setExportError(err)
    }
  }

  const activeLocations = (locations.data ?? []).filter((l) => l.status === 'active')
  return (
    <div className="flex flex-col gap-6">
      <PageHeader title={t('inventory.movementsTitle')} subtitle={t('inventory.movementsSubtitle')} />
      <InventoryNav />
      <div className="flex flex-col gap-3 print:hidden">
        {variantId && (
          <div className="flex flex-wrap items-center gap-2 rounded-lg border border-border bg-surface px-3 py-2 text-sm">
            <span className="text-ink-muted">{t('inventory.product')}:</span>
            <span className="font-medium">{productRow ? `${productRow.name} (${productRow.sku})` : t('common.loading')}</span>
            <Button variant="ghost" className="ms-auto" onClick={clearProduct}>
              {t('inventory.clearProduct')}
            </Button>
          </div>
        )}
        <div className="flex flex-wrap items-end gap-3">
          <Select
            label={t('inventory.movementType')}
            value={type}
            onChange={(e) => setType(e.target.value)}
            options={[{ value: '', label: t('common.all') }, ...MOVEMENT_TYPES.map((m) => ({ value: m, label: typeLabel(m) }))]}
          />
          {activeLocations.length > 1 && (
            <Select
              label={t('inventory.location')}
              value={locationId}
              onChange={(e) => setLocationId(e.target.value)}
              options={[{ value: '', label: t('inventory.allLocations') }, ...activeLocations.map((l) => ({ value: l.id, label: l.name }))]}
            />
          )}
          <DateRangeFilter from={range.from} to={range.to} onChange={setRange} />
          <Button variant="secondary" className="ms-auto" disabled={rows.length === 0} onClick={() => void exportCsv()}>
            {t('common.exportCsv')}
          </Button>
        </div>
      </div>
      {exportError !== null && <Alert tone="danger">{problemMessage(exportError, t)}</Alert>}
      {movements.isSuccess ? (
        <>
          <DataTable caption={t('inventory.movementsTitle')} columns={columns} rows={rows} rowKey={(m) => m.id} empty={t('inventory.noMovements')} />
          <LoadMore query={movements} />
        </>
      ) : (
        <QueryStatus query={movements} />
      )}
    </div>
  )
}
