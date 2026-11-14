import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type ChangeEvent, type FormEvent, type KeyboardEvent, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useLocation, useNavigate, useSearchParams } from 'react-router'

import { type Product, catalogApi, categoryTree, useCategories } from '../../api/catalog'
import {
  ADJUSTMENT_REASONS,
  type AdjustmentInput,
  type AdjustmentReason,
  inventoryApi,
  inventoryKeys,
  useBatchOptions,
  useStockLocations,
} from '../../api/inventory'
import { newKey } from '../../api/query'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { PageHeader } from '../../components/ui/PageHeader'
import { Select } from '../../components/ui/Select'
import { TextField } from '../../components/ui/TextField'
import { downloadCsvRows, readText } from '../../lib/csv'
import { dec, mul, sum, toFixed, tryParse } from '../../lib/decimal'
import { formatMoney } from '../../lib/format'
import { optional } from '../../lib/form'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { openingSampleHeader, readOpeningRows } from '../imports/openingStock'
import { categoryLabel } from '../products/labels'
import { PurchaseProductSearch } from '../purchases/PurchaseProductSearch'
import { trimDecimal, useTenant } from '../money/utils'
import { batchLabel } from './batchLabel'
import type { WriteOff } from './ExpiryPage'

type Direction = 'add' | 'remove'

interface Line {
  /** Unique per line: a product may have one line per batch. */
  key: string
  variant_id: string
  name: string
  sku: string
  unit: string
  allow_fraction: boolean
  direction: Direction
  quantity: string
  unit_cost: string
  /** Stock on hand when the line was loaded (bulk load only). */
  in_stock?: string
  /** The product keeps batches (or the user opened the batch fields). */
  track_batches: boolean
  /** Stock coming in: its batch number and expiry date (`YYYY-MM-DD`). */
  batch_no: string
  expiry_date: string
  /** Stock going out: the batch it leaves; `''` = earliest expiry first. */
  batch_id: string
}

let lineCounter = 0
const lineKey = (variantId: string) => `${variantId}:${Date.now().toString(36)}${++lineCounter}`

const NEW_LINE = { track_batches: false, batch_no: '', expiry_date: '', batch_id: '' }

/** Drafts saved before batches had no line keys. */
function withKeys(lines: Line[]): Line[] {
  return lines.map((l) => ({ ...NEW_LINE, ...l, key: l.key || lineKey(l.variant_id) }))
}

const sameBatch = (a: { batch_no: string; expiry_date: string }, b: { batch_no: string; expiry_date: string }) =>
  a.batch_no.trim().toLowerCase() === b.batch_no.trim().toLowerCase() && a.expiry_date === b.expiry_date
const hasBatch = (l: { batch_no: string; expiry_date: string }) => l.batch_no.trim() !== '' || l.expiry_date !== ''
const ISO_DATE = /^\d{4}-\d{2}-\d{2}$/

/** The server takes at most this many lines per adjustment. */
export const MAX_LINES = 500
const LOAD_PAGE = 200

interface Draft {
  reason: string
  locationId: string
  note: string
  lines: Line[]
}

const draftKey = (tenantId: string, opening: boolean) => `pos.adjustment-draft.${tenantId}.${opening ? 'opening' : 'adjust'}`

function loadDraft(key: string): Draft | null {
  try {
    const raw = localStorage.getItem(key)
    const draft = raw ? (JSON.parse(raw) as Draft) : null
    return draft && Array.isArray(draft.lines) && draft.lines.length > 0 ? draft : null
  } catch {
    return null
  }
}

function storeDraft(key: string, draft: Draft | null) {
  try {
    if (draft && draft.lines.length > 0) localStorage.setItem(key, JSON.stringify(draft))
    else localStorage.removeItem(key)
  } catch {
    // storage unavailable: the entry is not kept across visits
  }
}

