# /reports/product-movement

- Page type: Report
- Audited URL: `/reports/product-movement` (Bengali)
- Status: **Fixed**

## Issues found (before)

- **Mobile**: Warn
  - 360: table scrolls sideways in its frame (875/326)
  - 390: table scrolls sideways in its frame (875/356)
- **Tablet**: Warn
  - 768: table scrolls sideways in its frame (875/446)
  - 1024: table scrolls sideways in its frame (875/702)
- **Laptop**: Pass
- **Desktop**: Pass
- Tap targets under 40px at 360px (incl. app header): 5; inputs under 16px at 390px: 5
- App-wide (fixed in shared components): sidebar took ~260px on tablets, 36px header buttons, 14px inputs (iOS zoom), wrapping tabs.

## Changes

- Shared: app shell drawer below lg, 44px header targets, 16px inputs below lg, scrolling tabs, Dialog bottom sheet on phones, Card padding, DataTable automatic cards, grid-cols-1 bases, InfoTip (see `../shared-components.md`).

## Verification (after)

- **Mobile**: Pass
- **Tablet**: Pass
- **Laptop**: Pass
- **Desktop**: Pass
- Tap targets under 40px at 360px: 1 (before 5); inputs under 16px at 390px: 0 (before 5)
- Screenshots: `../screenshots/before/` and `../screenshots/after/` (file prefix `reports-product-movement`)
