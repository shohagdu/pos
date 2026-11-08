# Barcode Label Printing — Feature Specification

> Reusable spec for the barcode label feature built in JSA MULTISERVICES (requirement **PROD-R05**, roadmap 6.4).
> It describes what the feature does, the data it needs, the API, the barcode encoder (no library), the
> print layout and the known pitfalls, so it can be rebuilt in another project.
>
> Reference implementation (this repo):
>
> | Part | File |
> |---|---|
> | Barcode encoder (EAN-13 / UPC-A / EAN-8 / Code 128) | `frontend/src/lib/barcode.ts` (+ `barcode.test.ts`) |
> | Barcode SVG component | `frontend/src/features/labels/Barcode.tsx` |
> | Label stock & pagination | `frontend/src/features/labels/layouts.ts` |
> | Copies from a purchase | `frontend/src/features/labels/copies.ts` |
> | Label page (`/labels`) | `frontend/src/features/labels/LabelPrintPage.tsx` |
> | Texts (en / bn / it) | `frontend/src/features/labels/i18n/*.json` |
> | API client | `frontend/src/api/catalog.ts` (`labels`, `generateBarcodes`, `nextCodes`) |
> | Backend: label facts + in-store codes | `backend/crates/catalog/src/labels.rs` |
> | Backend: barcode validation | `backend/crates/catalog/src/barcode.rs` |
> | Backend: routes | `backend/crates/api/src/routes/catalog.rs` |
> | Table | `product_barcodes` in `backend/migrations/0009_catalog.sql` |

---

## 1. Goals

1. Print price/barcode stickers for products on **thermal label rolls** and **A4 sticker sheets** from the browser (no printer SDK, no PDF library).
2. Label goods **right after a purchase**: one click from a purchase prints one label per unit received.
3. Give products that have **no barcode** a valid, unique **in-store EAN-13** so they can be scanned at the POS.
4. Draw barcodes **without a third-party library** — a small, tested encoder that outputs SVG.

Non-goals: QR codes, direct ZPL/TSPL printer commands, server-side PDF generation.

---

## 2. User flows

| Entry point | URL | Initial copies |
|---|---|---|
| Purchase detail → "Print barcodes" | `/labels?purchase=<purchase_id>` | Units in the shop for each line: `received − returned` once goods have arrived, the ordered quantity before that. Fractions round **up** (1.5 kg → 2 labels). |
| Product list → "Print barcode" (per row) | `/labels?variants=<variant_id>` | 1 |
| Several products | `/labels?variants=<id>,<id>,…` | 1 each |
| Catalog nav → "Barcode labels" | `/labels` | Empty; add by search/scan |

On the page the user can:

- **Search or scan** a product to add it (scanning the same product again adds +1 copy).
- Set **copies per product** (0–999), remove rows.
- **Generate barcodes** for products without one (needs `products.update`).
- Choose the **label stock** (layout) and what to **show** on the label: business name, product name, price, SKU.
- **SKU fallback**: products without a barcode print their SKU as Code 128 (the POS also finds products by SKU). When off, those products are skipped.
- On A4 sheets, **Start at label N** to reuse a partly used sheet (skips the first N−1 positions).
- See a **live preview** that is exactly what will be printed, then **Print** (`window.print()`).

Settings (layout + show options) are remembered per tenant in `localStorage` key `pos.labels.<tenantId>`. "Start at" is not remembered (it is per sheet).

Limits: at most **500 products per request** (backend) and **5,000 labels per print** (frontend; the Print button is disabled above that).

---

## 3. Data model

Barcodes live in their own table so a variant can have several (e.g. manufacturer code + in-store code, or one per pack unit):

