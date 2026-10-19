import { useTranslation } from 'react-i18next'

import type { Branch } from '../../api/tenancy'
import type { Role } from '../../api/users'
import { Checkbox } from '../../components/ui/Checkbox'
import { roleName } from './labels'

/** Role checkboxes. The owner role is never offered; the admin role only to owners. */
export function RolePicker({
  roles,
  selected,
  onChange,
  isOwner,
  error,
}: {
  roles: Role[]
  selected: string[]
  onChange: (ids: string[]) => void
  isOwner: boolean
  error?: string
}) {
  const { t } = useTranslation()
  const offered = roles.filter((r) => r.code !== 'owner' && (isOwner || r.code !== 'admin'))
  const toggle = (id: string, on: boolean) => onChange(on ? [...selected, id] : selected.filter((s) => s !== id))
  return (
    <fieldset className="flex flex-col gap-3">
      <legend className="mb-2 text-sm font-medium">{t('users.roles')}</legend>
      {offered.map((r) => (
        <Checkbox
          key={r.id}
          label={roleName(t, r)}
          hint={r.description ? t(`systemRoleHints.${r.code}`, { defaultValue: r.description }) : undefined}
          checked={selected.includes(r.id)}
          onChange={(e) => toggle(r.id, e.target.checked)}
        />
      ))}
      {error && <p className="text-sm text-danger">{error}</p>}
    </fieldset>
  )
}

/** Branch scope (R-TEN-04). Hidden while the business has a single branch. */
export function BranchScopePicker({
  branches,
  selected,
  onChange,
}: {
  branches: Branch[]
  selected: string[]
  onChange: (ids: string[]) => void
}) {
  const { t } = useTranslation()
  if (branches.length < 2) return null
  const toggle = (id: string, on: boolean) => onChange(on ? [...selected, id] : selected.filter((s) => s !== id))
  return (
    <fieldset className="flex flex-col gap-3">
      <legend className="mb-2 text-sm font-medium">{t('users.branchScope')}</legend>
      <Checkbox label={t('users.allBranches')} checked={selected.length === 0} onChange={() => onChange([])} />
      {branches.map((b) => (
        <Checkbox key={b.id} label={b.name} checked={selected.includes(b.id)} onChange={(e) => toggle(b.id, e.target.checked)} />
      ))}
    </fieldset>
  )
}
