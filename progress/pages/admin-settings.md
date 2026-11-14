# /admin/settings

- Page type: Settings form
- Audited URL: `/admin/settings` (Bengali)
- Status: **Passed**

## Issues found (before)

- **Mobile**: Pass
- **Tablet**: Pass
- **Laptop**: Pass
- **Desktop**: Pass
- Tap targets under 40px at 360px (incl. app header): 8; inputs under 16px at 390px: 6
- App-wide (fixed in shared components): sidebar took ~260px on tablets, 36px header buttons, 14px inputs (iOS zoom), wrapping tabs.

## Changes

- Shared: app shell drawer below lg, 44px header targets, 16px inputs below lg, scrolling tabs, Dialog bottom sheet on phones, Card padding, DataTable automatic cards, grid-cols-1 bases, InfoTip (see `../shared-components.md`).

## Verification (after)

- **Mobile**: Pass
- **Tablet**: Pass
- **Laptop**: Pass
- **Desktop**: Pass
- Tap targets under 40px at 360px: 8 (before 8); inputs under 16px at 390px: 0 (before 6)
- Screenshots: `../screenshots/before/` and `../screenshots/after/` (file prefix `admin-settings`)
