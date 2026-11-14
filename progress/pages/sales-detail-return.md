# /sales/:id/return

- Page type: Form
- Audited URL: `/sales/01a1d52b-4ff8-733e-824d-4c183c043fd3/return` (Bengali)
- Status: **Fixed**

## Issues found (before)

- **Mobile**: Warn
  - 360: table scrolls sideways in its frame (557/326)
  - 390: table scrolls sideways in its frame (557/356)
- **Tablet**: Fail
  - 768: page is 797px wide (sideways scroll)
  - 768: table scrolls sideways in its frame (557/446)
- **Laptop**: Pass
- **Desktop**: Pass
- Tap targets under 40px at 360px (incl. app header): 6; inputs under 16px at 390px: 5
- App-wide (fixed in shared components): sidebar took ~260px on tablets, 36px header buttons, 14px inputs (iOS zoom), wrapping tabs.

## Changes

- Shared: app shell drawer below lg, 44px header targets, 16px inputs below lg, scrolling tabs, Dialog bottom sheet on phones, Card padding, DataTable automatic cards, grid-cols-1 bases, InfoTip (see `../shared-components.md`).
- Page: Line table uses table-stack (one card per line with labelled sold / returnable / quantity / condition / refund). Files: features/sales/SaleReturnPage.tsx.

## Verification (after)

- **Mobile**: Pass
- **Tablet**: Pass
- **Laptop**: Pass
- **Desktop**: Pass
- Tap targets under 40px at 360px: 2 (before 6); inputs under 16px at 390px: 0 (before 5)
- Screenshots: `../screenshots/before/` and `../screenshots/after/` (file prefix `sales-sale-return`)
