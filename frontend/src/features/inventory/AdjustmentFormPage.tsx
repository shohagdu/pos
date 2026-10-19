import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate, useSearchParams } from 'react-router'

import { type Product, catalogApi, catalogKeys } from '../../api/catalog'
import { ADJUSTMENT_REASONS, type AdjustmentInput, type AdjustmentReason, inventoryApi, inventoryKeys, useStockLocations } from '../../api/inventory'
import { newKey } from '../../api/query'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { PageHeader } from '../../components/ui/PageHeader'
import { Select } from '../../components/ui/Select'
import { Spinner } from '../../components/ui/Spinner'
import { TextField } from '../../components/ui/TextField'
import { dec, mul, sum, toFixed, tryParse } from '../../lib/decimal'
import { formatMoney } from '../../lib/format'
import { optional } from '../../lib/form'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { trimDecimal, useTenant } from '../money/utils'

type Direction = 'add' | 'remove'

interface Line {
  variant_id: string
  name: string
  sku: string
  unit: string
  allow_fraction: boolean
  direction: Direction
  quantity: string
  unit_cost: string
}

const INPUT =
  'min-h-10 w-full rounded-lg border border-border bg-surface px-2 text-sm outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100 aria-invalid:border-danger'

/** Checks one line's quantity; returns an i18n key or null. */
function quantityProblem(line: Line): string | null {
  const q = tryParse(line.quantity)
  if (q === null || q <= 0n) return 'inventory.errors.quantity'
  if (!line.allow_fraction && q % 10n ** 9n !== 0n) return 'inventory.errors.whole'
  return null
}

/** `/inventory/adjustments/new` (`?reason=opening` → opening stock). */
export function AdjustmentFormPage() {
  const { t } = useTranslation()
  const [params] = useSearchParams()
  const opening = params.get('reason') === 'opening'
  const { can } = useTenant()
  const allowed = opening ? can('inventory.opening_stock') : can('inventory.adjust')
  if (!allowed) {
    return (
      <div className="flex max-w-3xl flex-col gap-4">
        <h1 className="text-2xl font-bold">{opening ? t('inventory.openingStock') : t('inventory.newAdjustment')}</h1>
        <Alert tone="warning">{t('inventory.noPermission')}</Alert>
      </div>
    )
  }
  return <AdjustmentForm key={opening ? 'opening' : 'adjust'} opening={opening} />
}