```sql
CREATE TABLE product_barcodes (
    tenant_id   BINARY(16)  NOT NULL,
    id          BINARY(16)  NOT NULL,
    variant_id  BINARY(16)  NOT NULL,
    barcode     VARCHAR(64) NOT NULL,
    symbology   VARCHAR(10),                 -- ean13 | ean8 | upca | code128 | qr | other
    unit_id     BINARY(16),                  -- optional: barcode of a pack unit
    is_primary  BOOLEAN     NOT NULL DEFAULT FALSE,
    created_at  DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (tenant_id, id),
    UNIQUE KEY uq_barcodes (tenant_id, barcode),          -- a code belongs to one product per shop
    KEY idx_barcodes_variant (tenant_id, variant_id),
    FOREIGN KEY (tenant_id, variant_id) REFERENCES product_variants (tenant_id, id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, unit_id)    REFERENCES units (tenant_id, id),
    CHECK (symbology IS NULL OR symbology IN ('ean13','ean8','upca','code128','qr','other'))
);
```

The label uses the variant's **primary** barcode: `ORDER BY is_primary DESC, created_at, barcode LIMIT 1`.

### Barcode validation (on product save)

- Trim; 1–64 printable ASCII characters, no spaces.
- Symbology detected by shape: all digits and length 13 → `ean13`, 12 → `upca`, 8 → `ean8`, otherwise `other`.
- For `ean13`/`upca`/`ean8` the **GS1 check digit must be correct**, otherwise error `INVALID_CHECK_DIGIT`.
- Duplicate within the tenant → `DUPLICATE_BARCODE` (from `uq_barcodes`).

---

## 4. In-store barcodes (GS1 prefix `20`)

Products without a manufacturer barcode get an EAN-13 from the GS1 **restricted-circulation** range `20`, which is valid only inside the shop and never clashes with real retail codes:

```
2 0 | n n n n n n n n n n | c
prefix   10-digit running number   GS1 check digit
```

- The running number continues from the tenant's **highest existing** `20…` 13-digit code (`MAX(barcode)`), starting at 1 → first code `2000000000015`.
- When generating for real, the range is read with `SELECT … FOR UPDATE` so two terminals do not take the same number; `uq_barcodes` catches any remaining race (`409 DUPLICATE_BARCODE`, retry).
- The product form can **suggest** the next code without reserving it (`GET /products/next-codes`); on save the form sends `generate_barcode: true` and the server takes the next free number inside the save transaction.
- Each generated code bumps the product `version` (open edit forms must reload) and writes an audit entry `product.barcode_generated` with `{ sku, barcode }`.

GS1 check digit (same in Rust and TypeScript): weights 3, 1, 3, 1 … **from the right** over the digits before the check digit; `check = (10 − sum mod 10) mod 10`.

```rust
fn ean13_check(body: &str) -> u32 {
    let sum: u32 = body.bytes().rev().enumerate()
        .map(|(i, b)| u32::from(b - b'0') * if i % 2 == 0 { 3 } else { 1 })
        .sum();
    (10 - sum % 10) % 10
}

fn in_store_code(n: u64) -> String {
    let body = format!("20{n:010}");
    format!("{body}{}", ean13_check(&body))
}
// in_store_code(1) == "2000000000015"
```

---

## 5. API

All endpoints are tenant-scoped and need a bearer token.

### `POST /api/v1/labels` — label facts

Permission: `products.view`.

```json
// request
{ "variant_ids": ["<uuid>", "<uuid>"] }

// 200 response — in the order asked; unknown/deleted variants left out; duplicates once
[
  {
    "variant_id": "…", "product_id": "…",
    "product_name": "Lux Soap", "product_name_bn": "লাক্স সাবান", "variant_name": "100 g",
    "sku": "P0001",
    "barcode": "2000000000015",        // primary barcode, or null
    "selling_price": "45.00",          // decimal as string
    "unit_short_name": "pcs"
  }
]
```

Errors: `400` field `variant_ids` — `REQUIRED` (empty) or `TOO_MANY` (> 500).

### `POST /api/v1/labels/generate-barcodes` — fill in missing barcodes

