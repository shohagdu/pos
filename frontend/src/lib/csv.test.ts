import { describe, expect, it } from 'vitest'

import { csvRecords, parseCsv, toCsv } from './csv'

describe('csv', () => {
  it('quotes commas, quotes and newlines', () => {
    const rows = [{ name: 'Rice, 5kg', note: 'say "hi"' }, { name: 'চাল', note: null }]
    expect(toCsv(rows, [{ header: 'Name', value: (r) => r.name }, { header: 'Note', value: (r) => r.note }])).toBe(
      'Name,Note\r\n"Rice, 5kg","say ""hi"""\r\nচাল,',
    )
  })

  it('reads what it writes, quoted cells and all', () => {
    const text = toCsv([{ a: 'Rice, 5kg', b: 'say "hi"\nok' }], [{ header: 'A', value: (r) => r.a }, { header: 'B', value: (r) => r.b }])
    expect(parseCsv(`\uFEFF${text}\r\n\r\n`)).toEqual([['A', 'B'], ['Rice, 5kg', 'say "hi"\nok']])
  })

  it('reads semicolon files and keys records by header', () => {
    const { headers, records } = csvRecords('Name;Selling Price\nTea;20\n;\nSalt\n')
    expect(headers).toEqual(['name', 'selling_price'])
    expect(records).toEqual([
      { line: 2, values: { name: 'Tea', selling_price: '20' } },
      { line: 3, values: { name: 'Salt', selling_price: '' } },
    ])
  })
})
