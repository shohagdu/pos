import type { ReactNode } from 'react'

export interface Column<T> {
  key: string
  header: ReactNode
  /** Right-align numbers. */
  align?: 'left' | 'right' | 'center'
  render: (row: T, index: number) => ReactNode
  className?: string
}

/**
 * A plain, accessible table in the app's card style. `footer` cells line up
 * with the columns (totals rows).
 */
export function DataTable<T>({
  columns,
  rows,
  rowKey,
  empty,
  footer,
  onRowClick,
  caption,
}: {
  columns: Column<T>[]
  rows: T[]
  rowKey: (row: T, index: number) => string
  empty?: ReactNode
  footer?: ReactNode[]
  onRowClick?: (row: T) => void
  caption?: string
}) {
  const align = (a?: Column<T>['align']) => (a === 'right' ? 'text-right' : a === 'center' ? 'text-center' : 'text-left')
  if (rows.length === 0 && empty) return <p className="text-sm text-ink-muted">{empty}</p>
  return (
    <div className="overflow-x-auto rounded-xl border border-border bg-surface">
      <table className="w-full text-sm">
        {caption && <caption className="sr-only">{caption}</caption>}
        <thead className="border-b border-border bg-canvas text-xs text-ink-muted uppercase">
          <tr>
            {columns.map((c) => (
              <th key={c.key} scope="col" className={`px-3 py-2.5 font-semibold whitespace-nowrap ${align(c.align)}`}>
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
                <td key={c.key} className={`px-3 py-2.5 ${align(c.align)} ${c.align === 'right' ? 'tabular-nums whitespace-nowrap' : ''} ${c.className ?? ''}`}>
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
                <td key={c.key} className={`px-3 py-2.5 ${align(c.align)} ${c.align === 'right' ? 'tabular-nums whitespace-nowrap' : ''}`}>
                  {footer[i] ?? null}
                </td>
              ))}
            </tr>
          </tfoot>
        )}
      </table>
    </div>
  )
}
