# /products/:id

- Page type: Form
- Audited URL: `/products/01a1d51b-35e3-7361-b48a-1c2b6e508085` (Bengali)
- Status: **Fixed**

## Issues found (before)

- **Mobile**: Fail
  - 360: page is 398px wide (sideways scroll)
  - 390: page is 398px wide (sideways scroll)
- **Tablet**: Fail
  - 768: page is 835px wide (sideways scroll)
- **Laptop**: Fail
  - 1366: page is 1412px wide (sideways scroll)
  - 1366: scrolls at 1366×768 (776px)
- **Desktop**: Fail
  - 1440: page is 1486px wide (sideways scroll)
  - 1920: page is 1966px wide (sideways scroll)
- Tap targets under 40px at 360px (incl. app header): 35; inputs under 16px at 390px: 17
- App-wide (fixed in shared components): sidebar took ~260px on tablets, 36px header buttons, 14px inputs (iOS zoom), wrapping tabs.

## Changes

- Shared: app shell drawer below lg, 44px header targets, 16px inputs below lg, scrolling tabs, Dialog bottom sheet on phones, Card padding, DataTable automatic cards, grid-cols-1 bases, InfoTip (see `../shared-components.md`).
- Page: Same as /products/new.

## Verification (after)

- **Mobile**: Pass
- **Tablet**: Pass
- **Laptop**: Pass
- **Desktop**: Pass
- Tap targets under 40px at 360px: 31 (before 35); inputs under 16px at 390px: 0 (before 17)
- Screenshots: `../screenshots/before/` and `../screenshots/after/` (file prefix `products-product`)