const isBlank = (l: Line) => l.quantity.trim() === ''

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
  const categories = useCategories(tenantId)
  const activeLocations = (locations.data ?? []).filter((l) => l.status === 'active')
  const storageKey = draftKey(tenantId, opening)
  // Batches handed over by the expiry report to write off; they replace any draft.
  const routeState = useLocation().state as { writeOff?: WriteOff[] } | null
  const handed = opening ? undefined : routeState?.writeOff
  const [restored] = useState(() => (handed ? null : loadDraft(storageKey)))

  const [reason, setReason] = useState<string>(handed ? 'expired' : (restored?.reason ?? ''))
  const [locationId, setLocationId] = useState(restored?.locationId ?? '')
  const [note, setNote] = useState(restored?.note ?? '')
  const [lines, setLines] = useState<Line[]>(() =>
    handed
      ? handed.map((w) => ({
          ...NEW_LINE,
          key: lineKey(w.variant_id),
          variant_id: w.variant_id,
          name: w.name,
          sku: w.sku,
          unit: w.unit,
          allow_fraction: true,
          direction: 'remove' as const,
          quantity: trimDecimal(w.quantity),
          unit_cost: '',
          track_batches: true,
          batch_id: w.batch_id,
        }))
      : withKeys(restored?.lines ?? []),
  )
  const [showRestored, setShowRestored] = useState(restored !== null)
  const [message, setMessage] = useState<{ tone: 'info' | 'warning'; text: string } | null>(null)
  const [localErrors, setLocalErrors] = useState<Record<string, string>>({})
  const [idempotencyKey, setIdempotencyKey] = useState(newKey)
  const [loadCategory, setLoadCategory] = useState('')
  const [filter, setFilter] = useState('')
  const searchRef = useRef<HTMLInputElement>(null)
  const fileRef = useRef<HTMLInputElement>(null)
  const saved = useRef(false)

  // Keep the entry while the user works: a long opening stock survives a reload.
  useEffect(() => {
    if (!saved.current) storeDraft(storageKey, { reason, locationId, note, lines })
  }, [storageKey, reason, locationId, note, lines])

  /** Moves the cursor to a line's quantity once the new rows are on screen. */
  function focusLine(key: string) {
    setTimeout(() => {
      const input = document.getElementById(`qty-${key}`) as HTMLInputElement | null
      input?.focus()
      input?.select()
    }, 0)
  }

  function addProduct(p: Product) {
    setMessage(null)
    if (!p.track_inventory) {
      setMessage({ tone: 'warning', text: t('inventory.notStocked', { name: p.name }) })
      return
    }
    setFilter('')
    const existing = lines.find((l) => l.variant_id === p.variant_id)
    if (existing) {
      // Scanning the same item again counts one more.
      const q = tryParse(existing.quantity)
      if (q !== null && q > 0n) update(existing.key, { quantity: trimDecimal(toFixed(q + 10n ** 9n, 3)) })
      focusLine(existing.key)
      return
    }
    if (lines.length >= MAX_LINES) {
      setMessage({ tone: 'warning', text: t('inventory.bulk.full', { max: MAX_LINES }) })
      return
    }
    const key = lineKey(p.variant_id)
    setLines((ls) => [
      ...ls,
      {
        ...NEW_LINE,
        key,
        variant_id: p.variant_id,
        name: p.name,
        sku: p.sku,
        unit: p.unit_short_name,
        allow_fraction: p.allow_fraction,
        direction: 'add',
        quantity: '',
        unit_cost: opening && p.purchase_price ? trimDecimal(p.purchase_price) : '',
        track_batches: p.track_batches,
      },
    ])
    focusLine(key)
  }

  const update = (key: string, patch: Partial<Line>) => setLines((ls) => ls.map((l) => (l.key === key ? { ...l, ...patch } : l)))
  const removeLine = (key: string) => setLines((ls) => ls.filter((l) => l.key !== key))

  /** One more line of the same product, for stock of another batch. */
  function addBatchLine(after: Line) {
    if (lines.length >= MAX_LINES) {
      setMessage({ tone: 'warning', text: t('inventory.bulk.full', { max: MAX_LINES }) })
      return
    }
    const key = lineKey(after.variant_id)
    setLines((ls) => {
      const at = ls.findIndex((l) => l.key === after.key)
      const copy: Line = { ...after, key, quantity: '', batch_no: '', expiry_date: '', batch_id: '', in_stock: undefined, track_batches: true }
      return [...ls.slice(0, at + 1), copy, ...ls.slice(at + 1)]
    })
    focusLine(key)
  }

  /** Every stocked product with nothing on hand, so only quantities need typing. */
  const load = useMutation({
    mutationFn: async () => {
      const room = MAX_LINES - lines.length
      const have = new Set(lines.map((l) => l.variant_id))
      const found: Line[] = []
      let cursor: string | null = null
      let more = false
      do {
        const page = await inventoryApi.stock(
          { status: 'out', location_id: locationId || undefined, category_id: loadCategory || undefined },
          cursor,
          LOAD_PAGE,
        )
        for (const r of page.data) {
          if (have.has(r.variant_id)) continue
          if (found.length >= room) {
            more = true
            break
          }
          found.push({
            ...NEW_LINE,
            key: lineKey(r.variant_id),
            track_batches: r.track_batches,
            variant_id: r.variant_id,
            name: r.name,
            sku: r.sku,
            unit: r.unit_short_name,
            allow_fraction: r.allow_fraction,
            direction: 'add',
            quantity: '',
            unit_cost: r.purchase_price ? trimDecimal(r.purchase_price) : '',
            in_stock: r.quantity,
          })
        }
        cursor = page.next_cursor
      } while (cursor && !more)
      return { found, more: more || cursor !== null }
    },
    onSuccess: ({ found, more }) => {
      setLines((ls) => [...ls, ...found])
      setMessage({
        tone: more ? 'warning' : 'info',
        text:
          found.length === 0
            ? t('inventory.bulk.none')
            : `${t('inventory.bulk.loaded', { count: found.length })}${more ? ` ${t('inventory.bulk.more', { max: MAX_LINES })}` : ''}`,
      })
      if (found[0]) focusLine(found[0].key)
    },
  })

  /** Stocked products with nothing on hand, as rows of a CSV to fill in. */
  const sample = useMutation({
    mutationFn: async () => {
      const rows: string[][] = []
      let cursor: string | null = null
      do {
        const page = await inventoryApi.stock(
          { status: 'out', location_id: locationId || undefined, category_id: loadCategory || undefined },
          cursor,
          LOAD_PAGE,
        )
        for (const r of page.data) {
          rows.push([r.sku, r.name, r.unit_short_name, '', ...(showCost ? [r.purchase_price ? trimDecimal(r.purchase_price) : ''] : []), '', ''])
        }
        cursor = page.next_cursor
      } while (cursor && rows.length < 5000)
      return rows
    },
    onSuccess: (rows) => {
      downloadCsvRows('opening-stock-sample', [openingSampleHeader(showCost), ...rows])
      setMessage(rows.length === 0 ? { tone: 'info', text: t('imports.stock.noProducts') } : null)
    },
  })

  /** Lines from an uploaded opening stock file: added to (or updating) the list below. */
  const upload = useMutation({
    mutationFn: async (file: File) => {
      const rows = readOpeningRows(await readText(file))
      if (rows === null) return null
      const codes = [...new Set(rows.map((r) => r.code).filter(Boolean))]
      const found = new Map<string, Product>()
      for (let i = 0; i < codes.length; i += 1000) {
        for (const m of await catalogApi.lookupProducts(codes.slice(i, i + 1000))) found.set(m.code, m.product)
      }
      return { rows, found }
    },
    onSuccess: (result) => {
      if (result === null) {
        setMessage({ tone: 'warning', text: t('imports.stock.missingColumns') })
        return
      }
      const next = [...lines]
      const skipped: string[] = []
      let added = 0
      for (const row of result.rows) {
        const p = result.found.get(row.code)
        if (!p) {
          skipped.push(`${row.line}: ${t('imports.errors.unknownProduct')}${row.code ? ` (${row.code})` : ''}`)
          continue
        }
        if (!p.track_inventory) {
          skipped.push(`${row.line}: ${t('imports.errors.notStocked')} (${p.name})`)
          continue
        }
        if (row.badExpiry !== undefined) {
          skipped.push(`${row.line}: ${t('imports.errors.expiry')} (${row.badExpiry})`)
          continue
        }
        const cost = showCost ? row.unit_cost || (p.purchase_price ? trimDecimal(p.purchase_price) : '') : ''
        // The same product and batch updates its line; a product's untouched line takes the first batch.
        let i = next.findIndex((l) => l.variant_id === p.variant_id && sameBatch(l, row))
        if (i < 0 && hasBatch(row)) i = next.findIndex((l) => l.variant_id === p.variant_id && !hasBatch(l) && isBlank(l))
        const old = next[i]
        if (old) {
          next[i] = {
            ...old,
            quantity: row.quantity,
            unit_cost: row.unit_cost && showCost ? row.unit_cost : old.unit_cost,
            batch_no: row.batch_no,
            expiry_date: row.expiry_date,
            track_batches: old.track_batches || hasBatch(row),
          }
        } else if (next.length >= MAX_LINES) {
          skipped.push(`${row.line}: ${t('inventory.bulk.full', { max: MAX_LINES })}`)
          continue
        } else {
          next.push({
            key: lineKey(p.variant_id),
            batch_id: '',
            batch_no: row.batch_no,
            expiry_date: row.expiry_date,
            track_batches: p.track_batches || hasBatch(row),
            variant_id: p.variant_id,
            name: p.name,
            sku: p.sku,
            unit: p.unit_short_name,
            allow_fraction: p.allow_fraction,
            direction: 'add',
            quantity: row.quantity,
            unit_cost: cost,
          })
        }
        added++
      }
      setLines(next)
      setLocalErrors({})
      const shownSkips = skipped.length > 10 ? [...skipped.slice(0, 10), '…'] : skipped
      setMessage({
        tone: skipped.length > 0 ? 'warning' : 'info',
        text: [t('imports.stock.added', { count: added }), skipped.length > 0 ? t('imports.stock.skipped', { list: shownSkips.join('; ') }) : '']
          .filter(Boolean)
          .join(' '),
      })
    },
  })

  function onStockFile(e: ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0]
    e.target.value = ''
    if (file) upload.mutate(file)
  }

  // Lines without a quantity are left out, so a loaded list needs no tidying.
  const filled = lines.filter((l) => !isBlank(l))
  const indexOf = new Map(lines.map((l, i) => [l.key, i]))
  const incoming = (l: Line) => opening || l.direction === 'add'
  // Batches to pick from for stock going out of batch-tracked products.
  const outgoing = [...new Set(lines.filter((l) => !incoming(l) && l.track_batches).map((l) => l.variant_id))].sort()
  const batchOptions = useBatchOptions(tenantId, outgoing, locationId || undefined)
  const showBatch = lines.some((l) => l.track_batches || hasBatch(l) || l.batch_id)

  const save = useMutation({
    mutationFn: () => {
      const input: AdjustmentInput = {
        reason: (opening ? 'opening' : reason) as AdjustmentReason,
        location_id: locationId || undefined,
        note: optional(note),
        lines: filled.map((l) => {
          const qty = l.quantity.trim()
          return {
            variant_id: l.variant_id,
            quantity: !opening && l.direction === 'remove' ? `-${qty}` : qty,
            unit_cost: showCost ? optional(l.unit_cost) : undefined,
            ...(incoming(l) ? { batch_no: optional(l.batch_no), expiry_date: optional(l.expiry_date) } : { batch_id: optional(l.batch_id) }),
          }
        }),
      }
      return inventoryApi.createAdjustment(input, idempotencyKey)
    },
    onSuccess: async (created) => {
      saved.current = true
      storeDraft(storageKey, null)
      setIdempotencyKey(newKey())
      queryClient.setQueryData(inventoryKeys.adjustment(tenantId, created.id), created)
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: inventoryKeys.all(tenantId) }),
        queryClient.invalidateQueries({ queryKey: ['reports', tenantId] }),
      ])
      navigate(`/inventory/adjustments/${created.id}`, { replace: true })
    },
  })

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    const errs: Record<string, string> = {}
    if (!opening && !reason) errs.reason = t('inventory.errors.reason')
    if (filled.length === 0) errs.lines = lines.length === 0 ? t('inventory.errors.noLines') : t('inventory.errors.noQuantities')
    const seen = new Set<string>()
    for (const l of filled) {
      const problem = quantityProblem(l)
      if (problem) errs[`${l.key}.quantity`] = t(problem)
      if (showCost && l.unit_cost.trim() && (tryParse(l.unit_cost) ?? -1n) < 0n) errs[`${l.key}.unit_cost`] = t('inventory.errors.cost')
      if (incoming(l)) {
        if (l.batch_no.trim().length > 40) errs[`${l.key}.batch_no`] = t('inventory.errors.batchNo')
        if (l.expiry_date && !ISO_DATE.test(l.expiry_date)) errs[`${l.key}.expiry_date`] = t('inventory.errors.expiry')
      }
      const id = `${l.variant_id}|${incoming(l) ? `${l.batch_no.trim().toLowerCase()}|${l.expiry_date}` : l.batch_id}`
      if (seen.has(id)) errs[`${l.key}.variant_id`] = t('inventory.errors.sameBatch')
      seen.add(id)
    }
    setLocalErrors(errs)
    if (Object.keys(errs).length === 0) save.mutate()
  }

  // Server errors name submitted lines by position: map them back to products.
  const serverErrors: Record<string, string> = {}
  for (const [key, text] of Object.entries(fieldErrors(save.error, t))) {
    const m = /^lines\[(\d+)\]\.(\w+)$/.exec(key)
    const line = m ? filled[Number(m[1])] : undefined
    serverErrors[line && m ? `${line.key}.${m[2]}` : key] = text
  }
  const errors = { ...serverErrors, ...localErrors }
  const totalValue = showCost ? sum(filled.map((l) => mul(dec(l.quantity), dec(l.unit_cost)))) : 0n
  const title = opening ? t('inventory.openingStock') : t('inventory.newAdjustment')
  const needle = filter.trim().toLowerCase()
  const shown = needle ? lines.filter((l) => l.name.toLowerCase().includes(needle) || l.sku.toLowerCase().includes(needle)) : lines

  /** Enter moves down the list; from the last row back to the search box for the next scan. */
  function onQuantityKey(e: KeyboardEvent<HTMLInputElement>, key: string) {
    if (e.key !== 'Enter') return
    e.preventDefault()
    const at = shown.findIndex((l) => l.key === key)
    const next = shown[at + 1]
    if (next) focusLine(next.key)
    else searchRef.current?.focus()
  }

  function discardDraft() {
    setLines([])
    setNote('')
    setReason('')
    setShowRestored(false)
    setMessage(null)
    setLocalErrors({})
    searchRef.current?.focus()
  }

  return (
    <div className="flex max-w-5xl flex-col gap-6">
      <Link to="/inventory/adjustments" className="text-sm font-medium text-brand-700 hover:underline">
        ← {t('inventory.adjustments')}
      </Link>
      <PageHeader title={title} subtitle={opening ? t('inventory.openingHint') : t('inventory.adjustHint')} />
      {showRestored && (
        <Alert>
          <div className="flex flex-wrap items-center justify-between gap-3">
            <span>{t('inventory.draft.restored', { count: lines.length })}</span>
            <Button variant="secondary" className="min-h-9" onClick={discardDraft}>
              {t('inventory.draft.discard')}
            </Button>
          </div>
        </Alert>
      )}
      {save.isError && hasFormError(save.error) && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}

      <Card className="flex flex-col gap-4">
        <div className="flex flex-col gap-1.5">
          <h2 className="font-semibold">{t('inventory.addProducts')}</h2>
          <p className="text-sm text-ink-muted">{t('inventory.scanHint')}</p>
        </div>
        <PurchaseProductSearch tenantId={tenantId} inputRef={searchRef} onAdd={addProduct} />
        {opening && (
          <div className="flex flex-wrap items-end gap-2 border-t border-border pt-4">
            <Select
              className="min-w-56 flex-1"
              label={t('inventory.bulk.category')}
              value={loadCategory}
              onChange={(e) => setLoadCategory(e.target.value)}
              options={[
                { value: '', label: t('products.allCategories') },
                ...categoryTree(categories.data ?? []).map((c) => ({ value: c.id, label: categoryLabel(c) })),
              ]}
            />
            <Button variant="secondary" disabled={load.isPending || lines.length >= MAX_LINES} onClick={() => load.mutate()}>
              {load.isPending ? t('common.loading') : t('inventory.bulk.load')}
            </Button>
            <p className="w-full text-sm text-ink-muted">{t('inventory.bulk.hint')}</p>
            <div className="flex w-full flex-col gap-2 border-t border-border pt-4">
              <h3 className="font-semibold">{t('imports.stock.title')}</h3>
              <p className="text-sm text-ink-muted">{t('imports.stock.hint')}</p>
              <div className="flex flex-wrap gap-2">
                <Button variant="secondary" disabled={sample.isPending} onClick={() => sample.mutate()}>
                  {sample.isPending ? t('common.loading') : t('imports.downloadSample')}
                </Button>
                <Button variant="secondary" disabled={upload.isPending} onClick={() => fileRef.current?.click()}>
                  {upload.isPending ? t('imports.stock.loading') : t('imports.chooseFile')}
                </Button>
                <input ref={fileRef} type="file" accept=".csv,text/csv" className="hidden" aria-label={t('imports.chooseFile')} onChange={onStockFile} />
              </div>
            </div>
          </div>
        )}
        {message && <Alert tone={message.tone === 'warning' ? 'warning' : undefined}>{message.text}</Alert>}
        {load.isError && <Alert tone="danger">{problemMessage(load.error, t)}</Alert>}
        {sample.isError && <Alert tone="danger">{problemMessage(sample.error, t)}</Alert>}
        {upload.isError && <Alert tone="danger">{problemMessage(upload.error, t)}</Alert>}
      </Card>

      <form onSubmit={onSubmit} className="flex flex-col gap-6" noValidate>
        <Card className="flex flex-col gap-4">
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
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
          <TextField
            label={`${t('common.note')} (${t('common.optional')})`}
            value={note}
            onChange={(e) => setNote(e.target.value)}
            maxLength={2000}
            error={errors.note}
          />
        </Card>

        <Card className="flex flex-col gap-3 p-0 sm:p-0">
          <div className="flex flex-wrap items-center justify-between gap-3 px-6 pt-5">
            <h2 className="font-semibold">
              {t('inventory.lines')}
              {lines.length > 0 && (
                <span className="ml-2 text-sm font-normal text-ink-muted">{t('inventory.bulk.filled', { filled: filled.length, total: lines.length })}</span>
              )}
            </h2>
            {lines.length > 0 && (
              <div className="flex flex-wrap items-center gap-2">
                {lines.length > 8 && (
                  <input
                    type="search"
                    aria-label={t('inventory.bulk.filter')}
                    placeholder={t('inventory.bulk.filter')}
                    value={filter}
                    onChange={(e) => setFilter(e.target.value)}
                    className={`${INPUT} w-48`}
                  />
                )}
                {filled.length < lines.length && (
                  <Button variant="ghost" className="min-h-9" onClick={() => setLines((ls) => ls.filter((l) => !isBlank(l)))}>
                    {t('inventory.bulk.removeBlank')}
                  </Button>
                )}
              </div>
            )}
          </div>
          {errors.lines && <p className="px-6 text-sm text-danger">{errors.lines}</p>}
          {lines.length === 0 ? (
            <p className="px-6 pb-5 text-sm text-ink-muted">{opening ? t('inventory.noLinesOpening') : t('inventory.noLines')}</p>
          ) : (
            <div className="max-h-[65vh] overflow-auto">
              <table className="table-stack w-full text-sm">
                <caption className="sr-only">{t('inventory.lines')}</caption>
                <thead className="sticky top-0 z-10 border-y border-border bg-canvas text-xs text-ink-muted uppercase">
                  <tr>
                    <th scope="col" className="px-3 py-2 text-left">
                      {t('inventory.product')}
                    </th>
                    {!opening && (
                      <th scope="col" className="px-3 py-2 text-left">
                        {t('inventory.direction')}
                      </th>
                    )}
                    <th scope="col" className="px-3 py-2 text-left">
                      {t('inventory.quantity')}
                    </th>
                    {showBatch && (
                      <th scope="col" className="px-3 py-2 text-left">
                        {t('inventory.batch')}
                      </th>
                    )}
                    {showCost && (
                      <th scope="col" className="px-3 py-2 text-left">
                        {t('inventory.unitCost')}
                      </th>
                    )}
                    {showCost && (
                      <th scope="col" className="px-3 py-2 text-right">
                        {t('inventory.value')}
                      </th>
                    )}
                    <th scope="col" className="px-3 py-2">
                      <span className="sr-only">{t('common.actions')}</span>
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {shown.map((l) => {
                    const qErr = errors[`${l.key}.quantity`]
                    const cErr = errors[`${l.key}.unit_cost`]
                    const vErr = errors[`${l.key}.variant_id`]
                    const bErr = errors[`${l.key}.batch_no`] ?? errors[`${l.key}.batch_id`]
                    const eErr = errors[`${l.key}.expiry_date`]
                    const blank = isBlank(l)
                    const options = (batchOptions.data ?? []).filter((b) => b.variant_id === l.variant_id)
                    return (
                      <tr key={l.key} className={`border-b border-border align-top last:border-b-0 ${blank ? '' : 'bg-brand-50/40'}`}>
                        <td className="px-3 py-2">
                          <div className="font-medium">
                            <span className="mr-1 text-xs text-ink-muted tabular-nums">{(indexOf.get(l.key) ?? 0) + 1}.</span>
                            {l.name}
                          </div>
                          <div className="text-xs text-ink-muted">{l.sku}</div>
                          {vErr && <p className="text-sm text-danger">{vErr}</p>}
                        </td>
                        {!opening && (
                          <td data-label={t('inventory.direction')} className="px-3 py-2">
                            <select
                              aria-label={t('inventory.directionFor', { name: l.name })}
                              className={INPUT}
                              value={l.direction}
                              onChange={(e) => update(l.key, { direction: e.target.value as Direction })}
                            >
                              <option value="add">{t('inventory.directionAdd')}</option>
                              <option value="remove">{t('inventory.directionRemove')}</option>
                            </select>
                          </td>
                        )}
                        <td data-label={t('inventory.quantity')} className="px-3 py-2">
                          <div className="flex items-center gap-2">
                            <input
                              id={`qty-${l.key}`}
                              aria-label={t('inventory.quantityFor', { name: l.name })}
                              aria-invalid={qErr ? true : undefined}
                              className={`${INPUT} max-w-28 text-right`}
                              inputMode={l.allow_fraction ? 'decimal' : 'numeric'}
                              autoComplete="off"
                              value={l.quantity}
                              onChange={(e) => update(l.key, { quantity: e.target.value })}
                              onKeyDown={(e) => onQuantityKey(e, l.key)}
                            />
                            <span className="text-ink-muted">{l.unit}</span>
                          </div>
                          {qErr && <p className="text-sm text-danger">{qErr}</p>}
                        </td>
                        {showBatch && (
                          <td data-label={t('inventory.batch')} className="px-3 py-2">
                            {incoming(l) && l.track_batches ? (
                              <div className="flex flex-col gap-1">
                                <input
                                  aria-label={t('inventory.batchFor', { name: l.name })}
                                  aria-invalid={bErr ? true : undefined}
                                  placeholder={t('inventory.batchNo')}
                                  className={`${INPUT} max-w-36`}
                                  maxLength={40}
                                  autoComplete="off"
                                  value={l.batch_no}
                                  onChange={(e) => update(l.key, { batch_no: e.target.value })}
                                />
                                <input
                                  type="date"
                                  aria-label={t('inventory.expiryFor', { name: l.name })}
                                  aria-invalid={eErr ? true : undefined}
                                  className={`${INPUT} max-w-36`}
                                  value={l.expiry_date}
                                  onChange={(e) => update(l.key, { expiry_date: e.target.value })}
                                />
                                <button
                                  type="button"
                                  className="self-start text-xs font-semibold text-brand-700 hover:underline"
                                  onClick={() => addBatchLine(l)}
                                >
                                  {t('inventory.anotherBatch')}
                                </button>
                              </div>
                            ) : !incoming(l) && l.track_batches ? (
                              <select
                                aria-label={t('inventory.batchPick', { name: l.name })}
                                aria-invalid={bErr ? true : undefined}
                                className={`${INPUT} max-w-48`}
                                value={l.batch_id}
                                onChange={(e) => update(l.key, { batch_id: e.target.value })}
                              >
                                <option value="">{t('inventory.batchAuto')}</option>
                                {options.map((b) => (
                                  <option key={b.batch_id} value={b.batch_id}>
                                    {batchLabel(b, t, lang)} · {trimDecimal(b.quantity)} {l.unit}
                                  </option>
                                ))}
                              </select>
                            ) : incoming(l) ? (
                              <button
                                type="button"
                                className="text-xs font-semibold text-brand-700 hover:underline"
                                onClick={() => update(l.key, { track_batches: true })}
                              >
                                {t('inventory.addBatch')}
                              </button>
                            ) : (
                              <span className="text-ink-muted">—</span>
                            )}
                            {bErr && <p className="text-sm text-danger">{bErr}</p>}
                            {eErr && <p className="text-sm text-danger">{eErr}</p>}
                          </td>
                        )}
                        {showCost && (
                          <td data-label={t('inventory.unitCost')} className="px-3 py-2">
                            <input
                              aria-label={t('inventory.costFor', { name: l.name })}
                              aria-invalid={cErr ? true : undefined}
                              className={`${INPUT} max-w-32 text-right`}
                              inputMode="decimal"
                              autoComplete="off"
                              value={l.unit_cost}
                              onChange={(e) => update(l.key, { unit_cost: e.target.value })}
                              onKeyDown={(e) => onQuantityKey(e, l.key)}
                            />
                            {cErr && <p className="text-sm text-danger">{cErr}</p>}
                          </td>
                        )}
                        {showCost && (
                          <td data-label={t('inventory.value')} className="px-3 py-2 text-right tabular-nums whitespace-nowrap">
                            {blank ? '—' : formatMoney(toFixed(mul(dec(l.quantity), dec(l.unit_cost))), lang)}
                          </td>
                        )}
                        <td data-label={''} className="px-3 py-2 text-right">
                          <Button
                            variant="ghost"
                            className="min-h-9"
                            onClick={() => removeLine(l.key)}
                            aria-label={t('inventory.removeLine', { name: l.name })}
                          >
                            {t('common.remove')}
                          </Button>
                        </td>
                      </tr>
                    )
                  })}
                </tbody>
                {showCost && (
                  <tfoot className="sticky bottom-0 border-t-2 border-border bg-canvas font-semibold">
                    <tr>
                      <td className="px-3 py-2" colSpan={showBatch ? 4 : 3}>
                        {t('common.total')}
                      </td>
                      <td data-label={t('inventory.value')} className="px-3 py-2 text-right tabular-nums">
                        {formatMoney(toFixed(totalValue), lang)}
                      </td>
                      <td />
                    </tr>
                  </tfoot>
                )}
              </table>
            </div>
          )}
        </Card>
        <div className="flex flex-wrap items-center gap-2">
          <Button type="submit" disabled={save.isPending}>
            {save.isPending ? t('common.saving') : opening ? t('inventory.saveOpeningCount', { count: filled.length }) : t('inventory.saveAdjustment')}
          </Button>
          <Link to="/inventory/adjustments" className="inline-flex min-h-11 items-center px-4 text-sm font-semibold text-ink-muted hover:text-ink">
            {t('common.cancel')}
          </Link>
          {lines.length > 0 && <span className="text-sm text-ink-muted">{t('inventory.draft.autosaved')}</span>}
        </div>
      </form>
    </div>
  )
}
