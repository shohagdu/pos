import { describe, expect, it } from 'vitest'

import { dec, toFixed } from './decimal'
import { type Discount, PricingError, calculate } from './pricing'
import vectors from './pricing.vectors.json'

interface RawDiscount {
  type: string
  value: string
}

const discount = (d?: RawDiscount): Discount | null =>
  d ? { type: d.type === 'percent' ? 'percent' : 'amount', value: dec(d.value) } : null

// The same vectors drive the Rust engine (crates/sales/src/pricing.rs).
describe('pricing engine (shared vectors)', () => {
  for (const v of vectors) {
    it(v.name, () => {
      const input = v.input as {
        lines: { quantity: string; unit_price: string; tax_rate_pct: string; discount?: RawDiscount }[]
        invoice_discount?: RawDiscount
        shipping?: string
        other?: string
        prices_include_tax: boolean
        cash_rounding: string
      }
      const run = () =>
        calculate({
          lines: input.lines.map((l) => ({
            quantity: dec(l.quantity),
            unitPrice: dec(l.unit_price),
            taxRatePct: dec(l.tax_rate_pct),
            discount: discount(l.discount),
          })),
          invoiceDiscount: discount(input.invoice_discount),
          shipping: dec(input.shipping ?? '0'),
          other: dec(input.other ?? '0'),
          pricesIncludeTax: input.prices_include_tax,
          cashRounding: dec(input.cash_rounding),
        })
      const expected = v.expected as Record<string, unknown>
      if (typeof expected.error === 'string') {
        expect(run).toThrowError(PricingError)
        try {
          run()
        } catch (err) {
          expect((err as PricingError).code).toBe(expected.error)
        }
        return
      }
      const t = run()
      expect(toFixed(t.subtotal)).toBe(expected.subtotal)
      expect(toFixed(t.itemDiscountTotal)).toBe(expected.item_discount_total)
      expect(toFixed(t.invoiceDiscount)).toBe(expected.invoice_discount)
      expect(toFixed(t.taxTotal)).toBe(expected.tax_total)
      expect(toFixed(t.roundingAdjustment)).toBe(expected.rounding_adjustment)
      expect(toFixed(t.grandTotal)).toBe(expected.grand_total)
      const lines = expected.lines as { invoice_discount: string; tax: string; line_total: string }[]
      lines.forEach((line, i) => {
        expect(toFixed(t.lines[i]!.invoiceDiscount)).toBe(line.invoice_discount)
        expect(toFixed(t.lines[i]!.tax)).toBe(line.tax)
        expect(toFixed(t.lines[i]!.lineTotal)).toBe(line.line_total)
      })
    })
  }
})
