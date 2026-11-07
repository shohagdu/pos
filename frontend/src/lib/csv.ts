// CSV export of on-screen reports (Requirement Analysis §15: CSV for P0).
// Values are written as shown; amounts stay decimal strings.

export interface CsvColumn<T> {
  header: string
  value: (row: T) => string | number | null | undefined
}

/**
 * A text cell that starts like a formula (`=`, `+`, `-`, `@`, tab, CR) would run
 * when opened in Excel (e.g. a customer named `=HYPERLINK(...)`), so it gets a
 * leading `'`. Numbers, and text that is just a number such as "-12.50", stay as they are.
 */
function neutralise(text: string): string {
  if (/^[=@\t\r]/.test(text)) return `'${text}`
  if (/^[+-]/.test(text) && !/^[+-]?\d+(\.\d+)?$/.test(text)) return `'${text}`
  return text
}

function cell(value: string | number | null | undefined): string {
  const text = value === null || value === undefined ? '' : typeof value === 'number' ? String(value) : neutralise(value)
  return /[",\n\r]/.test(text) ? `"${text.replace(/"/g, '""')}"` : text
}

export function toCsv<T>(rows: T[], columns: CsvColumn<T>[]): string {
  const lines = [columns.map((c) => cell(c.header)).join(',')]
  for (const row of rows) lines.push(columns.map((c) => cell(c.value(row))).join(','))
  return lines.join('\r\n')
}

/** Downloads `rows` as a UTF-8 CSV file (with BOM so Excel shows Bengali). */
export function downloadCsv<T>(filename: string, rows: T[], columns: CsvColumn<T>[]) {
  downloadCsvText(filename, toCsv(rows, columns))
}

/** Downloads plain rows of cells (the first is the header) as a CSV file. */
export function downloadCsvRows(filename: string, rows: (string | number | null | undefined)[][]) {
  downloadCsvText(filename, rows.map((r) => r.map(cell).join(',')).join('\r\n'))
}

function downloadCsvText(filename: string, text: string) {
  const blob = new Blob(['\uFEFF', text], { type: 'text/csv;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename.endsWith('.csv') ? filename : `${filename}.csv`
  document.body.appendChild(a)
  a.click()
  a.remove()
  URL.revokeObjectURL(url)
}

/**
 * Parses CSV text (RFC 4180: quoted cells, doubled quotes, CRLF or LF, a
 * leading BOM). Also reads `;`-separated files as Excel saves them in some
 * locales. Blank lines are dropped.
 */
export function parseCsv(text: string): string[][] {
  const src = text.replace(/^\uFEFF/, '')
  const firstLine = src.slice(0, src.search(/\r?\n|$/))
  const sep = firstLine.split(';').length > firstLine.split(',').length ? ';' : ','
  const rows: string[][] = []
  let row: string[] = []
  let field = ''
  let quoted = false
  for (let i = 0; i < src.length; i++) {
    const c = src[i]
    if (quoted) {
      if (c === '"' && src[i + 1] === '"') {
        field += '"'
        i++
      } else if (c === '"') quoted = false
      else field += c
    } else if (c === '"' && field === '') quoted = true
    else if (c === sep) {
      row.push(field)
      field = ''
    } else if (c === '\n' || c === '\r') {
      if (c === '\r' && src[i + 1] === '\n') i++
      row.push(field)
      rows.push(row)
      row = []
      field = ''
    } else field += c
  }
  if (field !== '' || row.length > 0) {
    row.push(field)
    rows.push(row)
  }
  return rows.filter((r) => r.some((cellText) => cellText.trim() !== ''))
}

/** Lower-case, words joined by `_`: `Selling Price` → `selling_price`. */
export const headerKey = (header: string) =>
  header
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '_')
    .replace(/^_|_$/g, '')

/**
 * CSV rows as records keyed by header (see {@link headerKey}), each with its
 * line number in the file (the header is line 1).
 */
export function csvRecords(text: string): { headers: string[]; records: { line: number; values: Record<string, string> }[] } {
  const [head, ...body] = parseCsv(text)
  const headers = (head ?? []).map(headerKey)
  return {
    headers,
    records: body.map((cells, i) => ({
      line: i + 2,
      values: Object.fromEntries(headers.map((h, j) => [h, (cells[j] ?? '').trim()])),
    })),
  }
}

/** The text of a file the user picked. */
export function readText(file: Blob): Promise<string> {
  if (typeof file.text === 'function') return file.text()
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => resolve(String(reader.result ?? ''))
    reader.onerror = () => reject(reader.error ?? new Error('read failed'))
    reader.readAsText(file)
  })
}
