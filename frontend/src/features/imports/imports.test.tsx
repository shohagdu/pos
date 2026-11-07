import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { currentTenant, membership, mockApi, problem, token, user } from '../../test/api'
import { renderRoute } from '../../test/render'
import { readOpeningRows } from './openingStock'
import { type Masters, checkRow, readRows, sampleRows } from './productImport'
import { csvRecords } from '../../lib/csv'

afterEach(() => {
  vi.restoreAllMocks()
  localStorage.clear()
})

const perms = ['dashboard.view', 'products.view', 'products.create', 'products.view_cost', 'inventory.view', 'inventory.opening_stock']

const units = [
  { id: 'u-pcs', name: 'Piece', short_name: 'pcs', allow_fraction: false, status: 'active' },
  { id: 'u-kg', name: 'Kilogram', short_name: 'kg', allow_fraction: true, status: 'active' },
]
const taxRates = [
  { id: 't0', name: 'No VAT', rate_pct: '0.0000', is_default: true, status: 'active' },
  { id: 't15', name: 'VAT 15%', rate_pct: '15.0000', is_default: false, status: 'active' },
]
const categories = [{ id: 'c-rice', parent_id: null, name: 'Rice', depth: 0, sort_order: 0, status: 'active', product_count: 0 }]

function signedIn(permissions = perms) {
  return {
    'POST /api/v1/auth/refresh': token('t1'),
    'GET /api/v1/auth/me': { user, memberships: [membership('t1', 'Rahim Store')], tenant: currentTenant('t1', 'Rahim Store', permissions) },
    'GET /api/v1/units': units,
    'GET /api/v1/tax-rates': taxRates,
    'GET /api/v1/categories': categories,
    'GET /api/v1/brands': [],
    'GET /api/v1/stock-locations': [],
  }
}

const masters = { units, taxRates, categories, brands: [] } as unknown as Masters

const product = (over: object) => ({
  id: 'p1', variant_id: 'v1', product_type: 'standard', name: 'Miniket Rice 5kg', name_bn: null, description: null, sku: 'P000001',
  barcodes: [], category_id: null, category_name: null, brand_id: null, brand_name: null, unit_id: 'u-pcs', unit_short_name: 'pcs',
  allow_fraction: false, tax_rate_id: null, tax_rate_pct: null, track_inventory: true, purchase_price: '400.0000', selling_price: '450.0000',
  wholesale_price: null, min_selling_price: null, reorder_level: null, status: 'active', version: 1, created_at: '', updated_at: '',
  ...over,
})

const csvFile = (text: string, name = 'products.csv') => new File([text], name, { type: 'text/csv' })

describe('CSV files', () => {
  it('the product sample reads back as valid rows', () => {
    const { headers, records } = csvRecords(sampleRows(masters, true).map((r) => r.join(',')).join('\n'))
    const { rows, missing, ignored } = readRows(headers, records, masters)
    expect(missing).toEqual([])
    expect(ignored).toEqual([])
    expect(rows.map((r) => [r.name, r.unit_id, r.service, r.track_inventory, r.opening_stock])).toEqual([
      ['Miniket Rice 5kg', 'u-pcs', false, true, '20'],
      ['Soybean Oil 1L', 'u-pcs', false, true, '24'],
      ['Sugar (loose)', 'u-kg', false, true, '50'],
      ['Home delivery', 'u-pcs', true, false, ''],
    ])
    expect(rows.every((r) => Object.keys(checkRow(r, true, units as never)).length === 0)).toBe(true)
    // Without the opening stock permission the column is left out and skipped.
    expect(sampleRows(masters, true, false)[0]).not.toContain('opening_stock')
    expect(rows[1]?.opening_cost).toBe('165')
    expect(readRows(headers, records, masters, false).ignored).toEqual(['opening_stock', 'opening_cost'])
    // Without seeing costs, the cost column is left out and skipped.
    expect(sampleRows(masters, false)[0]).not.toContain('opening_cost')
    expect(readRows(headers, records, masters, true, false).ignored).toEqual(['opening_cost'])
  })

  it('reads opening stock rows by SKU or barcode and skips blank quantities', () => {
    expect(readOpeningRows('sku,barcode,name,quantity,unit_cost\nP1,,Rice,"1,200",৳400\n,8941100500019,Oil,3,\nP3,,Salt,,\n')).toEqual([
      { line: 2, code: 'P1', quantity: '1200', unit_cost: '400', batch_no: '', expiry_date: '' },
      { line: 3, code: '8941100500019', quantity: '3', unit_cost: '', batch_no: '', expiry_date: '' },
    ])
    // Older files with quantity / unit_cost still read.
    expect(readOpeningRows('sku,quantity,unit_cost\nP1,2,50')).toEqual([{ line: 2, code: 'P1', quantity: '2', unit_cost: '50', batch_no: '', expiry_date: '' }])
    expect(readOpeningRows('name,quantity\nRice,3')).toBeNull()
  })
})

