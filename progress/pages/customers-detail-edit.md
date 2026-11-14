# /customers/:id/edit

- Page type: Form
- Audited URL: `/customers/01a17389-c81c-76ec-9bb5-83d2246c37ba/edit` (Bengali)
- Status: **Passed**

## Issues found (before)

- **Mobile**: Pass
- **Tablet**: Pass
- **Laptop**: Pass
- **Desktop**: Pass
- Tap targets under 40px at 360px (incl. app header): 6; inputs under 16px at 390px: 8
- App-wide (fixed in shared components): sidebar took ~260px on tablets, 36px header buttons, 14px inputs (iOS zoom), wrapping tabs.

## Changes

- Shared: app shell drawer below lg, 44px header targets, 16px inputs below lg, scrolling tabs, Dialog bottom sheet on phones, Card padding, DataTable automatic cards, grid-cols-1 bases, InfoTip (see `../shared-components.md`).

## Verification (after)

- **Mobile**: Pass
- **Tablet**: Pass
- **Laptop**: Pass
- **Desktop**: Pass
- Tap targets under 40px at 360px: 2 (before 6); inputs under 16px at 390px: 0 (before 8)
- Screenshots: `../screenshots/before/` and `../screenshots/after/` (file prefix `customers-customer-edit`)