Permission: `products.update`. Same request body. Every listed variant **without** a barcode gets a new in-store EAN-13 (primary, symbology `ean13`); variants that already have one keep it. Returns the same list as `/labels`, with the new codes. `409 DUPLICATE_BARCODE` on a lost race — retry.

### `GET /api/v1/products/next-codes` — suggestions for the product form

Permission: `products.create` or `products.update`. Returns `{ "sku": "P0042" | null, "barcode": "2000000000428" }`. Not reserved.

---

## 6. Barcode encoder (frontend, no library)

`encodeBarcode(value)` picks the symbology retail scanners expect:

| Input | Symbology |
|---|---|
| 13 digits with a valid check digit | EAN-13 |
| 12 digits with a valid check digit | UPC-A (drawn as EAN-13 with a leading `0`) |
| 8 digits with a valid check digit | EAN-8 |
| Anything else in ASCII 32–126 (SKUs, free codes) | Code 128 (set C for all-digit values of ≥ 4 digits, a trailing odd digit via Code B; set B otherwise) |
| Empty / non-ASCII | `null` → the label shows the plain text instead |

Output is a string of **modules** (`1` = bar, `0` = space, one module wide each), quiet zones excluded, plus the human-readable text.

Full source (copy as-is; ~120 lines):

```ts
export type Symbology = 'ean13' | 'upca' | 'ean8' | 'code128'

export interface EncodedBarcode {
  symbology: Symbology
  /** `1`/`0` per module, quiet zones excluded. */
  modules: string
  /** Text printed under the bars. */
  text: string
}

// ---------------------------------------------------------------- EAN / UPC
const EAN_L = ['0001101', '0011001', '0010011', '0111101', '0100011', '0110001', '0101111', '0111011', '0110111', '0001011']
const EAN_R = EAN_L.map((p) => [...p].map((b) => (b === '1' ? '0' : '1')).join(''))
const EAN_G = EAN_R.map((p) => [...p].reverse().join(''))
/** Left-half parity per leading EAN-13 digit. */
const EAN_PARITY = ['LLLLLL', 'LLGLGG', 'LLGGLG', 'LLGGGL', 'LGLLGG', 'LGGLLG', 'LGGGLL', 'LGLGLG', 'LGLGGL', 'LGGLGL']

/** GS1 check digit for the digits before it (weights 3, 1 from the right). */
export function gs1CheckDigit(body: string): number {
  let sum = 0
  for (let i = 0; i < body.length; i++) {
    const digit = body.charCodeAt(body.length - 1 - i) - 48
    sum += digit * (i % 2 === 0 ? 3 : 1)
  }
  return (10 - (sum % 10)) % 10
}

const isDigits = (s: string) => /^[0-9]+$/.test(s)
const hasValidCheck = (code: string) => gs1CheckDigit(code.slice(0, -1)) === Number(code.at(-1))

function ean13Modules(code: string): string {
  const parity = EAN_PARITY[Number(code[0])] ?? ''
  let out = '101'
  for (let i = 1; i <= 6; i++) out += (parity[i - 1] === 'L' ? EAN_L : EAN_G)[Number(code[i])]
  out += '01010'
  for (let i = 7; i <= 12; i++) out += EAN_R[Number(code[i])]
  return out + '101'
}

function ean8Modules(code: string): string {
  let out = '101'
  for (let i = 0; i < 4; i++) out += EAN_L[Number(code[i])]
  out += '01010'
  for (let i = 4; i < 8; i++) out += EAN_R[Number(code[i])]
  return out + '101'
}

// ---------------------------------------------------------------- Code 128
/** Bar/space widths of symbols 0–105, then the stop symbol (106). */
const CODE128 = [
  '212222', '222122', '222221', '121223', '121322', '131222', '122213', '122312', '132212', '221213',
  '221312', '231212', '112232', '122132', '122231', '113222', '123122', '123221', '223211', '221132',
  '221231', '213212', '223112', '312131', '311222', '321122', '321221', '312212', '322112', '322211',
  '212123', '212321', '232121', '111323', '131123', '131321', '112313', '132113', '132311', '211313',
  '231113', '231311', '112133', '112331', '132131', '113123', '113321', '133121', '313121', '211331',
  '231131', '213113', '213311', '213131', '311123', '311321', '331121', '312113', '312311', '332111',
  '314111', '221411', '431111', '111224', '111422', '121124', '121421', '141122', '141221', '112214',
  '112412', '122114', '122411', '142112', '142211', '241211', '221114', '413111', '241112', '134111',
  '111242', '121142', '121241', '114212', '124112', '124211', '411212', '421112', '421211', '212141',
  '214121', '412121', '111143', '111341', '131141', '114113', '114311', '411113', '411311', '113141',
  '114131', '311141', '411131', '211412', '211214', '211232', '2331112',
]
const START_B = 104
const START_C = 105
const CODE_B = 100
const STOP = 106

function widthsToModules(widths: string): string {
  let out = ''
  for (let i = 0; i < widths.length; i++) out += (i % 2 === 0 ? '1' : '0').repeat(Number(widths[i]))
  return out
}

/** Symbol values for `text`: set C for digit pairs, set B otherwise. */
function code128Values(text: string): number[] | null {
  if (![...text].every((c) => c >= ' ' && c <= '~')) return null
  if (isDigits(text) && text.length >= 4) {
    const values = [START_C]
    const pairs = text.length - (text.length % 2)
    for (let i = 0; i < pairs; i += 2) values.push(Number(text.slice(i, i + 2)))
    if (pairs < text.length) values.push(CODE_B, text.charCodeAt(pairs) - 32)
    return values
  }
  return [START_B, ...[...text].map((c) => c.charCodeAt(0) - 32)]
}

/** Modules for `text` in Code 128 (start, data, check, stop); `null` outside ASCII 32–126. */
export function code128Modules(text: string): string | null {
  const values = code128Values(text)
  if (!values || text.length === 0) return null
  const check = values.reduce((sum, v, i) => sum + v * Math.max(i, 1), 0) % 103
  return [...values, check, STOP].map((v) => widthsToModules(CODE128[v] ?? '')).join('')
}

// ---------------------------------------------------------------- choice
/** Encodes `value` in the best-fitting symbology, or `null` when it cannot be drawn. */
export function encodeBarcode(raw: string): EncodedBarcode | null {
  const value = raw.trim()
  if (!value) return null
  if (isDigits(value) && hasValidCheck(value)) {
    if (value.length === 13) return { symbology: 'ean13', modules: ean13Modules(value), text: value }
    if (value.length === 12) return { symbology: 'upca', modules: ean13Modules(`0${value}`), text: value }
    if (value.length === 8) return { symbology: 'ean8', modules: ean8Modules(value), text: value }
  }
  const modules = code128Modules(value)
  return modules ? { symbology: 'code128', modules, text: value } : null
}
```

