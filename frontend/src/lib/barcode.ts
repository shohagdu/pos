/**
 * Barcode encoding for printed labels (PROD-R05), without a library.
 *
 * Valid EAN-13, UPC-A and EAN-8 codes are drawn in their own symbology, the
 * one shop scanners expect on retail goods; anything else (SKUs, free-format
 * codes) is drawn as Code 128. The result is a run of modules: `1` = bar,
 * `0` = space, each one module wide.
 */

export type Symbology = 'ean13' | 'upca' | 'ean8' | 'code128'

export interface EncodedBarcode {
  symbology: Symbology
  /** `1`/`0` per module, quiet zones excluded. */
  modules: string
  /** Text printed under the bars. */
  text: string
}

// ------------------------------------------------------------------ EAN / UPC

const EAN_L = ['0001101', '0011001', '0010011', '0111101', '0100011', '0110001', '0101111', '0111011', '0110111', '0001011']
const EAN_R = EAN_L.map((p) => [...p].map((b) => (b === '1' ? '0' : '1')).join(''))
const EAN_G = EAN_R.map((p) => [...p].reverse().join(''))
/** Left-half parity per leading EAN-13 digit. */
const EAN_PARITY = ['LLLLLL', 'LLGLGG', 'LLGGLG', 'LLGGGL', 'LGLLGG', 'LGGLLG', 'LGGGLL', 'LGLGLG', 'LGLGGL', 'LGGLGL']

/** GS1 check digit for the digits before it (weights 3, 1 from the right). */
export function gs1CheckDigit(body: string): number {
  let sum = 0
  for (let i = 0; i < body.length; i++) {
    const digit = body.charCodeAt(body.length - 1 - i) - 48
    sum += digit * (i % 2 === 0 ? 3 : 1)
  }
  return (10 - (sum % 10)) % 10
}

const isDigits = (s: string) => /^[0-9]+$/.test(s)
const hasValidCheck = (code: string) => gs1CheckDigit(code.slice(0, -1)) === Number(code.at(-1))

function ean13Modules(code: string): string {
  const parity = EAN_PARITY[Number(code[0])] ?? ''
  let out = '101'
  for (let i = 1; i <= 6; i++) out += (parity[i - 1] === 'L' ? EAN_L : EAN_G)[Number(code[i])]
  out += '01010'
  for (let i = 7; i <= 12; i++) out += EAN_R[Number(code[i])]
  return out + '101'
}

function ean8Modules(code: string): string {
  let out = '101'
  for (let i = 0; i < 4; i++) out += EAN_L[Number(code[i])]
  out += '01010'
  for (let i = 4; i < 8; i++) out += EAN_R[Number(code[i])]
  return out + '101'
}

// ------------------------------------------------------------------ Code 128

/** Bar/space widths of symbols 0–105, then the stop symbol (106). */
const CODE128 = [
  '212222',
  '222122',
  '222221',
  '121223',
  '121322',
  '131222',
  '122213',
  '122312',
  '132212',
  '221213',
  '221312',
  '231212',
  '112232',
  '122132',
  '122231',
  '113222',
  '123122',
  '123221',
  '223211',
  '221132',
  '221231',
  '213212',
  '223112',
  '312131',
  '311222',
  '321122',
  '321221',
  '312212',
  '322112',
  '322211',
  '212123',
  '212321',
  '232121',
  '111323',
  '131123',
  '131321',
  '112313',
  '132113',
  '132311',
  '211313',
  '231113',
  '231311',
  '112133',
  '112331',
  '132131',
  '113123',
  '113321',
  '133121',
  '313121',
  '211331',
  '231131',
  '213113',
  '213311',
  '213131',
  '311123',
  '311321',
  '331121',
  '312113',
  '312311',
  '332111',
  '314111',
  '221411',
  '431111',
  '111224',
  '111422',
  '121124',
  '121421',
  '141122',
  '141221',
  '112214',
  '112412',
  '122114',
  '122411',
  '142112',
  '142211',
  '241211',
  '221114',
  '413111',
  '241112',
  '134111',
  '111242',
  '121142',
  '121241',
  '114212',
  '124112',
  '124211',
  '411212',
  '421112',
  '421211',
  '212141',
  '214121',
  '412121',
  '111143',
  '111341',
  '131141',
  '114113',
  '114311',
  '411113',
  '411311',
  '113141',
  '114131',
  '311141',
  '411131',
  '211412',
  '211214',
  '211232',
  '2331112',
]
export const CODE128_PATTERNS: readonly string[] = CODE128
const START_B = 104
const START_C = 105
const CODE_B = 100
const STOP = 106

function widthsToModules(widths: string): string {
  let out = ''
  for (let i = 0; i < widths.length; i++) out += (i % 2 === 0 ? '1' : '0').repeat(Number(widths[i]))
  return out
}

/** Symbol values for `text`: set C for digit pairs, set B otherwise. */
function code128Values(text: string): number[] | null {
  if (![...text].every((c) => c >= ' ' && c <= '~')) return null
  if (isDigits(text) && text.length >= 4) {
    const values = [START_C]
    const pairs = text.length - (text.length % 2)
    for (let i = 0; i < pairs; i += 2) values.push(Number(text.slice(i, i + 2)))
    if (pairs < text.length) values.push(CODE_B, text.charCodeAt(pairs) - 32)
    return values
  }
  return [START_B, ...[...text].map((c) => c.charCodeAt(0) - 32)]
}

/** Modules for `text` in Code 128 (start, data, check, stop); `null` for characters outside ASCII 32–126. */
export function code128Modules(text: string): string | null {
  const values = code128Values(text)
  if (!values || text.length === 0) return null
  const check = values.reduce((sum, v, i) => sum + v * Math.max(i, 1), 0) % 103
  return [...values, check, STOP].map((v) => widthsToModules(CODE128[v] ?? '')).join('')
}

// ------------------------------------------------------------------ choice

/** Encodes `value` in the best-fitting symbology, or `null` when it cannot be drawn. */
export function encodeBarcode(raw: string): EncodedBarcode | null {
  const value = raw.trim()
  if (!value) return null
  if (isDigits(value) && hasValidCheck(value)) {
    if (value.length === 13) return { symbology: 'ean13', modules: ean13Modules(value), text: value }
    if (value.length === 12) return { symbology: 'upca', modules: ean13Modules(`0${value}`), text: value }
    if (value.length === 8) return { symbology: 'ean8', modules: ean8Modules(value), text: value }
  }
  const modules = code128Modules(value)
  return modules ? { symbology: 'code128', modules, text: value } : null
}
