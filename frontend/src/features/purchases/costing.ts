// Live preview of a purchase bill's landed cost (Requirement Analysis §8.4),
// mirroring `backend/crates/purchasing/src/costing.rs`. Exact decimals only
// (lib/decimal); the server's numbers are authoritative after saving.

import { type Dec, ZERO, dec, div, mul, round, sum, toFixed, tryParse } from '../../lib/decimal'

const HUNDRED = dec('100')
const round2 = (a: Dec) => round(a, 2)
const round4 = (a: Dec) => round(a, 4)

export interface CostLineInput {
  quantity: string
  unit_cost: string
  discount_type: 'amount' | 'percent'
  discount_value: string
  tax_rate_pct: string
}

export interface CostedLine {
  gross: Dec
  line_discount: Dec
  bill_discount: Dec
  /** After line and bill discounts, excluding VAT and charges. */
  net: Dec
  tax: Dec
  charges: Dec
  /** Net + VAT. */
  line_total: Dec
  landed_unit_cost: Dec
  discountTooLarge: boolean
}

export interface BillPreview {
  lines: CostedLine[]
  subtotal: Dec
  item_discount_total: Dec
  bill_discount: Dec
  tax_total: Dec
  shipping: Dec
  other: Dec
  grand_total: Dec
  billDiscountTooLarge: boolean
}

/** Splits `total` pro-rata over `weights` (2 decimals); the last non-zero weight takes the residue. */
export function allocate(total: Dec, weights: Dec[]): Dec[] {
  if (weights.length === 0 || total === ZERO) return weights.map(() => ZERO)
  const ws = sum(weights) === ZERO ? weights.map(() => dec('1')) : weights
  const total_w = sum(ws)
  let last = 0
  ws.forEach((w, i) => {
    if (w !== ZERO) last = i
  })
  let given = ZERO
  return ws.map((w, i) => {
    if (i === last) return total - given
    const share = round2(div(mul(total, w), total_w))
    given += share
    return share
  })
}

export function costBill(lines: CostLineInput[], billDiscount: string, shipping: string, other: string, taxRecoverable: boolean): BillPreview {
  const pre = lines.map((l) => {
    const gross = round2(mul(dec(l.quantity), dec(l.unit_cost)))
    const value = dec(l.discount_value)
    const disc = value === ZERO ? ZERO : l.discount_type === 'percent' ? round2(div(mul(gross, value), HUNDRED)) : round2(value)
    return { gross, disc, tooLarge: disc > gross || disc < ZERO }
  })
  const beforeBill = pre.map((p) => p.gross - p.disc)
  const netSum = sum(beforeBill)
  const billDisc = round2(dec(billDiscount))
  const billShares = allocate(billDisc, beforeBill)
  const nets = beforeBill.map((n, i) => n - (billShares[i] ?? ZERO))
  const ship = round2(dec(shipping))
  const oth = round2(dec(other))
  const charges = allocate(ship + oth, nets)
  const out: CostedLine[] = lines.map((l, i) => {
    const net = nets[i] ?? ZERO
    const charge = charges[i] ?? ZERO
    const p = pre[i] ?? { gross: ZERO, disc: ZERO, tooLarge: false }
    const tax = round2(div(mul(net, dec(l.tax_rate_pct)), HUNDRED))
    const qty = dec(l.quantity)
    const value = net + charge + (taxRecoverable ? ZERO : tax)
    return {
      gross: p.gross,
      line_discount: p.disc,
      bill_discount: billShares[i] ?? ZERO,
      net,
      tax,
      charges: charge,
      line_total: net + tax,
      landed_unit_cost: qty === ZERO ? ZERO : round4(div(value, qty)),
      discountTooLarge: p.tooLarge,
    }
  })
  return {
    lines: out,
    subtotal: sum(out.map((l) => l.gross)),
    item_discount_total: sum(out.map((l) => l.line_discount)),
    bill_discount: billDisc,
    tax_total: sum(out.map((l) => l.tax)),
    shipping: ship,
    other: oth,
    grand_total: sum(out.map((l) => l.line_total)) + ship + oth,
    billDiscountTooLarge: billDisc > netSum || billDisc < ZERO,
  }
}

/** Unit cost vs the last purchase price: amount and percent (1 decimal); null without a usable last price. */
export function variance(unitCost: string, lastPrice: string | null | undefined): { amount: string; pct: string; sign: -1 | 0 | 1 } | null {
  const cost = tryParse(unitCost)
  const last = tryParse(lastPrice)
  if (cost === null || last === null || last === ZERO) return null
  const diff = cost - last
  return {
    amount: toFixed(diff, 2),
    pct: toFixed(div(mul(diff, HUNDRED), last), 1),
    sign: diff > ZERO ? 1 : diff < ZERO ? -1 : 0,
  }
}

/** Money string for display/formatMoney from a Dec. */
export const money = (a: Dec) => toFixed(a, 2)

/** Estimated credit for returning `qty` of a line, pro-rata of its line total (the server's figure is final). */
export function estimateCredit(lineTotal: string, quantity: string, qty: Dec): Dec {
  const base = dec(quantity)
  if (base === ZERO || qty <= ZERO) return ZERO
  return round2(div(mul(dec(lineTotal), qty), base))
}
