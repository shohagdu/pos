import { describe, expect, it } from 'vitest'

import { dec, toFixed, toPlain } from './decimal'
import { type EngineLine, type Promotion, applyPromotions, scheduleOpen } from './promotions'
import vectors from './promotions.vectors.json'

interface RawLine {
  variant_id: string
  product_id: string
  category_ids?: string[]
  brand_id?: string | null
  quantity: string
  unit_price: string
  min_unit_price?: string | null
  promotions_off?: boolean
}

interface Expected {
  lines: { amount: string; free_quantity: string; promotions: string[]; flash_partial: boolean; capped_by_min_price: boolean }[]
  promotion_total: string
  coupon_total: string
  coupon: string | null
  offers: { promotion_id: string; target_id: string; quantity: string }[]
}

const line = (l: RawLine): EngineLine => ({
  variantId: l.variant_id,
  productId: l.product_id,
  categoryIds: l.category_ids ?? [],
  brandId: l.brand_id ?? null,
  quantity: dec(l.quantity),
  unitPrice: dec(l.unit_price),
  minUnitPrice: l.min_unit_price ? dec(l.min_unit_price) : null,
  promotionsOff: !!l.promotions_off,
})

// The same vectors drive the Rust engine (crates/sales/src/promotions/engine.rs).
describe('promotion engine (shared vectors)', () => {
  for (const v of vectors) {
    it(v.name, () => {
      const out = applyPromotions((v.lines as RawLine[]).map(line), v.promotions as unknown as Promotion[], (v.coupon as unknown as Promotion | null) ?? null, {
        promotionsStack: v.settings.promotions_stack,
        couponStacks: v.settings.coupon_stacks,
      })
      const e = v.expected as unknown as Expected
      expect(toFixed(out.promotionTotal)).toBe(toFixed(dec(e.promotion_total)))
      expect(toFixed(out.couponTotal)).toBe(toFixed(dec(e.coupon_total)))
      expect(out.coupon).toBe(e.coupon)
      e.lines.forEach((el, i) => {
        const l = out.lines[i]!
        expect(toFixed(l.amount), `line ${i} amount`).toBe(toFixed(dec(el.amount)))
        expect(toPlain(l.freeQuantity), `line ${i} free`).toBe(toPlain(dec(el.free_quantity)))
        expect(l.flashPartial, `line ${i} flash`).toBe(el.flash_partial)
        expect(l.cappedByMinPrice, `line ${i} capped`).toBe(el.capped_by_min_price)
        expect(l.discounts.map((d) => d.promotion.id)).toEqual(el.promotions)
      })
      expect(out.offers.map((o) => [o.promotion.id, o.targetId, toPlain(o.quantity)])).toEqual(
        e.offers.map((o) => [o.promotion_id, o.target_id, toPlain(dec(o.quantity))]),
      )
    })
  }
})

// Same cases as the server's `schedule_windows` test (2026-11-13 is a Friday).
describe('promotion schedule', () => {
  const at = (hm: string) => `2026-11-13T${hm}:00`
  it('happy hour', () => {
    const s = { active_time_from: '17:00:00', active_time_to: '19:00:00' }
    expect(scheduleOpen(s, at('17:00')) && scheduleOpen(s, at('18:59'))).toBe(true)
    expect(scheduleOpen(s, at('19:00')) || scheduleOpen(s, at('16:59'))).toBe(false)
  })
  it('a window over midnight', () => {
    const s = { active_time_from: '22:00:00', active_time_to: '02:00:00' }
    expect(scheduleOpen(s, at('23:00')) && scheduleOpen(s, at('01:00'))).toBe(true)
    expect(scheduleOpen(s, at('12:00'))).toBe(false)
  })
  it('days of the week', () => {
    expect(scheduleOpen({ active_days: ['fri', 'sat'] }, at('10:00'))).toBe(true)
    expect(scheduleOpen({ active_days: ['mon'] }, at('10:00'))).toBe(false)
  })
  it('start and end dates', () => {
    const s = { starts_at: at('10:00'), ends_at: at('12:00') }
    expect(scheduleOpen(s, at('09:59'))).toBe(false)
    expect(scheduleOpen(s, at('10:00'))).toBe(true)
    expect(scheduleOpen(s, at('12:00'))).toBe(false)
  })
})
