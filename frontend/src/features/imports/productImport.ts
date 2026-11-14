/**
 * Product CSV import: the file's columns, reading rows into editable form,
 * checking them before anything is sent, and turning them into API input.
 * Names (unit, tax, category, brand) are matched without regard to case.
 */
import type { Brand, Category, ProductInput, TaxRate, Unit } from '../../api/catalog'

export const MAX_IMPORT_ROWS = 1000

export interface ImportRow {
  key: string
  /** Line in the file (header = 1); 0 for rows added on screen. */
  line: number
  name: string
  name_bn: string
  sku: string
  barcodes: string
  category: string
  brand: string
  /** Unit id; `''` until one is chosen. */
  unit_id: string
  /** Tax rate id; `''` = the business default. */
  tax_rate_id: string
  purchase_price: string
  selling_price: string
  wholesale_price: string
  min_selling_price: string
  reorder_level: string
  /** Opening stock quantity, posted with the product; `''` = none. */
  opening_stock: string
  /** Cost per unit of the opening stock; `''` = the purchase price. */
  opening_cost: string
  service: boolean
  track_inventory: boolean
  /** What the file said when it named a unit or tax rate that does not exist. */
  unknownUnit?: string
  unknownTax?: string
}

export type RowField = Exclude<keyof ImportRow, 'key' | 'line' | 'unknownUnit' | 'unknownTax'>

/** Column header (as written in the sample) → accepted spellings. */
export const COLUMNS = {
  name: ['name', 'product_name', 'product', 'item', 'item_name'],
  name_bn: ['name_bn', 'name_bangla', 'bangla_name', 'bengali_name', 'name_in_bengali'],
  sku: ['sku', 'code', 'item_code', 'product_code'],
  barcodes: ['barcode', 'barcodes'],
  category: ['category'],
  brand: ['brand'],
  unit: ['unit', 'uom'],
  tax_rate: ['tax_rate', 'tax', 'vat'],
  purchase_price: ['purchase_price', 'cost', 'cost_price', 'buying_price'],
  selling_price: ['selling_price', 'price', 'sale_price', 'sales_price', 'mrp'],
  wholesale_price: ['wholesale_price', 'wholesale'],
  min_selling_price: ['min_selling_price', 'minimum_price', 'min_price'],
  reorder_level: ['reorder_level', 'reorder', 'alert_quantity'],
  type: ['type', 'product_type'],
  track_stock: ['track_stock', 'track_inventory'],
  opening_stock: ['opening_stock', 'opening_qty', 'opening_quantity', 'stock', 'quantity', 'qty'],
  opening_cost: ['opening_cost', 'opening_unit_cost', 'unit_cost', 'stock_cost'],
} as const

type Column = keyof typeof COLUMNS

/** Which known column each file header is, if any. */
function columnOf(header: string): Column | undefined {
  return (Object.keys(COLUMNS) as Column[]).find((c) => (COLUMNS[c] as readonly string[]).includes(header))
}

const same = (a: string, b: string) => a.trim().toLowerCase() === b.trim().toLowerCase()
const NO = ['no', 'n', 'false', '0', 'off']

export interface Masters {
  units: Unit[]
  taxRates: TaxRate[]
  categories: Category[]
  brands: Brand[]
}

export function findUnit(units: Unit[], text: string) {
  const live = units.filter((u) => u.status === 'active')
  return live.find((u) => same(u.short_name, text)) ?? live.find((u) => same(u.name, text))
}

/** `VAT 15%`, `15`, `15%` or `15.00` all find the 15 % rate. */
export function findTax(rates: TaxRate[], text: string) {
  const live = rates.filter((r) => r.status === 'active')
  const byName = live.find((r) => same(r.name, text))
  if (byName) return byName
  const pct = text.replace('%', '').trim()
  if (!/^\d+(\.\d+)?$/.test(pct)) return undefined
  return live.find((r) => Number(r.rate_pct) === Number(pct))
}

