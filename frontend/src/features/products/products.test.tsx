import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { currentTenant, membership, mockApi, problem, token, user } from '../../test/api'
import { renderRoute } from '../../test/render'

afterEach(() => vi.restoreAllMocks())

const managerPerms = ['dashboard.view', 'products.view', 'products.create', 'products.update', 'products.archive', 'products.view_cost', 'settings.manage']

const units = [
  { id: 'u-pcs', name: 'Piece', short_name: 'pcs', allow_fraction: false, status: 'active' },
  { id: 'u-kg', name: 'Kilogram', short_name: 'kg', allow_fraction: true, status: 'active' },
]
const taxRates = [
  { id: 't0', name: 'No VAT', rate_pct: '0.0000', is_default: true, status: 'active' },
  { id: 't15', name: 'VAT 15%', rate_pct: '15.0000', is_default: false, status: 'active' },
]
const categories = [
  { id: 'c-food', parent_id: null, name: 'Food', depth: 0, sort_order: 0, status: 'active', product_count: 0 },
  { id: 'c-rice', parent_id: 'c-food', name: 'Rice', depth: 1, sort_order: 0, status: 'active', product_count: 1 },
]

const rice = {
  id: 'p1', variant_id: 'v1', product_type: 'standard', name: 'Miniket Rice 5kg', name_bn: 'মিনিকেট চাল', description: null,
  sku: 'P000001', barcodes: ['4006381333931'], category_id: 'c-rice', category_name: 'Rice', brand_id: null, brand_name: null,
  unit_id: 'u-pcs', unit_short_name: 'pcs', allow_fraction: false, tax_rate_id: 't0', tax_rate_pct: '0.0000', track_inventory: true,
  purchase_price: '400.0000', selling_price: '450.5000', wholesale_price: null, min_selling_price: null, reorder_level: null,
  status: 'active', version: 1, created_at: '2026-10-17T00:00:00Z', updated_at: '2026-10-17T00:00:00Z',
}

function signedIn(perms = managerPerms) {
  return {
    'POST /api/v1/auth/refresh': token('t1'),
    'GET /api/v1/auth/me': { user, memberships: [membership('t1', 'Rahim Store')], tenant: currentTenant('t1', 'Rahim Store', perms) },
    'GET /api/v1/units': units,
    'GET /api/v1/tax-rates': taxRates,
    'GET /api/v1/categories': categories,
    'GET /api/v1/brands': [],
    'GET /api/v1/products/next-codes': { sku: 'P000002', barcode: '2000000000015' },
    // Where a new product's "saved" pop-up leads.
    'GET /api/v1/products': { data: [], next_cursor: null },
  }
}

