import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type KeyboardEvent, useEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate, useParams } from 'react-router'

import type { Product } from '../../api/catalog'
import { ApiError } from '../../api/client'
import { useStockLocations } from '../../api/inventory'
import { type LastPrice, type PurchaseDetail, type PurchaseInput, purchasingApi, purchasingKeys, useLastPrices, usePurchase } from '../../api/purchasing'
import { useSettings } from '../../api/tenancy'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { PageHeader } from '../../components/ui/PageHeader'
import { Select } from '../../components/ui/Select'
import { Spinner } from '../../components/ui/Spinner'
import { TextField } from '../../components/ui/TextField'
import { dec, toPlain, tryParse } from '../../lib/decimal'
import { formatDate, today } from '../../lib/dates'
import { currencySymbol, formatMoney, formatQuantity } from '../../lib/format'
import { optional } from '../../lib/form'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { type CostLineInput, costBill, money, variance } from './costing'
import { LINK, trimDecimal } from './helpers'
import { PurchaseProductSearch } from './PurchaseProductSearch'
import { PostDialog } from './shared'
import { type BillSupplier, SupplierPanel } from './SupplierPanel'

/** `/purchases/new` and `/purchases/:id/edit` (drafts only). */
export function PurchaseFormPage() {
  const { t } = useTranslation()
  const { id } = useParams()
  const { me } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const purchase = usePurchase(tenantId, id ?? '', Boolean(id))
  const locations = useStockLocations(tenantId)

  if (locations.isPending || (id && purchase.isPending)) return <Spinner />
  if (id && purchase.isError) return <Alert tone="danger">{problemMessage(purchase.error, t)}</Alert>
  if (purchase.data && purchase.data.status !== 'draft') {
    return (
      <div className="flex max-w-3xl flex-col gap-4">
        <Alert tone="warning">{t('purchases.form.notDraft')}</Alert>
        <Link to={`/purchases/${purchase.data.id}`} className={LINK}>
          ← {purchase.data.purchase_no ?? t('purchases.draft')}
        </Link>
      </div>
    )
  }
  return <PurchaseForm key={id ?? 'new'} tenantId={tenantId} purchase={id ? purchase.data : undefined} />
}

interface Line extends CostLineInput {
  key: string
  variant_id: string
  product_name: string
  sku: string
  unit_short_name: string
  allow_fraction: boolean
}

let lineSeq = 0
const nextKey = () => `l${++lineSeq}`
const fieldId = (key: string, field: 'qty' | 'cost') => `purchase-${key}-${field}`

function fromItems(purchase: PurchaseDetail): Line[] {
  return purchase.items.map((i) => ({
    key: nextKey(),
    variant_id: i.variant_id,
    product_name: i.product_name,
    sku: i.sku,
    unit_short_name: i.unit_short_name,
    allow_fraction: true,
    quantity: trimDecimal(i.quantity),
    unit_cost: trimDecimal(i.unit_cost),
    discount_type: i.discount_type ?? 'amount',
    discount_value: trimDecimal(i.discount_value),
    tax_rate_pct: trimDecimal(i.tax_rate_pct) || '0',
  }))
}

function focusField(id: string) {
  const el = document.getElementById(id) as HTMLInputElement | null
  el?.focus()
  el?.select()
}

const INPUT =
  'min-h-10 w-full rounded-lg border border-border bg-surface px-2 text-sm outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100 aria-invalid:border-danger'

const SHORTCUTS = [
  ['F2', 'search'],
  ['F4', 'supplier'],
  ['F8', 'save'],
  ['F9', 'post'],
  ['Enter', 'next'],
  ['Ctrl+Del', 'removeLine'],
] as const

/**
 * The bill works like the POS cart: scan or search on the left (F2), each
 * added line takes the cursor to its quantity, Enter moves quantity → cost →
 * back to the search; supplier (F4), bill details, charges and totals sit on
 * the right. F8 saves the draft; F9 saves and posts it (with an optional
 * payment), like confirming a POS sale.
 */
