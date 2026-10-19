import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { useCurrentShift } from '../../api/registers'

/** POS header chip: the cashier's open shift, or a link to open one. */
export function ShiftIndicator({ tenantId }: { tenantId: string }) {
  const { t } = useTranslation()
  const current = useCurrentShift(tenantId)
  if (!current.isSuccess) return null
  const shift = current.data
  return shift ? (
    <Link to={`/shifts/${shift.id}`} className="rounded-lg bg-brand-50 px-2 py-1 text-sm font-medium text-brand-700 hover:underline">
      {t('shifts.indicator', { register: shift.register_name, no: shift.shift_no })}
    </Link>
  ) : (
    <Link to="/shifts" className="rounded-lg bg-warning/15 px-2 py-1 text-sm font-medium text-ink hover:underline">
      {t('shifts.noneOpen')}
    </Link>
  )
}
