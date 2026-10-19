import { useId } from 'react'
import { useTranslation } from 'react-i18next'

import { useBranch } from './useBranch'

/** Branch picker in the shell; hidden while only one branch is usable. */
export function BranchSwitcher() {
  const { t } = useTranslation()
  const id = useId()
  const { branchId, choices, canSeeAll, select } = useBranch()
  if (choices.length < 2) return null
  return (
    <div className="flex flex-col gap-1 px-4 pb-3">
      <label htmlFor={id} className="text-xs font-medium text-ink-muted">
        {t('branches.working')}
      </label>
      <select
        id={id}
        value={branchId ?? ''}
        onChange={(e) => select(e.target.value || null)}
        className="min-h-10 rounded-lg border border-border bg-surface px-2 text-sm outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100"
      >
        {canSeeAll && <option value="">{t('branches.all')}</option>}
        {choices.map((b) => (
          <option key={b.id} value={b.id}>
            {b.is_default ? t('branches.mainNamed', { name: b.name }) : b.name}
          </option>
        ))}
      </select>
    </div>
  )
}