Test vectors worth keeping (from `barcode.test.ts` / `barcode.rs`): valid `4006381333931` (EAN-13), `036000291452` (UPC-A), `96385074` (EAN-8); invalid check digits `4006381333932`, `036000291453`, `96385075`; EAN-13 is always 95 modules, EAN-8 67.

### Rendering as SVG

The modules become one SVG `<path>`; the SVG **stretches to its box** (`preserveAspectRatio="none"`), so the same component prints on a 38 mm roll or an A4 sheet. A **10-module quiet zone** on each side is required for scanners. `shape-rendering="crispEdges"` keeps bars sharp.

```tsx
const QUIET = 10

export function Barcode({ value, className = '', showText = true }: { value: string; className?: string; showText?: boolean }) {
  const encoded = useMemo(() => encodeBarcode(value), [value])
  if (!encoded) return <span className="font-mono text-[0.8em]">{value}</span>
  const { modules } = encoded
  let path = ''
  let i = 0
  while (i < modules.length) {
    if (modules[i] === '1') {
      let w = 1
      while (modules[i + w] === '1') w++
      path += `M${i + QUIET} 0h${w}v1h-${w}z`   // one rect per run of bars
      i += w
    } else i++
  }
  return (
    <div className={`flex min-h-0 flex-col items-stretch ${className}`}>
      <svg role="img" aria-label={value} viewBox={`0 0 ${modules.length + QUIET * 2} 1`}
           preserveAspectRatio="none" shapeRendering="crispEdges" className="min-h-0 w-full flex-1"
           data-symbology={encoded.symbology}>
        <path d={path} fill="#000" />
      </svg>
      {showText && <span className="text-center font-mono leading-none tracking-wider">{encoded.text}</span>}
    </div>
  )
}
```

