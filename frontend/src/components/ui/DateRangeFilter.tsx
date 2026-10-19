import { useTranslation } from 'react-i18next'

import { type RangePreset, presetRange } from '../../lib/dates'
import { Select } from './Select'
import { TextField } from './TextField'

const PRESETS: RangePreset[] = ['today', 'yesterday', 'last7', 'thisMonth', 'lastMonth', 'thisYear']

/** From/to business dates with quick presets (REP-R01 header filters). */
export function DateRangeFilter({
  from,
  to,
  onChange,
}: {
  from: string
  to: string
  onChange: (range: { from: string; to: string }) => void
}) {
  const { t } = useTranslation()
  return (
    <div className="flex flex-wrap items-end gap-3">
      <Select
        label={t('common.period')}
        value=""
        onChange={(e) => e.target.value && onChange(presetRange(e.target.value as RangePreset))}
        options={[{ value: '', label: t('common.choosePeriod') }, ...PRESETS.map((p) => ({ value: p, label: t(`common.presets.${p}`) }))]}
      />
      <TextField label={t('common.from')} type="date" value={from} max={to || undefined} onChange={(e) => onChange({ from: e.target.value, to })} />
      <TextField label={t('common.to')} type="date" value={to} min={from || undefined} onChange={(e) => onChange({ from, to: e.target.value })} />
    </div>
  )
}
