import { useTranslation } from 'react-i18next'

import { formatDate } from '../../lib/dates'
import { dec, div, max, mul, sum, toFixed, toPlain } from '../../lib/decimal'
import { formatMoney } from '../../lib/format'

/** Daily sales as a bar chart of plain divs, with a visually hidden table for screen readers. */
export function SalesChart({ days, today }: { days: { date: string; sales: string }[]; today?: string }) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const top = days.reduce((m, d) => max(m, dec(d.sales)), 0n)
  const total = sum(days.map((d) => dec(d.sales)))
  const height = (sales: string) => (top > 0n ? toFixed(div(mul(max(dec(sales), 0n), dec('100')), top), 1) : '0')
  const dayLabel = (date: string) => new Intl.DateTimeFormat(lang === 'bn' ? 'bn-BD' : 'en-GB', { day: 'numeric' }).format(new Date(`${date}T00:00:00`))
  const isToday = (date: string) => date === (today ?? days[days.length - 1]?.date)

  return (
    <figure className="flex flex-col gap-2">
      <figcaption className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
        <span className="font-semibold">{t('dashboard.chart.title')}</span>
        <span className="text-sm text-ink-muted">
          {t('dashboard.chart.total')} <span className="font-semibold text-ink tabular-nums">{formatMoney(toPlain(total), lang)}</span>
        </span>
      </figcaption>
      <div className="relative">
        <div aria-hidden className="flex h-40 items-end gap-1 border-b border-border sm:gap-2">
          {days.map((d) => (
            <div key={d.date} className="flex h-full flex-1 flex-col justify-end" title={`${formatDate(d.date, lang)}: ${formatMoney(d.sales, lang)}`}>
              <div className={`min-h-px rounded-t ${isToday(d.date) ? 'bg-brand-600' : 'bg-brand-500/50'}`} style={{ height: `${height(d.sales)}%` }} />
            </div>
          ))}
        </div>
        {top === 0n && <p className="absolute inset-0 flex items-center justify-center px-4 text-center text-sm text-ink-muted">{t('dashboard.chart.empty')}</p>}
      </div>
      <div aria-hidden className="flex gap-1 text-center text-[10px] text-ink-muted sm:gap-2 sm:text-xs">
        {days.map((d) => (
          <span key={d.date} className={`flex-1 ${isToday(d.date) ? 'font-bold text-ink' : ''}`}>
            {dayLabel(d.date)}
          </span>
        ))}
      </div>
      <table className="sr-only">
        <caption>{t('dashboard.chart.title')}</caption>
        <thead>
          <tr>
            <th scope="col">{t('common.date')}</th>
            <th scope="col">{t('dashboard.sales')}</th>
          </tr>
        </thead>
        <tbody>
          {days.map((d) => (
            <tr key={d.date}>
              <th scope="row">{formatDate(d.date, lang)}</th>
              <td>{formatMoney(d.sales, lang)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </figure>
  )
}
