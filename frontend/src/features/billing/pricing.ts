// Plan price arithmetic for the renewal screen. Exact decimals only
// (src/lib/decimal.ts), never JS floats.

import type { BillingCycle, PlanOffer } from '../../api/billing'
import { dec, div, mul, round, sub, toFixed, toPlain } from '../../lib/decimal'

export const CYCLES: BillingCycle[] = ['monthly', 'quarterly', 'annual', 'lifetime']

/** Months a cycle buys; 0 for a one-time licence, which never ends. */
export const CYCLE_MONTHS: Record<BillingCycle, number> = { monthly: 1, quarterly: 3, annual: 12, lifetime: 0 }

/** The plan's price for a billing cycle, or `null` when it is not sold that way. */
export function priceFor(plan: PlanOffer, cycle: BillingCycle): string | null {
  return plan.prices.find((p) => p.billing_cycle === cycle)?.amount ?? null
}

/** What the cycle costs per month, rounded to paisa: "1,200 per year" → "100.00". */
export function perMonth(amount: string, cycle: BillingCycle): string {
  return toFixed(div(dec(amount), dec(CYCLE_MONTHS[cycle])), 2)
}

/**
 * Savings against paying monthly for the same number of months; `null` when
 * there is no monthly price or nothing is saved.
 */
export function savings(plan: PlanOffer, cycle: BillingCycle): { amount: string; pct: string } | null {
  const monthly = priceFor(plan, 'monthly')
  const price = priceFor(plan, cycle)
  if (cycle === 'monthly' || cycle === 'lifetime' || monthly === null || price === null) return null
  const full = mul(dec(monthly), dec(CYCLE_MONTHS[cycle]))
  const saved = sub(full, dec(price))
  if (saved <= 0n || full <= 0n) return null
  const pct = round(div(mul(saved, dec(100)), full), 0)
  return { amount: toFixed(saved, 2), pct: toPlain(pct) }
}
