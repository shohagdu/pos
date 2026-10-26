import type { RefObject } from 'react'
import { useTranslation } from 'react-i18next'

import type { DiscountType } from '../../api/sales'
import { type Dec, ZERO, toFixed } from '../../lib/decimal'
import { currencySymbol, formatMoney } from '../../lib/format'
import type { Totals } from '../../lib/pricing'
import type { LineProblem } from './cart'

const input =
  'min-h-10 rounded-lg border border-border bg-surface px-2 text-sm tabular-nums outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100 aria-invalid:border-danger'

/** Invoice discount (F7) and the totals from the pricing engine. */
export function TotalsPanel({
  totals,
  pricesIncludeTax,
  discountType,
  discountValue,
  discountProblem,
  onDiscount,
  discountRef,
}: {
  totals: Totals | null
  pricesIncludeTax: boolean
  discountType: DiscountType
  discountValue: string
  discountProblem: LineProblem | null
  onDiscount: (type: DiscountType, value: string) => void
  discountRef: RefObject<HTMLInputElement | null>
}) {
  const { t, i18n } = useTranslation()
  const money = (v: Dec) => formatMoney(toFixed(v, 2), i18n.language)
  const rows: { key: string; label: string; value: Dec; negative?: boolean; always?: boolean }[] = totals
    ? [
        { key: 'subtotal', label: t('pos.subtotal'), value: totals.subtotal, always: true },
        { key: 'itemDiscount', label: t('pos.itemDiscounts'), value: totals.itemDiscountTotal, negative: true },
        { key: 'invoiceDiscount', label: t('pos.invoiceDiscount'), value: totals.invoiceDiscount, negative: true },
        { key: 'vat', label: pricesIncludeTax ? t('pos.vatIncluded') : t('pos.vat'), value: totals.taxTotal, always: true },
        { key: 'rounding', label: t('pos.rounding'), value: totals.roundingAdjustment },
      ]
    : []

  return (
    <div className="flex flex-col gap-3">
      <div>
        <label htmlFor="pos-invoice-discount" className="text-sm font-medium">
          {t('pos.invoiceDiscount')} <kbd className="text-xs text-ink-muted">F7</kbd>
        </label>
        <div className="mt-1 flex gap-1">
          <input
            ref={discountRef}
            id="pos-invoice-discount"
            className={`${input} min-w-0 flex-1 text-right`}
            inputMode="decimal"
            placeholder="0"
            aria-invalid={discountProblem ? true : undefined}
            value={discountValue}
            onChange={(e) => onDiscount(discountType, e.target.value)}
          />
          <select
            className={input}
            aria-label={t('pos.invoiceDiscountType')}
            value={discountType}
            onChange={(e) => onDiscount(e.target.value as DiscountType, discountValue)}
          >
            <option value="amount">{currencySymbol()}</option>
            <option value="percent">{t('pos.discountPercent')}</option>
          </select>
        </div>
        {discountProblem && (
          <p role="alert" className="mt-1 text-xs text-danger">
            {t(`pos.errors.${discountProblem}`)}
          </p>
        )}
      </div>
      <dl className="flex flex-col gap-1 text-sm">
        {rows
          .filter((r) => r.always || r.value !== ZERO)
          .map((r) => (
            <div key={r.key} className="flex justify-between gap-3">
              <dt className="text-ink-muted">{r.label}</dt>
              <dd className="tabular-nums">
                {r.negative ? '−' : ''}
                {money(r.value)}
              </dd>
            </div>
          ))}
        <div className="mt-1 flex items-baseline justify-between gap-3 border-t border-border pt-2">
          <dt className="text-base font-semibold">{t('pos.grandTotal')}</dt>
          <dd data-testid="grand-total" className="text-2xl font-bold text-brand-700 tabular-nums">
            {totals ? money(totals.grandTotal) : '—'}
          </dd>
        </div>
      </dl>
    </div>
  )
}