describe('product import', () => {
  it('shows the file for checking, flags problems, then creates new categories and imports', async () => {
    const calls = mockApi({
      ...signedIn(),
      'POST /api/v1/categories': { id: 'c-drinks', parent_id: null, name: 'Drinks', depth: 0, sort_order: 0, status: 'active', product_count: 0 },
      'POST /api/v1/products/import': () => Response.json([product({ id: 'p1' }), product({ id: 'p2', track_inventory: false })], { status: 201 }),
    })
    renderRoute('/products/import')
    const file = csvFile('Name,Unit,Selling Price,Category,Barcode,Tax Rate,Notes,Opening Stock,Opening Cost\nTea,pcs,20,Drinks,,15,x,1.5,18\nSalt,box,,rice,8941100500019|SALT,,,,900\n')
    await userEvent.upload(await screen.findByLabelText('Upload CSV file'), file)

    expect(await screen.findByText('Check before saving: products.csv')).toBeInTheDocument()
    expect(screen.getByText('These columns are not used and were skipped: notes.')).toBeInTheDocument()
    expect(screen.getByText('New categories will be created: Drinks.')).toBeInTheDocument()
    expect(screen.getByLabelText('Tax rate, line 2')).toHaveValue('t15')
    expect(screen.getByLabelText('Barcodes, line 2')).toHaveAttribute('placeholder', 'Auto')
    expect(screen.getByText('No unit "box": choose one')).toBeInTheDocument()
    expect(screen.getByLabelText('Price, line 3')).toHaveAttribute('aria-invalid', 'true')
    expect(screen.getByText('Whole numbers only for pcs')).toBeInTheDocument()
    expect(screen.getByText('Enter the opening stock first')).toBeInTheDocument()

    await userEvent.click(screen.getByRole('button', { name: 'Import 2 products' }))
    expect(screen.getByText('2 rows need fixing before saving (marked red).')).toBeInTheDocument()
    expect(calls.some((c) => c.path === '/api/v1/products/import')).toBe(false)

    await userEvent.clear(screen.getByLabelText('Opening stock, line 2'))
    await userEvent.type(screen.getByLabelText('Opening stock, line 2'), '24')
    await userEvent.type(screen.getByLabelText('Opening stock, line 3'), '2.5')

    await userEvent.selectOptions(screen.getByLabelText('Unit, line 3'), 'u-kg')
    await userEvent.type(screen.getByLabelText('Price, line 3'), '৳1,250.5')
    expect(screen.getByText('Enter an amount like 120 or 120.50')).toBeInTheDocument()
    await userEvent.clear(screen.getByLabelText('Price, line 3'))
    await userEvent.type(screen.getByLabelText('Price, line 3'), '1250.5')
    await userEvent.click(screen.getByRole('button', { name: 'Import 2 products' }))

    expect(await screen.findByText('2 products imported. Opening stock saved for 2 products.')).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'Enter opening stock' })).toHaveAttribute('href', '/inventory/adjustments/new?reason=opening')
    expect(calls.find((c) => c.method === 'POST' && c.path === '/api/v1/categories')?.body).toEqual({ name: 'Drinks' })
    expect(calls.find((c) => c.path === '/api/v1/products/import')?.body).toEqual({
      products: [
        { product_type: 'standard', name: 'Tea', barcodes: [], generate_barcode: true, category_id: 'c-drinks', unit_id: 'u-pcs', tax_rate_id: 't15', track_inventory: true, selling_price: '20' },
        { product_type: 'standard', name: 'Salt', barcodes: ['8941100500019', 'SALT'], category_id: 'c-rice', unit_id: 'u-kg', track_inventory: true, selling_price: '1250.5' },
      ],
      opening_stock: [
        { row: 0, quantity: '24', unit_cost: '18' },
        { row: 1, quantity: '2.5', unit_cost: '900' },
      ],
    })
  })

  it('shows server problems on their rows; nothing is saved', async () => {
    mockApi({
      ...signedIn(),
      'POST /api/v1/products/import': () =>
        problem(400, 'VALIDATION_FAILED', { errors: [{ field: 'products[1].sku', code: 'DUPLICATE_SKU', message: 'SKU TEA is already in use' }] }),
    })
    renderRoute('/products/import')
    await userEvent.upload(await screen.findByLabelText('Upload CSV file'), csvFile('name,sku,selling_price\nTea,TEA,20\nGreen tea,tea,25\n'))
    await userEvent.click(await screen.findByRole('button', { name: 'Import 2 products' }))
    expect(await screen.findByText('Nothing was saved: 1 row has a problem. Fix it and save again.')).toBeInTheDocument()
    expect(screen.getByText('This SKU is already used by another product.')).toBeInTheDocument()
    // Only the row with the problem is listed; editing the cell clears the server's mark.
    expect(screen.queryByLabelText('Name, line 2')).not.toBeInTheDocument()
    await userEvent.type(screen.getByLabelText('SKU, line 3'), '-2')
    expect(screen.queryByText('This SKU is already used by another product.')).not.toBeInTheDocument()
  })

  it('tells what is wrong with a file that is not a product list', async () => {
    mockApi(signedIn())
    renderRoute('/products/import')
    await userEvent.upload(await screen.findByLabelText('Upload CSV file'), csvFile('sku,quantity\nP1,3\n'))
    expect(await screen.findByText(/The file has no name, selling_price column/)).toBeInTheDocument()
  })
})

