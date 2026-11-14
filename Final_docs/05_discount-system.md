# Discount & Promotion System — Design Document

POS / Inventory app · Covers: Coupon discounts, Flash discounts, Buy One Get One (BOGO), and Flat % discounts on selected products.

---

## 1. Goals

- Let the shop owner create promotions without developer help.
- Apply discounts **automatically** at the POS where possible, so cashiers don't have to remember offers.
- Keep pricing **predictable**: clear rules for which discount wins when several apply.
- Keep accounting **correct**: VAT, minimum selling price, returns, and reports must all reflect discounts properly.
- Work per **branch**, with full audit history.

---

## 2. Promotion Types

| Type | How it's triggered | Example |
|---|---|---|
| **Product discount (Flat %)** | Automatic, while active | 20% off all Battery products |
| **Product discount (Fixed amount)** | Automatic, while active | ৳50 off E-bike helmet |
| **Flash discount** | Automatic, only inside a short time window | 30% off Ata 800, today 5–7 PM, first 50 units |
| **Buy X Get Y (BOGO)** | Automatic, when cart meets quantity | Buy 1 Battery, get 1 free |
| **Coupon** | Cashier enters a code | Code `EID20` → 20% off selected products, max ৳500 |

All types share one core concept: a **Promotion** with a schedule, targets (which products), and a benefit (what discount). A coupon is a promotion that additionally requires a code.

---

## 3. Data Model

### 3.1 `promotions`

| Field | Type | Notes |
|---|---|---|
| id | PK | |
| name | string | Shown in admin, on cart line, and on receipt |
| name_bn | string, nullable | Bengali name |
| type | enum | `product_discount`, `flash`, `bxgy`, `coupon` |
| discount_type | enum | `percent`, `fixed_amount`, `fixed_price`, `free` |
| discount_value | decimal | 20 for 20%, 50 for ৳50, etc. |
| max_discount_amount | decimal, nullable | Cap per invoice (e.g. max ৳500) |
| min_purchase_amount | decimal, nullable | Cart subtotal required (mostly coupons) |
| starts_at / ends_at | datetime | Required for flash; optional for others |
| active_days | json, nullable | e.g. `["fri","sat"]` |
| active_time_from / to | time, nullable | e.g. happy hour 17:00–19:00 |
| priority | int | Higher wins when promotions conflict |
| stackable | bool | Can combine with other promotions on the same line |
| applies_to_customer | enum | `all`, `registered_only`, `customer_group` |
| total_usage_limit | int, nullable | Total redemptions allowed |
| per_customer_limit | int, nullable | Redemptions per customer |
| quantity_limit | int, nullable | Total discounted units (flash sale stock) |
| quantity_used | int | Counter |
| status | enum | `draft`, `scheduled`, `active`, `paused`, `expired` |
| created_by / updated_by | FK users | Audit |
| timestamps, soft deletes | | Never hard-delete used promotions |

### 3.2 `promotion_targets` (which products qualify)

| Field | Notes |
|---|---|
| promotion_id | FK |
| target_type | `product`, `category`, `brand`, `all_products` |
| target_id | nullable when `all_products` |
| is_exclusion | bool — exclude these from a broader target (e.g. all Electric except P000003) |

### 3.3 `promotion_bxgy_rules` (BOGO / Buy X Get Y)

| Field | Notes |
|---|---|
| promotion_id | FK |
| buy_quantity | e.g. 1 |
| get_quantity | e.g. 1 |
| get_target_type | `same_product`, `product`, `category` |
| get_target_id | nullable when `same_product` |
| get_discount_type | `free` or `percent` (e.g. second item 50% off) |
| get_discount_value | used when `percent` |
| repeat | bool — buy 2 get 2, buy 3 get 3… or only once per invoice |
| auto_add_free_item | bool — POS adds the free item automatically |

### 3.4 `promotion_branches`

`promotion_id`, `branch_id`. No rows = all branches.

### 3.5 `coupons`

