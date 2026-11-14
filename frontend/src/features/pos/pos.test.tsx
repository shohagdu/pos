import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { currentTenant, membership, mockApi, token, user } from '../../test/api'
import { renderRoute } from '../../test/render'
import { posContext, rice, sale } from '../sales/fixtures'

beforeEach(() => {
  vi.spyOn(window, 'print').mockImplementation(() => {})
})
afterEach(() => vi.restoreAllMocks())

const cash = {
  id: 'm-cash',
  name: 'Cash',
  method_type: 'cash',
  default_account_id: 'a1',
  default_account_name: 'Cash drawer',
  requires_reference: false,
  sort_order: 0,
  is_active: true,
}

function routes(extra: Record<string, unknown> = {}) {
  return {
    'POST /api/v1/auth/refresh': token('t1'),
    'GET /api/v1/auth/me': {
      user,
      memberships: [membership('t1', 'Rahim Store')],
      tenant: currentTenant('t1', 'Rahim Store', ['dashboard.view', 'pos.sell', 'sales.view']),
    },
    'GET /api/v1/pos/context': posContext,
    'GET /api/v1/pos/products': [rice],
    'GET /api/v1/pos/categories': [],
    'GET /api/v1/payment-methods': [cash],
    'GET /api/v1/held-carts': [],
    ...extra,
  } as Parameters<typeof mockApi>[0]
}

async function scanTwiceAndPay() {
  const search = await screen.findByRole('combobox', { name: 'Search products' })
  await userEvent.type(search, '4006381333931{Enter}')
  expect(await screen.findByLabelText('Quantity of Miniket Rice 5kg')).toHaveValue('1')
  expect(search).toHaveValue('')
  await userEvent.type(search, '4006381333931{Enter}')
  await waitFor(() => expect(screen.getByLabelText('Quantity of Miniket Rice 5kg')).toHaveValue('2'))
  expect(screen.getByTestId('grand-total')).toHaveTextContent('901.00')
  await userEvent.keyboard('{F8}')
  const dialog = await screen.findByRole('dialog', { name: 'Payment' })
  await userEvent.clear(within(dialog).getByLabelText('Amount 1'))
  await userEvent.type(within(dialog).getByLabelText('Amount 1'), '1000')
  expect(within(dialog).getByTestId('change')).toHaveTextContent('99.00')
  return dialog
}

