import type { LedgerAccount } from '../../api/accounting'

/** Depth of each ledger account in the chart (0 = top level), for indenting statements. */
export function accountDepths(accounts: LedgerAccount[] | undefined): Map<string, number> {
  const byId = new Map((accounts ?? []).map((a) => [a.id, a]))
  const depths = new Map<string, number>()
  const depthOf = (id: string, guard = 0): number => {
    const known = depths.get(id)
    if (known !== undefined) return known
    const parent = byId.get(id)?.parent_id
    const d = parent && byId.has(parent) && guard < 10 ? depthOf(parent, guard + 1) + 1 : 0
    depths.set(id, d)
    return d
  }
  for (const a of accounts ?? []) depthOf(a.id)
  return depths
}

/** Accounts in chart order: parents before children, by code. */
export function chartOrder(accounts: LedgerAccount[]): LedgerAccount[] {
  const children = new Map<string | null, LedgerAccount[]>()
  const ids = new Set(accounts.map((a) => a.id))
  for (const a of accounts) {
    const parent = a.parent_id && ids.has(a.parent_id) ? a.parent_id : null
    const list = children.get(parent) ?? []
    list.push(a)
    children.set(parent, list)
  }
  const out: LedgerAccount[] = []
  const walk = (parent: string | null) => {
    for (const a of (children.get(parent) ?? []).sort((x, y) => x.code.localeCompare(y.code))) {
      out.push(a)
      walk(a.id)
    }
  }
  walk(null)
  return out
}

/** The next free code under a group: last sibling + 10 (6080 → 6090), or the group's + 10. */
export function suggestCode(accounts: LedgerAccount[], parent: LedgerAccount | null): string {
  const base = parent ? Number(parent.code) : NaN
  if (!parent || !Number.isInteger(base)) return ''
  const taken = new Set(accounts.map((a) => a.code))
  const siblings = accounts.filter((a) => a.parent_id === parent.id).map((a) => Number(a.code)).filter(Number.isInteger)
  let next = Math.floor(Math.max(base, ...siblings) / 10) * 10 + 10
  while (taken.has(String(next))) next += 10
  return String(next)
}

export function accountLabel(a: { code: string; name: string }): string {
  return `${a.code} · ${a.name}`
}

export const SOURCE_TYPES = [
  'sale',
  'sale_return',
  'purchase',
  'purchase_return',
  'payment',
  'expense',
  'transfer',
  'stock_adjustment',
  'opening_balance',
  'manual',
] as const
