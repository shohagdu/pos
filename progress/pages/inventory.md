# /inventory

- Page type: List / table
- Audited URL: `/inventory` (Bengali)
- Status: **Fixed**

## Issues found (before)

- **Mobile**: Fail
  - 360: page is 863px wide (sideways scroll)
  - 390: page is 863px wide (sideways scroll)
  - 360: table scrolls sideways in its frame (898/326)
  - 390: table scrolls sideways in its frame (898/356)
- **Tablet**: Fail
  - 768: page is 1135px wide (sideways scroll)
  - 1024: page is 1135px wide (sideways scroll)
  - 768: table scrolls sideways in its frame (898/446)
  - 1024: table scrolls sideways in its frame (898/702)
- **Laptop**: Pass
- **Desktop**: Pass
- Tap targets under 40px at 360px (incl. app header): 11; inputs under 16px at 390px: 5
- App-wide (fixed in shared components): sidebar took ~260px on tablets, 36px header buttons, 14px inputs (iOS zoom), wrapping tabs.

## Changes

- Shared: app shell drawer below lg, 44px header targets, 16px inputs below lg, scrolling tabs, Dialog bottom sheet on phones, Card padding, DataTable automatic cards, grid-cols-1 bases, InfoTip (see `../shared-components.md`).
- Page: Quantity is the card headline; table becomes cards whenever it does not fit (1024 with sidebar). Files: features/inventory/StockPage.tsx.

## Verification (after)

- **Mobile**: Pass
- **Tablet**: Pass
- **Laptop**: Pass
- **Desktop**: Pass
- Tap targets under 40px at 360px: 28 (before 11); inputs under 16px at 390px: 0 (before 5)
- Screenshots: `../screenshots/before/` and `../screenshots/after/` (file prefix `inventory`)
