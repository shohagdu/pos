import { Fragment, type ReactNode, isValidElement, useLayoutEffect, useRef, useState } from 'react'

import { useMediaQuery } from '../../lib/useMediaQuery'

export interface Column<T> {
  key: string
  header: ReactNode
  /** Right-align numbers. */
  align?: 'left' | 'right' | 'center'
  render: (row: T, index: number) => ReactNode
  className?: string
  /** Lower-priority column: hidden in the table on screens narrower than this (cards still show it). */
  showFrom?: 'xl' | '2xl'
  /** Its place on the automatic phone/tablet card: the title, the amount on the right, or left off. */
  card?: 'title' | 'amount' | 'hide'
}

// Hidden on smaller screens only; a printout always has every column.
const SHOW_FROM = { xl: 'max-xl:hidden print:table-cell', '2xl': 'max-2xl:hidden print:table-cell' } as const
// Columns that make the best headline amount on a card, in order of preference.
const AMOUNT_KEYS = ['total', 'grandTotal', 'grand_total', 'grand', 'net', 'amount', 'balance', 'value', 'closing']
const TABLE_FROM = {
  lg: '(min-width: 64rem)',
  xl: '(min-width: 80rem)',
} as const

/** A column without a visible header (row actions, icons). */
function headless<T>(c: Column<T>): boolean {
  if (c.header === null || c.header === undefined || c.header === false) return true
  if (typeof c.header === 'string') return c.header.trim() === ''
  // <span className="sr-only">…</span> headers label action columns.
  return isValidElement<{ className?: string }>(c.header) && /\bsr-only\b/.test(c.header.props.className ?? '')
}

/** A row-number column ("#", "SL"). */
function numbering<T>(c: Column<T>): boolean {
  return ['no', 'sl', 'line', 'n', '#'].includes(c.key.toLowerCase()) || (typeof c.header === 'string' && ['#', 'SL'].includes(c.header.trim()))
}

/**
 * A plain, accessible table in the app's card style. `footer` cells line up
 * with the columns (totals rows).
 *
 * Phones and tablets (below lg, or below xl with `tableFrom="xl"`) get a
 * list of cards instead: `card` draws a row by hand; otherwise each card is
 * built from the columns (first column as the title, the last right-aligned
 * column as the amount, the rest as label/value pairs, header-less columns
 * as the card's actions). `cards={false}` keeps the table, scrolling
 * sideways inside its frame. Only one of the two is rendered.
 */
