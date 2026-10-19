import type { Category, TaxRate, Unit } from '../../api/catalog'

/** A category name indented by its depth, for flat selects and lists. */
export function categoryLabel(c: Category): string {
  return `${'— '.repeat(c.depth)}${c.name}`
}

export function unitLabel(u: Unit): string {
  return `${u.name} (${u.short_name})`
}

/** "VAT 15% (15%)" without trailing zeros. */
export function taxLabel(r: TaxRate): string {
  return `${r.name} (${trimDecimal(r.rate_pct)}%)`
}

/** "450.5000" → "450.5", "400.0000" → "400" (for form inputs; no float conversion). */
export function trimDecimal(value: string): string {
  return value.includes('.') ? value.replace(/0+$/, '').replace(/\.$/, '') : value
}
