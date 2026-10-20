import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useParams } from 'react-router'

import { type BookLine, useBook } from '../../api/money'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { DateRangeFilter } from '../../components/ui/DateRangeFilter'
import { PageHeader } from '../../components/ui/PageHeader'
import { downloadCsv } from '../../lib/csv'
import { dec } from '../../lib/decimal'
import { formatDate, startOfMonth, today } from '../../lib/dates'
import { formatMoney } from '../../lib/format'
import { LinkButton, QueryStatus, TextLink } from './shared'
import { referencePath, stamp, useTenant } from './utils'

/** Cash / bank book of one payment account (`/accounts/:id`). */
export function AccountBookPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { id = '' } = useParams()
  const { tenantId } = useTenant()
  const [range, setRange] = useState({ from: startOfMonth(), to: today() })
  const book = useBook(tenantId, id, range.from || undefined, range.to || undefined)
  const b = book.data
  const money = (v: string) => (dec(v) === 0n ? '' : formatMoney(v, lang))
  const typeLabel = (type: string) => t(`money.txnTypes.${type}`, { defaultValue: type })

  function exportCsv() {
    if (!b) return
    downloadCsv(`book-${b.account.name}-${stamp()}`, b.lines, [
      { header: t('common.date'), value: (l: BookLine) => l.business_date },
      { header: t('money.txnType'), value: (l) => typeLabel(l.txn_type) },
      { header: t('money.referenceNo'), value: (l) => l.reference_no },
      { header: t('common.note'), value: (l) => l.note },
      { header: t('money.moneyIn'), value: (l) => l.money_in },
      { header: t('money.moneyOut'), value: (l) => l.money_out },
      { header: t('money.balance'), value: (l) => l.balance_after },
    ])
  }

  const cell = 'px-3 py-2.5'
  const num = `${cell} text-right tabular-nums whitespace-nowrap`
  return (
    <div className="flex flex-col gap-6">
      <Link to="/accounts" className="text-sm font-medium text-brand-700 hover:underline print:hidden">
        ← {t('money.title')}
      </Link>
      <PageHeader
        title={b ? b.account.name : t('money.book')}
        subtitle={
          b && (
            <span className="inline-flex flex-wrap items-center gap-2">
              <Badge tone="info">{t(`money.types.${b.account.account_type}`)}</Badge>
              {[b.account.account_number, b.account.bank_name].filter(Boolean).join(' · ')}
              <span>
                {t('money.book')} · {formatDate(range.from, lang)} – {formatDate(range.to, lang)}
              </span>
            </span>
          )
        }
        actions={
          <div className="flex gap-2 print:hidden">
            {b && b.account.account_type !== 'cash' && (
              <LinkButton to={`/accounts/reconciliations?account_id=${b.account.id}`}>{t('money.reconcile.reconcile')}</LinkButton>
            )}
            <Button variant="secondary" disabled={!b} onClick={exportCsv}>
              {t('common.exportCsv')}
            </Button>
            <Button variant="secondary" disabled={!b} onClick={() => window.print()}>
              {t('common.print')}
            </Button>
          </div>
        }
      />
      <div className="print:hidden">
        <DateRangeFilter from={range.from} to={range.to} onChange={setRange} />
      </div>
      {!b ? (
        <QueryStatus query={book} />
      ) : (
        <div className="overflow-x-auto rounded-xl border border-border bg-surface">
          <table className="w-full text-sm">
            <caption className="sr-only">{t('money.book')}</caption>
            <thead className="border-b border-border bg-canvas text-xs text-ink-muted uppercase">
              <tr>
                <th scope="col" className={`${cell} text-left`}>{t('common.date')}</th>
                <th scope="col" className={`${cell} text-left`}>{t('money.txnType')}</th>
                <th scope="col" className={`${cell} text-left`}>{t('money.referenceNo')}</th>
                <th scope="col" className={`${cell} text-left`}>{t('common.note')}</th>
                <th scope="col" className={`${cell} text-right`}>{t('money.moneyIn')}</th>
                <th scope="col" className={`${cell} text-right`}>{t('money.moneyOut')}</th>
                <th scope="col" className={`${cell} text-right`}>{t('money.balance')}</th>
              </tr>
            </thead>
            <tbody>
              <tr className="border-b border-border font-medium">
                <td className={cell} colSpan={6}>
                  {t('money.openingBalance')}
                </td>
                <td className={num}>{formatMoney(b.opening_balance, lang)}</td>
              </tr>
              {b.lines.length === 0 && (
                <tr className="border-b border-border">
                  <td className={`${cell} text-ink-muted`} colSpan={7}>
                    {t('money.noBookLines')}
                  </td>
                </tr>
              )}
              {b.lines.map((l) => {
                const path = referencePath(l.reference_type, l.reference_id)
                return (
                  <tr key={l.id} className="border-b border-border">
                    <td className={`${cell} whitespace-nowrap`}>{formatDate(l.business_date, lang)}</td>
                    <td className={cell}>{typeLabel(l.txn_type)}</td>
                    <td className={cell}>{path && l.reference_no ? <TextLink to={path}>{l.reference_no}</TextLink> : (l.reference_no ?? '—')}</td>
                    <td className={cell}>{l.note ?? ''}</td>
                    <td className={`${num} text-brand-700`}>{money(l.money_in)}</td>
                    <td className={`${num} text-danger`}>{money(l.money_out)}</td>
                    <td className={num}>{formatMoney(l.balance_after, lang)}</td>
                  </tr>
                )
              })}
            </tbody>
            <tfoot className="border-t-2 border-border bg-canvas font-semibold">
              <tr>
                <td className={cell} colSpan={4}>
                  {t('money.closingBalance')}
                </td>
                <td className={num}>{formatMoney(b.total_in, lang)}</td>
                <td className={num}>{formatMoney(b.total_out, lang)}</td>
                <td className={num}>{formatMoney(b.closing_balance, lang)}</td>
              </tr>
            </tfoot>
          </table>
        </div>
      )}
    </div>
  )
}
