import { render, screen, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { receipt, sale } from './fixtures'
import { SaleReceipt } from './Receipt'

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
