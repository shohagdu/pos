import { useTranslation } from 'react-i18next'

import { dec } from '../../lib/decimal'
import { formatMoney } from '../../lib/format'
import { absText } from '../money/utils'

/** Counted − expected cash with a text label (over / short / balanced), not color only. */
export function ShiftDifference({ value }: { value: string | null }) {
  const { t, i18n } = useTranslation()
  if (value === null) return <>—</>
  const d = dec(value)
  if (d === 0n) return <span className="text-brand-700">{t('shifts.balanced')}</span>
  const amount = formatMoney(absText(value), i18n.language)
  return d > 0n ? (
    <span className="text-brand-700">{t('shifts.over', { amount })}</span>
  ) : (
    <span className="font-semibold text-danger">{t('shifts.short', { amount })}</span>
  )
}
