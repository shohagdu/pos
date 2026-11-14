# /pos

- Page type: POS sale screen
- Audited URL: `/pos` (Bengali)
- Status: **Fixed**

## Issues found (before)

- **Mobile**: Pass
- **Tablet**: Pass
- **Laptop**: Pass
- **Desktop**: Pass
- Tap targets under 40px at 360px (incl. app header): 2; inputs under 16px at 390px: 1
- App-wide (fixed in shared components): sidebar took ~260px on tablets, 36px header buttons, 14px inputs (iOS zoom), wrapping tabs.

## Changes

- Shared: app shell drawer below lg, 44px header targets, 16px inputs below lg, scrolling tabs, Dialog bottom sheet on phones, Card padding, DataTable automatic cards, grid-cols-1 bases, InfoTip (see `../shared-components.md`).
- Page: Products/Cart switch + sticky total/Pay bar below lg; customer & discount bottom sheet (F4/F7 open it); 44px targets; serial-number badges; product images setting also covers cart and search. Files: features/pos/*.

## Verification (after)

- **Mobile**: Pass
- **Tablet**: Pass
- **Laptop**: Pass
- **Desktop**: Pass
- Tap targets under 40px at 360px: 2 (before 2); inputs under 16px at 390px: 0 (before 1)
- Screenshots: `../screenshots/before/` and `../screenshots/after/` (file prefix `pos`)
