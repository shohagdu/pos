import { useMutation, useQueries, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, type KeyboardEvent, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate } from 'react-router'

import { type Product } from '../../api/catalog'
import { type TransferInput, inventoryApi, inventoryKeys, useStockLocations } from '../../api/inventory'
import { newKey } from '../../api/query'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { PageHeader } from '../../components/ui/PageHeader'
import { dec, toPlain, tryParse } from '../../lib/decimal'
import { formatQuantity } from '../../lib/format'
import { optional } from '../../lib/form'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { useBranch } from '../branches/useBranch'
import { useTenant } from '../money/utils'
import { ProductThumb } from '../products/ProductThumb'
import { PurchaseProductSearch } from '../purchases/PurchaseProductSearch'

interface Line {
  variant_id: string
  name: string
  sku: string
  unit: string
  allow_fraction: boolean
  image_file_id: string | null
  quantity: string
}

const FIELD =
  'min-h-10 rounded-lg border border-border bg-surface px-2 text-sm outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100 aria-invalid:border-danger'
const INPUT = `${FIELD} w-full`

const SHORTCUTS = [
  ['F2', 'search'],
  ['F4', 'destination'],
  ['F8', 'draft'],
  ['F9', 'dispatch'],
  ['Enter', 'next'],
  ['Ctrl+Del', 'removeLine'],
] as const

const qtyId = (variantId: string) => `transfer-${variantId}-qty`
const lastDestinationKey = (tenantId: string) => `pos.transfer.to.${tenantId}`

function focusField(id: string) {
  const el = document.getElementById(id) as HTMLInputElement | null
  el?.focus()
  el?.select()
}

function readLastDestination(tenantId: string): string {
  try {
    return localStorage.getItem(lastDestinationKey(tenantId)) ?? ''
  } catch {
    return ''
  }
}

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

/**
 * Works like the purchase bill: scan or search (F2) adds a line with
 * quantity 1 and puts the cursor on it (scanning again adds one more), Enter
 * goes back to the search; each line shows the stock available at the
 * source. The route, note and Save sit on the right; the last destination is
 * remembered and a single possible destination is picked automatically.
 */