function AdjustmentForm({ opening }: { opening: boolean }) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const { tenantId, can } = useTenant()
  const canCost = can('products.view_cost')
  const showCost = opening && canCost
  const locations = useStockLocations(tenantId)
  const activeLocations = (locations.data ?? []).filter((l) => l.status === 'active')

  const [reason, setReason] = useState<string>('')
  const [locationId, setLocationId] = useState('')
  const [note, setNote] = useState('')
  const [lines, setLines] = useState<Line[]>([])
  const [search, setSearch] = useState('')
  const [q, setQ] = useState('')
  const [message, setMessage] = useState<string | null>(null)
  const [localErrors, setLocalErrors] = useState<Record<string, string>>({})
  const [idempotencyKey, setIdempotencyKey] = useState(newKey)

  const results = useQuery({
    queryKey: [...catalogKeys.productList(tenantId, { q, status: 'active' }), 'adjust-search'],
    queryFn: () => catalogApi.products({ q, status: 'active' }, null),
    enabled: q.length > 0,
  })

  function addProduct(p: Product) {
    setMessage(null)
    if (!p.track_inventory) {
      setMessage(t('inventory.notStocked', { name: p.name }))
      return
    }
    if (lines.some((l) => l.variant_id === p.variant_id)) {
      setMessage(t('inventory.alreadyAdded', { name: p.name }))
      return
    }
    setLines((ls) => [
      ...ls,
      {
        variant_id: p.variant_id,
        name: p.name,
        sku: p.sku,
        unit: p.unit_short_name,
        allow_fraction: p.allow_fraction,
        direction: 'add',
        quantity: '',
        unit_cost: opening ? trimDecimal(p.purchase_price) : '',
      },
    ])
    setSearch('')
    setQ('')
  }

  const update = (i: number, patch: Partial<Line>) => setLines((ls) => ls.map((l, j) => (j === i ? { ...l, ...patch } : l)))
  const removeLine = (i: number) => setLines((ls) => ls.filter((_, j) => j !== i))

  const save = useMutation({
    mutationFn: () => {
      const input: AdjustmentInput = {
        reason: (opening ? 'opening' : reason) as AdjustmentReason,
        location_id: locationId || undefined,
        note: optional(note),
        lines: lines.map((l) => {
          const qty = l.quantity.trim()
          return {
            variant_id: l.variant_id,
            quantity: !opening && l.direction === 'remove' ? `-${qty}` : qty,
            unit_cost: showCost ? optional(l.unit_cost) : undefined,
          }
        }),
      }
      return inventoryApi.createAdjustment(input, idempotencyKey)
    },
    onSuccess: async (saved) => {
      setIdempotencyKey(newKey())
      queryClient.setQueryData(inventoryKeys.adjustment(tenantId, saved.id), saved)
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: inventoryKeys.all(tenantId) }),
        queryClient.invalidateQueries({ queryKey: ['reports', tenantId] }),
      ])
      navigate(`/inventory/adjustments/${saved.id}`, { replace: true })
    },
  })

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    const errs: Record<string, string> = {}
    if (!opening && !reason) errs.reason = t('inventory.errors.reason')
    if (lines.length === 0) errs.lines = t('inventory.errors.noLines')
    lines.forEach((l, i) => {
      const problem = quantityProblem(l)
      if (problem) errs[`lines[${i}].quantity`] = t(problem)
      if (showCost && l.unit_cost.trim() && (tryParse(l.unit_cost) ?? -1n) < 0n) errs[`lines[${i}].unit_cost`] = t('inventory.errors.cost')
    })
    setLocalErrors(errs)
    if (Object.keys(errs).length === 0) save.mutate()
  }

  const errors = { ...fieldErrors(save.error, t), ...localErrors }
  const totalValue = showCost ? sum(lines.map((l) => mul(dec(l.quantity), dec(l.unit_cost)))) : 0n
  const title = opening ? t('inventory.openingStock') : t('inventory.newAdjustment')
  const rows = results.data?.data ?? []

  return (
    <div className="flex max-w-5xl flex-col gap-6">
      <Link to="/inventory/adjustments" className="text-sm font-medium text-brand-700 hover:underline">
        ← {t('inventory.adjustments')}
      </Link>
      <PageHeader title={title} subtitle={opening ? t('inventory.openingHint') : t('inventory.adjustHint')} />
      {save.isError && hasFormError(save.error) && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}

      <Card className="flex flex-col gap-3">
        <h2 className="font-semibold">{t('inventory.addProducts')}</h2>
        <div
          role="search"
          className="flex flex-wrap items-end gap-2"
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              e.preventDefault()
              setQ(search.trim())
            }
          }}
        >
          <TextField
            className="min-w-60 flex-1"
            label={t('inventory.searchProducts')}
            type="search"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder={t('inventory.searchHint')}
          />
          <Button variant="secondary" onClick={() => setQ(search.trim())}>
            {t('common.search')}
          </Button>
        </div>
        {message && <Alert tone="warning">{message}</Alert>}
        {q && results.isPending && <Spinner />}
        {q && results.isError && <Alert tone="danger">{problemMessage(results.error, t)}</Alert>}
        {q && results.isSuccess && rows.length === 0 && <p className="text-sm text-ink-muted">{t('common.noResults')}</p>}
        {q && rows.length > 0 && (
          <ul className="flex flex-col divide-y divide-border rounded-lg border border-border" aria-label={t('inventory.searchResults')}>
            {rows.map((p) => (
              <li key={p.variant_id} className="flex flex-wrap items-center justify-between gap-2 px-3 py-2 text-sm">
                <div className="flex flex-col">
                  <span className="font-medium">{p.name}</span>
                  <span className="text-xs text-ink-muted">
                    {p.sku}
                    {!p.track_inventory && ` · ${t('inventory.notTracked')}`}
                  </span>
                </div>
                <Button variant="secondary" className="min-h-9" onClick={() => addProduct(p)} aria-label={t('inventory.addProduct', { name: p.name })}>
                  {t('common.add')}
                </Button>
              </li>
            ))}
          </ul>
        )}
      </Card>

      <form onSubmit={onSubmit} className="flex flex-col gap-6" noValidate>
        <Card className="flex flex-col gap-4">
          <div className="grid gap-4 sm:grid-cols-2">
            {!opening && (
              <Select
                label={t('common.reason')}
                value={reason}
                onChange={(e) => setReason(e.target.value)}
                error={errors.reason}
                options={[{ value: '', label: t('common.choosePeriod') }, ...ADJUSTMENT_REASONS.map((r) => ({ value: r, label: t(`inventory.reasons.${r}`) }))]}
              />
            )}
            {activeLocations.length > 1 && (
              <Select
                label={t('inventory.location')}
                value={locationId}
                onChange={(e) => setLocationId(e.target.value)}
                error={errors.location_id}
                options={[{ value: '', label: t('inventory.defaultLocation') }, ...activeLocations.map((l) => ({ value: l.id, label: l.name }))]}
              />
            )}
          </div>
          <TextField label={`${t('common.note')} (${t('common.optional')})`} value={note} onChange={(e) => setNote(e.target.value)} maxLength={2000} error={errors.note} />
        </Card>

        <Card className="flex flex-col gap-3 p-0 sm:p-0">
          <h2 className="px-6 pt-5 font-semibold">{t('inventory.lines')}</h2>
          {errors.lines && <p className="px-6 text-sm text-danger">{errors.lines}</p>}
          {lines.length === 0 ? (
            <p className="px-6 pb-5 text-sm text-ink-muted">{t('inventory.noLines')}</p>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <caption className="sr-only">{t('inventory.lines')}</caption>
                <thead className="border-y border-border bg-canvas text-xs text-ink-muted uppercase">
                  <tr>
                    <th scope="col" className="px-3 py-2 text-left">{t('inventory.product')}</th>
                    {!opening && <th scope="col" className="px-3 py-2 text-left">{t('inventory.direction')}</th>}
                    <th scope="col" className="px-3 py-2 text-left">{t('inventory.quantity')}</th>
                    {showCost && <th scope="col" className="px-3 py-2 text-left">{t('inventory.unitCost')}</th>}
                    {showCost && <th scope="col" className="px-3 py-2 text-right">{t('inventory.value')}</th>}
                    <th scope="col" className="px-3 py-2">
                      <span className="sr-only">{t('common.actions')}</span>
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {lines.map((l, i) => {
                    const qErr = errors[`lines[${i}].quantity`]
                    const cErr = errors[`lines[${i}].unit_cost`]
                    const vErr = errors[`lines[${i}].variant_id`]
                    return (
                      <tr key={l.variant_id} className="border-b border-border align-top last:border-b-0">
                        <td className="px-3 py-2">
                          <div className="font-medium">{l.name}</div>
                          <div className="text-xs text-ink-muted">{l.sku}</div>
                          {vErr && <p className="text-sm text-danger">{vErr}</p>}
                        </td>
                        {!opening && (
                          <td className="px-3 py-2">
                            <select
                              aria-label={t('inventory.directionFor', { name: l.name })}
                              className={INPUT}
                              value={l.direction}
                              onChange={(e) => update(i, { direction: e.target.value as Direction })}
                            >
                              <option value="add">{t('inventory.directionAdd')}</option>
                              <option value="remove">{t('inventory.directionRemove')}</option>
                            </select>
                          </td>
                        )}
                        <td className="px-3 py-2">
                          <div className="flex items-center gap-2">
                            <input
                              aria-label={t('inventory.quantityFor', { name: l.name })}
                              aria-invalid={qErr ? true : undefined}
                              className={`${INPUT} max-w-28 text-right`}
                              inputMode={l.allow_fraction ? 'decimal' : 'numeric'}
                              value={l.quantity}
                              onChange={(e) => update(i, { quantity: e.target.value })}
                            />
                            <span className="text-ink-muted">{l.unit}</span>
                          </div>
                          {qErr && <p className="text-sm text-danger">{qErr}</p>}
                        </td>
                        {showCost && (
                          <td className="px-3 py-2">
                            <input
                              aria-label={t('inventory.costFor', { name: l.name })}
                              aria-invalid={cErr ? true : undefined}
                              className={`${INPUT} max-w-32 text-right`}
                              inputMode="decimal"
                              value={l.unit_cost}
                              onChange={(e) => update(i, { unit_cost: e.target.value })}
                            />
                            {cErr && <p className="text-sm text-danger">{cErr}</p>}
                          </td>
                        )}
                        {showCost && (
                          <td className="px-3 py-2 text-right tabular-nums whitespace-nowrap">
                            {formatMoney(toFixed(mul(dec(l.quantity), dec(l.unit_cost))), lang)}
                          </td>
                        )}
                        <td className="px-3 py-2 text-right">
                          <Button variant="ghost" className="min-h-9" onClick={() => removeLine(i)} aria-label={t('inventory.removeLine', { name: l.name })}>
                            {t('common.remove')}
                          </Button>
                        </td>
                      </tr>
                    )
                  })}
                </tbody>
                {showCost && (
                  <tfoot className="border-t-2 border-border bg-canvas font-semibold">
                    <tr>
                      <td className="px-3 py-2" colSpan={3}>
                        {t('common.total')}
                      </td>
                      <td className="px-3 py-2 text-right tabular-nums">{formatMoney(toFixed(totalValue), lang)}</td>
                      <td />
                    </tr>
                  </tfoot>
                )}
              </table>
            </div>
          )}
        </Card>
        <div className="flex flex-wrap gap-2">
          <Button type="submit" disabled={save.isPending}>
            {save.isPending ? t('common.saving') : opening ? t('inventory.saveOpening') : t('inventory.saveAdjustment')}
          </Button>
          <Link to="/inventory/adjustments" className="inline-flex min-h-11 items-center px-4 text-sm font-semibold text-ink-muted hover:text-ink">
            {t('common.cancel')}
          </Link>
        </div>
      </form>
    </div>
  )
}
