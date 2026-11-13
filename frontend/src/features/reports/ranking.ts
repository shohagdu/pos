import type { ProductSalesRow } from '../../api/reports'
import { dec } from '../../lib/decimal'

/** What a product ranking orders by. */
export type RankBy = 'quantity' | 'net_sales' | 'gross_profit'

export interface RankedProduct {
  rank: number
  row: ProductSalesRow
  /** The ranked value, for the share bar. */
  value: string
  /** This value as a share of the top one, 0–100 (bar width). */
  share: number
}

function valueOf(row: ProductSalesRow, by: RankBy): string | null {
  return by === 'quantity' ? row.quantity : by === 'net_sales' ? row.net_sales : row.gross_profit
}

/**
 * The top `limit` products by `by`, highest first. Products whose value is
 * zero or less (fully returned, or sold at a loss for profit) are left out,
 * so "best" never lists a product that did not earn its place. Ties keep the
 * report's order (by net sales).
 */
export function rankProducts(rows: ProductSalesRow[], by: RankBy, limit: number): RankedProduct[] {
  const scored = rows
    .map((row, i) => ({ row, i, value: valueOf(row, by) }))
    .filter((x): x is { row: ProductSalesRow; i: number; value: string } => x.value !== null && dec(x.value) > 0n)
    .sort((a, b) => {
      const d = dec(b.value) - dec(a.value)
      return d > 0n ? 1 : d < 0n ? -1 : a.i - b.i
    })
    .slice(0, limit)
  const top = scored[0] ? Number(scored[0].value) : 0
  return scored.map((x, i) => ({
    rank: i + 1,
    row: x.row,
    value: x.value,
    share: top > 0 ? Math.max(2, Math.round((Number(x.value) / top) * 100)) : 0,
  }))
}
