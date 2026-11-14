import type { PromotionSummary } from '../../api/promotions'
import { dec, toPlain } from '../../lib/decimal'
import { formatMoney } from '../../lib/format'

/** "20% off", "৳50 off each", "Sells at ৳400", "Buy 1 Get 1", "10% off the order". */
export function benefitText(
  p: Pick<PromotionSummary, 'type' | 'discount_type' | 'discount_value'> & {
    bxgy?: { buy_quantity: string; get_quantity: string; get_discount_type: string; get_discount_value?: string | null } | null
  },
  t: (key: string, opts?: Record<string, unknown>) => string,
  lang: string,
): string {
  if (p.type === 'bxgy') {
    if (!p.bxgy) return t('promotions.types.bxgy')
    const buy = toPlain(dec(p.bxgy.buy_quantity))
    const get = toPlain(dec(p.bxgy.get_quantity))
    return p.bxgy.get_discount_type === 'percent'
      ? t('promotions.benefit.bxgyPercent', { buy, get, pct: toPlain(dec(p.bxgy.get_discount_value ?? '0')) })
      : t('promotions.benefit.bxgy', { buy, get })
  }
  const v = toPlain(dec(p.discount_value))
  const money = formatMoney(p.discount_value, lang)
  if (p.discount_type === 'percent') return t(p.type === 'coupon' ? 'promotions.benefit.percentOrder' : 'promotions.benefit.percent', { pct: v })
  if (p.discount_type === 'fixed_amount') return t(p.type === 'coupon' ? 'promotions.benefit.amountOrder' : 'promotions.benefit.amount', { amount: money })
  return t('promotions.benefit.fixedPrice', { price: money })
}

/** Business local `YYYY-MM-DDTHH:MM:SS` → "14 Nov 2026, 17:00". */
export function localDateTime(value: string | null, lang: string): string {
  if (!value) return ''
  const [d, time = ''] = value.split('T')
  const [y, m, day] = (d ?? '').split('-').map(Number)
  const date = new Date(Date.UTC(y!, m! - 1, day!))
  const formatted = new Intl.DateTimeFormat(lang === 'bn' ? 'bn-BD' : lang === 'it' ? 'it-IT' : 'en-GB', {
    day: 'numeric',
    month: 'short',
    year: 'numeric',
    timeZone: 'UTC',
  }).format(date)
  return time ? `${formatted}, ${time.slice(0, 5)}` : formatted
}