function TransferForm() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const { tenantId } = useTenant()
  const { branchId, choices } = useBranch()
  const locations = useStockLocations(tenantId)
  const active = (locations.data ?? []).filter((l) => l.status === 'active')
  // Without a picked branch, stock leaves the main branch (BRN-R02).
  const sourceBranch = branchId ?? choices.find((b) => b.is_default)?.id ?? choices[0]?.id
  const sourceLocations = active.filter((l) => l.branch_id === sourceBranch)
  const pickSource = sourceLocations.length > 1

  const [fromChoice, setFromChoice] = useState('')
  const [toChoice, setToChoice] = useState(() => readLastDestination(tenantId))
  const [note, setNote] = useState('')
  const [lines, setLines] = useState<Line[]>([])
  const [localErrors, setLocalErrors] = useState<Record<string, string>>({})
  const [idempotencyKey, setIdempotencyKey] = useState(newKey)
  const searchRef = useRef<HTMLInputElement>(null)
  const toRef = useRef<HTMLSelectElement>(null)
  const pendingFocus = useRef<string | null>(null)

  // With several locations here the shop floor is suggested; one location needs no choice.
  const fromId = pickSource
    ? sourceLocations.some((l) => l.id === fromChoice)
      ? fromChoice
      : ((sourceLocations.find((l) => l.is_sellable) ?? sourceLocations[0])?.id ?? '')
    : (sourceLocations[0]?.id ?? '')
  const destinations = active.filter((l) => l.id !== fromId)
  const toId = destinations.some((l) => l.id === toChoice) ? toChoice : destinations.length === 1 ? (destinations[0]?.id ?? '') : ''

  // What the source holds of each line, to send no more than is there.
  const stock = useQueries({
    queries: lines.map((l) => {
      const filter = { location_id: fromId, variant_id: l.variant_id }
      return {
        queryKey: inventoryKeys.stock(tenantId, filter),
        queryFn: () => inventoryApi.stock(filter, null, 1),
        enabled: Boolean(fromId),
        staleTime: 30_000,
      }
    }),
  })
  const available = (i: number) => stock[i]?.data?.data[0]?.quantity ?? null

  useEffect(() => {
    if (pendingFocus.current) {
      focusField(pendingFocus.current)
      pendingFocus.current = null
    }
  })

  function addProduct(p: Product) {
    setLocalErrors((e) => ({ ...e, lines: '' }))
    const existing = lines.find((l) => l.variant_id === p.variant_id)
    if (existing) {
      // Scanning the same item again counts one more.
      setLines((ls) => ls.map((l) => (l.variant_id === p.variant_id ? { ...l, quantity: toPlain(dec(l.quantity || '0') + dec('1')) } : l)))
    } else {
      setLines((ls) => [
        ...ls,
        {
          variant_id: p.variant_id,
          name: p.name,
          sku: p.sku,
          unit: p.unit_short_name,
          allow_fraction: p.allow_fraction,
          image_file_id: p.image_file_id,
          quantity: '1',
        },
      ])
    }
    pendingFocus.current = qtyId(p.variant_id)
  }

  const update = (i: number, quantity: string) => setLines((ls) => ls.map((l, j) => (j === i ? { ...l, quantity } : l)))
  const removeLine = (i: number) => {
    setLines((ls) => ls.filter((_, j) => j !== i))
    searchRef.current?.focus()
  }

  const save = useMutation({
    mutationFn: (dispatch: boolean) => {
      const input: TransferInput = {
        from_location_id: pickSource ? fromId : undefined,
        to_location_id: toId,
        note: optional(note),
        lines: lines.map((l) => ({ variant_id: l.variant_id, quantity: l.quantity.trim() })),
        dispatch: dispatch || undefined,
      }
      return inventoryApi.createTransfer(input, idempotencyKey)
    },
    onSuccess: async (saved) => {
      setIdempotencyKey(newKey())
      try {
        localStorage.setItem(lastDestinationKey(tenantId), toId)
      } catch {
        // storage unavailable: the destination is just not remembered
      }
      queryClient.setQueryData(inventoryKeys.transfer(tenantId, saved.id), saved)
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: inventoryKeys.all(tenantId) }),
        queryClient.invalidateQueries({ queryKey: ['reports', tenantId] }),
      ])
      navigate(`/inventory/transfers/${saved.id}`, { replace: true })
    },
  })

  function submit(dispatch: boolean) {
    if (save.isPending) return
    const errs: Record<string, string> = {}
    if (!toId) errs.to_location_id = t('inventory.errors.destination')
    if (lines.length === 0) errs.lines = t('inventory.errors.noLines')
    lines.forEach((l, i) => {
      const problem = quantityProblem(l)
      if (problem) errs[`lines[${i}].quantity`] = t(problem)
    })
    setLocalErrors(errs)
    if (Object.keys(errs).length > 0) {
      if (errs.to_location_id) toRef.current?.focus()
      else if (errs.lines) searchRef.current?.focus()
      else {
        const bad = lines.findIndex((_, i) => errs[`lines[${i}].quantity`])
        if (bad >= 0 && lines[bad]) focusField(qtyId(lines[bad].variant_id))
      }
      return
    }
    save.mutate(dispatch)
  }

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    submit(false)
  }

  // Same keys as the purchase bill.
  useEffect(() => {
    function onKey(e: globalThis.KeyboardEvent) {
      if (e.key === 'F2') {
        e.preventDefault()
        searchRef.current?.focus()
      } else if (e.key === 'F4') {
        e.preventDefault()
        toRef.current?.focus()
      } else if (e.key === 'F8' && !e.repeat) {
        e.preventDefault()
        submit(false)
      } else if (e.key === 'F9' && !e.repeat) {
        e.preventDefault()
        submit(true)
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  })

  function onQtyKey(e: KeyboardEvent<HTMLInputElement>, i: number) {
    if (e.key === 'Enter') {
      e.preventDefault()
      searchRef.current?.focus()
    } else if (e.key === 'Delete' && e.ctrlKey) {
      e.preventDefault()
      removeLine(i)
    }
  }

  const errors = { ...fieldErrors(save.error, t), ...localErrors }
  const totalQty = lines.reduce((sum, l) => sum + (tryParse(l.quantity) ?? 0n), 0n)
  const from = sourceLocations.find((l) => l.id === fromId)

  return (
    <div className="flex flex-col gap-4">
      <Link to="/inventory/transfers" className="text-sm font-medium text-brand-700 hover:underline">
        ← {t('inventory.transfers')}
      </Link>
      <PageHeader title={t('inventory.newTransfer')} subtitle={t('inventory.transferHint')} />
      {save.isError && hasFormError(save.error) && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}

      <form onSubmit={onSubmit} noValidate className="grid grid-cols-1 gap-4 lg:grid-cols-[minmax(0,1fr)_22rem] lg:items-start">
        <section className="flex min-w-0 flex-col gap-3" aria-label={t('inventory.lines')}>
          <div className="flex items-center gap-2">
            <div className="min-w-0 flex-1">
              <PurchaseProductSearch tenantId={tenantId} inputRef={searchRef} onAdd={addProduct} showCost={false} />
            </div>
            <kbd className="hidden text-xs text-ink-muted sm:inline">F2</kbd>
          </div>
          {errors.lines && <p className="text-sm text-danger">{errors.lines}</p>}

          <Card className="flex flex-col overflow-hidden p-0">
            <div className="flex items-center justify-between gap-2 border-b border-border px-4 py-3">
              <h2 className="flex items-center gap-2 font-semibold">
                {t('inventory.lines')}
                {lines.length > 0 && (
                  <span className="rounded-full bg-brand-50 px-2 py-0.5 text-xs font-semibold text-brand-700 tabular-nums">
                    {t('inventory.itemCount', { count: lines.length })}
                  </span>
                )}
              </h2>
              {lines.length > 0 && (
                <span className="text-sm text-ink-muted">
                  {t('inventory.totalQuantity')}: <span className="font-semibold text-ink tabular-nums">{formatQuantity(toPlain(totalQty), lang, true)}</span>
                </span>
              )}
            </div>
            {lines.length === 0 ? (
              <div className="flex flex-col items-center gap-2 px-4 py-12 text-center text-sm text-ink-muted">
                <svg viewBox="0 0 24 24" className="size-10 text-ink-muted/60" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true">
                  <path d="M3 7h11v9H3zM14 10h4l3 3v3h-7z" />
                  <circle cx="7" cy="17.5" r="1.5" />
                  <circle cx="17" cy="17.5" r="1.5" />
                </svg>
                <p>{t('inventory.noLinesTransfer')}</p>
              </div>
            ) : (
              <ol className="flex flex-col text-sm" aria-label={t('inventory.lines')}>
                {lines.map((l, i) => {
                  const qErr = errors[`lines[${i}].quantity`]
                  const vErr = errors[`lines[${i}].variant_id`]
                  const have = available(i)
                  const want = tryParse(l.quantity)
                  const over = have !== null && want !== null && want > dec(have)
                  return (
                    <li
                      key={l.variant_id}
                      className="flex items-center gap-3 border-t border-border px-4 py-3 transition-colors first:border-t-0 hover:bg-canvas/60 focus-within:bg-brand-50/40"
                    >
                      <span className="inline-flex size-6 shrink-0 items-center justify-center rounded-full bg-canvas text-xs font-semibold text-ink-muted tabular-nums">
                        {i + 1}
                      </span>
                      <span className="hidden shrink-0 sm:block">
                        <ProductThumb fileId={l.image_file_id} size="md" />
                      </span>
                      <div className="min-w-0 flex-1">
                        <span className="block truncate font-medium" title={l.name}>
                          {l.name}
                        </span>
                        <span className="block truncate text-xs text-ink-muted">
                          <span className="font-mono">{l.sku}</span>
                          {have !== null && (
                            <>
                              {' · '}
                              <span className={over ? 'font-semibold text-danger' : ''}>
                                {t('inventory.available', { qty: formatQuantity(have, lang, true), unit: l.unit })}
                              </span>
                            </>
                          )}
                        </span>
                        {vErr && <p className="text-xs text-danger">{vErr}</p>}
                      </div>
                      <div className="flex shrink-0 flex-col items-end gap-1">
                        <div className="flex items-center gap-2">
                          {have !== null && dec(have) > 0n && want !== dec(have) && (
                            <button
                              type="button"
                              className="shrink-0 rounded-md px-2 py-1 text-xs font-semibold whitespace-nowrap text-brand-700 hover:bg-brand-50"
                              onClick={() => {
                                update(i, toPlain(dec(have)))
                                pendingFocus.current = qtyId(l.variant_id)
                              }}
                            >
                              {t('inventory.sendAll')}
                            </button>
                          )}
                          <input
                            id={qtyId(l.variant_id)}
                            aria-label={t('inventory.quantityFor', { name: l.name })}
                            aria-invalid={qErr || over ? true : undefined}
                            className={`${FIELD} w-20 shrink-0 text-right tabular-nums`}
                            inputMode={l.allow_fraction ? 'decimal' : 'numeric'}
                            autoComplete="off"
                            value={l.quantity}
                            onFocus={(e) => e.target.select()}
                            onChange={(e) => update(i, e.target.value)}
                            onKeyDown={(e) => onQtyKey(e, i)}
                          />
                          <span className="w-8 shrink-0 truncate text-xs text-ink-muted">{l.unit}</span>
                          <Button
                            variant="ghost"
                            className="min-h-9 shrink-0 px-2 text-ink-muted hover:text-danger"
                            onClick={() => removeLine(i)}
                            aria-label={t('inventory.removeLine', { name: l.name })}
                          >
                            ✕
                          </Button>
                        </div>
                        {qErr ? (
                          <p className="text-xs whitespace-nowrap text-danger">{qErr}</p>
                        ) : (
                          over && <p className="text-xs whitespace-nowrap text-danger">{t('inventory.overAvailable')}</p>
                        )}
                      </div>
                    </li>
                  )
                })}
              </ol>
            )}
          </Card>
        </section>

        <aside className="flex flex-col gap-4 rounded-xl border border-border bg-surface p-4 lg:sticky lg:top-4">
          <section className="flex flex-col gap-3" aria-label={t('inventory.route')}>
            <h2 className="text-xs font-medium text-ink-muted uppercase">{t('inventory.route')}</h2>
            {pickSource ? (
              <label className="flex flex-col gap-1.5 text-sm font-medium">
                {t('inventory.fromLocation')}
                <select className={INPUT} value={fromId} onChange={(e) => setFromChoice(e.target.value)}>
                  {sourceLocations.map((l) => (
                    <option key={l.id} value={l.id}>
                      {l.name}
                    </option>
                  ))}
                </select>
                {errors.from_location_id && <span className="text-sm font-normal text-danger">{errors.from_location_id}</span>}
              </label>
            ) : (
              <div className="flex flex-col gap-0.5 rounded-lg bg-canvas px-3 py-2 text-sm">
                <span className="text-xs text-ink-muted">{t('inventory.fromLocation')}</span>
                <span className="font-medium">{from ? `${from.branch_name} — ${from.name}` : t('inventory.defaultFromLocation')}</span>
              </div>
            )}
            <div className="flex justify-center text-ink-muted" aria-hidden="true">
              <svg viewBox="0 0 24 24" className="size-5" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M12 5v14M6 13l6 6 6-6" />
              </svg>
            </div>
            <label className="flex flex-col gap-1.5 text-sm font-medium">
              <span className="flex items-center justify-between">
                {t('inventory.toLocation')}
                <kbd className="hidden text-xs font-normal text-ink-muted sm:inline">F4</kbd>
              </span>
              <select
                ref={toRef}
                className={INPUT}
                value={toId}
                aria-invalid={errors.to_location_id ? true : undefined}
                onChange={(e) => {
                  setToChoice(e.target.value)
                  setLocalErrors((x) => ({ ...x, to_location_id: '' }))
                }}
              >
                {!toId && <option value="">{t('inventory.chooseDestination')}</option>}
                {destinations.map((l) => (
                  <option key={l.id} value={l.id}>
                    {`${l.branch_name} — ${l.name}`}
                  </option>
                ))}
              </select>
              {errors.to_location_id && <span className="text-sm font-normal text-danger">{errors.to_location_id}</span>}
            </label>
          </section>

          <label className="flex flex-col gap-1.5 text-sm font-medium">
            {`${t('common.note')} (${t('common.optional')})`}
            <textarea
              value={note}
              onChange={(e) => setNote(e.target.value)}
              rows={2}
              maxLength={2000}
              className="rounded-lg border border-border bg-surface px-3 py-2 text-sm font-normal outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100"
            />
            {errors.note && <span className="text-sm font-normal text-danger">{errors.note}</span>}
          </label>

          <dl className="flex flex-col gap-1 border-t border-border pt-3 text-sm">
            <div className="flex justify-between">
              <dt className="text-ink-muted">{t('inventory.lines')}</dt>
              <dd className="tabular-nums">{lines.length}</dd>
            </div>
            <div className="flex justify-between text-base font-semibold">
              <dt>{t('inventory.totalQuantity')}</dt>
              <dd className="tabular-nums">{formatQuantity(toPlain(totalQty), lang, true)}</dd>
            </div>
          </dl>

          <div className="flex flex-col gap-2">
            <Button disabled={save.isPending} onClick={() => submit(true)} className="w-full">
              {save.isPending && save.variables === true ? t('common.saving') : t('inventory.saveDispatch')}
              <kbd className="ml-2 text-xs font-normal opacity-70">F9</kbd>
            </Button>
            <Button type="submit" variant="secondary" disabled={save.isPending} className="w-full">
              {save.isPending && save.variables === false ? t('common.saving') : t('inventory.saveDraft')}
              <kbd className="ml-2 text-xs font-normal opacity-70">F8</kbd>
            </Button>
            <Link to="/inventory/transfers" className="inline-flex min-h-10 items-center justify-center text-sm font-semibold text-ink-muted hover:text-ink">
              {t('common.cancel')}
            </Link>
          </div>

          <dl className="grid grid-cols-2 gap-x-3 gap-y-1 border-t border-border pt-3 text-xs text-ink-muted">
            {SHORTCUTS.map(([key, name]) => (
              <div key={key} className="flex gap-1.5">
                <dt>
                  <kbd className="rounded border border-border bg-canvas px-1 font-mono">{key}</kbd>
                </dt>
                <dd>{t(`inventory.shortcuts.${name}`)}</dd>
              </div>
            ))}
          </dl>
        </aside>
      </form>
    </div>
  )
}
