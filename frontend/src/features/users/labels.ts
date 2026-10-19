import type { TFunction } from 'i18next'

import type { Permission } from '../../api/users'

/** System roles are named in the UI language; custom roles keep their own name. */
export function roleName(t: TFunction, role: { code: string; name: string }): string {
  return t(`systemRoles.${role.code}`, { defaultValue: role.name })
}

export function permissionLabel(t: TFunction, p: Permission): string {
  return t(`permissions.${p.code.replace('.', '_')}`, { defaultValue: p.description })
}

export function moduleLabel(t: TFunction, module: string): string {
  return t(`permissionModules.${module}`, { defaultValue: module })
}

/** Groups the catalogue by module, keeping the API's order. */
export function groupByModule(permissions: Permission[]): [string, Permission[]][] {
  const groups = new Map<string, Permission[]>()
  for (const p of permissions) groups.set(p.module, [...(groups.get(p.module) ?? []), p])
  return [...groups.entries()]
}
