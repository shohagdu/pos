# /audit

- Page type: List / table
- Audited URL: `/audit` (Bengali)
- Status: **Fixed**

## Issues found (before)

- **Mobile**: Warn
  - 360: table scrolls sideways in its frame (809/326)
  - 390: table scrolls sideways in its frame (809/356)
- **Tablet**: Warn
  - 768: table scrolls sideways in its frame (809/446)
  - 1024: table scrolls sideways in its frame (809/702)
- **Laptop**: Pass
- **Desktop**: Pass
- Tap targets under 40px at 360px (incl. app header): 55; inputs under 16px at 390px: 6
- App-wide (fixed in shared components): sidebar took ~260px on tablets, 36px header buttons, 14px inputs (iOS zoom), wrapping tabs.

## Changes

- Shared: app shell drawer below lg, 44px header targets, 16px inputs below lg, scrolling tabs, Dialog bottom sheet on phones, Card padding, DataTable automatic cards, grid-cols-1 bases, InfoTip (see `../shared-components.md`).
- Page: Audit table uses table-stack; expand button 44px. Files: features/audit/AuditLogPage.tsx.

## Verification (after)

- **Mobile**: Pass
- **Tablet**: Pass
- **Laptop**: Pass
- **Desktop**: Pass
- Tap targets under 40px at 360px: 51 (before 55); inputs under 16px at 390px: 0 (before 6)
- Screenshots: `../screenshots/before/` and `../screenshots/after/` (file prefix `audit`)
