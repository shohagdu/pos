# /sales/warranties

- Page type: List / table
- Audited URL: `/sales/warranties` (Bengali)
- Status: **Fixed**

## Issues found (before)

- **Mobile**: Warn
  - 360: table scrolls sideways in its frame (676/326)
  - 390: table scrolls sideways in its frame (676/356)
- **Tablet**: Warn
  - 768: table scrolls sideways in its frame (676/446)
- **Laptop**: Pass
- **Desktop**: Pass
- Tap targets under 40px at 360px (incl. app header): 5; inputs under 16px at 390px: 3
- App-wide (fixed in shared components): sidebar took ~260px on tablets, 36px header buttons, 14px inputs (iOS zoom), wrapping tabs.

## Changes

- Shared: app shell drawer below lg, 44px header targets, 16px inputs below lg, scrolling tabs, Dialog bottom sheet on phones, Card padding, DataTable automatic cards, grid-cols-1 bases, InfoTip (see `../shared-components.md`).

## Verification (after)

- **Mobile**: Pass
- **Tablet**: Pass
- **Laptop**: Pass
- **Desktop**: Pass
- Tap targets under 40px at 360px: 3 (before 5); inputs under 16px at 390px: 0 (before 3)
- Screenshots: `../screenshots/before/` and `../screenshots/after/` (file prefix `sales-warranties`)
