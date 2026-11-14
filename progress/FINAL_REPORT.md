# Responsive work: final report

**Date:** 2026-11-14 · **Scope:** every route of the frontend (109 audited, incl. auth, onboarding, settings sections, admin console, detail pages), dialogs and print views · **UI-only**: no change to logic, APIs, validation, routes or keyboard shortcuts.

## Result

| | Routes |
|---|---|
| Passed (no page-specific problem before; shared fixes still apply) | 62 |
| Fixed | 47 |
| Still open | 0 |

Verified in Bengali at 360, 390, 768, 1024, 1366×768, 1440×900 and 1920×1080 with headless Chrome:
- **Width:** no page scrolls sideways at any size, and no table scrolls sideways on a laptop or desktop.
- **Text:** no amounts are cut off with "…".
- **Work screens:** the POS screen, the new-product form and the product edit form fit 1366×768 (and 1440×900) without page scrolling.
- **Keyboard:** POS shortcuts (F2, F4, F7, F8, F9, F10, Ctrl+Del, Esc) are unchanged. F4/F7 also open the customer & discount sheet on phones.
- **Checks:** all 210 frontend tests pass, typecheck is clean, and a production build succeeds. Lint shows only 2 warnings, both pre-existing in files not touched (`api/files.ts`, `features/sales/WarrantyTable.tsx`).

Per-page detail (issues before, changes, verification): `pages/<page>.md`. Table of all routes: `audit.md`. Screenshots: `screenshots/before/` and `screenshots/after/` (`<page>-<width>.jpg`, 29 MB each).

## What changed app-wide (shared components; see `shared-components.md`)

| Area | Before | After |
|---|---|---|
| Sidebar | Open from 768px: tablets had ~450px for the page; dashboard amounts cut to "-৬…" | Slide-out menu below 1024px (focus moves in, Tab stays inside, Esc/backdrop/link close it) |
| Header buttons | 36px | 44px below 1024px |
| Form fields | 14px text: iPhone zooms in on every tap | At least 16px below 1024px (global rule, covers hand-written inputs) |
| Tabs | Wrapped into 2–3 rows on phones | One row that scrolls sideways; 44px tall |
| Dialogs | Centered box; Save button off-screen on phones until you scroll | Bottom sheet on phones; Save/Cancel pinned to the bottom at every size |
| Tables | Scrolled sideways inside their frame on phones/tablets (34 pages), and in half-width panels on laptops | Automatic cards (title, headline amount, labelled values, actions), also when a table doesn't fit its panel; printouts keep the real table |
| Hand-written line tables (returns, adjustments, ledgers, audit log) | Scrolled sideways | `table-stack`: each row a labelled card below 1024px |
| ⓘ help tooltips | Invisible bubble widened the product form by 46px at **every** size | Not rendered until shown; opens towards the side with room |
| Grids | 76 grids could grow past the screen (e.g. `/labels` 11px too wide at 360) | Explicit single column on small screens |
| Page padding | 32px from 640px | 16 / 24 / 32px; tighter top/bottom on laptops so work screens fit 1366×768 |

## Pages fixed: before → after

| Page | Mobile | Tablet | Laptop / Desktop |
|---|---|---|---|
| `/pos` | Products and cart stacked, page wider than screen → Products/Cart switch, sticky total + Pay bar, customer & discount bottom sheet | Same as mobile in portrait | Unchanged layout; fits 1366×768 |
| `/products/new`, `/products/:id` | 38px too wide → fits | 67px too wide at 768 → fits | 46px too wide at 1366–1920, edit page scrolled 8px at 1366 → fits exactly |
| `/products` | 564px wide at 360, table scroll → cards, tabs scroll | 836px at 768 → cards | unchanged |
| `/inventory` | 863px wide → cards (quantity as headline) | 1135px wide at 768/1024 → cards | unchanged |
| `/plan` | 815px wide → fits, cards | 1087px wide at 768/1024 → fits | unchanged |
| `/accounts` | 521px wide, table scroll → stacked payment methods | 793px at 768 → fits | unchanged |
| `/settings/branches`, `/settings/registers` | 420–450px wide → fits, cards | table scroll at 768 → cards | unchanged |
| `/expenses/categories` | 383px wide → fits | – | – |
| `/sales/:id/return` | table scroll → one card per line (sold, returnable, quantity, condition, refund) | 797px wide at 768 → fits | unchanged |
| `/sales`, `/sales/:id` | 481px wide, 10-column table → cards, folded filters | cards | Table from 1280 (sales list) / 1024; low-priority columns from 1536 |
| `/` dashboard | table scroll → cards | amounts cut "-৬…" at 768 → full amounts | amounts cut at 1366 → wrap; 6 KPIs per row only from 1536 |
| 34 list/report/detail pages (purchases, suppliers, customers, inventory movements/adjustments/transfers, shifts, accounts book, expenses/incomes categories, sales/purchase returns, warranty lookup, reports: sales, purchases, inventory, payments, trial balance, supplier dues, discounts, product movement; journals, audit log, admin tenants…) | table scrolled sideways in its frame → cards | same | unchanged (tables) |

Earlier in this session (before the full audit), at your request: the POS screen, the customer & discount bottom sheet, product images setting for cart/search, cart serial-number badges, the sale detail and sales list pages, and "Print without returned items".

## Needs your decision
See `decisions.md`:
1. `table-stack` rows lose table semantics for some screen readers on small screens (VoiceOver). Alternative: convert those 7 hand-written tables to `DataTable`.
2. Small 2–3-column tables kept as tables on phones (profit report, statements, notification matrix, admin plan table, CSV import preview, label sheet). Want them as cards too?
3. ~~No Prettier config~~ Added `frontend/.prettierrc` (single quotes, no semicolons, width 160). Whole `src` then formatted with it.

## Commands
- Nothing special: Vite picks the changes up live. For production run `cd frontend && npm run build` (verified: builds cleanly; the >500 kB chunk warning was there before).
- Optional checks: `cd frontend && npx vitest run --pool=threads --maxWorkers=2` (210 tests), `npx tsc -b --noEmit`, `npx oxlint`.
- Nothing is committed. 100 files changed under `frontend/src/` plus this `progress/` folder (58 MB of screenshots; consider leaving `progress/screenshots/` out of git).
