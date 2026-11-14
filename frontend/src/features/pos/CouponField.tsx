import { useMutation } from '@tanstack/react-query'
import { type RefObject, useId, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { promotionsApi } from '../../api/promotions'
import { Button } from '../../components/ui/Button'
import { type Dec, ZERO, toFixed } from '../../lib/decimal'
import { formatMoney } from '../../lib/format'
import { fieldErrors, problemMessage } from '../../lib/problem'
import type { CouponOutcome, Promotion } from '../../lib/promotions'

/**
 * Coupon code under the invoice discount (§6): Apply checks the code with the
 * server (branch, time, limits, customer); whether it fits the cart (minimum
 * purchase, qualifying items, better than the automatic offers) comes from
 * the engine and is shown here. F6 focuses the field.
 */
export function CouponField({
  coupon,
  outcome,
  saved,
  customerId,
  onApply,
  onRemove,
  inputRef,
}: {
  coupon: Promotion | null
  outcome: CouponOutcome | null
  /** What the coupon takes off this cart. */
  saved: Dec
  customerId: string | null
  onApply: (coupon: Promotion) => void
  onRemove: () => void
  inputRef?: RefObject<HTMLInputElement | null>
}) {
  const { t, i18n } = useTranslation()
  const id = useId()
  const [code, setCode] = useState('')
  const check = useMutation({
    mutationFn: (c: string) => promotionsApi.validateCoupon(c, customerId),
    onSuccess: (c) => {
      setCode('')
      onApply(c)
    },
  })
  const error = check.error ? (fieldErrors(check.error, t).coupon_code ?? problemMessage(check.error, t)) : null
  const money = (v: Dec) => formatMoney(toFixed(v, 2), i18n.language)

  if (coupon) {
    const why: Record<Exclude<CouponOutcome, 'applied'>, string> = {
      not_better: t('pos.coupon.notBetter'),
      min_purchase: t('pos.coupon.minPurchase', { amount: formatMoney(coupon.min_purchase_amount ?? '0', i18n.language) }),
      no_qualifying_items: t('pos.coupon.noItems'),
    }
    return (
      <div className="flex flex-col gap-1">
        <span className="text-sm font-medium">{t('pos.coupon.label')}</span>
        <div
          className={`flex items-center justify-between gap-2 rounded-lg border px-3 py-1.5 ${
            outcome === 'applied' ? 'border-brand-500 bg-brand-50' : 'border-warning bg-warning/10'
          }`}
        >
          <span className="min-w-0">
            <span className="block font-mono text-sm font-semibold tracking-wide">{coupon.coupon_code}</span>
            <span className={`block text-xs ${outcome === 'applied' ? 'text-brand-700' : 'text-ink'}`} role="status">
              {outcome === 'applied' ? t('pos.coupon.saved', { amount: money(saved > ZERO ? saved : ZERO) }) : outcome ? why[outcome] : ''}
            </span>
          </span>
          <Button variant="ghost" className="min-h-11 shrink-0 text-sm lg:min-h-9" onClick={onRemove}>
            {t('pos.coupon.remove')}
          </Button>
        </div>
      </div>
    )
  }

  return (
    <form
      className="flex flex-col gap-1"
      onSubmit={(e) => {
        e.preventDefault()
        if (code.trim()) check.mutate(code.trim())
      }}
    >
      <label htmlFor={id} className="text-sm font-medium">
        {t('pos.coupon.label')} <kbd className="hidden text-xs text-ink-muted lg:inline">F6</kbd>
      </label>
      <div className="flex gap-1">
        <input
          ref={inputRef}
          id={id}
          value={code}
          onChange={(e) => {
            setCode(e.target.value.toUpperCase())
            if (check.error) check.reset()
          }}
          autoComplete="off"
          spellCheck={false}
          placeholder={t('pos.coupon.placeholder')}
          aria-invalid={error ? true : undefined}
          aria-describedby={error ? `${id}-error` : undefined}
          className="min-h-11 min-w-0 flex-1 rounded-lg border border-border bg-surface px-3 font-mono text-sm tracking-wide uppercase outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100 aria-invalid:border-danger lg:min-h-10"
        />
        <Button type="submit" variant="secondary" className="min-h-11 shrink-0 lg:min-h-10" disabled={!code.trim() || check.isPending}>
          {check.isPending ? t('common.loading') : t('pos.coupon.apply')}
        </Button>
      </div>
      {error && (
        <p id={`${id}-error`} role="alert" className="text-xs text-danger">
          {error}
        </p>
      )}
    </form>
  )
}