export function DataTable<T>({
  columns,
  rows,
  rowKey,
  empty,
  footer,
  onRowClick,
  caption,
  card,
  cards = true,
  tableFrom = 'lg',
}: {
  columns: Column<T>[]
  rows: T[]
  rowKey: (row: T, index: number) => string
  empty?: ReactNode
  footer?: ReactNode[]
  onRowClick?: (row: T) => void
  caption?: string
  /** One row as a card for phones and tablets. */
  card?: (row: T, index: number) => ReactNode
  /** `false` keeps the table at every size. */
  cards?: boolean
  /** The first screen size that shows the table (very wide tables: xl). */
  tableFrom?: 'lg' | 'xl'
}) {
  // Without matchMedia (tests) the table is shown.
  const wide = useMediaQuery(TABLE_FROM[tableFrom], true)
  // A wide screen can still hold a narrow panel (dashboard halves, side
  // columns): when the table is wider than its frame, cards take over until
  // the frame is wide enough again.
  const outer = useRef<HTMLDivElement>(null)
  const frame = useRef<HTMLDivElement>(null)
  const [need, setNeed] = useState(0)
  const [room, setRoom] = useState(Number.POSITIVE_INFINITY)
  const tooNarrow = need > 0 && room < need
  const showCards = cards && rows.length > 0 && (!wide || tooNarrow)
  useLayoutEffect(() => {
    const el = frame.current
    const table = el?.querySelector('table')
    if (!el || !table || !cards) return
    // Border and a little slack, so the two modes cannot flip back and forth at the edge.
    if (table.scrollWidth > el.clientWidth + 1) setNeed(table.scrollWidth + (el.offsetWidth - el.clientWidth) + 4)
  }, [rows, columns, cards, showCards])
  useLayoutEffect(() => {
    const el = outer.current
    if (!el || typeof ResizeObserver === 'undefined') return
    const ro = new ResizeObserver(([entry]) => entry && setRoom(entry.contentRect.width))
    ro.observe(el)
    return () => ro.disconnect()
  }, [showCards])
  const align = (a?: Column<T>['align']) => (a === 'right' ? 'text-right' : a === 'center' ? 'text-center' : 'text-left')
  const shown = (c: Column<T>) => (c.showFrom ? SHOW_FROM[c.showFrom] : '')
  if (rows.length === 0 && empty) return <p className="text-sm text-ink-muted">{empty}</p>

  const table = (
    <>
      {/* relative: sr-only text in cells is positioned inside the frame, not the page. */}
      <div ref={frame} className="relative overflow-x-auto rounded-xl border border-border bg-surface">
        <table className="w-full text-sm">
          {caption && <caption className="sr-only">{caption}</caption>}
          <thead className="border-b border-border bg-canvas text-xs text-ink-muted uppercase">
            <tr>
              {columns.map((c) => (
                <th key={c.key} scope="col" className={`px-3 py-2.5 font-semibold whitespace-nowrap ${align(c.align)} ${shown(c)}`}>
                  {c.header}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {rows.map((row, i) => (
              <tr
                key={rowKey(row, i)}
                onClick={onRowClick ? () => onRowClick(row) : undefined}
                className={`border-b border-border last:border-b-0 ${onRowClick ? 'cursor-pointer hover:bg-canvas' : ''}`}
              >
                {columns.map((c) => (
                  <td
                    key={c.key}
                    className={`px-3 py-2.5 ${align(c.align)} ${c.align === 'right' ? 'tabular-nums whitespace-nowrap' : ''} ${c.className ?? ''} ${shown(c)}`}
                  >
                    {c.render(row, i)}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
          {footer && (
            <tfoot className="border-t-2 border-border bg-canvas font-semibold">
              <tr>
                {columns.map((c, i) => (
                  <td key={c.key} className={`px-3 py-2.5 ${align(c.align)} ${c.align === 'right' ? 'tabular-nums whitespace-nowrap' : ''} ${shown(c)}`}>
                    {footer[i] ?? null}
                  </td>
                ))}
              </tr>
            </tfoot>
          )}
        </table>
      </div>
    </>
  )

  if (showCards) {
    const numberCol = columns[0] && numbering(columns[0]) ? columns[0] : null
    const rest = (numberCol ? columns.slice(1) : columns).filter((c) => c.card !== 'hide')
    const title = rest.find((c) => c.card === 'title') ?? rest[0]
    const actions = rest.filter((c) => c !== title && headless(c))
    const numeric = rest.filter((c) => c !== title && c.align === 'right' && !headless(c))
    const amount = rest.find((c) => c.card === 'amount') ?? AMOUNT_KEYS.map((k) => numeric.find((c) => c.key === k)).find(Boolean) ?? numeric.at(-1)
    const pairs = rest.filter((c) => c !== title && c !== amount && !actions.includes(c))
    const auto = (row: T, i: number) => (
      <div className="flex flex-col gap-2">
        <div className="flex items-start justify-between gap-3">
          <div className="flex min-w-0 items-start gap-2">
            {numberCol && (
              <span className="mt-0.5 flex h-6 min-w-6 shrink-0 items-center justify-center rounded-full bg-canvas px-1.5 text-xs font-semibold text-ink-muted tabular-nums">
                {numberCol.render(row, i)}
              </span>
            )}
            {title && <div className="min-w-0 font-medium break-words">{title.render(row, i)}</div>}
          </div>
          {amount && (
            <div className="shrink-0 text-right font-semibold whitespace-nowrap tabular-nums">
              <span className="sr-only">{amount.header} </span>
              {amount.render(row, i)}
            </div>
          )}
        </div>
        {pairs.length > 0 && (
          <dl className="grid grid-cols-1 gap-x-6 gap-y-1 text-[0.8125rem] sm:grid-cols-2">
            {pairs.map((c) => (
              <div key={c.key} className="flex min-w-0 items-baseline justify-between gap-3">
                <dt className="shrink-0 text-ink-muted">{c.header}</dt>
                <dd className={`min-w-0 text-right break-words ${c.align === 'right' ? 'tabular-nums' : ''}`}>{c.render(row, i)}</dd>
              </div>
            ))}
          </dl>
        )}
        {actions.length > 0 && (
          // Actions stay above a clickable card's own handler.
          <div className="relative z-10 flex flex-wrap justify-end gap-2 border-t border-border pt-2" onClick={(e) => e.stopPropagation()}>
            {actions.map((c) => (
              <Fragment key={c.key}>{c.render(row, i)}</Fragment>
            ))}
          </div>
        )}
      </div>
    )
    const draw = card ?? auto
    const totals = footer ? columns.map((c, i) => ({ c, v: footer[i] })).filter(({ v }) => v !== null && v !== undefined && v !== '') : []
    return (
      <div ref={outer}>
        <ul aria-label={caption} className="flex flex-col gap-2 print:hidden">
          {rows.map((row, i) => (
            <li
              key={rowKey(row, i)}
              onClick={onRowClick ? () => onRowClick(row) : undefined}
              className={`rounded-xl border border-border bg-surface p-3 text-sm ${onRowClick ? 'cursor-pointer active:bg-canvas' : ''}`}
            >
              {draw(row, i)}
            </li>
          ))}
          {totals.length > 0 && (
            <li className="rounded-xl border-2 border-border bg-canvas p-3 text-sm font-semibold">
              <dl className="flex flex-col gap-1">
                {totals.map(({ c, v }, i) =>
                  i === 0 && c === columns[0] ? (
                    <dt key={c.key}>{v}</dt>
                  ) : (
                    <div key={c.key} className="flex justify-between gap-3">
                      <dt className="font-normal text-ink-muted">{c.header}</dt>
                      <dd className="text-right tabular-nums">{v}</dd>
                    </div>
                  ),
                )}
              </dl>
            </li>
          )}
        </ul>
        {/* A printout (A4 is narrower than lg) keeps the table. */}
        <div className="hidden print:block">{table}</div>
      </div>
    )
  }

  return <div ref={outer}>{table}</div>
}
