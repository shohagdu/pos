import { render, screen, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import type { SaleDetail, SaleReturnDetail } from '../../api/sales'
import { toFixed } from '../../lib/decimal'
import { receipt, sale } from './fixtures'
import { SaleReceipt } from './Receipt'
import { netOfReturns } from './returnMath'

describe('receipt header', () => {
  it('prints the BIN for a Bangladeshi business', () => {
    render(<SaleReceipt sale={{ ...sale, receipt: { ...receipt, bin_number: '000123456-0101' } }} />)
    expect(within(screen.getByTestId('receipt')).getByText('BIN: 000123456-0101')).toBeInTheDocument()
  })

  it('prints the Partita IVA, codice fiscale and R.E.A. for an Italian business', () => {
    const italian = {
      ...receipt,
      country_code: 'IT',
      legal_name: 'di Uddin Md Jashim',
      bin_number: '03580690984',
      tax_code: 'DDNMJS87T15Z249I',
      rea_number: '546330',
      registry_office: 'BS',
    }
    render(<SaleReceipt sale={{ ...sale, receipt: italian }} />)
    const printed = within(screen.getByTestId('receipt'))
    expect(printed.getByText('di Uddin Md Jashim')).toBeInTheDocument()
    expect(printed.getByText('P.IVA 03580690984')).toBeInTheDocument()
    expect(printed.getByText('C.F. DDNMJS87T15Z249I')).toBeInTheDocument()
    expect(printed.getByText('REA BS 546330')).toBeInTheDocument()
    expect(printed.queryByText(/^BIN:/)).not.toBeInTheDocument()
  })
})

describe('receipt items', () => {
  it('numbers the item lines under an SL header', () => {
    const [first] = sale.items
    if (!first) throw new Error('fixture has no items')
    const second = { ...first, id: 'si2', line_no: 2, product_name: 'Soybean Oil 1L' }
    render(<SaleReceipt sale={{ ...sale, items: [first, second] }} />)
    const printed = within(screen.getByTestId('receipt'))
    expect(printed.getByText('SL')).toBeInTheDocument()
    expect(printed.getByText('Item')).toBeInTheDocument()
    expect(printed.getByText('1.').parentElement).toHaveTextContent(first.product_name)
    expect(printed.getByText('2.').parentElement).toHaveTextContent('Soybean Oil 1L')
  })
})

describe('invoice printed without returned items', () => {
  const [base] = sale.items
  if (!base) throw new Error('fixture has no items')
  // 3 × 100 with 30 off and 5% VAT; one unit returned. Soap fully returned.
  const rice = {
    ...base,
    quantity: '3.0000',
    returned_quantity: '1.0000',
    unit_price: '100.0000',
    gross_amount: '300.0000',
    line_discount: '30.0000',
    taxable_amount: '270.0000',
    tax_rate_pct: '5.0000',
    tax_amount: '13.5000',
    line_total: '283.5000',
  }
  const soap = {
    ...rice,
    id: 'si2',
    line_no: 2,
    product_name: 'Bath Soap',
    quantity: '1.0000',
    returned_quantity: '1.0000',
    unit_price: '50.0000',
    gross_amount: '50.0000',
    line_discount: '0.0000',
    taxable_amount: '50.0000',
    tax_amount: '2.5000',
    line_total: '52.5000',
  }
  const sold: SaleDetail = {
    ...sale,
    status: 'partially_returned',
    subtotal: '350.0000',
    item_discount_total: '30.0000',
    tax_total: '16.0000',
    grand_total: '336.0000',
    paid_total: '336.0000',
    change_given: '0.0000',
    returned_total: '147.0000',
    items: [rice, soap],
    returns: [{ id: 'r1', return_no: 'RET-0001', business_date: '2026-10-19', refund_mode: 'cash', total: '147.0000' }],
  }
  const ret = {
    return_no: 'RET-0001',
    refunded_amount: '147.0000',
    credited_amount: '0.0000',
    items: [
      { sale_item_id: 'si1', line_total: '94.5000', tax_amount: '4.5000' },
      { sale_item_id: 'si2', line_total: '52.5000', tax_amount: '2.5000' },
    ],
  } as unknown as SaleReturnDetail

  it('works out what is left: kept lines, exact totals that add up', () => {
    const net = netOfReturns(sold, [ret])
    expect(net.lines.map((l) => [l.item.id, toFixed(l.quantity, 0), toFixed(l.lineTotal)])).toEqual([['si1', '2', '189.00']])
    expect(toFixed(net.tax)).toBe('9.00')
    expect(toFixed(net.discount)).toBe('20.00')
    expect(toFixed(net.subtotal)).toBe('200.00')
    expect(toFixed(net.grandTotal)).toBe('189.00')
    expect(toFixed(net.refunded)).toBe('147.00')
  })

  it('prints only what was kept, marked and with the returns listed', () => {
    render(<SaleReceipt sale={sold} net={netOfReturns(sold, [ret])} />)
    const printed = screen.getByTestId('receipt')
    expect(printed).toHaveTextContent('AFTER RETURNS')
    expect(printed).not.toHaveTextContent('Bath Soap')
    expect(printed).toHaveTextContent(/2 pcs × .*100\.00/)
    expect(printed).toHaveTextContent(/TOTAL.*189\.00/)
    expect(printed).toHaveTextContent(/Refunded.*147\.00/)
    expect(printed).toHaveTextContent('Returns: RET-0001')
    expect(printed).not.toHaveTextContent(/Returned\b/)
  })
})
