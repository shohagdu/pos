# /customers/:id

- Page type: Detail
- Audited URL: `/customers/01a17389-c81c-76ec-9bb5-83d2246c37ba` (Bengali)
- Status: **Fixed**

## Issues found (before)

- **Mobile**: Warn
  - 360: table scrolls sideways in its frame (417/326)
  - 390: table scrolls sideways in its frame (417/356)
- **Tablet**: Pass
- **Laptop**: Pass
- **Desktop**: Pass
- Tap targets under 40px at 360px (incl. app header): 10; inputs under 16px at 390px: 4
- App-wide (fixed in shared components): sidebar took ~260px on tablets, 36px header buttons, 14px inputs (iOS zoom), wrapping tabs.

## Changes

- Shared: app shell drawer below lg, 44px header targets, 16px inputs below lg, scrolling tabs, Dialog bottom sheet on phones, Card padding, DataTable automatic cards, grid-cols-1 bases, InfoTip (see `../shared-components.md`).
- Page: Statement table uses table-stack; name is the card title in lists. Files: features/customers/CustomerTabs.tsx, CustomersPage.tsx.

## Verification (after)

- **Mobile**: Pass
- **Tablet**: Pass
- **Laptop**: Pass
- **Desktop**: Pass
- Tap targets under 40px at 360px: 2 (before 10); inputs under 16px at 390px: 0 (before 4)
- Screenshots: `../screenshots/before/` and `../screenshots/after/` (file prefix `customers-customer`)
