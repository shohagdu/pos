# /accounting/journals/:id

- Page type: Detail
- Audited URL: `/accounting/journals/01a1d843-8b8b-77c9-9545-b8b472ad72b0` (Bengali)
- Status: **Fixed**

## Issues found (before)

- **Mobile**: Warn
  - 360: table scrolls sideways in its frame (377/326)
  - 390: table scrolls sideways in its frame (377/356)
- **Tablet**: Pass
- **Laptop**: Pass
- **Desktop**: Pass
- Tap targets under 40px at 360px (incl. app header): 9; inputs under 16px at 390px: 1
- App-wide (fixed in shared components): sidebar took ~260px on tablets, 36px header buttons, 14px inputs (iOS zoom), wrapping tabs.

## Changes

- Shared: app shell drawer below lg, 44px header targets, 16px inputs below lg, scrolling tabs, Dialog bottom sheet on phones, Card padding, DataTable automatic cards, grid-cols-1 bases, InfoTip (see `../shared-components.md`).

## Verification (after)

- **Mobile**: Pass
- **Tablet**: Pass
- **Laptop**: Pass
- **Desktop**: Pass
- Tap targets under 40px at 360px: 5 (before 9); inputs under 16px at 390px: 0 (before 1)
- Screenshots: `../screenshots/before/` and `../screenshots/after/` (file prefix `accounting-journals-journal`)