function PurchaseForm({ tenantId, purchase }: { tenantId: string; purchase?: PurchaseDetail }) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const { can } = useSession()
  const locations = (useStockLocations(tenantId).data ?? []).filter((l) => l.status === 'active' || l.id === purchase?.location_id)
  const vatRegistered = useSettings(tenantId).data?.is_vat_registered ?? false
  const searchRef = useRef<HTMLInputElement>(null)
  const supplierRef = useRef<HTMLInputElement>(null)

  const [supplier, setSupplier] = useState<BillSupplier | null>(
    purchase ? { id: purchase.supplier_id, name: purchase.supplier_name, phone: null, balance: null } : null,
  )
  const [head, setHead] = useState({
    supplier_invoice_no: purchase?.supplier_invoice_no ?? '',
    purchase_date: purchase?.purchase_date ?? today(),
    expected_date: purchase?.expected_date ?? '',
    location_id: purchase?.location_id ?? locations.find((l) => l.is_sellable)?.id ?? '',
    bill_discount: trimDecimal(purchase?.bill_discount_amount),
    shipping_cost: trimDecimal(purchase?.shipping_cost),
    other_cost: trimDecimal(purchase?.other_cost),
    note: purchase?.note ?? '',
  })
  const setField = (name: keyof typeof head) => (e: { target: { value: string } }) => setHead((h) => ({ ...h, [name]: e.target.value }))
  const [lines, setLines] = useState<Line[]>(() => (purchase ? fromItems(purchase) : []))
  const [clientErrors, setClientErrors] = useState<Record<string, string>>({})
  // Once saved here (e.g. Save & post whose post then failed), later saves
  // update that draft instead of creating another.
  const [saved, setSaved] = useState<PurchaseDetail | undefined>(undefined)
  const current = saved ?? purchase
  const [posting, setPosting] = useState(false)
  const postDraft = useRef<PurchaseDetail | null>(null)
  // The input to focus once the next render has put it on screen (a just-added line's quantity).
  const pendingFocus = useRef<string | null>(null)
  useEffect(() => {
    if (pendingFocus.current) focusField(pendingFocus.current)
    pendingFocus.current = null
  })

  const variantIds = useMemo(() => [...new Set(lines.map((l) => l.variant_id))].sort(), [lines])
  const lastPrices = useLastPrices(tenantId, variantIds)
  const lastByVariant = new Map<string, LastPrice>((lastPrices.data ?? []).map((p) => [p.variant_id, p]))

  const bill = costBill(lines, head.bill_discount, head.shipping_cost, head.other_cost, vatRegistered)

  function addProduct(p: Product) {
    const existing = lines.find((l) => l.variant_id === p.variant_id)
    if (existing) {
      setLines((ls) => ls.map((l) => (l.key === existing.key ? { ...l, quantity: toPlain(dec(l.quantity) + dec('1')) } : l)))
      pendingFocus.current = fieldId(existing.key, 'qty')
      return
    }
    const key = nextKey()
    setLines((ls) => [
      ...ls,
      {
        key,
        variant_id: p.variant_id,
        product_name: p.name,
        sku: p.sku,
        unit_short_name: p.unit_short_name,
        allow_fraction: p.allow_fraction,
        quantity: '1',
        unit_cost: trimDecimal(p.purchase_price),
        discount_type: 'amount',
        discount_value: '',
        tax_rate_pct: '0',
      },
    ])
    pendingFocus.current = fieldId(key, 'qty')
  }
  const updateLine = (key: string, patch: Partial<Line>) => setLines((ls) => ls.map((l) => (l.key === key ? { ...l, ...patch } : l)))
  const removeLine = (key: string) => {
    setLines((ls) => ls.filter((l) => l.key !== key))
    searchRef.current?.focus()
  }

  // Enter walks a line: quantity → unit cost → back to the product search
  // (from the discount and VAT boxes too); Ctrl+Delete removes the line.
  function onLineKey(e: KeyboardEvent<HTMLElement>, line: Line, field?: 'qty' | 'cost') {
    if (e.key === 'Delete' && e.ctrlKey) {
      e.preventDefault()
      removeLine(line.key)
    } else if (e.key === 'Enter' && (e.target as HTMLElement).tagName === 'INPUT') {
      e.preventDefault()
      if (field === 'qty') focusField(fieldId(line.key, 'cost'))
      else searchRef.current?.focus()
    }
  }

  const save = useMutation({
    mutationFn: () => {
      const input: PurchaseInput = {
        supplier_id: supplier?.id ?? '',
        supplier_invoice_no: optional(head.supplier_invoice_no),
        purchase_date: optional(head.purchase_date),
        expected_date: optional(head.expected_date),
        location_id: optional(head.location_id),
        lines: lines.map((l) => {
          const discount = optional(l.discount_value)
          return {
            variant_id: l.variant_id,
            quantity: l.quantity.trim(),
            unit_cost: l.unit_cost.trim(),
            ...(discount && dec(discount) !== 0n ? { discount_type: l.discount_type, discount_value: discount } : {}),
            tax_rate_pct: optional(l.tax_rate_pct),
          }
        }),
        bill_discount: optional(head.bill_discount),
        shipping_cost: optional(head.shipping_cost),
        other_cost: optional(head.other_cost),
        note: optional(head.note),
      }
      return current ? purchasingApi.updatePurchase(current.id, current.version, input) : purchasingApi.createPurchase(input)
    },
    onSuccess: async (draft) => {
      setSaved(draft)
      queryClient.setQueryData(purchasingKeys.purchase(tenantId, draft.id), draft)
      await queryClient.invalidateQueries({ queryKey: [...purchasingKeys.all(tenantId), 'list'] })
    },
    onError: (err) => {
      if (err instanceof ApiError && err.status === 412 && current) {
        void queryClient.invalidateQueries({ queryKey: purchasingKeys.purchase(tenantId, current.id) })
      }
    },
  })

  const serverErrors = fieldErrors(save.error, t)
  const conflict = save.error instanceof ApiError ? save.error.problem.code : ''
  if (conflict === 'DUPLICATE_SUPPLIER_INVOICE') serverErrors.supplier_invoice_no = problemMessage(save.error, t)
  const errors: Record<string, string | undefined> = { ...serverErrors, ...clientErrors }

  function validate(): Record<string, string> {
    const found: Record<string, string> = {}
    if (!supplier) found.supplier_id = t('purchases.errors.supplierRequired')
    if (head.purchase_date && head.purchase_date > today()) found.purchase_date = t('purchases.errors.futureDate')
    if (head.expected_date && head.expected_date < head.purchase_date) found.expected_date = t('purchases.errors.expectedBeforeOrder')
    if (lines.length === 0) found.lines = t('purchases.errors.noLines')
    lines.forEach((l, i) => {
      const qty = tryParse(l.quantity)
      if (qty === null || qty <= 0n) found[`lines[${i}].quantity`] = t('purchases.errors.quantityPositive')
      else if (!l.allow_fraction && qty % 10n ** 9n !== 0n) found[`lines[${i}].quantity`] = t('purchases.errors.wholeNumber')
      const cost = tryParse(l.unit_cost)
      if (cost === null || cost < 0n) found[`lines[${i}].unit_cost`] = t('purchases.errors.costInvalid')
      const rate = l.tax_rate_pct.trim() ? tryParse(l.tax_rate_pct) : 0n
      if (rate === null || rate < 0n || rate > dec('100')) found[`lines[${i}].tax_rate_pct`] = t('purchases.errors.percentInvalid')
      if (l.discount_value.trim() && tryParse(l.discount_value) === null) found[`lines[${i}].discount_value`] = t('purchases.errors.amountInvalid')
      else if (bill.lines[i]?.discountTooLarge) found[`lines[${i}].discount_value`] = t('purchases.errors.discountTooLarge')
    })
    for (const f of ['bill_discount', 'shipping_cost', 'other_cost'] as const) {
      const v = head[f].trim()
      if (v && (tryParse(v) === null || dec(v) < 0n)) found[f] = t('purchases.errors.amountInvalid')
    }
    if (!found.bill_discount && bill.billDiscountTooLarge) found.bill_discount = t('purchases.errors.discountTooLarge')
    return found
  }

  function check(): boolean {
    const found = validate()
    setClientErrors(found)
    return Object.keys(found).length === 0
  }
  const opened = (id: string) => navigate(`/purchases/${id}`, { replace: !purchase })

  function submit() {
    if (save.isPending || !check()) return
    save.mutate(undefined, { onSuccess: (draft) => opened(draft.id) })
  }

  function saveAndPost() {
    if (save.isPending || !check()) return
    postDraft.current = null
    setPosting(true)
  }
  // The dialog saves the draft once, then posts it; a retry only re-posts.
  async function draftToPost(): Promise<PurchaseDetail> {
    postDraft.current ??= await save.mutateAsync()
    return postDraft.current
  }
  const canPost = can('purchases.post')

  // Keyboard shortcuts: F2 product search, F4 supplier, F8 save draft, F9
  // save & post. They stand down while a dialog is open.
  useEffect(() => {
    function onKey(e: globalThis.KeyboardEvent) {
      if (document.querySelector('dialog[open]')) return
      if (e.key === 'F2') {
        e.preventDefault()
        searchRef.current?.focus()
      } else if (e.key === 'F4') {
        e.preventDefault()
        supplierRef.current?.focus()
      } else if (e.key === 'F8' && !e.repeat) {
        e.preventDefault()
        submit()
      } else if (e.key === 'F9' && !e.repeat && canPost) {
        e.preventDefault()
        saveAndPost()
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  })

  const lineError = (i: number, f: string) => errors[`lines[${i}].${f}`]
  const m = (v: bigint) => formatMoney(money(v), lang)
  const back = purchase ? `/purchases/${purchase.id}` : '/purchases'

  return (
    <div className="flex flex-col gap-4">
      <Link to={back} className={`text-sm ${LINK}`}>
        ← {purchase ? (purchase.purchase_no ?? t('purchases.draft')) : t('purchases.title')}
      </Link>
      <PageHeader title={purchase ? t('purchases.form.editTitle') : t('purchases.new')} subtitle={t('purchases.form.subtitle')} />

      <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_22rem] lg:items-start">
        <section className="flex min-w-0 flex-col gap-3" aria-label={t('purchases.form.items')}>
          <div className="flex items-center gap-2">
            <div className="min-w-0 flex-1">
              <PurchaseProductSearch tenantId={tenantId} inputRef={searchRef} onAdd={addProduct} />
            </div>
            <kbd className="hidden text-xs text-ink-muted sm:inline">F2</kbd>
          </div>
          {errors.lines && <p className="text-sm text-danger">{errors.lines}</p>}

          <Card className="flex flex-col gap-2 p-0">
            <div className="flex items-center justify-between gap-2 border-b border-border px-4 py-2">
              <h2 className="font-semibold">{t('purchases.form.items')}</h2>
              {lines.length > 0 && <span className="text-sm text-ink-muted">{t('purchases.form.lineCount', { count: lines.length })}</span>}
            </div>
            {lines.length === 0 ? (
              <p className="px-4 py-10 text-center text-sm text-ink-muted">{t('purchases.form.noItems')}</p>
            ) : (
              <div className="overflow-x-auto">
                <table className="w-full min-w-[820px] text-sm">
                  <caption className="sr-only">{t('purchases.form.items')}</caption>
                  <thead className="border-b border-border text-xs text-ink-muted uppercase">
                    <tr>
                      <th scope="col" className="w-8 px-2 py-2 text-left">#</th>
                      <th scope="col" className="px-2 py-2 text-left">{t('purchases.product')}</th>
                      <th scope="col" className="w-28 px-2 py-2 text-left">{t('common.quantity')}</th>
                      <th scope="col" className="w-28 px-2 py-2 text-left">{t('purchases.unitCost')}</th>
                      <th scope="col" className="w-40 px-2 py-2 text-left">{t('purchases.discount')}</th>
                      <th scope="col" className="w-20 px-2 py-2 text-left">{t('purchases.vatPct')}</th>
                      <th scope="col" className="px-2 py-2 text-right">{t('purchases.lineTotal')}</th>
                      <th scope="col" className="px-2 py-2 text-right">{t('purchases.landedCost')}</th>
                      <th scope="col" className="px-2 py-2"><span className="sr-only">{t('common.actions')}</span></th>
                    </tr>
                  </thead>
                  <tbody>
                    {lines.map((l, i) => {
                      const costed = bill.lines[i]
                      if (!costed) return null
                      const last = lastByVariant.get(l.variant_id)
                      const v = variance(l.unit_cost, last?.last_purchase_price)
                      return (
                        <tr key={l.key} className="border-b border-border align-top last:border-b-0 focus-within:bg-brand-50/40" onKeyDown={(e) => onLineKey(e, l)}>
                          <td className="px-2 py-3 text-ink-muted tabular-nums">{i + 1}</td>
                          <td className="px-2 py-2">
                            <span className="font-medium">{l.product_name}</span>
                            <span className="block font-mono text-xs text-ink-muted">{l.sku}</span>
                            {last?.last_purchase_price ? (
                              <span className="mt-1 block text-xs text-ink-muted">
                                {t('purchases.form.lastPrice', {
                                  price: formatMoney(last.last_purchase_price, lang),
                                  date: formatDate(last.last_purchase_date, lang),
                                  qty: formatQuantity(last.last_purchase_qty ?? '0', lang, true),
                                  supplier: last.last_supplier_name ?? '—',
                                })}
                                {v && v.sign !== 0 && (
                                  <span className={`ml-1 font-semibold ${v.sign > 0 ? 'text-danger' : 'text-brand-700'}`}>
                                    {t('purchases.form.variance', { amount: formatMoney(v.amount, lang), pct: `${v.sign > 0 ? '+' : ''}${v.pct}` })}
                                  </span>
                                )}
                              </span>
                            ) : (
                              last && <span className="mt-1 block text-xs text-ink-muted">{t('purchases.form.firstPurchase')}</span>
                            )}
                          </td>
                          <td className="px-2 py-2">
                            <LineInput
                              id={fieldId(l.key, 'qty')}
                              label={`${t('common.quantity')} ${l.product_name}`}
                              value={l.quantity}
                              error={lineError(i, 'quantity')}
                              onChange={(v) => updateLine(l.key, { quantity: v })}
                              onKeyDown={(e) => {
                                e.stopPropagation()
                                onLineKey(e, l, 'qty')
                              }}
                              suffix={l.unit_short_name}
                            />
                          </td>
                          <td className="px-2 py-2">
                            <LineInput
                              id={fieldId(l.key, 'cost')}
                              label={`${t('purchases.unitCost')} ${l.product_name}`}
                              value={l.unit_cost}
                              error={lineError(i, 'unit_cost')}
                              onChange={(v) => updateLine(l.key, { unit_cost: v })}
                              onKeyDown={(e) => {
                                e.stopPropagation()
                                onLineKey(e, l, 'cost')
                              }}
                            />
                          </td>
                          <td className="px-2 py-2">
                            <div className="flex gap-1">
                              <select
                                aria-label={`${t('purchases.discountType')} ${l.product_name}`}
                                value={l.discount_type}
                                onChange={(e) => updateLine(l.key, { discount_type: e.target.value as Line['discount_type'] })}
                                className={`${INPUT} w-14`}
                              >
                                <option value="amount">{currencySymbol()}</option>
                                <option value="percent">%</option>
                              </select>
                              <LineInput label={`${t('purchases.discount')} ${l.product_name}`} value={l.discount_value} error={lineError(i, 'discount_value')} onChange={(v) => updateLine(l.key, { discount_value: v })} />
                            </div>
                          </td>
                          <td className="px-2 py-2">
                            <LineInput label={`${t('purchases.vatPct')} ${l.product_name}`} value={l.tax_rate_pct} error={lineError(i, 'tax_rate_pct')} onChange={(v) => updateLine(l.key, { tax_rate_pct: v })} />
                          </td>
                          <td className="px-2 py-3 text-right font-semibold tabular-nums whitespace-nowrap">
                            {m(costed.line_total)}
                            {costed.tax !== 0n && <span className="block text-xs font-normal text-ink-muted">{t('purchases.form.inclVat', { amount: m(costed.tax) })}</span>}
                          </td>
                          <td className="px-2 py-3 text-right tabular-nums whitespace-nowrap text-ink-muted">{m(costed.landed_unit_cost)}</td>
                          <td className="px-2 py-2 text-right">
                            <Button variant="ghost" className="min-h-9 px-2" onClick={() => removeLine(l.key)} aria-label={`${t('common.remove')} ${l.product_name}`}>
                              ✕
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

          <Card>
            <label className="flex flex-col gap-1.5 text-sm font-medium text-ink">
              {`${t('common.note')} (${t('common.optional')})`}
              <textarea
                value={head.note}
                onChange={setField('note')}
                rows={2}
                maxLength={1000}
                className="rounded-lg border border-border bg-surface px-3 py-2 text-sm font-normal outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100"
              />
            </label>
          </Card>
        </section>

        <aside className="flex flex-col gap-4 rounded-xl border border-border bg-surface p-4 lg:sticky lg:top-4">
          <SupplierPanel
            tenantId={tenantId}
            supplier={supplier}
            inputRef={supplierRef}
            error={errors.supplier_id}
            canAdd={can('suppliers.manage')}
            onSelect={(s) => {
              setSupplier(s)
              if (s) searchRef.current?.focus()
            }}
          />

          <section className="flex flex-col gap-3" aria-label={t('purchases.form.billDetails')}>
            <h2 className="text-xs font-medium text-ink-muted uppercase">{t('purchases.form.billDetails')}</h2>
            <TextField
              label={`${t('purchases.supplierInvoiceNo')} (${t('common.optional')})`}
              value={head.supplier_invoice_no}
              onChange={setField('supplier_invoice_no')}
              maxLength={64}
              error={errors.supplier_invoice_no}
            />
            <div className="grid grid-cols-2 gap-3">
              <TextField label={t('purchases.purchaseDate')} type="date" max={today()} value={head.purchase_date} onChange={setField('purchase_date')} error={errors.purchase_date} />
              <Select
                label={t('purchases.location')}
                value={head.location_id}
                onChange={setField('location_id')}
                options={[{ value: '', label: t('purchases.form.defaultLocation') }, ...locations.map((l) => ({ value: l.id, label: l.name }))]}
                error={errors.location_id}
              />
            </div>
            <TextField
              label={`${t('purchases.expectedDate')} (${t('common.optional')})`}
              type="date"
              min={head.purchase_date}
              value={head.expected_date}
              onChange={setField('expected_date')}
              hint={t('purchases.form.expectedHint')}
              error={errors.expected_date}
            />
          </section>

          <section className="flex flex-col gap-2 border-t border-border pt-3" aria-label={t('common.totals')}>
            <dl className="flex flex-col gap-1 text-sm">
              <Row label={t('purchases.subtotal')} value={m(bill.subtotal)} />
              {bill.item_discount_total !== 0n && <Row label={t('purchases.itemDiscounts')} value={`− ${m(bill.item_discount_total)}`} />}
              {bill.tax_total !== 0n && <Row label={t('purchases.vat')} value={m(bill.tax_total)} />}
            </dl>
            <div className="grid grid-cols-3 gap-2">
              <AmountField label={t('purchases.billDiscount')} value={head.bill_discount} onChange={setField('bill_discount')} error={errors.bill_discount} />
              <AmountField label={t('purchases.shipping')} value={head.shipping_cost} onChange={setField('shipping_cost')} error={errors.shipping_cost} />
              <AmountField label={t('purchases.otherCost')} value={head.other_cost} onChange={setField('other_cost')} error={errors.other_cost} />
            </div>
            <p className="text-xs text-ink-muted">{t('purchases.form.chargesHint')}</p>
            <div className="flex items-baseline justify-between gap-3 rounded-lg bg-brand-50 px-4 py-3">
              <span className="font-semibold">{t('purchases.grandTotal')}</span>
              <span data-testid="purchase-grand-total" className="text-2xl font-bold text-brand-700 tabular-nums">
                {m(bill.grand_total)}
              </span>
            </div>
            <p className="text-xs text-ink-muted">{vatRegistered ? t('purchases.form.vatRecoverable') : t('purchases.form.vatInCost')}</p>
          </section>

          {save.isError && hasFormError(save.error) && conflict !== 'DUPLICATE_SUPPLIER_INVOICE' && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}
          {Object.keys(clientErrors).length > 0 && <Alert tone="danger">{t('purchases.errors.fixFields')}</Alert>}

          {canPost && (
            <Button className="min-h-14 text-lg" disabled={save.isPending} onClick={saveAndPost} aria-keyshortcuts="F9">
              {t('purchases.form.saveAndPost')}{' '}
              <kbd aria-hidden="true" className="text-sm opacity-80">
                F9
              </kbd>
            </Button>
          )}
          <Button
            variant={canPost ? 'secondary' : 'primary'}
            className={canPost ? '' : 'min-h-14 text-lg'}
            disabled={save.isPending}
            onClick={submit}
            aria-keyshortcuts="F8"
          >
            {save.isPending && !posting ? t('common.saving') : t('purchases.form.saveDraft')}{' '}
            <kbd aria-hidden="true" className="text-sm opacity-80">
              F8
            </kbd>
          </Button>
          <Link to={back} className="inline-flex min-h-10 items-center justify-center rounded-lg px-4 text-sm font-semibold text-ink-muted hover:bg-canvas">
            {t('common.cancel')}
          </Link>

          <dl aria-label={t('purchases.form.shortcuts.title')} className="hidden grid-cols-2 gap-x-3 gap-y-0.5 text-xs text-ink-muted md:grid">
            {SHORTCUTS.map(([key, name]) => (
              <div key={key} className="flex gap-1.5">
                <dt>
                  <kbd className="rounded border border-border bg-canvas px-1 font-mono">{key}</kbd>
                </dt>
                <dd>{t(`purchases.form.shortcuts.${name}`)}</dd>
              </div>
            ))}
          </dl>
        </aside>
      </div>

      {posting && (
        <PostDialog
          tenantId={tenantId}
          grandTotal={money(bill.grand_total)}
          draft={draftToPost}
          onClose={() => setPosting(false)}
          onPosted={(posted) => opened(posted.id)}
        />
      )}
    </div>
  )
}

function LineInput({
  id,
  label,
  value,
  error,
  onChange,
  onKeyDown,
  suffix,
}: {
  id?: string
  label: string
  value: string
  error?: string
  onChange: (v: string) => void
  onKeyDown?: (e: KeyboardEvent<HTMLInputElement>) => void
  suffix?: string
}) {
  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center gap-1">
        <input
          id={id}
          aria-label={label}
          inputMode="decimal"
          autoComplete="off"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onFocus={(e) => e.target.select()}
          onKeyDown={onKeyDown}
          aria-invalid={error ? true : undefined}
          className={`${INPUT} tabular-nums`}
        />
        {suffix && <span className="text-xs text-ink-muted">{suffix}</span>}
      </div>
      {error && <p className="text-xs text-danger">{error}</p>}
    </div>
  )
}

function AmountField({ label, value, onChange, error }: { label: string; value: string; onChange: (e: { target: { value: string } }) => void; error?: string }) {
  return <TextField label={label} inputMode="decimal" autoComplete="off" placeholder="0" value={value} onChange={onChange} onFocus={(e) => e.target.select()} error={error} />
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex justify-between gap-4">
      <dt className="text-ink-muted">{label}</dt>
      <dd className="tabular-nums">{value}</dd>
    </div>
  )
}