| Field | Notes |
|---|---|
| id, promotion_id | One promotion can have many codes |
| code | Unique, case-insensitive, e.g. `EID20` |
| type | `public` (one shared code) or `unique` (single-use generated codes) |
| usage_limit | per code |
| used_count | counter |
| assigned_customer_id | nullable — personal coupon |
| expires_at | nullable — overrides promotion end |
| status | `active`, `used`, `disabled` |

### 3.6 `sale_item_discounts` (what was actually applied)

Store discounts on the sale, never recalculate old sales from current promotions.

| Field | Notes |
|---|---|
| sale_id, sale_item_id | |
| promotion_id, coupon_id | nullable |
| promotion_name_snapshot | Name at time of sale |
| discount_type, discount_value | Snapshot |
| discount_amount | Final ৳ discounted on this line |
| free_quantity | For BOGO lines |

### 3.7 `coupon_redemptions`

`coupon_id`, `sale_id`, `customer_id`, `branch_id`, `user_id`, `discount_amount`, `status` (`applied`, `reversed`), timestamps. Used for limits and reporting.

---

## 4. How Each Type Works

### 4.1 Flat % on selected products

- Admin picks products, categories, or brands (with optional exclusions) and sets e.g. 20%.
- POS: when the product is added, the line shows original price, discount, and final price.

Example: Battery ৳500 × 2, 20% off → line discount ৳200, line total ৳800.

### 4.2 Flash discount

- Same as product discount, but **requires** `starts_at` and `ends_at` (short window) and usually a `quantity_limit`.
- Promotion status moves `scheduled → active → expired` automatically (scheduled job every minute, plus a check at the moment of sale).
- POS shows a "Flash" badge and countdown on the product card.
- `quantity_used` increases only when a sale is **completed**, inside a database transaction with a row lock, so two registers can't oversell the flash quantity.
- When the limit is reached mid-cart, only remaining units get the discount; the rest are full price. Show a message on the line.
- The time check uses the **server time at sale completion**, not the cashier's device time.

### 4.3 Buy One Get One (Buy X Get Y)

- Rule example: buy 1 Battery → get 1 Battery free (`same_product`, `repeat = true`).
- Free quantity = `floor(qualifying_qty / (buy_qty + get_qty)) × get_qty` when the free item is the same product.
  - Customer takes 2 → 1 free. Takes 3 → 1 free. Takes 4 → 2 free.
- If `auto_add_free_item` is on and the cart has 1 unit, POS shows a prompt: "Add 1 free Battery?" rather than silently adding stock.
- Free item is shown as a separate line or a line note: `Battery × 1 — FREE (BOGO)`, price ৳0, with the original price shown struck out.
- **Stock is deducted for free items** (they leave the shop). Cost of goods still counts.
- When the free item is a different product, it must be in stock; if not, show a warning and skip it.
- When products have different prices in the qualifying set (category-based BOGO), the **cheapest** items are the free ones.

### 4.4 Coupon discount

- Cashier enters code in the right panel (under Invoice discount), or scans a coupon barcode.
- Validation order, with a clear message for each failure:
  1. Code exists and is active
  2. Within date/time and branch
  3. Total and per-code usage limits not reached
  4. Customer requirement met (registered customer selected, personal coupon matches)
  5. Per-customer limit not reached
  6. Minimum purchase amount met
  7. At least one cart item qualifies
- Coupon can target selected products (only those lines are discounted) or the whole invoice.
- `max_discount_amount` caps the total coupon discount; split the capped amount across qualifying lines in proportion to line value.
- Coupon usage is recorded only when the sale is completed. Removing the coupon or cancelling the cart releases nothing because nothing was reserved.
- One coupon per invoice by default (setting can allow more).

---

## 5. Which Discount Wins (Calculation Order)

Run this every time the cart changes.

1. **Base price** for each line (selling price, or wholesale price if a wholesale customer is selected).
2. **Automatic line promotions** (product discount, flash): for each line, collect eligible promotions.
   - If none are stackable, pick the one giving the **highest discount to the customer**; break ties by `priority`.
   - Stackable ones apply one after another on the reduced price, in `priority` order.
