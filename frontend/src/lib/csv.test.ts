import { describe, expect, it } from 'vitest'

import { toCsv } from './csv'

describe('csv', () => {
  it('quotes commas, quotes and newlines', () => {
    const rows = [{ name: 'Rice, 5kg', note: 'say "hi"' }, { name: 'চাল', note: null }]
    expect(toCsv(rows, [{ header: 'Name', value: (r) => r.name }, { header: 'Note', value: (r) => r.note }])).toBe(
      'Name,Note\r\n"Rice, 5kg","say ""hi"""\r\nচাল,',
    )
  })
})