export const findCategory = (list: Category[], text: string) => list.find((c) => same(c.name, text))
export const findBrand = (list: Brand[], text: string) => list.find((b) => same(b.name, text))

/** The unit a row gets when the file leaves it blank. */
export function defaultUnit(units: Unit[]) {
  return findUnit(units, 'pcs') ?? units.find((u) => u.status === 'active')
}

let counter = 0
const nextKey = () => `r${++counter}`

export function blankRow(masters: Masters): ImportRow {
  return {
    key: nextKey(),
    line: 0,
    name: '',
    name_bn: '',
    sku: '',
    barcodes: '',
    category: '',
    brand: '',
    unit_id: defaultUnit(masters.units)?.id ?? '',
    tax_rate_id: '',
    purchase_price: '',
    selling_price: '',
    wholesale_price: '',
    min_selling_price: '',
    reorder_level: '',
    opening_stock: '',
    opening_cost: '',
    service: false,
    track_inventory: true,
  }
}

export interface ReadResult {
  rows: ImportRow[]
  /** Headers the file has that are not import columns (ignored). */
  ignored: string[]
  /** Required columns the file lacks. */
  missing: Column[]
}

/**
 * Rows of a parsed file (see `csvRecords`). Without `withStock` (no
 * `inventory.opening_stock`) the opening stock columns are skipped; without
 * `withCost` (no `products.view_cost`) the opening cost column is.
 */
export function readRows(
  headers: string[],
  records: { line: number; values: Record<string, string> }[],
  masters: Masters,
  withStock = true,
  withCost = true,
): ReadResult {
  const allowed = (c: Column) => (withStock || (c !== 'opening_stock' && c !== 'opening_cost')) && (withCost || c !== 'opening_cost')
  const map = new Map<Column, string>()
  const ignored: string[] = []
  for (const h of headers) {
    const c = columnOf(h)
    if (c && !map.has(c) && allowed(c)) map.set(c, h)
    else if (h) ignored.push(h)
  }
  const missing = (['name', 'selling_price'] as Column[]).filter((c) => !map.has(c))
  const get = (values: Record<string, string>, c: Column) => {
    const h = map.get(c)
    return h ? (values[h] ?? '').trim() : ''
  }
  const rows = records.map(({ line, values }) => {
    const row = blankRow(masters)
    row.line = line
    row.name = get(values, 'name')
    row.name_bn = get(values, 'name_bn')
    row.sku = get(values, 'sku')
    row.barcodes = get(values, 'barcodes')
    row.category = get(values, 'category')
    row.brand = get(values, 'brand')
    const unit = get(values, 'unit')
    if (unit) {
      const found = findUnit(masters.units, unit)
      row.unit_id = found?.id ?? ''
      if (!found) row.unknownUnit = unit
    }
    const tax = get(values, 'tax_rate')
    if (tax) {
      const found = findTax(masters.taxRates, tax)
      row.tax_rate_id = found?.id ?? ''
      if (!found) row.unknownTax = tax
    }
    row.purchase_price = cleanNumber(get(values, 'purchase_price'))
    row.selling_price = cleanNumber(get(values, 'selling_price'))
    row.wholesale_price = cleanNumber(get(values, 'wholesale_price'))
    row.min_selling_price = cleanNumber(get(values, 'min_selling_price'))
    row.reorder_level = cleanNumber(get(values, 'reorder_level'))
    row.opening_stock = cleanNumber(get(values, 'opening_stock'))
    row.opening_cost = cleanNumber(get(values, 'opening_cost'))
    row.service = /service/i.test(get(values, 'type'))
    row.track_inventory = !row.service && !NO.includes(get(values, 'track_stock').toLowerCase())
    return row
  })
  return { rows, ignored, missing }
}

/** `৳1,250.50` → `1250.50`: spreadsheets add currency signs and thousands separators. */
export function cleanNumber(text: string): string {
  const t = text.replace(/[\s,৳$€£]/g, '').replace(/^tk\.?/i, '')
  return t
}

