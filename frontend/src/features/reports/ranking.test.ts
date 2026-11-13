import { describe, expect, it } from 'vitest'

import type { ProductSalesRow } from '../../api/reports'
import { rankProducts } from './ranking'

const p = (id: string, quantity: string, net: string, profit: string | null): ProductSalesRow => ({
  variant_id: id,
  product_name: id,
  sku: id.toUpperCase(),
  category_name: null,
  unit_short_name: 'pcs',
  quantity,
  net_sales: net,
  cogs: profit === null ? null : '0.00',
  gross_profit: profit,
  margin_pct: null,
})

// As the API returns them: by net sales.
const rows = [p('rice', '10.000', '4500.00', '500.00'), p('oil', '30.000', '5700.00', '540.00'), p('salt', '30.000', '1260.00', '210.00'), p('tea', '0.000', '0.00', '-20.00')]

describe('rankProducts', () => {
  it('ranks by quantity, keeping the report order on ties, and leaves out zero', () => {
    const ranked = rankProducts(rows, 'quantity', 10)
    expect(ranked.map((r) => [r.rank, r.row.variant_id, r.share])).toEqual([
      [1, 'oil', 100],
      [2, 'salt', 100],
      [3, 'rice', 33],
    ])
  })

  it('ranks by net sales and by gross profit, never listing a loss', () => {
    expect(rankProducts(rows, 'net_sales', 10).map((r) => r.row.variant_id)).toEqual(['oil', 'rice', 'salt'])
    expect(rankProducts(rows, 'gross_profit', 10).map((r) => r.row.variant_id)).toEqual(['oil', 'rice', 'salt'])
  })

  it('takes only the top N, and nothing when cost is hidden', () => {
    expect(rankProducts(rows, 'net_sales', 2).map((r) => r.row.variant_id)).toEqual(['oil', 'rice'])
    expect(rankProducts(rows.map((r) => ({ ...r, gross_profit: null })), 'gross_profit', 10)).toEqual([])
  })
})
