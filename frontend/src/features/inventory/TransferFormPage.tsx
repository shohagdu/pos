import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate } from 'react-router'

import { type Product, catalogApi, catalogKeys } from '../../api/catalog'
import { type TransferInput, inventoryApi, inventoryKeys, useStockLocations } from '../../api/inventory'
import { newKey } from '../../api/query'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { PageHeader } from '../../components/ui/PageHeader'
import { Select } from '../../components/ui/Select'
import { Spinner } from '../../components/ui/Spinner'
import { TextField } from '../../components/ui/TextField'
import { tryParse } from '../../lib/decimal'
import { optional } from '../../lib/form'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { useBranch } from '../branches/useBranch'
import { useTenant } from '../money/utils'

interface Line {
  variant_id: string
  name: string
  sku: string
  unit: string
  allow_fraction: boolean
  quantity: string
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

/** `/inventory/transfers/new`: a draft or a dispatched transfer. */
export function TransferFormPage() {
  const { t } = useTranslation()
  const { can, hasFeature } = useTenant()
  if (!can('inventory.transfer') || !hasFeature('inventory.transfers')) {
    return (
      <div className="flex max-w-3xl flex-col gap-4">
        <h1 className="text-2xl font-bold">{t('inventory.newTransfer')}</h1>
        <Alert tone="warning">{t('inventory.noPermission')}</Alert>
      </div>
    )
  }
  return <TransferForm />
}

function TransferForm() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const { tenantId } = useTenant()
  const { branchId, choices } = useBranch()
  const locations = useStockLocations(tenantId)
  const active = (locations.data ?? []).filter((l) => l.status === 'active')
  // Without a picked branch, stock leaves the main branch (BRN-R02).
  const sourceBranch = branchId ?? choices.find((b) => b.is_default)?.id ?? choices[0]?.id
  const sourceLocations = active.filter((l) => l.branch_id === sourceBranch)

  const [fromId, setFromId] = useState('')
  const [toId, setToId] = useState('')
  const [note, setNote] = useState('')
  const [lines, setLines] = useState<Line[]>([])
  const [search, setSearch] = useState('')
  const [q, setQ] = useState('')
  const [message, setMessage] = useState<string | null>(null)
  const [localErrors, setLocalErrors] = useState<Record<string, string>>({})
  const [idempotencyKey, setIdempotencyKey] = useState(newKey)

  const destinations = active.filter((l) => l.id !== fromId)

  const results = useQuery({
    queryKey: [...catalogKeys.productList(tenantId, { q, status: 'active' }), 'transfer-search'],
    queryFn: () => catalogApi.products({ q, status: 'active' }, null),
    enabled: q.length > 0,
  })

  function addProduct(p: Product) {
    setMessage(null)
    if (!p.track_inventory) {
      setMessage(t('inventory.notStockedTransfer', { name: p.name }))
      return
    }
    if (lines.some((l) => l.variant_id === p.variant_id)) {
      setMessage(t('inventory.alreadyAdded', { name: p.name }))
      return
    }
    setLines((ls) => [...ls, { variant_id: p.variant_id, name: p.name, sku: p.sku, unit: p.unit_short_name, allow_fraction: p.allow_fraction, quantity: '' }])
    setSearch('')
    setQ('')
  }

  const update = (i: number, quantity: string) => setLines((ls) => ls.map((l, j) => (j === i ? { ...l, quantity } : l)))
  const removeLine = (i: number) => setLines((ls) => ls.filter((_, j) => j !== i))

