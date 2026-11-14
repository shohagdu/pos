// The promotion engine (Final_docs/05_discount-system.md §4–5): the same
// algorithm as the server's `pos_sales::promotions::engine`, locked by the
// shared test vectors in `promotions.vectors.json`. It turns the cart and the
// promotions live for it into a discount per line, which `pricing.ts` takes
// off before the cashier's line discount, the invoice discount and VAT. The
// server recalculates on sale completion and stays authoritative.

import { type Dec, ZERO, dec, div, max, min, mul, round, sum } from './decimal'

export type PromoKind = 'product_discount' | 'flash' | 'bxgy' | 'coupon'
export type BenefitType = 'percent' | 'fixed_amount' | 'fixed_price' | 'free'
export type TargetType = 'product' | 'category' | 'brand' | 'all_products'
export type GetTargetType = 'same_product' | 'product' | 'category'
export type CouponOutcome = 'applied' | 'not_better' | 'min_purchase' | 'no_qualifying_items'

export interface Target {
  target_type: TargetType
  target_id: string | null
  is_exclusion?: boolean
}

export interface Bxgy {
  buy_quantity: string
  get_quantity: string
  get_target_type: GetTargetType
  get_target_id?: string | null
  get_discount_type: BenefitType
  get_discount_value?: string
  repeat?: boolean
  auto_add_free_item?: boolean
}

/** A promotion as the server sends it (decimals as strings). */
export interface Promotion {
  id: string
  name: string
  name_bn?: string | null
  kind: PromoKind
  discount_type: BenefitType
  discount_value: string
  max_discount_amount?: string | null
  min_purchase_amount?: string | null
  priority?: number
  stackable?: boolean
  targets: Target[]
  quantity_remaining?: string | null
  bxgy?: Bxgy | null
  coupon_id?: string | null
  coupon_code?: string | null
}

export interface EngineLine {
  variantId: string
  productId: string
  /** The product's category and its ancestors. */
  categoryIds: string[]
  brandId: string | null
  quantity: Dec
  unitPrice: Dec
  minUnitPrice: Dec | null
  /** The cashier removed automatic promotions from this line. */
  promotionsOff: boolean
}

export interface Settings {
  promotionsStack: boolean
  couponStacks: boolean
}

export interface Applied {
  promotion: Promotion
  amount: Dec
  units: Dec
  freeQuantity: Dec
}

export interface LineOut {
  discounts: Applied[]
  amount: Dec
  freeQuantity: Dec
  flashPartial: boolean
  cappedByMinPrice: boolean
}

export interface Offer {
  promotion: Promotion
  getTargetType: GetTargetType
  /** Variant for `same_product`, product or category id otherwise. */
  targetId: string
  quantity: Dec
}

export interface Output {
  lines: LineOut[]
  promotionTotal: Dec
  couponTotal: Dec
  offers: Offer[]
  coupon: CouponOutcome | null
}

const HUNDRED = dec(100)
const ONE = dec(1)
const r2 = (v: Dec) => round(v, 2)
const pos = (v: Dec) => max(v, ZERO)
/** floor(a / b) for non-negative values, exactly (no rounding in between). */
const floorDiv = (a: Dec, b: Dec): Dec => (b <= ZERO ? ZERO : (a / b) * ONE)

function targetMatches(t: Target, line: EngineLine): boolean {
  switch (t.target_type) {
    case 'all_products':
      return true
    case 'product':
      return t.target_id === line.productId
    case 'category':
      return t.target_id !== null && line.categoryIds.includes(t.target_id)
    case 'brand':
      return t.target_id !== null && t.target_id === line.brandId
  }
}

/** Included by a target and not excluded by one. */
export function matches(targets: Target[], line: EngineLine): boolean {
  const included = targets.some((t) => !t.is_exclusion && targetMatches(t, line))
  return included && !targets.some((t) => t.is_exclusion && targetMatches(t, line))
}

function getMatches(b: Bxgy, line: EngineLine): boolean {
  if (b.get_target_type === 'product') return (b.get_target_id ?? null) === line.productId
  if (b.get_target_type === 'category') return !!b.get_target_id && line.categoryIds.includes(b.get_target_id)
  return false
}