3. **BOGO** rules on remaining quantities.
4. **Coupon** on qualifying lines (only if the coupon is stackable with already-applied promotions; otherwise apply whichever gives the customer more, and tell the cashier).
5. **Invoice discount** (the existing F7 manual discount) on the subtotal after all promotions.
6. **Minimum selling price guard**: no line's final unit price may go below the product's minimum selling price unless the user has the `override_min_price` permission. Show a warning on the line.
7. **VAT** calculated on the discounted amount.
8. **Rounding** to the shop's rounding setting at the invoice total.

Default settings (configurable in Settings → Discounts):

| Setting | Default |
|---|---|
| Promotions stack with each other | Off |
| Coupon stacks with automatic promotions | Off (best one wins) |
| Manual invoice discount allowed with promotions | On |
| Max coupons per invoice | 1 |
| Max manual discount % for cashier role | 10% |

---

## 6. POS Screen Behavior

- **Product card**: badge for active offer — `-20%`, `Flash -30%` with countdown, `Buy 1 Get 1`.
- **Cart line**: struck-out original price, final price, and a small tag with the promotion name. Tapping the tag shows details.
- **BOGO**: prompt to add the free item; free line clearly marked.
- **Coupon**: input field in the right panel with Apply/Remove; success message shows the saved amount; errors are specific ("Minimum purchase ৳1,000 required").
- **Totals panel** adds a line: `Promotions −৳X` above Invoice discount, so Subtotal → Promotions → Coupon → Invoice discount → VAT → Grand total.
- **Held carts**: when a held cart is resumed, recalculate promotions. If a promotion has ended, show "Offer ended — prices updated" before payment.
- **Offline / slow network**: if the coupon can't be validated with the server, don't accept it.
- Keep existing keyboard shortcuts. Optional new shortcut for coupon field if a free key exists.

---

## 7. Admin Screens

**Promotions list**: name, type, status, schedule, branches, usage (used / limit), quick actions (pause, duplicate, end now). Filters by type, status, branch.

**Create/Edit promotion** (step form, fits one screen):
1. Type and name (English + Bengali)
2. Benefit: discount type and value, cap, BOGO rule
3. Products: product/category/brand picker with exclusions and a live count of matched products
4. Schedule: start/end, days, time window
5. Limits: total, per customer, flash quantity, min purchase
6. Branches and customer eligibility
7. Preview: sample cart showing the calculated result

**Coupons**: create a public code or bulk-generate unique codes (export CSV/print), view redemptions, disable a code.

Editing rules: once a promotion has been used, its benefit and targets can't be changed (to keep reports accurate). Admin can end it and duplicate it instead.

---

## 8. Returns and Refunds

- Refund the amount the customer **actually paid** for the returned item, using the stored `sale_item_discounts`.
- Invoice-level and capped coupon discounts are spread across lines in proportion to line value at sale time; that share is deducted on return.
- **BOGO return**: if the customer returns the paid item but keeps the free one, the free item is charged at its normal price (or the customer returns both). If they return only the free item, refund ৳0.
- If a whole sale is returned, mark its coupon redemption `reversed` and decrement `used_count` only if the setting "Restore coupon on full return" is on (default off).
- Flash `quantity_used` is not restored on return by default.

---

## 9. Receipt and Invoice

- Show each discounted line with original price and discount: `Battery 2 × ৳500 = ৳1,000  (-20% Eid offer −৳200)`.
- BOGO free line: `Battery 1 × ৳500 FREE`.
- Summary block: Subtotal, Promotions, Coupon (with code), Invoice discount, VAT, Grand total, and a line "You saved ৳X".
- Bengali receipt uses `name_bn` where available.

---

## 10. Reports