  const save = useMutation({
    mutationFn: (dispatch: boolean) => {
      const input: TransferInput = {
        from_location_id: fromId || undefined,
        to_location_id: toId,
        note: optional(note),
        lines: lines.map((l) => ({ variant_id: l.variant_id, quantity: l.quantity.trim() })),
        dispatch: dispatch || undefined,
      }
      return inventoryApi.createTransfer(input, idempotencyKey)
    },
    onSuccess: async (saved) => {
      setIdempotencyKey(newKey())
      queryClient.setQueryData(inventoryKeys.transfer(tenantId, saved.id), saved)
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: inventoryKeys.all(tenantId) }),
        queryClient.invalidateQueries({ queryKey: ['reports', tenantId] }),
      ])
      navigate(`/inventory/transfers/${saved.id}`, { replace: true })
    },
  })

  function submit(dispatch: boolean) {
    const errs: Record<string, string> = {}
    if (!toId) errs.to_location_id = t('inventory.errors.destination')
    if (lines.length === 0) errs.lines = t('inventory.errors.noLines')
    lines.forEach((l, i) => {
      const problem = quantityProblem(l)
      if (problem) errs[`lines[${i}].quantity`] = t(problem)
    })
    setLocalErrors(errs)
    if (Object.keys(errs).length === 0) save.mutate(dispatch)
  }

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    submit(false)
  }

  const errors = { ...fieldErrors(save.error, t), ...localErrors }
  const rows = results.data?.data ?? []

  return (
    <div className="flex max-w-5xl flex-col gap-6">
      <Link to="/inventory/transfers" className="text-sm font-medium text-brand-700 hover:underline">
        ← {t('inventory.transfers')}
      </Link>
      <PageHeader title={t('inventory.newTransfer')} subtitle={t('inventory.transferHint')} />
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
            {sourceLocations.length > 1 ? (
              <Select
                label={t('inventory.fromLocation')}
                value={fromId}
                onChange={(e) => setFromId(e.target.value)}
                error={errors.from_location_id}
                options={[{ value: '', label: t('inventory.defaultFromLocation') }, ...sourceLocations.map((l) => ({ value: l.id, label: l.name }))]}
              />
            ) : (
              <div className="flex flex-col gap-1.5 text-sm">
                <span className="font-medium">{t('inventory.fromLocation')}</span>
                <span className="text-ink-muted">{sourceLocations[0] ? `${sourceLocations[0].branch_name} — ${sourceLocations[0].name}` : t('inventory.defaultFromLocation')}</span>
              </div>
            )}
            <Select
              label={t('inventory.toLocation')}
              value={toId}
              onChange={(e) => setToId(e.target.value)}
              error={errors.to_location_id}
              options={[{ value: '', label: t('inventory.chooseDestination') }, ...destinations.map((l) => ({ value: l.id, label: `${l.branch_name} — ${l.name}` }))]}
            />
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
                    <th scope="col" className="px-3 py-2 text-left">{t('inventory.quantity')}</th>
                    <th scope="col" className="px-3 py-2">
                      <span className="sr-only">{t('common.actions')}</span>
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {lines.map((l, i) => {
                    const qErr = errors[`lines[${i}].quantity`]
                    const vErr = errors[`lines[${i}].variant_id`]
                    return (
                      <tr key={l.variant_id} className="border-b border-border align-top last:border-b-0">
                        <td className="px-3 py-2">
                          <div className="font-medium">{l.name}</div>
                          <div className="text-xs text-ink-muted">{l.sku}</div>
                          {vErr && <p className="text-sm text-danger">{vErr}</p>}
                        </td>
                        <td className="px-3 py-2">
                          <div className="flex items-center gap-2">
                            <input
                              aria-label={t('inventory.quantityFor', { name: l.name })}
                              aria-invalid={qErr ? true : undefined}
                              className={`${INPUT} max-w-28 text-right`}
                              inputMode={l.allow_fraction ? 'decimal' : 'numeric'}
                              value={l.quantity}
                              onChange={(e) => update(i, e.target.value)}
                            />
                            <span className="text-ink-muted">{l.unit}</span>
                          </div>
                          {qErr && <p className="text-sm text-danger">{qErr}</p>}
                        </td>
                        <td className="px-3 py-2 text-right">
                          <Button variant="ghost" className="min-h-9" onClick={() => removeLine(i)} aria-label={t('inventory.removeLine', { name: l.name })}>
                            {t('common.remove')}
                          </Button>
                        </td>
                      </tr>
                    )
                  })}
                </tbody>
              </table>
            </div>
          )}
        </Card>
        <div className="flex flex-wrap gap-2">
          <Button type="submit" variant="secondary" disabled={save.isPending}>
            {save.isPending && save.variables === false ? t('common.saving') : t('inventory.saveDraft')}
          </Button>
          <Button disabled={save.isPending} onClick={() => submit(true)}>
            {save.isPending && save.variables === true ? t('common.saving') : t('inventory.saveDispatch')}
          </Button>
          <Link to="/inventory/transfers" className="inline-flex min-h-11 items-center px-4 text-sm font-semibold text-ink-muted hover:text-ink">
            {t('common.cancel')}
          </Link>
        </div>
      </form>
    </div>
  )
}