/** What a line promotion takes off `units` units at `price`. */
function lineAmount(kind: BenefitType, value: Dec, price: Dec, units: Dec): Dec {
  let amount: Dec
  if (kind === 'percent') amount = r2(div(mul(mul(price, units), value), HUNDRED))
  else if (kind === 'fixed_amount') amount = mul(pos(min(value, price)), units)
  else if (kind === 'fixed_price') amount = mul(pos(price - value), units)
  else amount = mul(price, units)
  return pos(min(r2(amount), r2(mul(price, units))))
}

/** Same, applied to what is left of the line after earlier stacked promotions. */
function stackedAmount(kind: BenefitType, value: Dec, price: Dec, quantity: Dec, units: Dec, left: Dec): Dec {
  if (quantity === ZERO) return ZERO
  const amount = kind === 'percent' ? r2(div(mul(mul(left, units), value), mul(quantity, HUNDRED))) : lineAmount(kind, value, price, units)
  return pos(min(amount, left))
}

function freeUnitValue(b: Bxgy, price: Dec, units: Dec): Dec {
  return b.get_discount_type === 'percent' ? r2(div(mul(mul(price, units), dec(b.get_discount_value ?? '0')), HUNDRED)) : r2(mul(price, units))
}

/** Spreads `total` over `weights` in proportion; the last non-zero weight takes the residue. */
function allocate(total: Dec, weights: Dec[]): Dec[] {
  const s = sum(weights)
  if (s === ZERO) return weights.map(() => ZERO)
  let last = 0
  weights.forEach((w, i) => {
    if (w !== ZERO) last = i
  })
  let given = ZERO
  return weights.map((w, i) => {
    const share = pos(min(i === last ? total - given : r2(div(mul(total, w), s)), w))
    given += share
    return share
  })
}

const lineSum = (l: LineOut) => sum(l.discounts.map((d) => d.amount))
const optDec = (v?: string | null) => (v === null || v === undefined ? null : dec(v))