- **Promotion performance**: sales count, units, revenue, total discount given, average basket, by promotion and branch.
- **Coupon usage**: redemptions per code, per customer, per cashier.
- **Flash sale**: units sold vs limit, sell-out time.
- **Discount by cashier**: manual discounts vs automatic, to spot misuse.
- **Profit impact**: revenue after discount minus cost (including cost of free BOGO items).

---

## 11. Permissions

| Permission | Owner | Manager | Cashier |
|---|---|---|---|
| Create / edit / delete promotions | ✓ | ✓ | |
| Pause / end promotions | ✓ | ✓ | |
| Generate coupons | ✓ | ✓ | |
| Apply coupon at POS | ✓ | ✓ | ✓ |
| Remove an automatic promotion from a line | ✓ | ✓ | |
| Override minimum selling price | ✓ | | |
| View promotion reports | ✓ | ✓ | |

All create/edit/pause actions and every promotion removal at POS are written to the audit log (user, time, before/after).

---

## 12. API Endpoints (suggested)

| Method | Endpoint | Purpose |
|---|---|---|
| GET | `/promotions` | List with filters |
| POST | `/promotions` | Create |
| GET/PUT | `/promotions/{id}` | View / update (blocked fields after first use) |
| POST | `/promotions/{id}/pause`, `/resume`, `/end`, `/duplicate` | Status actions |
| GET | `/pos/promotions/active?branch_id=` | Active promotions for POS (cached, refreshed every few minutes and on schedule change) |
| POST | `/pos/cart/calculate` | Server-side price calculation for a cart (single source of truth) |
| POST | `/pos/coupons/validate` | Validate a code for the current cart |
| POST | `/coupons/generate` | Bulk generate unique codes |
| GET | `/reports/promotions`, `/reports/coupons` | Reports |

The POS may calculate in the browser for instant feedback, but the **server recalculates on sale completion** and rejects the sale if totals differ (then the POS refreshes the cart).

---

## 13. Edge Cases Checklist

- Product in two active promotions → best-for-customer or priority rule.
- Flash sale ends while item is in cart → recalculated at payment with a message.
- Two registers selling the last flash units at the same time → row lock; second sale gets remaining units only.
- Discount makes a line negative or below cost → never below ৳0; min selling price guard.
- Fixed-amount discount larger than item price → capped at item price.
- Quantity changed on a BOGO line → free quantity recalculated.
- Wholesale customer selected → decide per promotion whether it applies to wholesale prices (setting on promotion).
- Walk-in customer with a "registered customers only" coupon → clear error, offer to select/create customer.
- Batch/expiry tracked products → discount doesn't change which batch is picked (earliest expiry first still applies).
- Promotion edited/deleted after use → blocked; old sales keep snapshots.
- Time zone → all schedules stored and checked in the shop's time zone (Asia/Dhaka).
- Product with "Out of stock" → offer badge still shows but the product can't be added unless negative stock is allowed.

---

## 14. Testing Checklist

- Unit tests for the calculation engine: each type alone, combinations, stacking on/off, caps, min purchase, min selling price, VAT, rounding.
- BOGO quantities: 1, 2, 3, 4, 5 units; cross-product BOGO; free item out of stock.
- Coupon validation: every failure reason returns the right message.
- Flash concurrency test: parallel sales against a small quantity limit.
- Returns: partial, full, BOGO paid-item return, BOGO free-item return, coupon proportional refund.
- Held cart resume after promotion end.
- Receipt output in English and Bengali.
- Permissions per role.

---

## 15. Implementation Phases

1. **Phase 1 — Core engine**: data model, product discount (% and fixed), calculation endpoint, POS cart display, receipt, sale snapshots.
2. **Phase 2 — Flash & schedule**: time windows, quantity limits with locking, status scheduler, countdown badges.
3. **Phase 3 — BOGO**: Buy X Get Y rules, free item prompt, stock and cost handling, returns.
4. **Phase 4 — Coupons**: codes, validation, bulk generation, redemptions, limits.
5. **Phase 5 — Reports, permissions, audit polish**, settings page for stacking rules.

Each phase ships with its tests and can go live on its own.
