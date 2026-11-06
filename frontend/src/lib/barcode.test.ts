import { describe, expect, it } from 'vitest'

import { CODE128_PATTERNS, code128Modules, encodeBarcode, gs1CheckDigit } from './barcode'

/** Run-length widths of a module string (bars and spaces alternate). */
function widths(modules: string): number[] {
  const out: number[] = []
  let run = 1
  for (let i = 1; i <= modules.length; i++) {
    if (modules[i] === modules[i - 1]) run++
    else {
      out.push(run)
      run = 1
    }
  }
  return out
}

/** Reads Code 128 symbol values back from modules (independent of the encoder's value logic). */
function decode128(modules: string): number[] {
  const values: number[] = []
  let pos = 0
  while (pos < modules.length) {
    const size = modules.length - pos === 13 ? 13 : 11
    const pattern = widths(modules.slice(pos, pos + size)).join('')
    values.push(CODE128_PATTERNS.indexOf(pattern))
    pos += size
  }
  return values
}

const L = ['0001101', '0011001', '0010011', '0111101', '0100011', '0110001', '0101111', '0111011', '0110111', '0001011']
const PARITY = ['LLLLLL', 'LLGLGG', 'LLGGLG', 'LLGGGL', 'LGLLGG', 'LGGLLG', 'LGGGLL', 'LGLGLG', 'LGLGGL', 'LGGLGL']
const flip = (p: string) => [...p].map((b) => (b === '1' ? '0' : '1')).join('')

/** Decodes 95 EAN-13 modules back to the 13 digits. */
function decodeEan13(modules: string): string {
  expect(modules).toHaveLength(95)
  expect(modules.slice(0, 3)).toBe('101')
  expect(modules.slice(45, 50)).toBe('01010')
  expect(modules.slice(92)).toBe('101')
  let parity = ''
  let digits = ''
  for (let i = 0; i < 6; i++) {
    const chunk = modules.slice(3 + i * 7, 10 + i * 7)
    const l = L.indexOf(chunk)
    if (l >= 0) {
      parity += 'L'
      digits += l
    } else {
      parity += 'G'
      digits += L.findIndex((p) => [...flip(p)].reverse().join('') === chunk)
    }
  }
  for (let i = 0; i < 6; i++) digits += L.findIndex((p) => flip(p) === modules.slice(50 + i * 7, 57 + i * 7))
  return `${PARITY.indexOf(parity)}${digits}`
}

describe('barcode', () => {
  it('Code 128 table: 107 distinct symbols of 11 modules (stop 13)', () => {
    expect(CODE128_PATTERNS).toHaveLength(107)
    expect(new Set(CODE128_PATTERNS).size).toBe(107)
    CODE128_PATTERNS.forEach((p, i) => {
      const total = [...p].reduce((s, d) => s + Number(d), 0)
      expect(total, `symbol ${i}`).toBe(i === 106 ? 13 : 11)
    })
  })

  it('GS1 check digits', () => {
    expect(gs1CheckDigit('400638133393')).toBe(1)
    expect(gs1CheckDigit('03600029145')).toBe(2)
    expect(gs1CheckDigit('9638507')).toBe(4)
    expect(gs1CheckDigit('200000000001')).toBe(5)
  })

  it('EAN-13 round-trips', () => {
    for (const code of ['4006381333931', '2000000000015', '8901030865275']) {
      const enc = encodeBarcode(code)
      expect(enc?.symbology).toBe('ean13')
      expect(decodeEan13(enc!.modules)).toBe(code)
    }
  })

  it('UPC-A is EAN-13 with a leading zero; EAN-8 has 67 modules', () => {
    const upc = encodeBarcode('036000291452')
    expect(upc?.symbology).toBe('upca')
    expect(decodeEan13(upc!.modules)).toBe('0036000291452')
    const ean8 = encodeBarcode('96385074')
    expect(ean8?.symbology).toBe('ean8')
    expect(ean8!.modules).toHaveLength(67)
  })

  it('wrong check digits and free text fall back to Code 128', () => {
    expect(encodeBarcode('4006381333932')?.symbology).toBe('code128')
    expect(encodeBarcode('P000042')?.symbology).toBe('code128')
  })

  it('Code 128 set B: start, data, check, stop', () => {
    const values = decode128(code128Modules('PJJ123C')!)
    const data = [48, 42, 42, 17, 18, 19, 35]
    expect(values.slice(0, 8)).toEqual([104, ...data])
    const check = (104 + data.reduce((s, v, i) => s + v * (i + 1), 0)) % 103
    expect(values.slice(8)).toEqual([check, 106])
  })

  it('Code 128 set C packs digit pairs and finishes an odd digit in set B', () => {
    expect(decode128(code128Modules('123456')!).slice(0, 4)).toEqual([105, 12, 34, 56])
    expect(decode128(code128Modules('12345')!).slice(0, 5)).toEqual([105, 12, 34, 100, 21])
  })

  it('rejects what cannot be drawn', () => {
    expect(encodeBarcode('')).toBeNull()
    expect(encodeBarcode('দাম')).toBeNull()
  })
})
