import { describe, expect, it } from 'vitest'

import type { SaleItem } from '../../api/sales'
import { dec, toFixed } from '../../lib/decimal'
import { sale } from './fixtures'
import { estimatedRefund } from './returnMath'

const base = sale.items[0]!
// Same cases as the server's `buy_one_get_one_returns` test.
const bogo = (quantity: string, free: string, lineTotal: string, returned = '0'): SaleItem => ({
  ...base,
  quantity,
  returned_quantity: returned,
  free_quantity: free,
  line_total: lineTotal,
  discounts: [
    {
      promotion_id: 'p',
      coupon_code: null,
      promotion_type: 'bxgy',
      name: 'BOGO',
      name_bn: null,
      discount_type: 'free',
      discount_value: '0',
      amount: '0',
      free_quantity: free,
      bxgy_buy_quantity: '1',
      bxgy_get_quantity: '1',
      bxgy_get_target_type: 'same_product',
    },
  ],
})

describe('return estimate with Buy 1 Get 1', () => {
  it('returning the paid unit and keeping the free one refunds nothing', () => {
    expect(toFixed(estimatedRefund(bogo('2', '1', '100'), dec(1)))).toBe('0.00')
  })
  it('returning both refunds the line', () => {
    expect(toFixed(estimatedRefund(bogo('2', '1', '100'), dec(2)))).toBe('100.00')
  })
  it('keeping two of four keeps one free', () => {
    expect(toFixed(estimatedRefund(bogo('4', '2', '200'), dec(2)))).toBe('100.00')
    expect(toFixed(estimatedRefund(bogo('4', '2', '200'), dec(1)))).toBe('0.00')
  })
  it('lines without free units stay pro rata', () => {
    expect(toFixed(estimatedRefund({ ...base, quantity: '2', line_total: '901', returned_quantity: '0' }, dec(1)))).toBe('450.50')
  })
})