export function applyPromotions(lines: EngineLine[], promotions: Promotion[], coupon: Promotion | null, settings: Settings): Output {
  const out: LineOut[] = lines.map(() => ({ discounts: [], amount: ZERO, freeQuantity: ZERO, flashPartial: false, cappedByMinPrice: false }))
  const gross = lines.map((l) => r2(mul(l.quantity, l.unitPrice)))
  const order = promotions.map((_, i) => i).sort((a, b) => (promotions[b]!.priority ?? 0) - (promotions[a]!.priority ?? 0) || a - b)

  // --- 1. Automatic line promotions: product discount and flash.
  const remaining = promotions.map((p) => optDec(p.quantity_remaining))
  lines.forEach((line, li) => {
    if (line.promotionsOff || line.quantity <= ZERO) return
    const candidates = order.filter((pi) => {
      const p = promotions[pi]!
      const r = remaining[pi]
      return (p.kind === 'product_discount' || p.kind === 'flash') && matches(p.targets, line) && (r === null || r === undefined || r > ZERO)
    })
    if (candidates.length === 0) return
    const unitsOf = (pi: number) => {
      const r = remaining[pi]
      return r === null || r === undefined ? line.quantity : min(r, line.quantity)
    }
    let best: [number, Dec] | null = null
    for (const pi of candidates) {
      const p = promotions[pi]!
      const a = lineAmount(p.discount_type, dec(p.discount_value), line.unitPrice, unitsOf(pi))
      if (best === null || a > best[1]) best = [pi, a]
    }
    let chosen: [number, Dec][] = best && best[1] > ZERO ? [best] : []
    if (settings.promotionsStack) {
      let left = gross[li]!
      const chain: [number, Dec][] = []
      for (const pi of candidates.filter((pi) => promotions[pi]!.stackable)) {
        const p = promotions[pi]!
        const a = stackedAmount(p.discount_type, dec(p.discount_value), line.unitPrice, line.quantity, unitsOf(pi), left)
        if (a > ZERO) {
          left -= a
          chain.push([pi, a])
        }
      }
      const chainTotal = sum(chain.map(([, a]) => a))
      const bestTotal = sum(chosen.map(([, a]) => a))
      if (chain.length > 1 && chainTotal > bestTotal) chosen = chain
    }
    for (const [pi, a] of chosen) {
      const units = unitsOf(pi)
      const r = remaining[pi]
      if (r !== null && r !== undefined) {
        remaining[pi] = r - units
        if (units < line.quantity) out[li]!.flashPartial = true
      }
      out[li]!.discounts.push({ promotion: promotions[pi]!, amount: a, units, freeQuantity: ZERO })
    }
  })

  // Per-invoice cap of each line promotion.
  for (const p of promotions) {
    if ((p.kind !== 'product_discount' && p.kind !== 'flash') || p.max_discount_amount == null) continue
    const cap = dec(p.max_discount_amount)
    const hits = out.flatMap((l) => l.discounts.filter((d) => d.promotion.id === p.id))
    const total = sum(hits.map((d) => d.amount))
    if (total > cap) {
      allocate(
        r2(pos(cap)),
        hits.map((d) => d.amount),
      ).forEach((share, i) => {
        hits[i]!.amount = share
      })
    }
  }

  // --- 2. Buy X Get Y on lines without a line promotion.
  const offers: Offer[] = []
  const inBxgy = lines.map(() => false)
  for (const pi of order.filter((pi) => promotions[pi]!.kind === 'bxgy')) {
    const p = promotions[pi]!
    const b = p.bxgy
    if (!b) continue
    const buyQ = dec(b.buy_quantity)
    const getQ = dec(b.get_quantity)
    if (buyQ <= ZERO || getQ <= ZERO) continue
    const freeLine = (li: number) => {
      const l = lines[li]!
      return !l.promotionsOff && l.quantity > ZERO && out[li]!.discounts.length === 0 && !inBxgy[li]
    }
    const buyLines = lines.map((_, li) => li).filter((li) => freeLine(li) && matches(p.targets, lines[li]!))
    const group = buyQ + getQ
    const grant = (li: number, free: Dec) => {
      const amount = freeUnitValue(b, lines[li]!.unitPrice, free)
      if (free > ZERO && amount > ZERO) {
        out[li]!.discounts.push({ promotion: p, amount, units: free, freeQuantity: free })
        out[li]!.freeQuantity += free
      }
    }

    if (b.get_target_type === 'same_product') {
      let once = false
      for (const li of buyLines) {
        const q = lines[li]!.quantity
        let sets = floorDiv(q, group)
        if (!b.repeat) sets = min(sets, once ? ZERO : ONE)
        const free = mul(sets, getQ)
        if (free > ZERO) {
          once = true
          inBxgy[li] = true
          grant(li, free)
        }
        const rest = q - mul(floorDiv(q, group), group)
        if (b.auto_add_free_item && (b.repeat || sets === ZERO) && rest >= buyQ) {
          offers.push({ promotion: p, getTargetType: 'same_product', targetId: lines[li]!.variantId, quantity: group - rest })
        }
      }
    } else {
      const getLines = lines.map((_, li) => li).filter((li) => freeLine(li) && getMatches(b, lines[li]!))
      const overlap = getLines.some((li) => buyLines.includes(li))
      const pool = [...getLines].sort((a, c) => {
        const pa = lines[a]!.unitPrice
        const pc = lines[c]!.unitPrice
        return pa < pc ? -1 : pa > pc ? 1 : a - c
      })
      let entitled: Dec
      if (overlap) {
        const units = [...buyLines, ...getLines.filter((li) => !buyLines.includes(li))]
        const sets = floorDiv(sum(units.map((li) => lines[li]!.quantity)), group)
        entitled = mul(b.repeat ? sets : min(sets, ONE), getQ)
      } else {
        const sets = floorDiv(sum(buyLines.map((li) => lines[li]!.quantity)), buyQ)
        entitled = mul(b.repeat ? sets : min(sets, ONE), getQ)
      }
      let left = entitled
      for (const li of pool) {
        if (left <= ZERO) break
        const free = min(left, lines[li]!.quantity)
        left -= free
        inBxgy[li] = true
        grant(li, free)
      }
      if (entitled > left) buyLines.forEach((li) => (inBxgy[li] = true))
      if (left > ZERO && !overlap && b.auto_add_free_item && b.get_target_id) {
        offers.push({ promotion: p, getTargetType: b.get_target_type, targetId: b.get_target_id, quantity: left })
      }
    }
  }

  // --- 3. Coupon.
  let couponOutcome: CouponOutcome | null = null
  if (coupon) {
    const subtotal = sum(gross)
    const qualifying = lines.map((_, li) => li).filter((li) => lines[li]!.quantity > ZERO && matches(coupon.targets, lines[li]!))
    const minPurchase = optDec(coupon.min_purchase_amount)
    if (minPurchase !== null && subtotal < minPurchase) couponOutcome = 'min_purchase'
    else if (qualifying.length === 0) couponOutcome = 'no_qualifying_items'
    else {
      const stacks = settings.couponStacks || !!coupon.stackable
      const value = dec(coupon.discount_value)
      const bases = qualifying.map((li) => pos(stacks ? gross[li]! - lineSum(out[li]!) : gross[li]!))
      const baseSum = sum(bases)
      let total = coupon.discount_type === 'percent' ? sum(bases.map((b) => r2(div(mul(b, value), HUNDRED)))) : min(r2(value), baseSum)
      const cap = optDec(coupon.max_discount_amount)
      if (cap !== null) total = min(total, r2(cap))
      total = pos(total)
      const autoThere = sum(qualifying.map((li) => lineSum(out[li]!)))
      if (total === ZERO) couponOutcome = 'no_qualifying_items'
      else if (!stacks && total <= autoThere) couponOutcome = 'not_better'
      else {
        if (!stacks) {
          for (const li of qualifying) {
            out[li] = { ...out[li]!, discounts: [], freeQuantity: ZERO, flashPartial: false }
          }
        }
        const shares =
          coupon.discount_type === 'percent' && (cap === null || total < r2(cap)) ? bases.map((b) => r2(div(mul(b, value), HUNDRED))) : allocate(total, bases)
        qualifying.forEach((li, i) => {
          const share = shares[i]!
          if (share > ZERO) out[li]!.discounts.push({ promotion: coupon, amount: share, units: lines[li]!.quantity, freeQuantity: ZERO })
        })
        couponOutcome = 'applied'
      }
    }
  }

  // --- 4. Never below the minimum selling price (free units excepted), never below zero.
  lines.forEach((line, li) => {
    const o = out[li]!
    const floor = line.minUnitPrice === null ? ZERO : r2(mul(line.minUnitPrice, pos(line.quantity - o.freeQuantity)))
    const room = pos(gross[li]! - floor)
    let total = lineSum(o)
    if (total > room) {
      if (line.minUnitPrice !== null) o.cappedByMinPrice = true
      for (let i = o.discounts.length - 1; i >= 0 && total > room; i--) {
        const d = o.discounts[i]!
        const cut = min(total - room, d.amount)
        d.amount -= cut
        total -= cut
      }
      o.discounts = o.discounts.filter((d) => d.amount > ZERO || d.freeQuantity > ZERO)
    }
    o.amount = lineSum(o)
  })

  const couponTotal = sum(out.flatMap((l) => l.discounts.filter((d) => d.promotion.kind === 'coupon').map((d) => d.amount)))
  const all = sum(out.map((l) => l.amount))
  return { lines: out, promotionTotal: all - couponTotal, couponTotal, offers, coupon: couponOutcome }
}