describe('POS promotions', () => {
  const offer = {
    id: 'promo1',
    name: 'Rice week',
    kind: 'product_discount',
    discount_type: 'percent',
    discount_value: '10',
    targets: [{ target_type: 'product', target_id: rice.product_id }],
    starts_at: null,
    ends_at: null,
    active_days: null,
    active_time_from: null,
    active_time_to: null,
  }

  it('prices a live offer on the line and sends the discounted total', async () => {
    const calls = mockApi(routes({ 'GET /api/v1/pos/promotions': [offer], 'POST /api/v1/sales': () => Response.json(sale, { status: 201 }) }))
    renderRoute('/pos')
    const search = await screen.findByRole('combobox', { name: 'Search products' })
    await userEvent.type(search, '4006381333931{Enter}')
    await userEvent.type(search, '4006381333931{Enter}')
    // 2 × 450.50 = 901.00, 10% off = 90.10.
    await waitFor(() => expect(screen.getByTestId('grand-total')).toHaveTextContent('810.90'))
    expect(screen.getAllByText('Rice week').length).toBeGreaterThan(0)
    // No pos.remove_promotion: the offer cannot be taken off.
    expect(screen.queryByRole('button', { name: 'Remove offer' })).not.toBeInTheDocument()
    await userEvent.keyboard('{F8}')
    const dialog = await screen.findByRole('dialog', { name: 'Payment' })
    await userEvent.click(within(dialog).getByRole('button', { name: 'Confirm sale' }))
    await screen.findByRole('dialog', { name: 'Sale INV-0001 completed' })
    const post = calls.find((c) => c.method === 'POST' && c.path === '/api/v1/sales')
    expect(post?.body).toMatchObject({ expected_total: '810.90' })
  })

  it('applies a coupon the server accepts and sends its code with the sale', async () => {
    const coupon = {
      ...offer,
      id: 'promo2',
      name: 'Ten off',
      kind: 'coupon',
      targets: [{ target_type: 'all_products', target_id: null }],
      coupon_id: 'c1',
      coupon_code: 'EID10',
    }
    const me = {
      user,
      memberships: [membership('t1', 'Rahim Store')],
      tenant: currentTenant('t1', 'Rahim Store', ['dashboard.view', 'pos.sell', 'sales.view', 'pos.apply_coupon']),
    }
    const calls = mockApi(
      routes({
        'GET /api/v1/auth/me': me,
        'GET /api/v1/pos/promotions': [],
        'POST /api/v1/pos/coupons/validate': coupon,
        'POST /api/v1/sales': () => Response.json(sale, { status: 201 }),
      }),
    )
    renderRoute('/pos')
    const search = await screen.findByRole('combobox', { name: 'Search products' })
    await userEvent.type(search, '4006381333931{Enter}')
    await userEvent.type(await screen.findByLabelText(/^Coupon/), 'eid10')
    await userEvent.click(screen.getByRole('button', { name: 'Apply' }))
    // 450.50 − 10% = 405.45.
    await waitFor(() => expect(screen.getByTestId('grand-total')).toHaveTextContent('405.45'))
    expect(screen.getByRole('status', { name: '' })).toHaveTextContent(/Applied: saves .*45\.05/)
    expect(calls.find((c) => c.path === '/api/v1/pos/coupons/validate')?.body).toMatchObject({ code: 'EID10' })
    await userEvent.keyboard('{F8}')
    const dialog = await screen.findByRole('dialog', { name: 'Payment' })
    await userEvent.click(within(dialog).getByRole('button', { name: 'Confirm sale' }))
    await screen.findByRole('dialog', { name: 'Sale INV-0001 completed' })
    expect(calls.find((c) => c.method === 'POST' && c.path === '/api/v1/sales')?.body).toMatchObject({ coupon_code: 'EID10', expected_total: '405.45' })
  })
})

