import { type Dispatch, useEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'

import type { DiscountType } from '../../api/sales'
import { formatDate } from '../../lib/dates'
import { ZERO, dec, toFixed } from '../../lib/decimal'
import { currencySymbol, formatMoney, formatQuantity } from '../../lib/format'
import type { LineResult } from '../../lib/pricing'
import { warrantyExpiresOn, warrantyTenure } from '../sales/warranty'
import type { CartAction, CartLine, LineProblem } from './cart'
import { Icon } from '../../app/layouts/Icon'
import { ProductThumb } from '../products/ProductThumb'

const input =
  'min-h-10 rounded-lg border border-border bg-surface px-2 text-sm tabular-nums outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100 aria-invalid:border-danger'
const stepButton = 'size-10 shrink-0 rounded-lg border border-border bg-surface text-lg font-semibold hover:bg-canvas'

// Narrow: name, total and remove on top; quantity, price and discount below.
// Wide (@4xl): one row with a column per field, under a matching header.
const rowGrid =
  'grid grid-cols-[minmax(0,1fr)_auto_auto] items-start gap-x-2 gap-y-2 [grid-template-areas:"item_total_remove"_"controls_controls_controls"] @4xl:grid-cols-[minmax(9rem,1fr)_9rem_6.5rem_15.5rem_6.5rem_2.25rem] @4xl:gap-x-3 @4xl:[grid-template-areas:"item_qty_price_disc_total_remove"]'
// Field names shown above the inputs only in the narrow layout.
const narrowLabel = 'mb-1 block text-xs text-ink-muted @4xl:hidden'

/** The cart lines: quantity, price, line discount and total. */
export function CartTable({
  lines,
  results,
  problems,
  selected,
  onSelect,
  dispatch,
  canOverridePrice,
  businessDate,
  flash = null,
}: {
  lines: CartLine[]
  /** Priced lines in cart order; `null` while an input is invalid. */
  results: LineResult[] | null
  problems: Record<string, LineProblem>
  selected: string | null
  onSelect: (variantId: string) => void
  dispatch: Dispatch<CartAction>
  canOverridePrice: boolean
  /** Warranties sold now start on this day (`YYYY-MM-DD`). */
  businessDate: string
  /** The line just added; `n` changes on every add so a repeat add flashes again. */
  flash?: { id: string; n: number } | null
}) {
  const { t, i18n } = useTranslation()
  const rows = useRef(new Map<string, HTMLLIElement>())

  // Show the cashier which line a tap or scan changed: bring it into view
  // and flash it once (skipped when reduced motion is preferred).
  useEffect(() => {
    if (!flash || typeof window === 'undefined') return
    const row = rows.current.get(flash.id)
    if (!row) return
    const still = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false
    row.scrollIntoView?.({ block: 'nearest', behavior: still ? 'auto' : 'smooth' })
    if (!still && typeof row.animate === 'function') {
      row.animate([{ backgroundColor: 'var(--color-brand-100)' }, { backgroundColor: 'transparent' }], { duration: 700, easing: 'ease-out' })
    }
  }, [flash])
  const lang = i18n.language

  const totalQty = lines.reduce((sum, l) => sum + (Number(l.quantity) || 0), 0)

  return (
    <section aria-labelledby="pos-cart-heading" className="flex flex-col rounded-xl border border-border bg-surface lg:min-h-0 lg:flex-1">
      <div className="flex items-baseline justify-between gap-3 border-b border-border px-3 py-2.5">
        <h2 id="pos-cart-heading" className="text-sm font-semibold">
          {t('pos.cart')}
        </h2>
        {lines.length > 0 && (
          <p className="text-xs text-ink-muted tabular-nums">
            {t('pos.cartSummary', { count: lines.length, qty: formatQuantity(String(totalQty), lang, !Number.isInteger(totalQty)) })}
          </p>
        )}
      </div>
      {lines.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-2 p-8 text-center">
          <span aria-hidden="true" className="flex size-12 items-center justify-center rounded-full bg-brand-50 text-brand-700">
            <Icon name="pos" className="size-6" />
          </span>
          <p className="max-w-[26ch] text-sm text-ink-muted">{t('pos.emptyCart')}</p>
        </div>
      ) : (
        // A container query picks the layout: table-like columns when the
        // cart is wide, two rows per line when it sits beside the products.
        <div className="@container lg:min-h-0 lg:flex-1 lg:overflow-y-auto">
          <div aria-hidden="true" className={`${rowGrid} sticky top-0 z-10 hidden border-b border-border bg-surface px-3 py-2 text-xs text-ink-muted @4xl:grid`}>
            <span className="flex gap-2 [grid-area:item]">
              <span className="w-5 shrink-0 text-right">{t('pos.sl')}</span>
              {t('pos.item')}
            </span>
            <span className="[grid-area:qty]">{t('common.quantity')}</span>
            <span className="text-right [grid-area:price]">{t('common.price')}</span>
            <span className="[grid-area:disc]">{t('pos.discount')}</span>
            <span className="text-right [grid-area:total]">{t('common.total')}</span>
          </div>
          <ul aria-label={t('pos.cartLines')} className="text-sm">
            {lines.map((line, i) => {
              const { item } = line
              const id = item.variant_id
              const name = lang === 'bn' && item.name_bn ? item.name_bn : item.name
              const problem = problems[id]
              const overStock = item.track_inventory && item.stock !== null && dec(line.quantity) > dec(item.stock)
              const result = results?.[i]
              const tenure = warrantyTenure(item, t)
              const warrantyEnd = warrantyExpiresOn(businessDate, item)
              const isSelected = selected === id
              const priceChanged = dec(line.unitPrice) !== dec(item.selling_price)
              // A % discount also shows what it comes to in money.
              const percentOff = line.discountType === 'percent' && result !== undefined && result.lineDiscount > ZERO
              return (
                <li
                  key={id}
                  ref={(el) => {
                    if (el) rows.current.set(id, el)
                    else rows.current.delete(id)
                  }}
                  data-selected={isSelected || undefined}
                  onClick={() => onSelect(id)}
                  onFocus={() => onSelect(id)}
                  className={`${rowGrid} border-b border-border px-3 py-2.5 last:border-b-0 ${isSelected ? 'bg-brand-50/60 shadow-[inset_3px_0_0_var(--color-brand-600)]' : ''}`}
                >
                  <div className="flex min-w-0 items-start gap-2 [grid-area:item]">
                    {/* Serial number of the line (SL), as on the receipt. */}
                    <span className="mt-2 w-5 shrink-0 text-right text-xs font-semibold text-ink-muted tabular-nums">
                      <span className="sr-only">{t('pos.sl')} </span>
                      {i + 1}
                    </span>
                    <ProductThumb fileId={item.image_file_id} />
                    <div className="min-w-0">
                      <span className="block leading-snug font-medium">{name}</span>
                      <span className="block text-xs text-ink-muted">
                        {item.sku}
                        {item.stock !== null &&
                          ` · ${t('pos.stock', { qty: formatQuantity(item.stock, lang, item.allow_fraction), unit: item.unit_short_name })}`}
                        {/* A changed price shows the regular one here, so the price input keeps its height. */}
                        {priceChanged && (
                          <>
                            {' · '}
                            <span className="sr-only">{t('pos.regularPrice')} </span>
                            <s className="tabular-nums">{formatMoney(item.selling_price, lang)}</s>
                          </>
                        )}
                      </span>
                      {tenure && warrantyEnd && (
                        <span className="mt-0.5 block text-xs text-brand-700">{t('pos.warranty', { tenure, date: formatDate(warrantyEnd, lang) })}</span>
                      )}
                      {overStock && <span className="mt-0.5 block text-xs font-medium text-warning">{t('pos.overStock')}</span>}
                      {problem && (
                        <span role="alert" className="mt-0.5 block text-xs text-danger">
                          {t(`pos.errors.${problem}`)}
                        </span>
                      )}
                    </div>
                  </div>

                  <div className="flex flex-wrap items-end gap-x-3 gap-y-2 [grid-area:controls] @4xl:contents">
                    <div className="[grid-area:qty]">
                      <span aria-hidden="true" className={narrowLabel}>{t('common.quantity')}</span>
                      <div className="flex items-center gap-1">
                        <button type="button" className={stepButton} aria-label={t('pos.decrease', { name })} onClick={() => dispatch({ type: 'step', variantId: id, delta: -1 })}>
                          −
                        </button>
                        <input
                          className={`${input} w-14 text-center`}
                          inputMode={item.allow_fraction ? 'decimal' : 'numeric'}
                          aria-label={t('pos.quantityFor', { name })}
                          aria-invalid={problem === 'quantity' || problem === 'wholeNumber' ? true : undefined}
                          value={line.quantity}
                          onChange={(e) => dispatch({ type: 'quantity', variantId: id, quantity: e.target.value })}
                        />
                        <button type="button" className={stepButton} aria-label={t('pos.increase', { name })} onClick={() => dispatch({ type: 'step', variantId: id, delta: 1 })}>
                          +
                        </button>
                      </div>
                    </div>

                    <div className="@4xl:text-right [grid-area:price]">
                      <span aria-hidden="true" className={narrowLabel}>{t('common.price')}</span>
                      {canOverridePrice ? (
                        <input
                          className={`${input} w-24 text-right`}
                          inputMode="decimal"
                          aria-label={t('pos.priceFor', { name })}
                          aria-invalid={problem === 'price' ? true : undefined}
                          value={line.unitPrice}
                          onChange={(e) => dispatch({ type: 'price', variantId: id, price: e.target.value })}
                        />
                      ) : (
                        <span className="flex min-h-10 items-center tabular-nums @4xl:justify-end">{formatMoney(line.unitPrice, lang)}</span>
                      )}
                    </div>

                    <div className="[grid-area:disc]">
                      <span aria-hidden="true" className={narrowLabel}>{t('pos.discount')}</span>
                      <div className="flex items-center gap-1">
                        <input
                          className={`${input} w-16 text-right`}
                          inputMode="decimal"
                          placeholder="0"
                          aria-label={t('pos.discountFor', { name })}
                          aria-describedby={percentOff ? `off-${id}` : undefined}
                          aria-invalid={problem?.startsWith('discount') ? true : undefined}
                          value={line.discountValue}
                          onChange={(e) => dispatch({ type: 'discount', variantId: id, discountType: line.discountType, value: e.target.value })}
                        />
                        <select
                          className={input}
                          aria-label={t('pos.discountTypeFor', { name })}
                          value={line.discountType}
                          onChange={(e) => dispatch({ type: 'discount', variantId: id, discountType: e.target.value as DiscountType, value: line.discountValue })}
                        >
                          <option value="amount">{currencySymbol()}</option>
                          <option value="percent">{t('pos.discountPercent')}</option>
                        </select>
                        {percentOff && result && (
                          <span id={`off-${id}`} className="pl-1 text-xs whitespace-nowrap text-brand-700 tabular-nums">
                            {t('pos.percentOff', { pct: line.discountValue.trim(), amount: formatMoney(toFixed(result.lineDiscount, 2), lang) })}
                          </span>
                        )}
                      </div>
                    </div>
                  </div>

                  <span className="text-right text-base font-bold tabular-nums whitespace-nowrap [grid-area:total] @4xl:pt-2">
                    {result ? formatMoney(toFixed(result.lineTotal, 2), lang) : '—'}
                  </span>

                  <button
                    type="button"
                    className="-mt-1 -mr-1 size-9 justify-self-end rounded-lg text-ink-muted [grid-area:remove] hover:bg-danger/10 hover:text-danger @4xl:mt-0.5"
                    aria-label={t('pos.removeLine', { name })}
                    onClick={(e) => {
                      e.stopPropagation()
                      dispatch({ type: 'remove', variantId: id })
                    }}
                  >
                    ✕
                  </button>
                </li>
              )
            })}
          </ul>
        </div>
      )}
    </section>
  )
}
