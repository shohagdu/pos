import type { TFunction } from 'i18next'

import { type StaffRole, useStaff } from '../../api/platform'

export const LINK = 'font-medium text-brand-700 hover:underline'

/** The signed-in staff member's role (the layout guarantees it is loaded). */
export function useStaffRole(): StaffRole | undefined {
  return useStaff().data?.role
}

/** Label of a subscription state or a count key (`none`, `business_suspended`). */
export function stateLabel(t: TFunction, state: string): string {
  if (state === 'none' || state === 'business_suspended') return t(`admin.states.${state}`)
  return t(`plan.status.${state}`, { defaultValue: state })
}

export const planLabel = (t: TFunction, code: string | null, name?: string | null) =>
  code ? t(`plan.names.${code}`, { defaultValue: name ?? code }) : '—'

export const cycleLabel = (t: TFunction, cycle: string | null) => (cycle ? t(`admin.cycles.${cycle}`, { defaultValue: cycle }) : '—')

export const methodLabel = (t: TFunction, method: string) => t(`admin.methods.${method}`, { defaultValue: method })

/** Feature names in the UI language, falling back to the API's English name. */
export const featureName = (t: TFunction, f: { code: string; name?: string }) =>
  t(`planFeatures.${f.code.replace('.', '_')}`, { defaultValue: f.name ?? f.code })
