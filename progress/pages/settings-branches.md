# /settings/branches

- Page type: Settings form
- Audited URL: `/settings/branches` (Bengali)
- Status: **Fixed**

## Issues found (before)

- **Mobile**: Fail
  - 360: page is 419px wide (sideways scroll)
  - 390: page is 420px wide (sideways scroll)
  - 360: table scrolls sideways in its frame (496/326)
  - 390: table scrolls sideways in its frame (496/356)
- **Tablet**: Warn
  - 768: table scrolls sideways in its frame (496/446)
- **Laptop**: Pass
- **Desktop**: Pass
- Tap targets under 40px at 360px (incl. app header): 13; inputs under 16px at 390px: 1
- App-wide (fixed in shared components): sidebar took ~260px on tablets, 36px header buttons, 14px inputs (iOS zoom), wrapping tabs.

## Changes

- Shared: app shell drawer below lg, 44px header targets, 16px inputs below lg, scrolling tabs, Dialog bottom sheet on phones, Card padding, DataTable automatic cards, grid-cols-1 bases, InfoTip (see `../shared-components.md`).

## Verification (after)

- **Mobile**: Pass
- **Tablet**: Pass
- **Laptop**: Pass
- **Desktop**: Pass
- Tap targets under 40px at 360px: 9 (before 13); inputs under 16px at 390px: 0 (before 1)
- Screenshots: `../screenshots/before/` and `../screenshots/after/` (file prefix `settings-branches`)
