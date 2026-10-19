import { useTranslation } from 'react-i18next'

import { formatDate } from '../../lib/dates'
import { dec, div, max, mul, toFixed } from '../../lib/decimal'
import { formatMoney } from '../../lib/format'

/** Daily sales as a bar chart of plain divs, with a visually hidden table for screen readers. */
export function SalesChart({ days }: { days: { date: string; sales: string }[] }) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const top = days.reduce((m, d) => max(m, dec(d.sales)), 0n)
  const height = (sales: string) => (top > 0n ? toFixed(div(mul(max(dec(sales), 0n), dec('100')), top), 1) : '0')
  const dayLabel = (date: string) => new Intl.DateTimeFormat(lang === 'bn' ? 'bn-BD' : 'en-GB', { day: 'numeric' }).format(new Date(`${date}T00:00:00`))

  return (
    <figure className="flex flex-col gap-2">
      <figcaption className="font-semibold">{t('dashboard.chart.title')}</figcaption>
      <div aria-hidden className="flex h-40 items-end gap-1 border-b border-border sm:gap-2">
        {days.map((d) => (
          <div key={d.date} className="flex h-full flex-1 flex-col justify-end" title={`${formatDate(d.date, lang)}: ${formatMoney(d.sales, lang)}`}>
            <div className="min-h-px rounded-t bg-brand-500" style={{ height: `${height(d.sales)}%` }} />
          </div>
        ))}
      </div>
      <div aria-hidden className="flex gap-1 text-center text-[10px] text-ink-muted sm:gap-2 sm:text-xs">
        {days.map((d) => (
          <span key={d.date} className="flex-1">
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
