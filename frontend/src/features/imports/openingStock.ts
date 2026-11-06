/**
 * Opening stock CSV: products named by SKU or barcode, with a quantity, an
 * optional unit cost, and an optional batch number and expiry date. The
 * rows land in the opening stock form, where they are checked and saved
 * like typed lines.
 */
import { parseExpiry } from './expiry'
import { cleanNumber } from './productImport'
import { csvRecords } from '../../lib/csv'

export interface OpeningRow {
  line: number
  code: string
  quantity: string
  unit_cost: string
  batch_no: string
  /** `YYYY-MM-DD`, or `''` when blank. */
  expiry_date: string
  /** What the file said when the expiry is not a date. */
  badExpiry?: string
}

const CODE = ['sku', 'code', 'item_code', 'product_code', 'barcode']
const QUANTITY = ['opening_stock', 'quantity', 'qty', 'opening_qty', 'opening_quantity', 'stock']
const COST = ['opening_cost', 'unit_cost', 'opening_unit_cost', 'stock_cost', 'cost', 'cost_price', 'purchase_price']
export const BATCH_COLUMNS = ['batch_no', 'batch', 'batch_number', 'lot', 'lot_no', 'lot_number']
export const EXPIRY_COLUMNS = ['expiry_date', 'expiry', 'exp_date', 'exp', 'expires', 'expiry_dt', 'best_before']

/** `null` when the file lacks a code or quantity column. Rows without a quantity are left out. */
export function readOpeningRows(text: string): OpeningRow[] | null {
  const { headers, records } = csvRecords(text)
  const pick = (names: string[]) => names.find((n) => headers.includes(n))
  const code = pick(CODE)
  const quantity = pick(QUANTITY)
  const cost = pick(COST)
  const batch = pick(BATCH_COLUMNS)
  const expiry = pick(EXPIRY_COLUMNS)
  // A file may name products by both SKU and barcode; the barcode is the fallback.
  const barcode = code !== 'barcode' && headers.includes('barcode') ? 'barcode' : undefined
  if (!code || !quantity) return null
  return records
    .map(({ line, values }) => {
      const rawExpiry = (expiry ? values[expiry] : '')?.trim() ?? ''
      const parsed = parseExpiry(rawExpiry)
      const row: OpeningRow = {
        line,
        code: (values[code] || (barcode ? values[barcode] : '') || '').trim(),
        quantity: cleanNumber(values[quantity] ?? ''),
        unit_cost: cost ? cleanNumber(values[cost] ?? '') : '',
        batch_no: (batch ? values[batch] : '')?.trim() ?? '',
        expiry_date: parsed ?? '',
      }
      if (parsed === null) row.badExpiry = rawExpiry
      return row
    })
    .filter((r) => r.quantity !== '')
}

/** Same column names as the product import; `opening_cost` only for users who see costs. */
export const openingSampleHeader = (withCost: boolean) => [
  'sku',
  'name',
  'unit',
  'opening_stock',
  ...(withCost ? ['opening_cost'] : []),
  'batch_no',
  'expiry_date',
]
