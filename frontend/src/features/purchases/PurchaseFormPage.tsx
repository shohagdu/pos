import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, type KeyboardEvent, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate, useParams } from 'react-router'

import { type Product, catalogApi } from '../../api/catalog'
import { ApiError } from '../../api/client'
import { useStockLocations } from '../../api/inventory'
import { useSuppliers } from '../../api/parties'
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
import { formatMoney, formatQuantity } from '../../lib/format'
import { optional } from '../../lib/form'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { type CostLineInput, costBill, money, variance } from './costing'
import { LINK, trimDecimal } from './helpers'

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

const PRODUCT_SEARCH_ID = 'purchase-product-search'
let lineSeq = 0
const nextKey = () => `l${++lineSeq}`

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

const INPUT =
  'min-h-11 w-full rounded-lg border border-border bg-surface px-2 text-sm outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100 aria-invalid:border-danger'

function PurchaseForm({ tenantId, purchase }: { tenantId: string; purchase?: PurchaseDetail }) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const { can } = useSession()
  const locations = (useStockLocations(tenantId).data ?? []).filter((l) => l.status === 'active' || l.id === purchase?.location_id)
  const vatRegistered = useSettings(tenantId).data?.is_vat_registered ?? false

  const [supplierQ, setSupplierQ] = useState('')
  const [supplier, setSupplier] = useState<{ id: string; name: string } | null>(
    purchase ? { id: purchase.supplier_id, name: purchase.supplier_name } : null,
  )
  const suppliers = useSuppliers(tenantId, { q: supplierQ.trim() || undefined, status: 'active' })
  const [head, setHead] = useState({
    supplier_invoice_no: purchase?.supplier_invoice_no ?? '',
    purchase_date: purchase?.purchase_date ?? today(),
    location_id: purchase?.location_id ?? locations.find((l) => l.is_sellable)?.id ?? '',
    bill_discount: trimDecimal(purchase?.bill_discount_amount),
    shipping_cost: trimDecimal(purchase?.shipping_cost),
    other_cost: trimDecimal(purchase?.other_cost),
    note: purchase?.note ?? '',
  })
  const setField = (name: keyof typeof head) => (e: { target: { value: string } }) => setHead((h) => ({ ...h, [name]: e.target.value }))
  const [lines, setLines] = useState<Line[]>(() => (purchase ? fromItems(purchase) : []))
  const [clientErrors, setClientErrors] = useState<Record<string, string>>({})

  // Product picker
  const [productText, setProductText] = useState('')
  const [productQ, setProductQ] = useState('')
  const products = useQuery({
    queryKey: ['catalog', tenantId, 'products', 'purchase-picker', productQ],
    queryFn: () => catalogApi.products({ q: productQ, status: 'active' }, null),
    enabled: productQ.length > 0,
  })

  const variantIds = useMemo(() => [...new Set(lines.map((l) => l.variant_id))].sort(), [lines])
  const lastPrices = useLastPrices(tenantId, variantIds)
  const lastByVariant = new Map<string, LastPrice>((lastPrices.data ?? []).map((p) => [p.variant_id, p]))

  const bill = costBill(lines, head.bill_discount, head.shipping_cost, head.other_cost, vatRegistered)

  function addProduct(p: Product) {
    setLines((ls) => {
      const existing = ls.find((l) => l.variant_id === p.variant_id)
      if (existing) {
        return ls.map((l) => (l === existing ? { ...l, quantity: toPlain(dec(l.quantity) + dec('1')) } : l))
      }
      return [
        ...ls,
        {
          key: nextKey(),
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
      ]
    })
    setProductText('')
    setProductQ('')
    document.getElementById(PRODUCT_SEARCH_ID)?.focus()
  }
  const updateLine = (key: string, patch: Partial<Line>) => setLines((ls) => ls.map((l) => (l.key === key ? { ...l, ...patch } : l)))
  const removeLine = (key: string) => setLines((ls) => ls.filter((l) => l.key !== key))

  function onProductKey(e: KeyboardEvent<HTMLInputElement>) {
    if (e.key === 'Enter') {
      e.preventDefault()
      setProductQ(productText.trim())
    }
  }

  const save = useMutation({
    mutationFn: () => {
      const input: PurchaseInput = {
        supplier_id: supplier?.id ?? '',
        supplier_invoice_no: optional(head.supplier_invoice_no),
        purchase_date: optional(head.purchase_date),
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
      return purchase ? purchasingApi.updatePurchase(purchase.id, purchase.version, input) : purchasingApi.createPurchase(input)
    },
    onSuccess: async (saved) => {
      queryClient.setQueryData(purchasingKeys.purchase(tenantId, saved.id), saved)
      await queryClient.invalidateQueries({ queryKey: [...purchasingKeys.all(tenantId), 'list'] })
      navigate(`/purchases/${saved.id}`, { replace: !purchase })
    },
    onError: (err) => {
      if (err instanceof ApiError && err.status === 412 && purchase) {
        void queryClient.invalidateQueries({ queryKey: purchasingKeys.purchase(tenantId, purchase.id) })
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

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    const found = validate()
    setClientErrors(found)
    if (Object.keys(found).length === 0) save.mutate()
  }

  const supplierRows = suppliers.data?.pages.flatMap((p) => p.data) ?? []
  const supplierOptions = [
    { value: '', label: t('purchases.form.chooseSupplier') },
    ...(supplier && !supplierRows.some((s) => s.id === supplier.id) ? [{ value: supplier.id, label: supplier.name }] : []),
    ...supplierRows.map((s) => ({ value: s.id, label: `${s.name}${s.phone ? ` · ${s.phone}` : ''}` })),
  ]
  const results = products.data?.data ?? []
  const lineError = (i: number, f: string) => errors[`lines[${i}].${f}`]
  const m = (v: bigint) => formatMoney(money(v), lang)

  return (
    <div className="flex max-w-6xl flex-col gap-6">
      <Link to={purchase ? `/purchases/${purchase.id}` : '/purchases'} className={`text-sm ${LINK}`}>
        ← {purchase ? (purchase.purchase_no ?? t('purchases.draft')) : t('purchases.title')}
      </Link>
      <PageHeader title={purchase ? t('purchases.form.editTitle') : t('purchases.new')} subtitle={t('purchases.form.subtitle')} />
      <form onSubmit={onSubmit} noValidate className="flex flex-col gap-6">
        <Card className="flex flex-col gap-4">
          <h2 className="font-semibold">{t('purchases.form.billDetails')}</h2>
          <div className="grid gap-4 sm:grid-cols-2">
            <TextField
              label={t('purchases.form.findSupplier')}
              type="search"
              value={supplierQ}
              onChange={(e) => setSupplierQ(e.target.value)}
              placeholder={t('purchases.form.findSupplierHint')}
            />
            <div className="flex flex-col gap-1">
              <Select
                label={t('purchases.supplier')}
                value={supplier?.id ?? ''}
                onChange={(e) => {
                  const s = supplierRows.find((x) => x.id === e.target.value)
                  setSupplier(s ? { id: s.id, name: s.name } : null)
                }}
                options={supplierOptions}
                error={errors.supplier_id}
              />
              {can('suppliers.manage') && (
                <Link to="/suppliers/new" className={`text-sm ${LINK}`}>
                  + {t('purchases.form.newSupplier')}
                </Link>
              )}
            </div>
            <TextField
              label={`${t('purchases.supplierInvoiceNo')} (${t('common.optional')})`}
              value={head.supplier_invoice_no}
              onChange={setField('supplier_invoice_no')}
              maxLength={64}
              error={errors.supplier_invoice_no}
            />
            <TextField
              label={t('purchases.purchaseDate')}
              type="date"
              max={today()}
              value={head.purchase_date}
              onChange={setField('purchase_date')}
              error={errors.purchase_date}
            />
            <Select
              label={t('purchases.location')}
              value={head.location_id}
              onChange={setField('location_id')}
              options={[{ value: '', label: t('purchases.form.defaultLocation') }, ...locations.map((l) => ({ value: l.id, label: l.name }))]}
              error={errors.location_id}
            />
          </div>
        </Card>

        <Card className="flex flex-col gap-4">
          <h2 className="font-semibold">{t('purchases.form.items')}</h2>
          <div className="flex flex-wrap items-end gap-2">
            <TextField
              id={PRODUCT_SEARCH_ID}
              className="min-w-60 flex-1"
              label={t('purchases.form.findProduct')}
              type="search"
              value={productText}
              onChange={(e) => setProductText(e.target.value)}
              onKeyDown={onProductKey}
              placeholder={t('purchases.form.findProductHint')}
            />
            <Button variant="secondary" onClick={() => setProductQ(productText.trim())}>
              {t('common.search')}
            </Button>
          </div>
          {productQ && (
            <div className="rounded-lg border border-border">
              {products.isPending ? (
                <Spinner className="p-3" />
              ) : products.isError ? (
                <Alert tone="danger">{problemMessage(products.error, t)}</Alert>
              ) : results.length === 0 ? (
                <p className="p-3 text-sm text-ink-muted">{t('common.noResults')}</p>
              ) : (
                <ul aria-label={t('purchases.form.results')} className="max-h-72 divide-y divide-border overflow-y-auto">
                  {results.map((p) => (
                    <li key={p.variant_id} className="flex flex-wrap items-center justify-between gap-2 px-3 py-2 text-sm">
                      <span>
                        <span className="font-medium">{p.name}</span> <span className="font-mono text-xs text-ink-muted">{p.sku}</span>
                        {p.purchase_price !== null && (
                          <span className="block text-xs text-ink-muted">
                            {t('purchases.form.currentCost')}: {formatMoney(p.purchase_price, lang)} / {p.unit_short_name}
                          </span>
                        )}
                      </span>
                      {p.track_inventory ? (
                        <Button variant="secondary" className="min-h-9" onClick={() => addProduct(p)} aria-label={`${t('common.add')} ${p.name}`}>
                          {t('common.add')}
                        </Button>
                      ) : (
                        <span className="text-xs text-ink-muted">{t('purchases.form.serviceNotStocked')}</span>
                      )}
                    </li>
                  ))}
                </ul>
              )}
            </div>
          )}
          {errors.lines && <p className="text-sm text-danger">{errors.lines}</p>}
          {lines.length > 0 && (
            <div className="overflow-x-auto">
              <table className="w-full min-w-[900px] text-sm">
                <caption className="sr-only">{t('purchases.form.items')}</caption>
                <thead className="border-b border-border text-xs text-ink-muted uppercase">
                  <tr>
                    <th scope="col" className="px-2 py-2 text-left">{t('purchases.product')}</th>
                    <th scope="col" className="w-24 px-2 py-2 text-left">{t('common.quantity')}</th>
                    <th scope="col" className="w-28 px-2 py-2 text-left">{t('purchases.unitCost')}</th>
                    <th scope="col" className="w-44 px-2 py-2 text-left">{t('purchases.discount')}</th>
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
                      <tr key={l.key} className="border-b border-border align-top last:border-b-0">
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
                          <LineInput label={`${t('common.quantity')} ${l.product_name}`} value={l.quantity} error={lineError(i, 'quantity')} onChange={(v) => updateLine(l.key, { quantity: v })} suffix={l.unit_short_name} />
                        </td>
                        <td className="px-2 py-2">
                          <LineInput label={`${t('purchases.unitCost')} ${l.product_name}`} value={l.unit_cost} error={lineError(i, 'unit_cost')} onChange={(v) => updateLine(l.key, { unit_cost: v })} />
                        </td>
                        <td className="px-2 py-2">
                          <div className="flex gap-1">
                            <select
                              aria-label={`${t('purchases.discountType')} ${l.product_name}`}
                              value={l.discount_type}
                              onChange={(e) => updateLine(l.key, { discount_type: e.target.value as Line['discount_type'] })}
                              className={`${INPUT} w-16`}
                            >
                              <option value="amount">৳</option>
                              <option value="percent">%</option>
                            </select>
                            <LineInput label={`${t('purchases.discount')} ${l.product_name}`} value={l.discount_value} error={lineError(i, 'discount_value')} onChange={(v) => updateLine(l.key, { discount_value: v })} />
                          </div>
                        </td>
                        <td className="px-2 py-2">
                          <LineInput label={`${t('purchases.vatPct')} ${l.product_name}`} value={l.tax_rate_pct} error={lineError(i, 'tax_rate_pct')} onChange={(v) => updateLine(l.key, { tax_rate_pct: v })} />
                        </td>
                        <td className="px-2 py-2 text-right tabular-nums whitespace-nowrap">
                          {m(costed.line_total)}
                          {costed.tax !== 0n && <span className="block text-xs text-ink-muted">{t('purchases.form.inclVat', { amount: m(costed.tax) })}</span>}
                        </td>
                        <td className="px-2 py-2 text-right tabular-nums whitespace-nowrap">{m(costed.landed_unit_cost)}</td>
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
          {lines.length === 0 && <p className="text-sm text-ink-muted">{t('purchases.form.noItems')}</p>}
        </Card>

        <div className="grid gap-6 lg:grid-cols-2">
          <Card className="flex flex-col gap-4">
            <h2 className="font-semibold">{t('purchases.form.charges')}</h2>
            <div className="grid gap-4 sm:grid-cols-3">
              <TextField label={t('purchases.billDiscount')} inputMode="decimal" value={head.bill_discount} onChange={setField('bill_discount')} error={errors.bill_discount} />
              <TextField label={t('purchases.shipping')} inputMode="decimal" value={head.shipping_cost} onChange={setField('shipping_cost')} error={errors.shipping_cost} />
              <TextField label={t('purchases.otherCost')} inputMode="decimal" value={head.other_cost} onChange={setField('other_cost')} error={errors.other_cost} />
            </div>
            <p className="text-xs text-ink-muted">{t('purchases.form.chargesHint')}</p>
            <label className="flex flex-col gap-1.5 text-sm font-medium text-ink">
              {t('common.note')}
              <textarea
                value={head.note}
                onChange={setField('note')}
                rows={3}
                maxLength={1000}
                className="rounded-lg border border-border bg-surface px-3 py-2 text-sm font-normal outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100"
              />
            </label>
          </Card>
          <Card className="flex flex-col gap-2">
            <h2 className="font-semibold">{t('common.totals')}</h2>
            <dl className="flex flex-col gap-1 text-sm" aria-label={t('common.totals')}>
              <Row label={t('purchases.subtotal')} value={m(bill.subtotal)} />
              <Row label={t('purchases.itemDiscounts')} value={`− ${m(bill.item_discount_total)}`} />
              <Row label={t('purchases.billDiscount')} value={`− ${m(bill.bill_discount)}`} />
              <Row label={t('purchases.vat')} value={m(bill.tax_total)} />
              <Row label={t('purchases.shipping')} value={m(bill.shipping)} />
              <Row label={t('purchases.otherCost')} value={m(bill.other)} />
              <Row label={t('purchases.grandTotal')} value={m(bill.grand_total)} strong />
            </dl>
            <p className="text-xs text-ink-muted">{vatRegistered ? t('purchases.form.vatRecoverable') : t('purchases.form.vatInCost')}</p>
          </Card>
        </div>

        {save.isError && hasFormError(save.error) && conflict !== 'DUPLICATE_SUPPLIER_INVOICE' && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}
        {Object.keys(clientErrors).length > 0 && <Alert tone="danger">{t('purchases.errors.fixFields')}</Alert>}
        <div className="flex flex-wrap justify-end gap-2">
          <Link to={purchase ? `/purchases/${purchase.id}` : '/purchases'} className="inline-flex min-h-11 items-center rounded-lg px-4 text-sm font-semibold text-ink-muted hover:bg-canvas">
            {t('common.cancel')}
          </Link>
          <Button type="submit" disabled={save.isPending}>
            {save.isPending ? t('common.saving') : t('purchases.form.saveDraft')}
          </Button>
        </div>
      </form>
    </div>
  )
}

function LineInput({ label, value, error, onChange, suffix }: { label: string; value: string; error?: string; onChange: (v: string) => void; suffix?: string }) {
  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center gap-1">
        <input aria-label={label} inputMode="decimal" value={value} onChange={(e) => onChange(e.target.value)} aria-invalid={error ? true : undefined} className={INPUT} />
        {suffix && <span className="text-xs text-ink-muted">{suffix}</span>}
      </div>
      {error && <p className="text-xs text-danger">{error}</p>}
    </div>
  )
}

function Row({ label, value, strong = false }: { label: string; value: string; strong?: boolean }) {
  return (
    <div className={`flex justify-between gap-4 ${strong ? 'border-t border-border pt-2 text-base font-bold' : ''}`}>
      <dt className={strong ? '' : 'text-ink-muted'}>{label}</dt>
      <dd className="tabular-nums">{value}</dd>
    </div>
  )
}