describe('POS', () => {
  it('adds scanned items and posts the sale with an idempotency key and the expected total', async () => {
    const calls = mockApi(routes({ 'POST /api/v1/sales': () => Response.json(sale, { status: 201 }) }))
    renderRoute('/pos')
    const dialog = await scanTwiceAndPay()
    await userEvent.click(within(dialog).getByRole('button', { name: 'Confirm sale' }))

    expect(await screen.findByRole('dialog', { name: 'Sale INV-0001 completed' })).toBeInTheDocument()
    const post = calls.find((c) => c.method === 'POST' && c.path === '/api/v1/sales')
    expect(post?.headers.get('Idempotency-Key')).toMatch(/^[0-9a-f-]{36}$/)
    expect(post?.body).toEqual({
      lines: [{ variant_id: 'v1', quantity: '2' }],
      payments: [{ method_id: 'm-cash', amount: '1000' }],
      expected_total: '901.00',
    })
    await userEvent.click(screen.getByRole('button', { name: 'New sale' }))
    expect(await screen.findByText(/Tap a product or scan a barcode/)).toBeInTheDocument()
  })

  it('lists every product with stock and price, adds one on click and marks it as in the cart', async () => {
    const calls = mockApi(routes())
    renderRoute('/pos')
    const grid = await screen.findByRole('region', { name: 'Products' })
    const card = await within(grid).findByRole('button', { name: 'Add Miniket Rice 5kg to cart' })
    expect(card).toHaveAccessibleDescription('10 pcs in stock')
    await userEvent.click(card)
    expect(await screen.findByLabelText('Quantity of Miniket Rice 5kg')).toHaveValue('1')
    expect(card).toHaveTextContent('1 in cart')
    await userEvent.click(card)
    await waitFor(() => expect(screen.getByLabelText('Quantity of Miniket Rice 5kg')).toHaveValue('2'))
    expect(card).toHaveTextContent('2 in cart')
    // Cart lines carry a serial number (SL), as on the receipt.
    const line = within(screen.getByRole('list', { name: 'Items in the cart' })).getAllByRole('listitem')[0]
    expect(line).toHaveTextContent(/^SL 1/)
    expect(screen.getByRole('region', { name: 'Cart' })).toHaveTextContent('1 item, 2 total')
    expect(calls.some((c) => c.path === '/api/v1/pos/products' && c.search.includes('limit=48'))).toBe(true)
  })

  it('filters the grid by category', async () => {
    const calls = mockApi(routes({ 'GET /api/v1/pos/categories': [{ id: 'cat-rice', name: 'Rice', product_count: 1 }] }))
    renderRoute('/pos')
    const filter = await screen.findByRole('group', { name: 'Filter by category' })
    expect(within(filter).getByRole('button', { name: 'All' })).toHaveAttribute('aria-pressed', 'true')
    await userEvent.click(within(filter).getByRole('button', { name: 'Rice (1)' }))
    expect(within(filter).getByRole('button', { name: 'Rice (1)' })).toHaveAttribute('aria-pressed', 'true')
    await waitFor(() => expect(calls.some((c) => c.path === '/api/v1/pos/products' && c.search.includes('category_id=cat-rice'))).toBe(true))
  })

  it('shows low stock and blocks out-of-stock products when negative stock is not allowed', async () => {
    const low = { ...rice, reorder_level: '12.000' }
    const out = { ...rice, variant_id: 'v2', name: 'Soybean Oil 1L', stock: '0.0000' }
    mockApi(routes({ 'GET /api/v1/pos/products': [low, out], 'GET /api/v1/pos/context': { ...posContext, allow_negative_stock: false } }))
    renderRoute('/pos')
    const grid = await screen.findByRole('region', { name: 'Products' })
    expect(await within(grid).findByRole('button', { name: 'Add Miniket Rice 5kg to cart' })).toHaveAccessibleDescription('Only 10 pcs left')
    const oil = within(grid).getByRole('button', { name: 'Add Soybean Oil 1L to cart' })
    expect(oil).toBeDisabled()
    expect(oil).toHaveAccessibleDescription('Out of stock')
  })

  it('shows the product code on each card', async () => {
    mockApi(routes())
    renderRoute('/pos')
    const grid = await screen.findByRole('region', { name: 'Products' })
    expect(await within(grid).findByRole('button', { name: 'Add Miniket Rice 5kg to cart' })).toHaveTextContent('P000001')
  })

  it('searches the product panel by name or code through the API and shows when nothing matches', async () => {
    const calls = mockApi(
      routes({
        'GET /api/v1/pos/products': (call: { search: string }) => {
          const q = new URLSearchParams(call.search).get('q') ?? ''
          return Response.json(q === '' || 'miniket rice 5kg p000001'.includes(q.toLowerCase()) ? [rice] : [])
        },
      }),
    )
    renderRoute('/pos')
    const grid = await screen.findByRole('region', { name: 'Products' })
    const filter = within(grid).getByRole('searchbox', { name: 'Filter products' })
    await within(grid).findByRole('button', { name: 'Add Miniket Rice 5kg to cart' })

    await userEvent.type(filter, 'P0000')
    await waitFor(() =>
      expect(calls.some((c) => c.path === '/api/v1/pos/products' && c.search.includes('q=P0000') && c.search.includes('limit=48'))).toBe(true),
    )
    expect(within(grid).getByRole('button', { name: 'Add Miniket Rice 5kg to cart' })).toBeInTheDocument()

    await userEvent.type(filter, 'zz')
    expect(await within(grid).findByText('No products found')).toBeInTheDocument()
    // Two ways to clear: the × in the box and the link in the empty state.
    const [inBox, inEmptyState] = within(grid).getAllByRole('button', { name: 'Clear search' })
    expect(inEmptyState).toBeDefined()
    if (!inBox) throw new Error('no clear button')
    await userEvent.click(inBox)
    expect(filter).toHaveValue('')
    expect(await within(grid).findByRole('button', { name: 'Add Miniket Rice 5kg to cart' })).toBeInTheDocument()
    // The scan bar is untouched by the panel search.
    expect(screen.getByRole('combobox', { name: 'Search products' })).toHaveValue('')
  })

  it('keeps Ctrl+Delete and Esc inside the panel search while F-keys still work', async () => {
    mockApi(routes())
    renderRoute('/pos')
    const grid = await screen.findByRole('region', { name: 'Products' })
    await userEvent.click(await within(grid).findByRole('button', { name: 'Add Miniket Rice 5kg to cart' }))
    expect(await screen.findByLabelText('Quantity of Miniket Rice 5kg')).toHaveValue('1')

    const filter = within(grid).getByRole('searchbox', { name: 'Filter products' })
    await userEvent.type(filter, 'rice')
    await userEvent.keyboard('{Control>}{Delete}{/Control}')
    // The selected cart line is still there.
    expect(screen.getByLabelText('Quantity of Miniket Rice 5kg')).toHaveValue('1')
    await userEvent.keyboard('{Escape}')
    expect(filter).toHaveValue('')
    await userEvent.keyboard('{F2}')
    expect(screen.getByRole('combobox', { name: 'Search products' })).toHaveFocus()
  })

  it('lays cards out as name on top, thumbnail beside code, price and stock, and drops the thumbnails (cards and cart) when images are off', async () => {
    mockApi(routes())
    const first = renderRoute('/pos')
    let card = await within(await screen.findByRole('region', { name: 'Products' })).findByRole('button', { name: 'Add Miniket Rice 5kg to cart' })
    // The placeholder icon stands in for the missing photo.
    expect(card.querySelector('svg')).not.toBeNull()
    expect(card).toHaveTextContent(/Miniket Rice 5kg.*P000001.*450\.50.*10 pcs in stock/)
    first.unmount()

    vi.restoreAllMocks()
    mockApi(routes({ 'GET /api/v1/pos/context': { ...posContext, show_product_images: false } }))
    renderRoute('/pos')
    card = await within(await screen.findByRole('region', { name: 'Products' })).findByRole('button', { name: 'Add Miniket Rice 5kg to cart' })
    expect(card.querySelector('svg')).toBeNull()
    expect(card).toHaveTextContent(/P000001.*450\.50.*10 pcs in stock/)
    await userEvent.click(card)
    const qty = await screen.findByLabelText('Quantity of Miniket Rice 5kg')
    expect(qty).toHaveValue('1')
    // The cart line drops its thumbnail too.
    expect(qty.closest('li')?.querySelector('svg')).toBeNull()
  })

  it('hides the product panel when it is turned off in settings and the cart still works', async () => {
    mockApi(routes({ 'GET /api/v1/pos/context': { ...posContext, show_product_panel: false } }))
    renderRoute('/pos')
    const search = await screen.findByRole('combobox', { name: 'Search products' })
    expect(screen.queryByRole('region', { name: 'Products' })).not.toBeInTheDocument()
    await userEvent.type(search, '4006381333931{Enter}')
    expect(await screen.findByLabelText('Quantity of Miniket Rice 5kg')).toHaveValue('1')
    await userEvent.keyboard('{F8}')
    expect(await screen.findByRole('dialog', { name: 'Payment' })).toBeInTheDocument()
  })

  it('shows a changed price next to the stock and what a % discount comes to', async () => {
    mockApi(
      routes({
        'GET /api/v1/auth/me': {
          user,
          memberships: [membership('t1', 'Rahim Store')],
          tenant: currentTenant('t1', 'Rahim Store', ['dashboard.view', 'pos.sell', 'sales.view', 'pos.price_override', 'pos.discount_over_limit']),
        },
      }),
    )
    renderRoute('/pos')
    await userEvent.type(await screen.findByRole('combobox', { name: 'Search products' }), '4006381333931{Enter}')
    const lines = await screen.findByRole('list', { name: 'Items in the cart' })
    const price = within(lines).getByLabelText('Unit price of Miniket Rice 5kg')
    await userEvent.clear(price)
    await userEvent.type(price, '400')
    // The regular price sits on the info line beside the stock, not under the input.
    const info = within(lines).getByText(/P000001 · Stock 10 pcs/)
    expect(info).toHaveTextContent('Regular price')
    expect(info.querySelector('s')).toHaveTextContent('450.50')
    expect(price.parentElement?.querySelector('s')).toBeNull()

    await userEvent.selectOptions(within(lines).getByLabelText('Discount type for Miniket Rice 5kg'), 'percent')
    await userEvent.type(within(lines).getByLabelText('Discount on Miniket Rice 5kg'), '10')
    expect(await within(lines).findByText(/10% off: −.*40\.00/)).toBeInTheDocument()
    // It sits beside the % selector and describes the discount box.
    expect(within(lines).getByLabelText('Discount on Miniket Rice 5kg')).toHaveAccessibleDescription(/10% off/)
    expect(within(lines).getByLabelText('Discount type for Miniket Rice 5kg').nextElementSibling).toHaveTextContent('10% off')
    await userEvent.selectOptions(within(lines).getByLabelText('Discount type for Miniket Rice 5kg'), 'amount')
    expect(within(lines).queryByText(/% off/)).not.toBeInTheDocument()
  })

  it('shows the warranty of a cart line and when it would run out', async () => {
    const powerBank = { ...rice, warranty_period: 3, warranty_unit: 'month' as const }
    mockApi(routes({ 'GET /api/v1/pos/products': [powerBank] }))
    renderRoute('/pos')
    await userEvent.type(await screen.findByRole('combobox', { name: 'Search products' }), '4006381333931{Enter}')
    const cart = await screen.findByRole('list', { name: 'Items in the cart' })
    // The POS business date is 18 Oct 2026.
    expect(within(cart).getByText('Warranty 3 months · until 18 Jan 2027')).toBeInTheDocument()
  })

  it('confirms the sale from the keyboard with F8', async () => {
    const calls = mockApi(routes({ 'POST /api/v1/sales': () => Response.json(sale, { status: 201 }) }))
    renderRoute('/pos')
    await scanTwiceAndPay()
    await userEvent.keyboard('{F8}')
    expect(await screen.findByRole('dialog', { name: 'Sale INV-0001 completed' })).toBeInTheDocument()
    expect(calls.filter((c) => c.method === 'POST' && c.path === '/api/v1/sales')).toHaveLength(1)
  })

  it('tabs from the amount straight to Confirm sale', async () => {
    mockApi(routes())
    renderRoute('/pos')
    const dialog = await scanTwiceAndPay()
    expect(within(dialog).getByLabelText('Amount 1')).toHaveFocus()
    await userEvent.tab()
    expect(within(dialog).getByRole('button', { name: 'Confirm sale' })).toHaveFocus()
  })

  it('confirms the sale with Ctrl+Enter', async () => {
    mockApi(routes({ 'POST /api/v1/sales': () => Response.json(sale, { status: 201 }) }))
    renderRoute('/pos')
    await scanTwiceAndPay()
    await userEvent.keyboard('{Control>}{Enter}{/Control}')
    expect(await screen.findByRole('dialog', { name: 'Sale INV-0001 completed' })).toBeInTheDocument()
  })

  it('reuses the idempotency key when retrying after a network error', async () => {
    let attempts = 0
    const calls = mockApi(
      routes({
        'POST /api/v1/sales': () => {
          attempts += 1
          if (attempts === 1) return Promise.reject(new TypeError('Failed to fetch'))
          return Response.json(sale, { status: 201 })
        },
      }),
    )
    renderRoute('/pos')
    const dialog = await scanTwiceAndPay()
    await userEvent.click(within(dialog).getByRole('button', { name: 'Confirm sale' }))
    expect(await within(dialog).findByText('Cannot reach the server. Check your connection.')).toBeInTheDocument()
    await userEvent.click(within(dialog).getByRole('button', { name: 'Confirm sale' }))
    expect(await screen.findByRole('dialog', { name: 'Sale INV-0001 completed' })).toBeInTheDocument()

    const keys = calls.filter((c) => c.method === 'POST' && c.path === '/api/v1/sales').map((c) => c.headers.get('Idempotency-Key'))
    expect(keys).toHaveLength(2)
    expect(keys[0]).toBe(keys[1])
  })

  it('finds a customer by phone in the inline search and puts them on the cart', async () => {
    const karim = {
      id: 'c1',
      code: 'C0001',
      name: 'Karim Mia',
      phone: '01711000002',
      email: null,
      address: null,
      credit_limit: null,
      opening_balance: '0.00',
      current_balance: '250.00',
      is_walk_in: false,
      notes: null,
      status: 'active',
      version: 1,
    }
    const calls = mockApi(routes({ 'GET /api/v1/customers': { data: [karim], next_cursor: null } }))
    renderRoute('/pos')
    await screen.findByRole('combobox', { name: 'Search products' })
    expect(screen.getByTestId('pos-customer')).toHaveTextContent('Walk-in customer')
    await userEvent.keyboard('{F4}')
    const search = screen.getByRole('combobox', { name: 'Search customers' })
    expect(search).toHaveFocus()
    await userEvent.type(search, '017110')
    const option = await screen.findByRole('option', { name: /Karim Mia/ })
    expect(option).toHaveTextContent('Due')
    expect(calls.find((c) => c.path === '/api/v1/customers')?.search).toContain('q=017110')
    await userEvent.keyboard('{Enter}')
    expect(screen.getByTestId('pos-customer')).toHaveTextContent('Karim Mia')
    expect(search).toHaveValue('')
    await userEvent.click(screen.getByRole('button', { name: 'Walk-in' }))
    expect(screen.getByTestId('pos-customer')).toHaveTextContent('Walk-in customer')
  })

  it('adds a new customer from what was typed and selects them', async () => {
    const calls = mockApi(
      routes({
        'GET /api/v1/customers': { data: [], next_cursor: null },
        'POST /api/v1/customers': (call: { body: unknown }) =>
          Response.json(
            {
              id: 'c9',
              code: 'C0009',
              name: 'Salma Begum',
              phone: '01811000009',
              email: null,
              address: null,
              credit_limit: null,
              opening_balance: '0.00',
              current_balance: '0.00',
              is_walk_in: false,
              notes: null,
              status: 'active',
              version: 1,
              ...(call.body as object),
            },
            { status: 201 },
          ),
      }),
    )
    renderRoute('/pos')
    const search = await screen.findByRole('combobox', { name: 'Search customers' })
    await userEvent.type(search, '01811000009')
    expect(await screen.findByText('No customer found.')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('option', { name: '+ Add “01811000009” as a new customer' }))
    const dialog = screen.getByRole('dialog', { name: 'New customer' })
    expect(within(dialog).getByLabelText('Phone')).toHaveValue('01811000009')
    await userEvent.type(within(dialog).getByLabelText('Name'), 'Salma Begum')
    await userEvent.click(within(dialog).getByRole('button', { name: 'Add and select' }))
    await waitFor(() => expect(screen.getByTestId('pos-customer')).toHaveTextContent('Salma Begum'))
    expect(calls.find((c) => c.method === 'POST' && c.path === '/api/v1/customers')?.body).toEqual({
      name: 'Salma Begum',
      phone: '01811000009',
    })
    expect(screen.getByRole('button', { name: '+ New customer' })).toBeInTheDocument()
  })
})
