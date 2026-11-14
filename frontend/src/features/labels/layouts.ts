/**
 * Label stock the shop prints on: thermal rolls (one label per page, as the
 * printer driver sees it) and A4 sticker sheets. All sizes in millimetres.
 */
export interface LabelLayout {
  id: string
  kind: 'roll' | 'sheet'
  pageWidth: number
  pageHeight: number
  columns: number
  rows: number
  labelWidth: number
  labelHeight: number
  marginTop: number
  marginLeft: number
  gapX: number
  gapY: number
}

const roll = (id: string, w: number, h: number): LabelLayout => ({
  id,
  kind: 'roll',
  pageWidth: w,
  pageHeight: h,
  columns: 1,
  rows: 1,
  labelWidth: w,
  labelHeight: h,
  marginTop: 0,
  marginLeft: 0,
  gapX: 0,
  gapY: 0,
})

export const DEFAULT_LAYOUT = roll('roll-38x25', 38, 25)

export const LABEL_LAYOUTS: LabelLayout[] = [
  DEFAULT_LAYOUT,
  roll('roll-50x25', 50, 25),
  roll('roll-50x30', 50, 30),
  // A4 sheets: 3 × 8 (70 × 37), 4 × 10 (52.5 × 29.7), 5 × 13 (38.1 × 21.2).
  {
    id: 'a4-24',
    kind: 'sheet',
    pageWidth: 210,
    pageHeight: 297,
    columns: 3,
    rows: 8,
    labelWidth: 70,
    labelHeight: 37,
    marginTop: 0.5,
    marginLeft: 0,
    gapX: 0,
    gapY: 0,
  },
  {
    id: 'a4-40',
    kind: 'sheet',
    pageWidth: 210,
    pageHeight: 297,
    columns: 4,
    rows: 10,
    labelWidth: 52.5,
    labelHeight: 29.7,
    marginTop: 0,
    marginLeft: 0,
    gapX: 0,
    gapY: 0,
  },
  {
    id: 'a4-65',
    kind: 'sheet',
    pageWidth: 210,
    pageHeight: 297,
    columns: 5,
    rows: 13,
    labelWidth: 38.1,
    labelHeight: 21.2,
    marginTop: 10.7,
    marginLeft: 4.7,
    gapX: 2.5,
    gapY: 0,
  },
]

export const layoutById = (id: string | undefined) => LABEL_LAYOUTS.find((l) => l.id === id) ?? DEFAULT_LAYOUT

export const perPage = (layout: LabelLayout) => layout.columns * layout.rows

/**
 * Splits labels into printed pages. On a sheet, `skip` leaves the first
 * positions empty so a partly used sheet can go through the printer again.
 */
export function paginate<T>(items: T[], layout: LabelLayout, skip = 0): (T | null)[][] {
  const size = perPage(layout)
  const offset = layout.kind === 'sheet' ? Math.min(Math.max(0, Math.floor(skip)), size - 1) : 0
  const cells: (T | null)[] = [...Array<null>(offset).fill(null), ...items]
  const pages: (T | null)[][] = []
  for (let i = 0; i < cells.length; i += size) pages.push(cells.slice(i, i + size))
  return pages
}