const PRICE = /^\d{1,14}(\.\d{1,4})?$/
const QTY = /^\d{1,15}(\.\d{1,3})?$/

export type RowErrors = Partial<Record<RowField, string>>

/** Problems with `row`, as i18n keys (under `imports.errors`) with values. */
export interface Problem {
  key: string
  values?: Record<string, string | number>
}

export function checkRow(row: ImportRow, canViewCost: boolean, units: Unit[] = []): Partial<Record<RowField, Problem>> {
  const out: Partial<Record<RowField, Problem>> = {}
  if (!row.name.trim()) out.name = { key: 'required' }
  else if (row.name.trim().length > 200) out.name = { key: 'tooLong', values: { max: 200 } }
  if (row.sku.trim() && !/^[A-Za-z0-9._-]{1,64}$/.test(row.sku.trim())) out.sku = { key: 'sku' }
  if (!row.unit_id) out.unit_id = row.unknownUnit ? { key: 'unknownUnit', values: { name: row.unknownUnit } } : { key: 'required' }
  if (row.unknownTax && !row.tax_rate_id) out.tax_rate_id = { key: 'unknownTax', values: { name: row.unknownTax } }
  if (!row.selling_price.trim()) out.selling_price = { key: 'required' }
  const prices: ('selling_price' | 'wholesale_price' | 'min_selling_price' | 'purchase_price')[] = [
    'selling_price',
    'wholesale_price',
    'min_selling_price',
    ...(canViewCost ? (['purchase_price'] as const) : []),
  ]
  for (const f of prices) {
    if (row[f].trim() && !PRICE.test(row[f].trim())) out[f] = { key: 'price' }
  }
  if (row.reorder_level.trim() && !QTY.test(row.reorder_level.trim())) out.reorder_level = { key: 'quantity' }
  const opening = row.opening_stock.trim()
  if (opening) {
    const unit = units.find((u) => u.id === row.unit_id)
    if (row.service || !row.track_inventory) out.opening_stock = { key: 'notStocked' }
    else if (!QTY.test(opening) || Number(opening) <= 0) out.opening_stock = { key: 'quantity' }
    else if (unit && !unit.allow_fraction && !Number.isInteger(Number(opening))) out.opening_stock = { key: 'wholeNumber', values: { unit: unit.short_name } }
  }
  const cost = row.opening_cost.trim()
  if (cost && canViewCost) {
    if (!PRICE.test(cost)) out.opening_cost = { key: 'price' }
    else if (!opening) out.opening_cost = { key: 'costWithoutStock' }
  }
  return out
}

export const splitBarcodes = (text: string) =>
  text
    .split(/[\s,;|]+/)
    .map((b) => b.trim())
    .filter(Boolean)

/** API input for `row`; category and brand ids come from `ids` (new ones already created). */
export function toInput(
  row: ImportRow,
  ids: { category: (name: string) => string | undefined; brand: (name: string) => string | undefined },
  canViewCost: boolean,
): ProductInput {
  const opt = (v: string) => (v.trim() ? v.trim() : undefined)
  const barcodes = splitBarcodes(row.barcodes)
  return {
    product_type: row.service ? 'service' : 'standard',
    name: row.name.trim(),
    name_bn: opt(row.name_bn),
    sku: opt(row.sku),
    barcodes,
    // Barcodes are required: a row without one gets a new in-store code.
    generate_barcode: barcodes.length === 0 ? true : undefined,
    category_id: row.category.trim() ? ids.category(row.category) : undefined,
    brand_id: row.brand.trim() ? ids.brand(row.brand) : undefined,
    unit_id: row.unit_id,
    tax_rate_id: opt(row.tax_rate_id),
    track_inventory: row.service ? false : row.track_inventory,
    purchase_price: canViewCost ? opt(row.purchase_price) : undefined,
    selling_price: row.selling_price.trim(),
    wholesale_price: opt(row.wholesale_price),
    min_selling_price: opt(row.min_selling_price),
    reorder_level: opt(row.reorder_level),
  }
}

