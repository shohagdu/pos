import { useMutation, useQuery } from '@tanstack/react-query'
import { type CSSProperties, useEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useSearchParams } from 'react-router'

import { type LabelInfo, type Product, catalogApi } from '../../api/catalog'
import { usePurchase } from '../../api/purchasing'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { Checkbox } from '../../components/ui/Checkbox'
import { PageHeader } from '../../components/ui/PageHeader'
import { Select } from '../../components/ui/Select'
import { Spinner } from '../../components/ui/Spinner'
import { TextField } from '../../components/ui/TextField'
import { formatMoney } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { PurchaseProductSearch } from '../purchases/PurchaseProductSearch'
import { Barcode } from './Barcode'
import { MAX_COPIES, purchaseCopies } from './copies'
import { DEFAULT_LAYOUT, LABEL_LAYOUTS, type LabelLayout, layoutById, paginate, perPage } from './layouts'

const MAX_LABELS = 5000

export interface LabelRow {
  label: LabelInfo
  copies: number
}

export interface LabelOptions {
  layout: string
  showBusiness: boolean
  showName: boolean
  showPrice: boolean
  showSku: boolean
  /** Print the SKU as Code 128 when a product has no barcode. */
  skuFallback: boolean
}

const DEFAULT_OPTIONS: LabelOptions = {
  layout: DEFAULT_LAYOUT.id,
  showBusiness: false,
  showName: true,
  showPrice: true,
  showSku: false,
  skuFallback: true,
}

const storageKey = (tenantId: string) => `pos.labels.${tenantId}`

function loadOptions(tenantId: string): LabelOptions {
  try {
    const saved = localStorage.getItem(storageKey(tenantId))
    if (saved) return { ...DEFAULT_OPTIONS, ...(JSON.parse(saved) as Partial<LabelOptions>) }
  } catch {
    // storage unavailable or corrupt: defaults
  }
  return DEFAULT_OPTIONS
}

function saveOptions(tenantId: string, options: LabelOptions) {
  try {
    localStorage.setItem(storageKey(tenantId), JSON.stringify(options))
  } catch {
    // storage unavailable: settings last for this visit only
  }
}

/**
 * `/labels`: barcode labels for a purchase (`?purchase=<id>`) or chosen
 * products (`?variants=<id>,<id>`), with copies per product, label stock,
 * and products added by search or scan (PROD-R05).
 */
export function LabelPrintPage() {
  const { t } = useTranslation()
  const [params] = useSearchParams()
  const { me } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const purchaseId = params.get('purchase') ?? ''
  const variantParam = params.get('variants') ?? ''
  const purchase = usePurchase(tenantId, purchaseId, purchaseId !== '')

  const source = useMemo(() => {
    if (purchaseId) return purchase.data ? purchaseCopies(purchase.data) : null
    return new Map(
      variantParam
        .split(',')
        .filter(Boolean)
        .map((id) => [id, 1]),
    )
  }, [purchaseId, purchase.data, variantParam])

  const ids = source ? [...source.keys()] : []
  const labels = useQuery({
    queryKey: ['catalog', tenantId, 'labels', ids],
    queryFn: () => catalogApi.labels(ids),
    enabled: ids.length > 0,
    staleTime: Infinity,
    gcTime: 0,
  })

  if (purchaseId && purchase.isPending) return <Spinner />
  if (purchaseId && purchase.isError) return <Alert tone="danger">{problemMessage(purchase.error, t)}</Alert>
  if (ids.length > 0 && labels.isPending) return <Spinner />
  if (labels.isError) return <Alert tone="danger">{problemMessage(labels.error, t)}</Alert>

  const initial: LabelRow[] = (labels.data ?? []).map((label) => ({ label, copies: source?.get(label.variant_id) ?? 1 }))
  return (
    <LabelEditor
      key={`${purchaseId}|${variantParam}`}
      tenantId={tenantId}
      initial={initial}
      back={
        purchaseId
          ? { to: `/purchases/${purchaseId}`, label: purchase.data?.purchase_no ?? t('purchases.draftTitle') }
          : { to: '/products', label: t('products.title') }
      }
    />
  )
}

