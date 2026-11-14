# Decisions

## Made (no action needed unless you disagree)
- **Breakpoints:** Tailwind defaults (sm 640, md 768, lg 1024, xl 1280, 2xl 1536); the project defines none of its own. Test sizes: 360, 390, 768, 1024, 1366×768, 1440, 1920.
- **Audit language:** Bengali (longer strings). English was spot-checked during the earlier POS/sales work.
- **Sidebar:** a slide-out menu below **1024px** (was 768px). On tablets the open sidebar left only ~450px for the page. The collapse-to-icons button still works from 1024px up.
- **Tables on phones/tablets → cards, one approach app-wide** (the skill asks to pick one):
  - Shared `DataTable`: automatic cards below 1024px, and on wider screens whenever the table doesn't fit its own frame (e.g. half-width dashboard panels, inventory with the sidebar at 1024).
  - Hand-written tables with inputs per row: the `table-stack` CSS (row → labelled card).
  - Printouts always get the real table.
- **Very wide lists** (sales list): cards up to 1279px, table from 1280px; low-priority columns appear only from 1536px (Paid, Cashier on the sales list) and stay in the cards and CSV.
- **Form fields** are at least 16px below 1024px (prevents iOS zoom). This is a global CSS rule so hand-written inputs are covered too.
- **Dialogs** are bottom sheets on phones; a form's final Save/Cancel row is pinned to the bottom of the dialog at every size.
- **POS on phones/tablets portrait:** Products / Cart switch, sticky total + Pay bar, customer & invoice discount in a bottom sheet (F4/F7 open it). Keyboard shortcuts unchanged.
- **Product images setting** covers product cards, cart lines and barcode search results, and no longer depends on the product panel (asked for earlier in this session).
- Sale detail: **"Print without returned items"** button (asked for earlier in this session).

## Needs your decision
1. **`table-stack` and screen readers.** Turning table rows into blocks on small screens makes some screen readers (Safari/VoiceOver) stop announcing them as a table; the labels are still read (`data-label` text is visible). If you rely on VoiceOver on iPhone/iPad for these pages, the alternative is to convert those hand-written tables to `DataTable` (more work, same look). Affects: sale/purchase return forms, stock adjustment form, payment methods, account book, customer statement, audit log.
2. **Tables kept as tables on phones** because they are 2–3 columns and fit: profit report, P&L / balance sheet statements, notification settings matrix, admin dashboard plan table, CSV import preview, label print sheet. Say if you want any of them as cards anyway.
3. ~~Prettier config~~ **Done (2026-11-14):** `frontend/.prettierrc` = single quotes, no semicolons, print width 160. `npx prettier --write src` was then run on the whole `src` (163 files reformatted; tests, tsc, lint and build still pass).

## Not layout-related (logged, not fixed)
- `src/features/auth/auth.test.tsx` › "sends an unverified number to the code screen instead of signing in" failed once in the full parallel run and passes when run alone (`npx vitest run src/features/auth -t "unverified number"`). Timing-sensitive under load (machine is heavily loaded); no auth code was changed. It passed in the final full run (210/210).
