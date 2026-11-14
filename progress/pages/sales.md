# /sales

- Page type: List / table
- Audited URL: `/sales` (Bengali)
- Status: **Fixed**

## Issues found (before)

- **Mobile**: Pass
- **Tablet**: Pass
- **Laptop**: Pass
- **Desktop**: Pass
- Tap targets under 40px at 360px (incl. app header): 9; inputs under 16px at 390px: 2
- App-wide (fixed in shared components): sidebar took ~260px on tablets, 36px header buttons, 14px inputs (iOS zoom), wrapping tabs.

## Changes

- Shared: app shell drawer below lg, 44px header targets, 16px inputs below lg, scrolling tabs, Dialog bottom sheet on phones, Card padding, DataTable automatic cards, grid-cols-1 bases, InfoTip (see `../shared-components.md`).
- Page: Hand-made cards below xl, filters fold behind a Filters button below lg, Paid/Cashier columns from 2xl. Files: features/sales/SalesPage.tsx.

## Verification (after)

- **Mobile**: Pass
- **Tablet**: Pass
- **Laptop**: Pass
- **Desktop**: Pass
- Tap targets under 40px at 360px: 6 (before 9); inputs under 16px at 390px: 0 (before 2)
- Screenshots: `../screenshots/before/` and `../screenshots/after/` (file prefix `sales`)
