import { describe, expect, it } from 'vitest'

import { warrantyExpiresOn } from './warranty'

// Same cases as pos_catalog::warranty::tests::expiry_dates.
describe('warrantyExpiresOn', () => {
  it.each([
    ['2026-11-06', 1, 'month', '2026-12-06'],
    ['2026-11-06', 3, 'month', '2027-02-06'],
    ['2027-01-31', 1, 'month', '2027-02-28'],
    ['2026-11-06', 7, 'day', '2026-11-13'],
    ['2026-12-28', 7, 'day', '2027-01-04'],
    ['2028-02-29', 1, 'year', '2029-02-28'],
  ] as const)('%s + %i %s = %s', (from, period, unit, expected) => {
    expect(warrantyExpiresOn(from, { warranty_period: period, warranty_unit: unit })).toBe(expected)
  })

  it('is null without a warranty', () => {
    expect(warrantyExpiresOn('2026-11-06', {})).toBeNull()
  })
})