describe('opening stock from CSV', () => {
  it('downloads products without stock and loads a filled file into the form', async () => {
    const out = { variant_id: 'v1', product_id: 'p1', name: 'Rice', name_bn: null, sku: 'P000001', unit_short_name: 'pcs', allow_fraction: false,
      category_name: null, brand_name: null, quantity: '0.000', avg_cost: null, value: null, selling_price: '450.0000', purchase_price: '400.0000',
      reorder_level: null, is_low: false, last_movement_at: null }
    const blobs: Blob[] = []
    URL.createObjectURL = vi.fn((b: Blob) => {
      blobs.push(b)
      return 'blob:sample'
    })
    URL.revokeObjectURL = vi.fn()
    const calls = mockApi({
      ...signedIn(),
      'GET /api/v1/stock': { data: [out], next_cursor: null, summary: {} },
      'POST /api/v1/products/lookup': [
        { code: 'p000001', product: product({}) },
        { code: '8941100500019', product: product({ id: 'p2', variant_id: 'v2', name: 'Oil', sku: 'OIL', purchase_price: '170.0000' }) },
        { code: 'SVC', product: product({ id: 'p3', variant_id: 'v3', name: 'Delivery', sku: 'SVC', track_inventory: false }) },
      ],
    })
    renderRoute('/inventory/adjustments/new?reason=opening')
    await userEvent.click(await screen.findByRole('button', { name: 'Download sample CSV' }))
    await waitFor(() => expect(blobs).toHaveLength(1))
    expect((await blobs[0]!.text()).replace(/^\uFEFF/, '')).toBe('sku,name,unit,opening_stock,opening_cost,batch_no,expiry_date\r\nP000001,Rice,pcs,,400,,')

    await userEvent.upload(
      screen.getByLabelText('Upload CSV file'),
      csvFile('sku,barcode,opening_stock,opening_cost\np000001,,12,390\n,8941100500019,4,\nNOPE,,1,\nSVC,,2,\nP000009,,,\n', 'stock.csv'),
    )
    expect(await screen.findByText(/2 products added from the file\./)).toBeInTheDocument()
    expect(screen.getByText(/4: No product with this SKU or barcode \(NOPE\); 5: Stock is not tracked for this product \(Delivery\)/)).toBeInTheDocument()
    expect(calls.find((c) => c.path === '/api/v1/products/lookup')?.body).toEqual({ codes: ['p000001', '8941100500019', 'NOPE', 'SVC'] })
    expect(screen.getByLabelText('Quantity for Miniket Rice 5kg')).toHaveValue('12')
    expect(screen.getByLabelText('Unit cost for Miniket Rice 5kg')).toHaveValue('390')
    expect(screen.getByLabelText('Quantity for Oil')).toHaveValue('4')
    expect(screen.getByLabelText('Unit cost for Oil')).toHaveValue('170')
    expect(screen.getByRole('button', { name: 'Save opening stock (2 products)' })).toBeEnabled()
  })
})
