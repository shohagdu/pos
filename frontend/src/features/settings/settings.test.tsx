import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { currentTenant, membership, mockApi, token, user } from '../../test/api'
import { renderRoute } from '../../test/render'

afterEach(() => vi.restoreAllMocks())

const settings = {
  name: 'Rahim Store',
  slug: 'rahim-store',
  legal_name: null,
  business_type: 'grocery',
  phone: null,
  email: null,
  address: null,
  district: null,
  country_code: 'BD',
  bin_number: null,
  trade_license_no: null,
  tax_code: null,
  rea_number: null,
  registry_office: null,
  currency_code: 'BDT',
  timezone: 'Asia/Dhaka',
  locale: 'en',
  fiscal_year_start_month: 7,
  invoice_prefix: 'INV',
  receipt_paper: '80mm',
  receipt_footer: null,
  sku_prefix: 'P',
  sku_digits: 6,
  barcode_prefix: '20',
  barcode_digits: 7,
  auto_print_receipt: true,
  show_product_panel: true,
  show_product_images: true,
  is_vat_registered: false,
  prices_include_tax: false,
  version: 3,
}

describe('settings: invoice, tax & POS', () => {
  it('turns the POS product panel off', async () => {
    const calls = mockApi({
      'POST /api/v1/auth/refresh': token('t1'),
      'GET /api/v1/auth/me': { user, memberships: [membership('t1', 'Rahim Store')], tenant: currentTenant('t1', 'Rahim Store', ['settings.manage']) },
      'GET /api/v1/settings': settings,
      'PATCH /api/v1/settings': (call) => Response.json({ ...settings, ...(call.body as object), version: 4 }),
    })
    renderRoute('/settings/invoice')

    const toggle = await screen.findByRole('checkbox', { name: 'Show product panel on POS screen' })
    expect(toggle).toBeChecked()
    expect(toggle).toHaveAccessibleDescription(/sell mostly by scanning barcodes/)
    await userEvent.click(toggle)
    await userEvent.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() => expect(calls.some((c) => c.method === 'PATCH')).toBe(true))
    const patch = calls.find((c) => c.method === 'PATCH' && c.path === '/api/v1/settings')
    expect(patch?.body).toMatchObject({ show_product_panel: false, auto_print_receipt: true })
    expect(patch?.headers.get('If-Match')).toContain('3')
  })

  it('keeps the image toggle usable while the product panel is off (it also covers cart and search) and saves both', async () => {
    const calls = mockApi({
      'POST /api/v1/auth/refresh': token('t1'),
      'GET /api/v1/auth/me': { user, memberships: [membership('t1', 'Rahim Store')], tenant: currentTenant('t1', 'Rahim Store', ['settings.manage']) },
      'GET /api/v1/settings': settings,
      'PATCH /api/v1/settings': (call) => Response.json({ ...settings, ...(call.body as object), version: 4 }),
    })
    renderRoute('/settings/invoice')

    const images = await screen.findByRole('checkbox', { name: 'Show product images on POS' })
    expect(images).toBeEnabled()
    expect(images).toBeChecked()
    await userEvent.click(images)
    await userEvent.click(screen.getByRole('checkbox', { name: 'Show product panel on POS screen' }))
    expect(images).toBeEnabled()
    expect(images).not.toBeChecked()
    expect(images).toHaveAccessibleDescription(/cart lines and search results/)
    await userEvent.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() => expect(calls.some((c) => c.method === 'PATCH')).toBe(true))
    expect(calls.find((c) => c.method === 'PATCH')?.body).toMatchObject({ show_product_panel: false, show_product_images: false })
  })

  it('sets how automatic SKUs and barcodes are numbered, with an example and the retail-length rule', async () => {
    const calls = mockApi({
      'POST /api/v1/auth/refresh': token('t1'),
      'GET /api/v1/auth/me': { user, memberships: [membership('t1', 'Rahim Store')], tenant: currentTenant('t1', 'Rahim Store', ['settings.manage']) },
      'GET /api/v1/settings': settings,
      'PATCH /api/v1/settings': (call) => Response.json({ ...settings, ...(call.body as object), version: 4 }),
    })
    renderRoute('/settings/codes')

    const sku = await screen.findByRole('group', { name: 'SKU' })
    expect(within(sku).getByText('P000001')).toBeInTheDocument()
    const skuPrefix = within(sku).getByLabelText('Starts with')
    await userEvent.clear(skuPrefix)
    await userEvent.type(skuPrefix, 'mir')
    await userEvent.selectOptions(within(sku).getByLabelText('Number of digits'), '4')
    expect(within(sku).getByText('MIR0001')).toBeInTheDocument()

    // 20 + 6 digits is 8 digits: it would read as an EAN-8.
    const barcode = screen.getByRole('group', { name: 'Barcode' })
    expect(within(barcode).getByText('200000001')).toBeInTheDocument()
    await userEvent.selectOptions(within(barcode).getByLabelText('Number of digits'), '6')
    expect(within(barcode).getByLabelText('Number of digits')).toHaveAccessibleDescription(/retail format/)
    await userEvent.click(screen.getByRole('button', { name: 'Save' }))
    expect(calls.some((c) => c.method === 'PATCH')).toBe(false)

    await userEvent.selectOptions(within(barcode).getByLabelText('Number of digits'), '7')
    await userEvent.click(screen.getByRole('button', { name: 'Save' }))
    await waitFor(() => expect(calls.some((c) => c.method === 'PATCH')).toBe(true))
    expect(calls.find((c) => c.method === 'PATCH')?.body).toEqual({ sku_prefix: 'mir', sku_digits: 4, barcode_prefix: '20', barcode_digits: 7 })
    expect(await screen.findByText('Saved.')).toBeInTheDocument()
  })
})
