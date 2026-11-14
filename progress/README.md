# Responsive work: progress

**Last updated:** 2026-11-14, end of session 1
**Current step:** Done. Steps 1–5 complete; see `FINAL_REPORT.md`. Next: your decisions in `decisions.md`.

## Status

| | Count |
|---|---|
| Total routes audited | 109 |
| Passed (no page-specific issue) | 62 |
| Fixed | 47 |
| In progress | 0 |
| Pending | 0 |
| Blocked | 0 |

Every page, including the "Passed" ones, also has the app-wide issues listed under step 3a; those are fixed once in shared components.

## Audit summary (step 2)
- Measured automatically in Bengali at 360, 390, 768, 1024, 1366×768, 1440 and 1920 with headless Chrome (script: see "How to re-run"). Before screenshots: `screenshots/before/<page>-<width>.jpg`.
- **Page scrolls sideways (10):** `/products` (phones/tablets), `/products/new` and `/products/:id` (phones, tablets **and laptops/desktops**: 46px too wide), `/plan` and `/inventory` (up to 1024), `/accounts`, `/settings/branches`, `/settings/registers`, `/expenses/categories`, `/sales/:id/return` (768).
- **Tables scroll sideways inside their frame on phones/tablets (34 pages):** fixed by one shared change: `DataTable` shows cards below 1024px.
- **App-wide:**
  - the sidebar stays open from 768px, leaving tablets ~450px of content; dashboard amounts are cut off with "…" at 768
  - header buttons are 36px tall, under the 44px minimum
  - inputs use 14px text, so iPhones zoom in when one is tapped
  - tabs wrap onto several lines on phones
  - the dialog close button is small
- Not measurable automatically: stock-count and bank-reconciliation detail pages (no records in the dev data), print/receipt views (fixed paper widths), dialogs (checked through the shared `Dialog`).

## Plan / next
1. ~~3a Shared components~~ done: see `shared-components.md` (12 items: shell drawer, header targets, 16px inputs, tabs, dialog sheet + pinned actions, card padding, DataTable auto cards, table-stack, InfoTip, grid bases, table frames, dashboard KPIs).
2. ~~3b Failing pages~~ done: products list, product form, plan, inventory, accounts, settings branches/registers, expense categories, sale return form, labels.
3. ~~3c Table pages~~ done by the shared DataTable change; hand-tuned: customers, stock, sales report (amount choice), products.
4. ~~Step 4 verify~~ done: all 109 routes × 7 sizes clean; 210/210 tests, tsc, build OK.
5. ~~Step 5~~ `FINAL_REPORT.md` written.
6. **Next session:** only the open questions in `decisions.md`; new pages should use `DataTable` (automatic cards) or `table-stack` for hand-written tables.

## How to re-run the audit
The script lives outside the repo (session scratchpad), so a new session must recreate it. It: logs in as 01700000001 / Test@12345 on http://localhost:5173 (dev server + API on 8080), sets `localStorage['pos.lang']='bn'`, visits every route in `audit.md` at each width, and records `max(scrollWidth, innerWidth) > width` (sideways scroll; with phone emulation the browser zooms out, so compare against the emulated width), tables whose frame scrolls, tap targets under 40px, inputs under 16px and page height at 1366×768.
