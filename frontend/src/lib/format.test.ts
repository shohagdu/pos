import { afterEach, describe, expect, it } from 'vitest'

import { currencySymbol, formatMoney, formatQuantity, setBusinessCurrency } from './format'

describe('formatMoney', () => {
  afterEach(() => setBusinessCurrency('BDT'))

  it('uses lakh/crore grouping in English', () => {
    expect(formatMoney('1234567.5')).toBe('৳12,34,567.50')
  })

  it('uses Bengali digits in Bengali', () => {
    expect(formatMoney('1234567.5', 'bn')).toBe('১২,৩৪,৫৬৭.৫০৳')
  })

  it("uses the business's currency, with thousands grouping outside South Asia", () => {
    setBusinessCurrency('EUR')
    expect(formatMoney('1234567.5')).toBe('€1,234,567.50')
    expect(currencySymbol()).toBe('€')
    setBusinessCurrency('INR')
    expect(formatMoney('1234567.5')).toBe('₹12,34,567.50')
  })

  it('keeps precision for large values passed as strings', () => {
    expect(formatMoney('9007199254740993.10')).toBe('৳9,00,71,99,25,47,40,993.10')
  })
})

describe('formatQuantity', () => {
  it('hides fractions for whole-unit products', () => {
    expect(formatQuantity('3')).toBe('3')
  })

  it('shows up to three decimals for weighed products', () => {
    expect(formatQuantity('1.25', 'en', true)).toBe('1.25')
  })
})
