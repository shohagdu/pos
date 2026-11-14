# /accounts/:id

- Page type: Detail
- Audited URL: `/accounts/11f1cab9-ed51-475a-ab00-bc542fed17aa` (Bengali)
- Status: **Fixed**

## Issues found (before)

- **Mobile**: Warn
  - 360: table scrolls sideways in its frame (635/326)
  - 390: table scrolls sideways in its frame (635/356)
- **Tablet**: Warn
  - 768: table scrolls sideways in its frame (635/446)
- **Laptop**: Pass
- **Desktop**: Pass
- Tap targets under 40px at 360px (incl. app header): 6; inputs under 16px at 390px: 4
- App-wide (fixed in shared components): sidebar took ~260px on tablets, 36px header buttons, 14px inputs (iOS zoom), wrapping tabs.

## Changes

- Shared: app shell drawer below lg, 44px header targets, 16px inputs below lg, scrolling tabs, Dialog bottom sheet on phones, Card padding, DataTable automatic cards, grid-cols-1 bases, InfoTip (see `../shared-components.md`).
- Page: Account book uses table-stack (opening balance and totals stay label + amount). Files: features/money/AccountBookPage.tsx.

## Verification (after)

- **Mobile**: Pass
- **Tablet**: Pass
- **Laptop**: Pass
- **Desktop**: Pass
- Tap targets under 40px at 360px: 2 (before 6); inputs under 16px at 390px: 0 (before 4)
- Screenshots: `../screenshots/before/` and `../screenshots/after/` (file prefix `accounts-account`)