/** The schedule fields a live promotion carries (`GET /pos/promotions`). */
export interface ScheduleFields {
  starts_at?: string | null
  ends_at?: string | null
  active_days?: string[] | null
  active_time_from?: string | null
  active_time_to?: string | null
}

const WEEKDAYS = ['sun', 'mon', 'tue', 'wed', 'thu', 'fri', 'sat'] as const

/**
 * Open at `now` (business local time `YYYY-MM-DDTHH:MM:SS`): within the
 * dates, on an active day, inside the time window (22:00–02:00 runs over
 * midnight). Same rule as the server's `Schedule::open_at`.
 */
export function scheduleOpen(s: ScheduleFields, now: string): boolean {
  const at = now.slice(0, 19)
  if (s.starts_at && at < s.starts_at.slice(0, 19)) return false
  if (s.ends_at && at >= s.ends_at.slice(0, 19)) return false
  if (s.active_days && s.active_days.length > 0) {
    const [y, m, d] = at.slice(0, 10).split('-').map(Number)
    const day = WEEKDAYS[new Date(Date.UTC(y!, m! - 1, d!)).getUTCDay()]!
    if (!s.active_days.includes(day)) return false
  }
  const t = at.slice(11, 19)
  const from = s.active_time_from?.slice(0, 8) ?? null
  const to = s.active_time_to?.slice(0, 8) ?? null
  if (from && to) return from <= to ? from <= t && t < to : t >= from || t < to
  if (from) return t >= from
  if (to) return t < to
  return true
}
