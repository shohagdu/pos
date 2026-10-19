import type { Dispatch } from 'react'
import { useTranslation } from 'react-i18next'

import type { DiscountType } from '../../api/sales'
import { dec, toFixed } from '../../lib/decimal'
import { formatMoney, formatQuantity } from '../../lib/format'
import type { LineResult } from '../../lib/pricing'
import type { CartAction, CartLine, LineProblem } from './cart'

const input =
  'min-h-10 rounded-lg border border-border bg-surface px-2 text-sm tabular-nums outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100 aria-invalid:border-danger'
const stepButton = 'size-10 shrink-0 rounded-lg border border-border bg-surface text-lg font-semibold hover:bg-canvas'

/** The cart lines: quantity, price, line discount and total. */
export function CartTable({
  lines,
  results,
  problems,
  selected,
  onSelect,
  dispatch,
  canOverridePrice,
}: {
  lines: CartLine[]
  /** Priced lines in cart order; `null` while an input is invalid. */
  results: LineResult[] | null
  problems: Record<string, LineProblem>
  selected: string | null
  onSelect: (variantId: string) => void
  dispatch: Dispatch<CartAction>
  canOverridePrice: boolean
}) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language

  if (lines.length === 0) {
    return (
      <div className="flex flex-1 items-center justify-center rounded-xl border border-dashed border-border p-8 text-center text-sm text-ink-muted">
        {t('pos.emptyCart')}
      </div>
    )
  }

  return (
    <div className="overflow-x-auto rounded-xl border border-border bg-surface">
      <table className="w-full text-sm">
        <caption className="sr-only">{t('pos.cart')}</caption>
        <thead className="border-b border-border bg-canvas text-xs text-ink-muted uppercase">
          <tr>
            <th scope="col" className="px-3 py-2 text-left">{t('pos.item')}</th>
            <th scope="col" className="px-3 py-2 text-left">{t('common.quantity')}</th>
            <th scope="col" className="px-3 py-2 text-right">{t('common.price')}</th>
            <th scope="col" className="px-3 py-2 text-left">{t('pos.discount')}</th>
            <th scope="col" className="px-3 py-2 text-right">{t('common.total')}</th>
            <th scope="col" className="px-1 py-2">
              <span className="sr-only">{t('common.actions')}</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {lines.map((line, i) => {
            const { item } = line
            const id = item.variant_id
            const name = lang === 'bn' && item.name_bn ? item.name_bn : item.name
            const problem = problems[id]
            const overStock = item.track_inventory && item.stock !== null && dec(line.quantity) > dec(item.stock)
            const result = results?.[i]
            return (
              <tr
                key={id}
                data-selected={selected === id || undefined}
                onClick={() => onSelect(id)}
                onFocus={() => onSelect(id)}
                className={`border-b border-border align-top last:border-b-0 ${selected === id ? 'bg-brand-50/60' : ''}`}
              >
                <td className="min-w-40 px-3 py-2">
                  <span className="block font-medium">{name}</span>
                  <span className="block text-xs text-ink-muted">
                    {item.sku}
                    {item.stock !== null &&
                      ` · ${t('pos.stock', { qty: formatQuantity(item.stock, lang, item.allow_fraction), unit: item.unit_short_name })}`}
                  </span>
                  {overStock && <span className="mt-0.5 block text-xs font-medium text-warning">{t('pos.overStock')}</span>}
                  {problem && (
                    <span role="alert" className="mt-0.5 block text-xs text-danger">
                      {t(`pos.errors.${problem}`)}
                    </span>
                  )}
                </td>
                <td className="px-3 py-2">
                  <div className="flex items-center gap-1">
                    <button type="button" className={stepButton} aria-label={t('pos.decrease', { name })} onClick={() => dispatch({ type: 'step', variantId: id, delta: -1 })}>
                      −
                    </button>
                    <input
                      className={`${input} w-16 text-center`}
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
                  <span className="mt-0.5 block text-xs text-ink-muted">{item.unit_short_name}</span>
                </td>
                <td className="px-3 py-2 text-right">
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
                    <span className="tabular-nums">{formatMoney(line.unitPrice, lang)}</span>
                  )}
                  {dec(line.unitPrice) !== dec(item.selling_price) && (
                    <span className="block text-xs text-ink-muted line-through">{formatMoney(item.selling_price, lang)}</span>
                  )}
                </td>
                <td className="px-3 py-2">
                  <div className="flex items-center gap-1">
                    <input
                      className={`${input} w-20 text-right`}
                      inputMode="decimal"
                      placeholder="0"
                      aria-label={t('pos.discountFor', { name })}
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
                      <option value="amount">{t('pos.discountAmount')}</option>
                      <option value="percent">{t('pos.discountPercent')}</option>
                    </select>
                  </div>
                </td>
                <td className="px-3 py-2 text-right font-semibold tabular-nums whitespace-nowrap">
                  {result ? formatMoney(toFixed(result.lineTotal, 2), lang) : '—'}
                </td>
                <td className="px-1 py-2 text-center">
                  <button
                    type="button"
                    className="rounded-lg p-2 text-ink-muted hover:bg-danger/10 hover:text-danger"
                    aria-label={t('pos.removeLine', { name })}
                    onClick={(e) => {
                      e.stopPropagation()
                      dispatch({ type: 'remove', variantId: id })
                    }}
                  >
                    ✕
                  </button>
                </td>
              </tr>
            )
          })}
        </tbody>
      </table>
    </div>
  )
}
