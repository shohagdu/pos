import type { TFunction } from 'i18next'

import type { LivePromotion } from '../../api/promotions'
import type { PosItem } from '../../api/sales'
import { dec, toPlain } from '../../lib/decimal'
import { formatMoney } from '../../lib/format'
import { type Promotion, matches, scheduleOpen } from '../../lib/promotions'
import { engineLine } from './cart'

/** Short text for an offer: "-20%", "Flash -30%", "Buy 1 Get 1", "Now ৳400". */
export function promotionLabel(p: Promotion, t: TFunction, lang: string): string {
  const v = toPlain(dec(p.discount_value))
  if (p.kind === 'bxgy' && p.bxgy) {
    const buy = toPlain(dec(p.bxgy.buy_quantity))
    const get = toPlain(dec(p.bxgy.get_quantity))
    return p.bxgy.get_discount_type === 'percent'
      ? t('pos.offer.bxgyPercent', { buy, get, pct: toPlain(dec(p.bxgy.get_discount_value ?? '0')) })
      : t('pos.offer.bxgy', { buy, get })
  }
  if (p.kind === 'coupon') return p.coupon_code ?? p.name
  const off =
    p.discount_type === 'percent'
      ? t('pos.offer.percent', { pct: v })
      : p.discount_type === 'fixed_amount'
        ? t('pos.offer.amount', { amount: formatMoney(p.discount_value, lang) })
        : t('pos.offer.fixedPrice', { price: formatMoney(p.discount_value, lang) })
  return p.kind === 'flash' ? t('pos.offer.flash', { off }) : off
}

/** The offer a product card shows: the live promotion that matches it, highest priority first. */
export function cardOffer(item: PosItem, live: LivePromotion[], now: string | null): LivePromotion | null {
  const line = engineLine({ item, quantity: '1', unitPrice: item.selling_price, discountType: 'amount', discountValue: '' }, dec(1), dec(item.selling_price))
  const open = live.filter((p) => p.kind !== 'coupon' && (now === null || scheduleOpen(p, now)) && matches(p.targets, line))
  open.sort((a, b) => (b.priority ?? 0) - (a.priority ?? 0))
  return open[0] ?? null
}

/** Time left until `end` (both business local `YYYY-MM-DDTHH:MM:SS`): "1h 24m", "12m", "<1m"; null when past. */
export function countdown(end: string | null | undefined, now: string | null): string | null {
  if (!end || !now) return null
  const ms = Date.parse(`${end.slice(0, 19)}Z`) - Date.parse(`${now.slice(0, 19)}Z`)
  if (!(ms > 0)) return null
  const minutes = Math.floor(ms / 60_000)
  const h = Math.floor(minutes / 60)
  const m = minutes % 60
  if (h > 0) return `${h}h ${m}m`
  return m > 0 ? `${m}m` : '<1m'
}