---

## 7. Label stock (layouts) and pagination

All sizes in millimetres. A **roll** is one label per printed page (that is how thermal printer drivers see it); a **sheet** is a grid on A4.

```ts
export interface LabelLayout {
  id: string
  kind: 'roll' | 'sheet'
  pageWidth: number; pageHeight: number
  columns: number; rows: number
  labelWidth: number; labelHeight: number
  marginTop: number; marginLeft: number
  gapX: number; gapY: number
}
```

| id | Kind | Page | Grid | Label |
|---|---|---|---|---|
| `roll-38x25` (default) | roll | 38 × 25 | 1 × 1 | 38 × 25 |
| `roll-50x25` | roll | 50 × 25 | 1 × 1 | 50 × 25 |
| `roll-50x30` | roll | 50 × 30 | 1 × 1 | 50 × 30 |
| `a4-24` | sheet | 210 × 297 | 3 × 8 | 70 × 37, top margin 0.5 |
| `a4-40` | sheet | 210 × 297 | 4 × 10 | 52.5 × 29.7 |
| `a4-65` | sheet | 210 × 297 | 5 × 13 | 38.1 × 21.2, margins top 10.7 / left 4.7, gapX 2.5 |

Pagination: build the print queue (each row repeated `copies` times; rows without any code dropped unless SKU fallback is on), prepend `skip` empty cells on sheets, then cut into pages of `columns × rows`:

```ts
export function paginate<T>(items: T[], layout: LabelLayout, skip = 0): (T | null)[][] {
  const size = layout.columns * layout.rows
  const offset = layout.kind === 'sheet' ? Math.min(Math.max(0, Math.floor(skip)), size - 1) : 0
  const cells: (T | null)[] = [...Array<null>(offset).fill(null), ...items]
  const pages: (T | null)[][] = []
  for (let i = 0; i < cells.length; i += size) pages.push(cells.slice(i, i + size))
  return pages
}
```

Copies from a purchase line: `received − returned` when status is `received`/`partially_received`, else `quantity`; same variant on several lines is summed; fractions round up; capped at 999.

---

## 8. Printing (browser)

Each page is a fixed-size CSS grid in mm; the preview on screen is the same DOM that prints.

```tsx
<div style={{
  width: `${l.pageWidth}mm`, height: `${l.pageHeight}mm`,
  paddingTop: `${l.marginTop}mm`, paddingLeft: `${l.marginLeft}mm`,
  display: 'grid',
  gridTemplateColumns: `repeat(${l.columns}, ${l.labelWidth}mm)`,
  gridAutoRows: `${l.labelHeight}mm`,
  columnGap: `${l.gapX}mm`, rowGap: `${l.gapY}mm`,
  overflow: 'hidden',
}}>…cells…</div>
```

While the page is open, a `<style>` is injected (and removed on unmount / layout change) so the browser's print dialog uses the label size:

