import { useTranslation } from 'react-i18next'

import { Badge } from '../../components/ui/Badge'
import { formatMoney } from '../../lib/format'
import { absText, isNegative, isPositive } from '../money/utils'

/** A customer balance: positive = owes us, negative = advance held. */
export function Balance({ value }: { value: string }) {
  const { t, i18n } = useTranslation()
  const owes = isPositive(value)
  const advance = isNegative(value)
  return (
    <span className="inline-flex items-center justify-end gap-2">
      <span className={owes ? 'font-semibold text-danger' : advance ? 'font-semibold text-brand-700' : ''}>{formatMoney(absText(value), i18n.language)}</span>
      {owes && <Badge tone="bad">{t('customers.owes')}</Badge>}
      {advance && <Badge tone="good">{t('customers.advance')}</Badge>}
    </span>
  )
}