/** Server field (`products[3].unit_id`) → row index and row field. */
export function serverField(field: string): { index: number; field: RowField | 'row' } | null {
  const m = /^products\[(\d+)\](?:\.(.+))?$/.exec(field)
  if (!m) return null
  const name = m[2] ?? ''
  const known: Record<string, RowField> = {
    name: 'name',
    name_bn: 'name_bn',
    sku: 'sku',
    barcodes: 'barcodes',
    category_id: 'category',
    brand_id: 'brand',
    unit_id: 'unit_id',
    tax_rate_id: 'tax_rate_id',
    purchase_price: 'purchase_price',
    selling_price: 'selling_price',
    wholesale_price: 'wholesale_price',
    min_selling_price: 'min_selling_price',
    reorder_level: 'reorder_level',
    product_type: 'service',
    track_inventory: 'track_inventory',
    opening_quantity: 'opening_stock',
    opening_cost: 'opening_cost',
  }
  return { index: Number(m[1]), field: known[name] ?? 'row' }
}

export interface OpeningStockInput {
  row: number
  quantity: string
  unit_cost?: string
}

/** Opening stock to post with the import: row index, quantity and (with `canViewCost`) cost. */
export function openingStock(rows: ImportRow[], canViewCost: boolean): OpeningStockInput[] {
  return rows.flatMap((r, i) => {
    const quantity = r.opening_stock.trim()
    if (!quantity || r.service || !r.track_inventory) return []
    const cost = r.opening_cost.trim()
    return [{ row: i, quantity, ...(canViewCost && cost ? { unit_cost: cost } : {}) }]
  })
}

/** The sample file: headers plus example rows the shop can overwrite. */
export function sampleRows(masters: Masters, canViewCost: boolean, withStock = true) {
  const pcs = defaultUnit(masters.units)?.short_name ?? 'pcs'
  const kg = findUnit(masters.units, 'kg')?.short_name ?? pcs
  const header = [
    'name',
    'name_bn',
    'sku',
    'barcode',
    'category',
    'brand',
    'unit',
    'tax_rate',
    ...(canViewCost ? ['purchase_price'] : []),
    'selling_price',
    'wholesale_price',
    'min_selling_price',
    'reorder_level',
    'type',
    'track_stock',
    ...(withStock ? ['opening_stock'] : []),
    ...(withStock && canViewCost ? ['opening_cost'] : []),
  ]
  const ex = (values: Record<string, string>) => header.map((h) => values[h] ?? '')
  return [
    header,
    ex({
      name: 'Miniket Rice 5kg',
      name_bn: 'মিনিকেট চাল ৫ কেজি',
      category: 'Rice',
      brand: 'Chashi',
      unit: pcs,
      purchase_price: '400',
      selling_price: '450',
      wholesale_price: '440',
      min_selling_price: '430',
      reorder_level: '10',
      type: 'standard',
      track_stock: 'yes',
      opening_stock: '20',
    }),
    ex({
      name: 'Soybean Oil 1L',
      sku: 'OIL-1L',
      barcode: '8941100500019',
      category: 'Oil',
      brand: 'Teer',
      unit: pcs,
      purchase_price: '170',
      selling_price: '190',
      reorder_level: '12',
      type: 'standard',
      track_stock: 'yes',
      opening_stock: '24',
      opening_cost: '165',
    }),
    ex({
      name: 'Sugar (loose)',
      name_bn: 'চিনি',
      category: 'Sugar',
      unit: kg,
      purchase_price: '120',
      selling_price: '135',
      reorder_level: '25',
      type: 'standard',
      track_stock: 'yes',
      opening_stock: '50',
    }),
    ex({ name: 'Home delivery', unit: pcs, selling_price: '50', type: 'service', track_stock: 'no' }),
  ]
}