```css
@page { size: 38mm 25mm; margin: 0; }           /* from the chosen layout */
@media print {
  html, body { background: #fff !important; }
  .label-pages > * { break-after: page; }
  .label-pages > *:last-child { break-after: auto; }
}
```

Everything except `.label-pages` is hidden in print (`print:hidden` on the app shell, header, sidebar, settings; `main` gets `print:p-0`).

Label content (top to bottom): business name (optional, uppercase, small) · product name (max 2 lines; Bengali name when the UI language is `bn`) · barcode (fills the remaining height) with its digits · SKU (optional, left) and price (optional, bold, right). Text scales with the label: font size ≈ 9 % of label height, minimum 1.8 mm; padding ≈ 6 % of height (max 1.5 mm) and 5 % of width (max 2 mm).

### Print dialog settings (shown as a hint on the page)

- **Roll:** choose the label printer, paper size = label size, **margins: none**, scale 100 %.
- **Sheet:** A4, **margins: none**, scale 100 %.

### Known pitfall: blank labels from printer margins

If the print dialog or the printer driver adds **any page margin**, the printable area becomes smaller than the 25 mm label; each label then runs onto the next page and the printer feeds **blank labels** (measured: 3 labels → 6 pages with only 2 mm margins). Fixes, in order of preference:

1. Tell users to set **Margins: None** (and the driver's paper size to the label size).
2. Optionally make the print CSS shrink a label to the printable area instead of spilling — in `@media print`:
   ```css
   .label-pages > * { max-width: 100vw; max-height: 100vh; }
   .label-pages > [data-kind="roll"] {
     width: 100vw !important; height: 100vh !important;
     grid-template-columns: minmax(0, 1fr) !important;
     grid-auto-rows: minmax(0, 1fr) !important;
   }
   ```
   (requires `data-kind={layout.kind}` on each page element and the barcode SVG being `min-h-0 flex-1` so it can shrink). Trade-off: with margins the barcode prints smaller.

---

## 9. Permissions

| Action | Permission |
|---|---|
| Open `/labels`, fetch label facts | `products.view` |
| Generate missing barcodes | `products.update` (button hidden otherwise) |
| Product-form code suggestions | `products.create` or `products.update` |
| Entry from a purchase | `purchases.view` to open the purchase |

---

## 10. Translations (en keys)

Namespace `labels.*`: `title` "Barcode labels", `subtitle` "Print price and barcode stickers for products", `print_one/_other` "Print {{count}} label(s)", `copies` "Labels", `noBarcode` "No barcode: skipped", `usesSku` "No barcode: prints the SKU", `missing_one/_other`, `generate_one/_other`, `tooMany` "Too many labels: print at most {{max}} at a time.", `layout` "Label stock", `layouts.<id>`, `startAt` / `startAtHint`, `show*` toggles, `skuFallback` + hint, `rollHint`, `sheetHint`, `preview_one/_other` "Preview: {{count}} page(s)", `printBarcode` / `printBarcodeOf`. Bengali and Italian files mirror the same keys.

---

## 11. Tests to port

- **Encoder:** check digits, valid/invalid vectors above, module counts (EAN-13 = 95, EAN-8 = 67), Code 128 set C/B choice and odd-digit tail, non-ASCII → `null`.
- **Backend:** `in_store_code(1) == "2000000000015"`; generated codes pass validation as `ean13`; generating twice does not change existing codes; numbering continues from the highest `20…` code; `REQUIRED` / `TOO_MANY` errors.
- **Page:** copies from a purchase (received − returned, fractions round up); scanning the same product twice gives 2 copies; products without barcode are skipped or use SKU per setting; `paginate` with `skip` on sheets and ignoring `skip` on rolls; Print disabled at 0 or > 5,000 labels.
- **Print check (manual or headless Chrome `Page.printToPDF` with `preferCSSPageSize: true`):** N labels on a roll layout → exactly N PDF pages.