function LabelEditor({ tenantId, initial, back }: { tenantId: string; initial: LabelRow[]; back: { to: string; label: string } | null }) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { me, can } = useSession()
  const [rows, setRows] = useState<LabelRow[]>(initial)
  const [options, setOptions] = useState<LabelOptions>(() => loadOptions(tenantId))
  const [skip, setSkip] = useState(0)
  const searchRef = useRef<HTMLInputElement>(null)
  const layout = layoutById(options.layout)

  const setOption = <K extends keyof LabelOptions>(key: K, value: LabelOptions[K]) => {
    const next = { ...options, [key]: value }
    setOptions(next)
    saveOptions(tenantId, next)
  }

  const add = useMutation({
    mutationFn: (product: Product) => catalogApi.labels([product.variant_id]),
    onSuccess: ([label]) => {
      if (!label) return
      setRows((current) =>
        current.some((r) => r.label.variant_id === label.variant_id)
          ? current.map((r) => (r.label.variant_id === label.variant_id ? { ...r, copies: Math.min(MAX_COPIES, r.copies + 1) } : r))
          : [...current, { label, copies: 1 }],
      )
    },
  })

  const missing = rows.filter((r) => !r.label.barcode && r.copies > 0)
  const generate = useMutation({
    mutationFn: () => catalogApi.generateBarcodes(missing.map((r) => r.label.variant_id)),
    onSuccess: (updated) => {
      const byId = new Map(updated.map((l) => [l.variant_id, l]))
      setRows((current) => current.map((r) => ({ ...r, label: byId.get(r.label.variant_id) ?? r.label })))
    },
  })

  const setCopies = (variantId: string, raw: string) => {
    const n = Math.min(MAX_COPIES, Math.max(0, Math.floor(Number(raw) || 0)))
    setRows((current) => current.map((r) => (r.label.variant_id === variantId ? { ...r, copies: n } : r)))
  }

  // What gets printed: each row repeated, rows without any code left out.
  const printable = rows.filter((r) => r.label.barcode || options.skuFallback)
  const total = printable.reduce((sum, r) => sum + r.copies, 0)
  const tooMany = total > MAX_LABELS
  const queue = tooMany ? [] : printable.flatMap((r) => Array<LabelInfo>(r.copies).fill(r.label))
  const pages = paginate(queue, layout, skip)

  return (
    <div className="flex max-w-6xl flex-col gap-6">
      <PageStyle layout={layout} />
      <div className="flex flex-col gap-6 print:hidden">
        {back && (
          <Link to={back.to} className="text-sm text-brand-700 hover:underline">
            ← {back.label}
          </Link>
        )}
        <PageHeader
          title={t('labels.title')}
          subtitle={t('labels.subtitle')}
          actions={
            <Button disabled={total === 0 || tooMany} onClick={() => window.print()}>
              {t('labels.print', { count: total })}
            </Button>
          }
        />

        <div className="grid grid-cols-1 gap-6 lg:grid-cols-[2fr_1fr]">
          <Card>
            <div className="flex flex-col gap-4">
              <PurchaseProductSearch tenantId={tenantId} inputRef={searchRef} onAdd={(p) => add.mutate(p)} />
              {add.isError && <Alert tone="danger">{problemMessage(add.error, t)}</Alert>}
              {rows.length === 0 ? (
                <p className="text-sm text-ink-muted">{t('labels.empty')}</p>
              ) : (
                <table className="w-full text-sm">
                  <caption className="sr-only">{t('labels.products')}</caption>
                  <thead>
                    <tr className="border-b border-border text-left text-xs text-ink-muted uppercase">
                      <th className="py-2 pr-2 font-medium">{t('labels.product')}</th>
                      <th className="py-2 pr-2 font-medium">{t('labels.barcode')}</th>
                      <th className="py-2 pr-2 text-right font-medium">{t('labels.price')}</th>
                      <th className="w-24 py-2 text-right font-medium">{t('labels.copies')}</th>
                      <th className="w-10 py-2">
                        <span className="sr-only">{t('common.actions')}</span>
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    {rows.map((r) => (
                      <tr key={r.label.variant_id} className="border-b border-border last:border-0">
                        <td className="py-2 pr-2">
                          <span className="font-medium">{displayName(r.label, lang)}</span>
                          <span className="block font-mono text-xs text-ink-muted">{r.label.sku}</span>
                        </td>
                        <td className="py-2 pr-2 font-mono text-xs">
                          {r.label.barcode ?? (
                            <span className="font-sans text-ink-muted italic">{options.skuFallback ? t('labels.usesSku') : t('labels.noBarcode')}</span>
                          )}
                        </td>
                        <td className="py-2 pr-2 text-right tabular-nums">{formatMoney(r.label.selling_price, lang)}</td>
                        <td className="py-2 text-right">
                          <input
                            type="number"
                            min={0}
                            max={MAX_COPIES}
                            step={1}
                            inputMode="numeric"
                            aria-label={t('labels.copiesOf', { name: r.label.product_name })}
                            value={r.copies}
                            onChange={(e) => setCopies(r.label.variant_id, e.target.value)}
                            className="min-h-9 w-20 rounded-lg border border-border bg-surface px-2 text-right tabular-nums"
                          />
                        </td>
                        <td className="py-2 text-right">
                          <button
                            type="button"
                            aria-label={t('labels.remove', { name: r.label.product_name })}
                            className="rounded px-2 text-ink-muted hover:text-danger"
                            onClick={() => setRows((current) => current.filter((x) => x.label.variant_id !== r.label.variant_id))}
                          >
                            ×
                          </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
              {missing.length > 0 && (
                <Alert tone="warning">
                  <div className="flex flex-wrap items-center justify-between gap-3">
                    <span>{t('labels.missing', { count: missing.length })}</span>
                    {can('products.update') && (
                      <Button variant="secondary" disabled={generate.isPending} onClick={() => generate.mutate()}>
                        {t('labels.generate', { count: missing.length })}
                      </Button>
                    )}
                  </div>
                </Alert>
              )}
              {generate.isError && <Alert tone="danger">{problemMessage(generate.error, t)}</Alert>}
              {tooMany && <Alert tone="danger">{t('labels.tooMany', { max: MAX_LABELS })}</Alert>}
            </div>
          </Card>

          <Card>
            <div className="flex flex-col gap-4">
              <Select
                label={t('labels.layout')}
                value={layout.id}
                onChange={(e) => setOption('layout', e.target.value)}
                options={LABEL_LAYOUTS.map((l) => ({ value: l.id, label: t(`labels.layouts.${l.id}`) }))}
              />
              {layout.kind === 'sheet' && (
                <TextField
                  label={t('labels.startAt')}
                  hint={t('labels.startAtHint', { max: perPage(layout) })}
                  type="number"
                  min={1}
                  max={perPage(layout)}
                  value={skip + 1}
                  onChange={(e) => setSkip(Math.min(perPage(layout) - 1, Math.max(0, Math.floor(Number(e.target.value) || 1) - 1)))}
                />
              )}
              <fieldset className="flex flex-col gap-3">
                <legend className="mb-2 text-sm font-medium">{t('labels.show')}</legend>
                <Checkbox label={t('labels.showBusiness')} checked={options.showBusiness} onChange={(e) => setOption('showBusiness', e.target.checked)} />
                <Checkbox label={t('labels.showName')} checked={options.showName} onChange={(e) => setOption('showName', e.target.checked)} />
                <Checkbox label={t('labels.showPrice')} checked={options.showPrice} onChange={(e) => setOption('showPrice', e.target.checked)} />
                <Checkbox label={t('labels.showSku')} checked={options.showSku} onChange={(e) => setOption('showSku', e.target.checked)} />
                <Checkbox
                  label={t('labels.skuFallback')}
                  hint={t('labels.skuFallbackHint')}
                  checked={options.skuFallback}
                  onChange={(e) => setOption('skuFallback', e.target.checked)}
                />
              </fieldset>
              <p className="text-sm text-ink-muted">{t(layout.kind === 'roll' ? 'labels.rollHint' : 'labels.sheetHint')}</p>
            </div>
          </Card>
        </div>

        <h2 className="text-lg font-semibold">{t('labels.preview', { count: pages.length })}</h2>
      </div>

      {/* Preview on screen; exactly these pages on paper. */}
      <div className="label-pages flex flex-col items-start gap-4 overflow-x-auto print:block print:gap-0 print:overflow-visible">
        {pages.map((page, i) => (
          <LabelPage key={i} layout={layout} cells={page} options={options} business={me?.tenant?.name ?? ''} lang={lang} />
        ))}
      </div>
    </div>
  )
}

const displayName = (l: LabelInfo, lang: string) => {
  const name = lang === 'bn' && l.product_name_bn ? l.product_name_bn : l.product_name
  return l.variant_name ? `${name} · ${l.variant_name}` : name
}

/** Paper size for the browser's print dialog while this page is open. */
function PageStyle({ layout }: { layout: LabelLayout }) {
  useEffect(() => {
    const style = document.createElement('style')
    style.dataset.labels = 'page'
    style.textContent =
      `@page { size: ${layout.pageWidth}mm ${layout.pageHeight}mm; margin: 0; }` +
      '@media print { html, body { background: #fff !important; } .label-pages > * { break-after: page; } .label-pages > *:last-child { break-after: auto; } }'
    document.head.appendChild(style)
    return () => style.remove()
  }, [layout])
  return null
}

function LabelPage({
  layout,
  cells,
  options,
  business,
  lang,
}: {
  layout: LabelLayout
  cells: (LabelInfo | null)[]
  options: LabelOptions
  business: string
  lang: string
}) {
  const style: CSSProperties = {
    width: `${layout.pageWidth}mm`,
    height: `${layout.pageHeight}mm`,
    paddingTop: `${layout.marginTop}mm`,
    paddingLeft: `${layout.marginLeft}mm`,
    gridTemplateColumns: `repeat(${layout.columns}, ${layout.labelWidth}mm)`,
    gridAutoRows: `${layout.labelHeight}mm`,
    columnGap: `${layout.gapX}mm`,
    rowGap: `${layout.gapY}mm`,
  }
  return (
    <div className="grid shrink-0 overflow-hidden bg-white text-black shadow ring-1 ring-border print:shadow-none print:ring-0" style={style}>
      {cells.map((cell, i) => (cell ? <Label key={i} label={cell} layout={layout} options={options} business={business} lang={lang} /> : <div key={i} />))}
    </div>
  )
}

function Label({ label, layout, options, business, lang }: { label: LabelInfo; layout: LabelLayout; options: LabelOptions; business: string; lang: string }) {
  const code = label.barcode ?? label.sku
  // Text scales with the label: about 9 % of its height, at least 1.8 mm.
  const font = Math.max(1.8, layout.labelHeight * 0.09)
  return (
    <div
      data-testid="label"
      className="flex flex-col overflow-hidden leading-tight"
      style={{ padding: `${Math.min(1.5, layout.labelHeight * 0.06)}mm ${Math.min(2, layout.labelWidth * 0.05)}mm`, fontSize: `${font}mm` }}
    >
      {options.showBusiness && business && <div className="truncate text-center text-[0.8em] uppercase">{business}</div>}
      {options.showName && <div className="line-clamp-2 text-center font-semibold break-words">{displayName(label, lang)}</div>}
      <Barcode value={code} className="mt-[0.5mm] flex-1" />
      <div className="flex items-baseline justify-between gap-1">
        {options.showSku ? <span className="truncate font-mono text-[0.8em]">{label.sku}</span> : <span />}
        {options.showPrice && <span className="font-bold whitespace-nowrap">{formatMoney(label.selling_price, lang)}</span>}
      </div>
    </div>
  )
}
