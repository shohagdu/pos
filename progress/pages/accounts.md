# /accounts

- Page type: List / table
- Audited URL: `/accounts` (Bengali)
- Status: **Fixed**

## Issues found (before)

- **Mobile**: Fail
  - 360: page is 521px wide (sideways scroll)
  - 390: page is 521px wide (sideways scroll)
  - 360: table scrolls sideways in its frame (597/326)
  - 390: table scrolls sideways in its frame (597/356)
- **Tablet**: Fail
  - 768: page is 793px wide (sideways scroll)
  - 768: table scrolls sideways in its frame (597/446)
- **Laptop**: Pass
- **Desktop**: Pass
- Tap targets under 40px at 360px (incl. app header): 5; inputs under 16px at 390px: 7
- App-wide (fixed in shared components): sidebar took ~260px on tablets, 36px header buttons, 14px inputs (iOS zoom), wrapping tabs.

## Changes

- Shared: app shell drawer below lg, 44px header targets, 16px inputs below lg, scrolling tabs, Dialog bottom sheet on phones, Card padding, DataTable automatic cards, grid-cols-1 bases, InfoTip (see `../shared-components.md`).
- Page: Payment methods table uses table-stack. Files: features/money/PaymentMethodsSection.tsx.

## Verification (after)

- **Mobile**: Pass
- **Tablet**: Pass
- **Laptop**: Pass
- **Desktop**: Pass
- Tap targets under 40px at 360px: 4 (before 5); inputs under 16px at 390px: 0 (before 7)
- Screenshots: `../screenshots/before/` and `../screenshots/after/` (file prefix `accounts`)
