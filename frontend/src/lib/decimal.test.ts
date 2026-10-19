import { describe, expect, it } from 'vitest'

import { dec, div, mul, round, toFixed, toPlain, tryParse } from './decimal'

describe('decimal', () => {
  it('parses and formats without floating point', () => {
    expect(toFixed(dec('0.1') + dec('0.2'))).toBe('0.30')
    expect(toFixed(dec('1.005'), 2)).toBe('1.01')
    expect(toFixed(dec('-1.005'), 2)).toBe('-1.01')
    expect(toPlain(dec('2.500'))).toBe('2.5')
    expect(toPlain(dec('3'))).toBe('3')
    expect(tryParse('abc')).toBeNull()
    expect(tryParse('')).toBeNull()
    expect(tryParse('.5')).toBe(dec('0.5'))
  })

  it('multiplies, divides and rounds half up', () => {
    expect(toFixed(mul(dec('3'), dec('33.33')))).toBe('99.99')
    expect(toFixed(div(dec('10'), dec('3')), 4)).toBe('3.3333')
    expect(toFixed(round(dec('25.875'), 2))).toBe('25.88')
  })
})
