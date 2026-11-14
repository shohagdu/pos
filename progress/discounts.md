# Discounts and promotions: implementation record

Branch `feature/discount-promotions` · design: `Final_docs/05_discount-system.md` · built 2026-11-14.

## Status by phase

| Phase (§15) | Status | Where |
|---|---|---|
| 1 Core engine: data model, product discount (% / amount / fixed price), calculation, POS cart, receipt, sale snapshots | Done | `backend/crates/sales/src/promotions/{engine,store}.rs`, `frontend/src/lib/promotions.ts`, migration `0121_promotions.sql` |
| 2 Flash and schedule: windows, quantity limits with row locks, status, countdown badges | Done | engine `remaining`, `store::live(.., lock=true)`, `Schedule::open_at`, `ProductGrid` badges |
| 3 Buy X Get Y: rules, free-item prompt, stock and cost, returns | Done | engine step 2, `OfferPrompts.tsx`, `returns::bogo_refund` |
| 4 Coupons: codes, validation, bulk generation, redemptions, limits | Done | `store::validate_coupon`, `admin.rs` coupons, `CouponField.tsx`, `CouponsPanel.tsx` |
| 5 Reports, permissions, audit, settings | Done | `reporting/src/promotions.rs`, `/reports/promotions`, Settings → Discounts, 4 permissions |

## How it fits together
- **One engine, two copies.** Rust (`engine.rs`) and TypeScript (`lib/promotions.ts`) run the same algorithm. 29 shared cases in `frontend/src/lib/promotions.vectors.json` keep them identical (`cargo test -p pos-sales`, `vitest src/lib`).
- **Pricing.** The engine produces a discount amount per line; `pricing.calculate` takes it as `promo_discount` before the cashier's line discount. A percentage line discount applies to the price after promotions, then the invoice discount, VAT and rounding follow (§5 order).
- **Server is authoritative.** `POST /sales` loads the live promotions (rows `FOR UPDATE`) and re-validates the coupon. It reruns the engine and refuses a different total with `TOTAL_MISMATCH`; the POS then refreshes its offers and prices.
- **Snapshots.** `sale_item_discounts` keeps the name, type, value, amount, units and free units of every applied offer, plus the Buy X Get Y rule. Old sales are never recalculated. Counters (`usage_count`, `quantity_used`, coupon `used_count`, `coupon_redemptions`) change only when a sale completes, and are given back on cancel.
- **Schedules** are in the business's local time (`PosContext.local_now`); the POS keeps a clock based on the server's time, not the device's.

## Deviations from the design doc (decided while building)
1. **Status is computed, not moved by a job.** The doc suggests a job moving `scheduled → active → expired` every minute. The stored status is only what the owner set (draft/active/paused/ended); scheduled/running/expired/sold out is computed from the dates and limits at every read and at sale time. This is always exact at the moment of sale, with nothing to run.
2. **Buy X Get Y rule fields** are columns on `promotions` (one rule per promotion) instead of a separate `promotion_bxgy_rules` table.
3. **Minimum selling price.** Promotions are capped so a paid unit never goes below the product's minimum price ("Offer reduced to keep the minimum price" on the line); free Buy X Get Y units are exempt. This way a promotion never blocks a sale. The existing check still applies to the cashier's own discounts (`pos.below_min_price`).
4. **Customer groups** don't exist in the app, so `applies_to_customer` is `all` or `registered_only` only. Per-customer limits need a named customer: they are skipped for the walk-in customer, and such coupons ask for a customer.
5. **Wholesale prices** aren't used by sales yet, so there is no "applies to wholesale" setting.
6. **One coupon per invoice.** The "max coupons per invoice" setting isn't offered; Settings → Discounts says so.
7. **Cross-product Buy X Get Y returns.** The doc's "keep the free item, return the paid one → charge the free item" is implemented for the same product (the common case: the kept units are charged as if bought alone). For "buy rice, get oil free", returning the rice refunds it pro rata and the free oil isn't charged back.
8. **Coupon "fixed amount"** means an amount off the order (spread over the qualifying lines by value); for line promotions "fixed amount" is per unit.
9. **The manual discount limit ignores promotions.** The cashier's limit (POS-R03) is measured on the price after promotions, because promotions are the business's own offers.
10. **F6** focuses the coupon field (the doc's "optional new shortcut").

## Permissions (seeded for existing businesses by 0121)
| Code | owner | admin | branch manager | cashier / salesperson | accountant |
|---|---|---|---|---|---|
| `promotions.view` | ✓ | ✓ | ✓ | | ✓ |
| `promotions.manage` | ✓ | ✓ | ✓ | | |
| `pos.apply_coupon` | ✓ | ✓ | ✓ | ✓ | |
| `pos.remove_promotion` | ✓ | ✓ | ✓ | | |

Every create / edit / pause / end / duplicate / delete, coupon change, and every offer removed at the POS (`promotion.removed_at_pos`) is in the audit log.

## Tests
- Engine: 29 shared vectors (Rust + TypeScript), schedule windows (both), Buy 1 Get 1 return rule (Rust + TypeScript estimate).
- `backend/crates/api/tests/promotions.rs`, one end-to-end flow:
  - product discount and `TOTAL_MISMATCH`; lock after use and editing limits
  - removal permission and audit
  - flash limit running out and sold-out time
  - Buy 1 Get 1, including its returns
  - coupons: validation, not-better, minimum purchase, single use, given back on cancel
  - the report
- POS screen: live offer priced and sent; coupon applied and sent.
- Checked in the browser against a copy of the dev database (`pos_copy`, API on 8081, Vite on 5174).

## Open / needs your decision
- Merge `feature/discount-promotions` into main when happy. Migration 0121 runs on the next API start (dev DB `pos` is untouched so far; the browser checks used `pos_copy`).
- `pos_copy` and the test servers on 8081 / 5174 are still running for you to try. Stop them and `DROP DATABASE pos_copy` when done.
- The deviations above (especially 4, 6 and 7), if any should change.