describe('products', () => {
  it('lists products and searches by name, SKU or barcode', async () => {
    const calls = mockApi({ ...signedIn(), 'GET /api/v1/products': { data: [rice], next_cursor: null } })
    renderRoute('/products')
    const table = await screen.findByRole('table')
    expect(within(table).getByRole('link', { name: 'Miniket Rice 5kg' })).toHaveAttribute('href', '/products/p1')
    expect(within(table).getByText('P000001')).toBeInTheDocument()
    expect(within(table).getByText('Rice')).toBeInTheDocument()
    expect(within(table).getByText(/450\.50/)).toBeInTheDocument()

    await userEvent.type(screen.getByLabelText('Search'), '4006381333931')
    await userEvent.click(screen.getByRole('button', { name: 'Search' }))
    await waitFor(() => expect(calls.some((c) => c.path === '/api/v1/products' && c.search.includes('q=4006381333931'))).toBe(true))
    expect(calls.find((c) => c.search.includes('q='))?.search).toContain('status=active')
  })

  it('creates a product with an automatic SKU', async () => {
    const calls = mockApi({
      ...signedIn(),
      'POST /api/v1/products': (call) => Response.json({ ...rice, id: 'p2', ...(call.body as object), sku: 'P000002' }, { status: 201 }),
      'GET /api/v1/products/p2': { ...rice, id: 'p2', name: 'Soybean Oil', sku: 'P000002' },
      'GET /api/v1/products/p2/price-history': [],
    })
    const { router } = renderRoute('/products/new')
    await userEvent.type(await screen.findByLabelText('Name'), 'Soybean Oil')
    // The new in-store barcode is filled in; the maker's codes are added after it.
    expect(screen.getByLabelText('Barcodes')).toHaveValue('2000000000015')
    await userEvent.type(screen.getByLabelText('Barcodes'), ', 4006381333931, OIL-1')
    await userEvent.selectOptions(screen.getByLabelText('Category'), 'c-rice')
    await userEvent.type(screen.getByLabelText('Selling price'), '190')
    await userEvent.type(screen.getByLabelText('Purchase price (cost)'), '170')
    await userEvent.click(screen.getByRole('button', { name: 'Save' }))

    // A "saved" pop-up, then the products list.
    const alert = await screen.findByRole('alertdialog', { name: 'Product saved' })
    expect(alert).toHaveAccessibleDescription('“Soybean Oil” has been added to your products.')
    // Only the pop-up says so, not a banner behind it as well.
    expect(screen.queryByText('Product saved.')).not.toBeInTheDocument()
    expect(router.state.location.pathname).toBe('/products/new')
    await userEvent.click(within(alert).getByRole('button', { name: 'Go to products' }))
    await waitFor(() => expect(router.state.location.pathname).toBe('/products'))
    expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument()
    expect(calls.find((c) => c.method === 'POST' && c.path === '/api/v1/products')?.body).toEqual({
      product_type: 'standard',
      name: 'Soybean Oil',
      barcodes: ['4006381333931', 'OIL-1'],
      generate_barcode: true,
      category_id: 'c-rice',
      unit_id: 'u-pcs',
      track_inventory: true,
      track_batches: false,
      purchase_price: '170',
      selling_price: '190',
    })
  })

  it('says while typing when the SKU or a barcode already belongs to another product', async () => {
    const calls = mockApi({
      ...signedIn(),
      'GET /api/v1/products/check-codes': (call) => {
        const q = new URLSearchParams(call.search)
        const taken = (code: string) => ({ code, product_name: 'Miniket Rice 5kg' })
        return Response.json({
          sku: q.get('sku') === 'P000001' ? taken('P000001') : null,
          barcodes: (q.get('barcodes') ?? '').split(',').map((c) => c.trim()).filter((c) => c === 'RICE-5').map(taken),
        })
      },
    })
    renderRoute('/products/new')
    const sku = await screen.findByLabelText('SKU')
    await userEvent.clear(sku)
    await userEvent.type(sku, 'P000001')
    await waitFor(() => expect(sku).toHaveAccessibleDescription('This SKU is already used by “Miniket Rice 5kg”.'))
    expect(sku).toHaveAttribute('aria-invalid', 'true')
    await userEvent.type(screen.getByLabelText('Barcodes'), ', RICE-5')
    await waitFor(() =>
      expect(screen.getByLabelText('Barcodes')).toHaveAccessibleDescription('Barcode RICE-5 is already used by “Miniket Rice 5kg”.'),
    )
    // A free SKU clears the message.
    await userEvent.type(sku, '9')
    await waitFor(() => expect(calls.some((c) => c.path === '/api/v1/products/check-codes' && c.search.includes('sku=P0000019'))).toBe(true))
    await waitFor(() => expect(sku).not.toHaveAttribute('aria-invalid'))
  })

  it('keeps the description folded until asked for and shows the profit per unit', async () => {
    mockApi(signedIn())
    renderRoute('/products/new')
    const toggle = await screen.findByRole('button', { name: 'Description (optional)' })
    expect(toggle).toHaveAttribute('aria-expanded', 'false')
    expect(screen.queryByRole('textbox', { name: 'Description (optional)' })).not.toBeInTheDocument()
    await userEvent.click(toggle)
    expect(toggle).toHaveAttribute('aria-expanded', 'true')
    await waitFor(() => expect(screen.getByRole('textbox', { name: 'Description (optional)' })).toHaveFocus())

    await userEvent.type(screen.getByLabelText('Selling price'), '250')
    await userEvent.type(screen.getByLabelText('Purchase price (cost)'), '190')
    expect(screen.getByText(/Profit per unit: .*60\.00 \(24\.0%\)/)).toBeInTheDocument()
    await userEvent.clear(screen.getByLabelText('Selling price'))
    await userEvent.type(screen.getByLabelText('Selling price'), '150')
    expect(screen.getByText(/Loss per unit: .*40\.00/)).toBeInTheDocument()
  })

  it('saves and opens a new, empty form with Save & Add Product', async () => {
    let next = 2
    const calls = mockApi({
      ...signedIn(),
      'GET /api/v1/products/next-codes': () => Response.json({ sku: `P00000${next}`, barcode: `20000000000${next}5` }),
      'POST /api/v1/products': (call) => {
        next += 1
        return Response.json({ ...rice, id: 'p2', ...(call.body as object) }, { status: 201 })
      },
    })
    const { router } = renderRoute('/products/new')
    await userEvent.type(await screen.findByLabelText('Name'), 'Soybean Oil')
    await userEvent.type(screen.getByLabelText('Selling price'), '190')
    await userEvent.click(screen.getByRole('button', { name: 'Save & Add Product' }))

    const alert = await screen.findByRole('alertdialog', { name: 'Product saved' })
    await userEvent.click(within(alert).getByRole('button', { name: 'Add another product' }))
    await waitFor(() => expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument())
    expect(router.state.location.pathname).toBe('/products/new')
    // A fresh form with the next automatic codes.
    expect(await screen.findByLabelText('SKU')).toHaveValue('P000003')
    expect(screen.getByLabelText('Name')).toHaveValue('')
    expect(screen.getByLabelText('Selling price')).toHaveValue('')
    expect(calls.filter((c) => c.method === 'POST' && c.path === '/api/v1/products')).toHaveLength(1)
  })

  describe('product image', () => {
    // jsdom has no object URLs.
    beforeEach(() => {
      Object.assign(URL, { createObjectURL: vi.fn(() => 'blob:preview'), revokeObjectURL: vi.fn() })
    })
    const PNG_SIGNATURE = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]
    const png = (size = 1000, extra = '') => {
      const bytes = new Uint8Array(size)
      bytes.set(PNG_SIGNATURE)
      bytes.set(new TextEncoder().encode(extra), 100)
      return new File([bytes], 'oil.png', { type: 'image/png' })
    }

    it('uploads the optional image right after creating the product', async () => {
      const calls = mockApi({
        ...signedIn(),
        'POST /api/v1/products': (call) => Response.json({ ...rice, id: 'p2', ...(call.body as object), image_file_id: null }, { status: 201 }),
        'PUT /api/v1/products/p2/image': () => Response.json({ ...rice, id: 'p2', name: 'Soybean Oil', image_file_id: 'f1', version: 2 }),
        'GET /api/v1/products/p2': { ...rice, id: 'p2', name: 'Soybean Oil', image_file_id: 'f1', version: 2 },
        'GET /api/v1/products/p2/price-history': [],
        'GET /api/v1/files/f1': () => new Response(new Blob(['png'], { type: 'image/png' })),
      })
      const { router } = renderRoute('/products/new')
      await userEvent.type(await screen.findByLabelText('Name'), 'Soybean Oil')
      expect(screen.getByText('No image')).toBeInTheDocument()
      // Wrong kinds and big files are refused before any upload.
      await userEvent.upload(screen.getByLabelText(/Product image/), png(3 * 1024 * 1024))
      expect(await screen.findByText('The image must be 2 MB or smaller.')).toBeInTheDocument()
      // A file named .png that is not one, or a real one with a script hidden inside, is refused too.
      await userEvent.upload(screen.getByLabelText(/Product image/), new File(['<html>hi</html>'], 'oil.png', { type: 'image/png' }))
      expect(await screen.findByText('This file is not a real JPG, PNG or WebP image.')).toBeInTheDocument()
      await userEvent.upload(screen.getByLabelText(/Product image/), png(1000, '<script>alert(1)</script>'))
      expect(await screen.findByText(/contains hidden code and cannot be used/)).toBeInTheDocument()
      expect(screen.queryByRole('img', { name: 'Product image' })).not.toBeInTheDocument()
      await userEvent.upload(screen.getByLabelText(/Product image/), png())
      expect(await screen.findByRole('img', { name: 'Product image' })).toHaveAttribute('src', 'blob:preview')
      await userEvent.type(screen.getByLabelText('Selling price'), '190')
      await userEvent.click(screen.getByRole('button', { name: 'Save' }))

      expect(await screen.findByRole('alertdialog', { name: 'Product saved' })).toHaveAccessibleDescription(/has been added to your products/)
      const upload = calls.find((c) => c.method === 'PUT' && c.path === '/api/v1/products/p2/image')
      expect(upload?.headers.get('Content-Type')).toBe('image/png')
      expect(upload?.body).toBeInstanceOf(File)
      expect(upload?.headers.get('X-File-Name')).toBe('oil.png')
      await userEvent.keyboard('{Enter}')
      await waitFor(() => expect(router.state.location.pathname).toBe('/products'))
    })

    it('keeps the product when the image upload fails and says so', async () => {
      mockApi({
        ...signedIn(),
        'POST /api/v1/products': (call) => Response.json({ ...rice, id: 'p2', ...(call.body as object), image_file_id: null }, { status: 201 }),
        'PUT /api/v1/products/p2/image': () => problem(400, 'VALIDATION_FAILED', { errors: [{ field: 'file', code: 'UNSAFE_FILE', message: '' }] }),
        'GET /api/v1/products/p2': { ...rice, id: 'p2', name: 'Soybean Oil', image_file_id: null },
        'GET /api/v1/products/p2/price-history': [],
      })
      const { router } = renderRoute('/products/new')
      await userEvent.type(await screen.findByLabelText('Name'), 'Soybean Oil')
      await userEvent.upload(screen.getByLabelText(/Product image/), png())
      await screen.findByRole('img', { name: 'Product image' })
      await userEvent.type(screen.getByLabelText('Selling price'), '190')
      await userEvent.click(screen.getByRole('button', { name: 'Save' }))
      const alert = await screen.findByRole('alertdialog', { name: 'Product saved' })
      expect(alert).toHaveAccessibleDescription(/“Soybean Oil” has been added, but the photo could not be uploaded: The image contains hidden code/)
      // No auto-close for a warning: it waits for the button.
      await new Promise((r) => setTimeout(r, 2700))
      expect(router.state.location.pathname).toBe('/products/new')
      await userEvent.click(within(alert).getByRole('button', { name: 'Go to products' }))
      await waitFor(() => expect(router.state.location.pathname).toBe('/products'))
    })

    it('removes the photo of a saved product at once', async () => {
      let saved = { ...rice, image_file_id: 'f1' as string | null }
      const calls = mockApi({
        ...signedIn(),
        'GET /api/v1/products/p1': () => Response.json(saved),
        'GET /api/v1/products/p1/price-history': [],
        'GET /api/v1/files/f1': () => new Response(new Blob(['png'], { type: 'image/png' })),
        'DELETE /api/v1/products/p1/image': () => {
          saved = { ...rice, image_file_id: null, version: 2 }
          return Response.json(saved)
        },
      })
      renderRoute('/products/p1')
      await userEvent.click(await screen.findByRole('button', { name: 'Remove image' }))
      expect(await screen.findByText('No image')).toBeInTheDocument()
      expect(calls.some((c) => c.method === 'DELETE' && c.path === '/api/v1/products/p1/image')).toBe(true)
    })

    it('has no image picker without permission to change products', async () => {
      mockApi(signedIn(['dashboard.view', 'products.view', 'products.create']))
      renderRoute('/products/new')
      expect(await screen.findByLabelText('Name')).toBeInTheDocument()
      expect(screen.queryByText('No image')).not.toBeInTheDocument()
    })
  })

  it('sets a warranty tenure', async () => {
    const calls = mockApi({
      ...signedIn(),
      'POST /api/v1/products': (call) => Response.json({ ...rice, id: 'p4', ...(call.body as object) }, { status: 201 }),
      'GET /api/v1/products/p4': { ...rice, id: 'p4', warranty_period: 3, warranty_unit: 'month' },
      'GET /api/v1/products/p4/price-history': [],
    })
    const { router } = renderRoute('/products/new')
    await userEvent.type(await screen.findByLabelText('Name'), 'Power Bank')
    await userEvent.type(screen.getByLabelText('Selling price'), '1500')
    expect(screen.queryByLabelText(/Warranty length/)).not.toBeInTheDocument()
    await userEvent.click(screen.getByLabelText(/Has warranty/))

    // Ticked without a length: flagged before anything is sent.
    await userEvent.click(screen.getByRole('button', { name: 'Save' }))
    expect(await screen.findByLabelText(/Warranty length/)).toHaveAttribute('aria-invalid', 'true')
    expect(calls.some((c) => c.method === 'POST' && c.path === '/api/v1/products')).toBe(false)

    await userEvent.type(screen.getByLabelText(/Warranty length/), '3')
    expect(screen.getByLabelText('Warranty in')).toHaveValue('month')
    await userEvent.click(screen.getByRole('button', { name: 'Save' }))
    expect(await screen.findByRole('alertdialog', { name: 'Product saved' })).toBeInTheDocument()
    expect(calls.find((c) => c.method === 'POST' && c.path === '/api/v1/products')?.body).toMatchObject({ warranty_period: 3, warranty_unit: 'month' })
    // The pop-up closes by itself and the list opens.
    await waitFor(() => expect(router.state.location.pathname).toBe('/products'), { timeout: 4000 })
  })

  it('never sends the cost for users who cannot see it', async () => {
    const calls = mockApi({
      ...signedIn(['products.view', 'products.create']),
      'POST /api/v1/products': () => Response.json({ ...rice, id: 'p3', purchase_price: null }, { status: 201 }),
      'GET /api/v1/products/p3': { ...rice, id: 'p3', purchase_price: null },
      'GET /api/v1/products/p3/price-history': [],
    })
    renderRoute('/products/new')
    await userEvent.type(await screen.findByLabelText('Name'), 'Tea')
    expect(screen.queryByLabelText('Purchase price (cost)')).not.toBeInTheDocument()
    await userEvent.type(screen.getByLabelText('Selling price'), '20')
    await userEvent.click(screen.getByRole('button', { name: 'Save' }))
    await waitFor(() => expect(calls.some((c) => c.method === 'POST' && c.path === '/api/v1/products')).toBe(true))
    const body = calls.find((c) => c.method === 'POST' && c.path === '/api/v1/products')?.body as Record<string, unknown>
    expect(body).not.toHaveProperty('purchase_price')
    expect(body).not.toHaveProperty('sku')
  })

  it('flags blank required fields on the form and allows a zero price', async () => {
    const calls = mockApi({
      ...signedIn(),
      'POST /api/v1/products': (call) => Response.json({ ...rice, id: 'p4', ...(call.body as object) }, { status: 201 }),
      'GET /api/v1/products/p4': { ...rice, id: 'p4' },
      'GET /api/v1/products/p4/price-history': [],
    })
    const { router } = renderRoute('/products/new')
    const sku = await screen.findByLabelText('SKU')
    expect(sku).toHaveValue('P000002')
    expect(sku).toBeRequired()
    expect(screen.getByLabelText('Type')).toBeRequired()
    expect(screen.getByLabelText('Unit')).toBeRequired()
    expect(screen.getByLabelText('Barcodes')).toBeRequired()
    await userEvent.clear(sku)
    await userEvent.clear(screen.getByLabelText('Barcodes'))
    await userEvent.click(screen.getByRole('button', { name: 'Save' }))
    expect(await screen.findAllByText('This field is required.')).toHaveLength(4)
    for (const field of ['Name', 'SKU', 'Barcodes', 'Selling price']) expect(screen.getByLabelText(field)).toHaveAttribute('aria-invalid', 'true')
    expect(calls.some((c) => c.method === 'POST' && c.path === '/api/v1/products')).toBe(false)

    await userEvent.type(screen.getByLabelText('Name'), 'Free sample')
    await userEvent.type(sku, 'SAMPLE-1')
    await userEvent.type(screen.getByLabelText('Barcodes'), 'SAMPLE')
    await userEvent.type(screen.getByLabelText('Selling price'), '0.00')
    await userEvent.click(screen.getByRole('button', { name: 'Save' }))
    expect(await screen.findByRole('alertdialog', { name: 'Product saved' })).toBeInTheDocument()
    expect(router.state.location.pathname).toBe('/products/new')
    expect(calls.find((c) => c.method === 'POST' && c.path === '/api/v1/products')?.body).toMatchObject({ sku: 'SAMPLE-1', barcodes: ['SAMPLE'], selling_price: '0.00' })
  })

  it('shows a duplicate barcode on the barcode field', async () => {
    mockApi({ ...signedIn(), 'POST /api/v1/products': () => problem(409, 'DUPLICATE_BARCODE') })
    renderRoute('/products/new')
    await userEvent.type(await screen.findByLabelText('Name'), 'Copy')
    const barcodes = screen.getByLabelText('Barcodes')
    await userEvent.clear(barcodes)
    await userEvent.type(barcodes, '4006381333931')
    await userEvent.type(screen.getByLabelText('Selling price'), '1')
    await userEvent.click(screen.getByRole('button', { name: 'Save' }))
    expect(await screen.findByText('This barcode already belongs to another product.')).toBeInTheDocument()
    expect(barcodes).toHaveAttribute('aria-invalid', 'true')
  })

  it('updates with the version and asks to check again after a conflict', async () => {
    const calls = mockApi({
      ...signedIn(),
      'GET /api/v1/products/p1': rice,
      'GET /api/v1/products/p1/price-history': [
        { price_field: 'selling_price', old_value: '440.0000', new_value: '450.5000', changed_by: 'u1', changed_at: '2026-10-16T00:00:00Z' },
      ],
      'PUT /api/v1/products/p1': () => problem(412, 'VERSION_MISMATCH'),
    })
    renderRoute('/products/p1')
    expect(await screen.findByLabelText('Selling price')).toHaveValue('450.5')
    // Price history is folded at the foot of the Prices card.
    await userEvent.click(screen.getByRole('button', { name: /Price history/ }))
    expect(screen.getByText(/Selling price: .*440\.00.*450\.50/)).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Save' }))
    expect(await screen.findByText(/Someone else changed this/)).toBeInTheDocument()
    // A failed save stays on the form: no pop-up.
    expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument()
    const put = calls.find((c) => c.method === 'PUT')
    expect(put?.headers.get('If-Match')).toBe('"1"')
    expect(put?.body).toMatchObject({ sku: 'P000001', barcodes: ['4006381333931'], tax_rate_id: 't0', purchase_price: '400' })
  })

  it('offers a new barcode for a saved product that has none', async () => {
    const calls = mockApi({
      ...signedIn(),
      'GET /api/v1/products/p1': { ...rice, barcodes: [] },
      'GET /api/v1/products/p1/price-history': [],
      'PUT /api/v1/products/p1': { ...rice, barcodes: ['2000000000015'], version: 2 },
    })
    const { router } = renderRoute('/products/p1')
    expect(await screen.findByLabelText('Barcodes')).toHaveValue('2000000000015')
    expect(screen.getByText(/A new in-store barcode is filled in/)).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Save' }))
    await waitFor(() => expect(calls.some((c) => c.method === 'PUT')).toBe(true))
    expect(calls.find((c) => c.method === 'PUT')?.body).toMatchObject({ barcodes: [], generate_barcode: true })
    // Adding another product is only offered on the New product form.
    expect(screen.queryByRole('button', { name: 'Save & Add Product' })).not.toBeInTheDocument()
    // Edits get the same "saved" pop-up, then the products list.
    const alert = await screen.findByRole('alertdialog', { name: 'Product updated' })
    expect(alert).toHaveAccessibleDescription('Your changes to “Miniket Rice 5kg” have been saved.')
    await userEvent.click(within(alert).getByRole('button', { name: 'Go to products' }))
    await waitFor(() => expect(router.state.location.pathname).toBe('/products'))
  })

  it('adds a sub-category and only offers parents with room below', async () => {
    const calls = mockApi({
      ...signedIn(),
      'GET /api/v1/categories': [...categories, { id: 'c-basmati', parent_id: 'c-rice', name: 'Basmati', depth: 2, sort_order: 0, status: 'active', product_count: 0 }],
      'POST /api/v1/categories': (call) => Response.json({ id: 'c-new', depth: 1, sort_order: 0, status: 'active', product_count: 0, ...(call.body as object) }, { status: 201 }),
    })
    renderRoute('/products/setup/categories')
    const parent = await screen.findByLabelText('Parent category')
    const options = within(parent).getAllByRole('option').map((o) => o.textContent)
    expect(options).toEqual(['None (top level)', 'Food', '— Rice'])
    await userEvent.type(screen.getByLabelText('Name'), 'Oil')
    await userEvent.selectOptions(parent, 'c-food')
    await userEvent.click(screen.getByRole('button', { name: 'Add' }))
    await waitFor(() =>
      expect(calls.find((c) => c.method === 'POST' && c.path === '/api/v1/categories')?.body).toEqual({
        name: 'Oil',
        parent_id: 'c-food',
        sort_order: 0,
        status: 'active',
      }),
    )
  })

  it('shows the Products menu only with products.view', async () => {
    mockApi({
      ...signedIn(['dashboard.view']),
      'GET /readyz': { status: 'ready' },
      'GET /api/v1/meta': { name: 'pos-api', version: '0.1.0', environment: 'test' },
    })
    renderRoute('/')
    const nav = await screen.findByRole('navigation', { name: 'Main' })
    expect(within(nav).queryByRole('link', { name: 'Products' })).not.toBeInTheDocument()
  })
})
