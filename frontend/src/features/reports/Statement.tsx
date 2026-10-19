import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { money } from './format'
import type { StatementRow } from './statementRows'

/** An accounting statement: indented account lines, bold totals. */
export function StatementTable({
  rows,
  caption,
  depths,
  ledgerQuery = '',
}: {
  rows: StatementRow[]
  caption: string
  depths: Map<string, number>
  /** Extra query (`&from=…&to=…`) for the general ledger links. */
  ledgerQuery?: string
}) {
  const { t, i18n } = useTranslation()
  return (
    <div className="overflow-x-auto rounded-xl border border-border bg-surface">
      <table className="w-full text-sm">
        <caption className="sr-only">{caption}</caption>
        <thead className="border-b border-border bg-canvas text-xs text-ink-muted uppercase">
          <tr>
            <th scope="col" className="px-3 py-2.5 text-left font-semibold">
              {t('reports.account')}
            </th>
            <th scope="col" className="px-3 py-2.5 text-right font-semibold">
              {t('common.amount')}
            </th>
          </tr>
        </thead>
        <tbody>
          {rows.map((r) => {
            if (r.kind === 'heading') {
              return (
                <tr key={r.key} className="border-b border-border">
                  <th scope="colgroup" colSpan={2} className="px-3 pt-4 pb-1.5 text-left font-semibold">
                    {r.label}
                  </th>
                </tr>
              )
            }
            if (r.kind === 'line') {
              const depth = (r.accountId ? (depths.get(r.accountId) ?? 0) : 0) + 1
              return (
                <tr key={r.key} className="border-b border-border">
                  <td className="px-3 py-2" style={{ paddingLeft: `${0.75 + depth}rem` }}>
                    {r.accountId ? (
                      <Link to={`/reports/general-ledger?account_id=${r.accountId}${ledgerQuery}`} className="hover:underline">
                        <span className="mr-2 font-mono text-xs text-ink-muted">{r.code}</span>
                        {r.label}
                      </Link>
                    ) : (
                      r.label
                    )}
                  </td>
                  <td className="px-3 py-2 text-right whitespace-nowrap tabular-nums">{money(r.amount, i18n.language)}</td>
                </tr>
              )
            }
            return (
              <tr key={r.key} className={`border-b border-border font-semibold ${r.kind === 'grand' ? 'border-t-2 bg-canvas text-base' : ''}`}>
                <th scope="row" className="px-3 py-2.5 text-left">
                  {r.label}
                </th>
                <td className={`px-3 py-2.5 text-right whitespace-nowrap tabular-nums ${r.kind === 'grand' ? 'underline decoration-double' : ''}`}>
                  {money(r.amount, i18n.language)}
                </td>
              </tr>
            )
          })}
        </tbody>
      </table>
    </div>
  )
}
