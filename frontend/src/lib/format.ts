// Money is exchanged with the API as decimal strings (Requirement Analysis
// API-08) and formatted without converting to floating point.

const LOCALE: Record<string, string> = { en: 'en-IN', bn: 'bn-BD' }

/** Formats a decimal string as BDT with South Asian digit grouping (৳12,34,567.50). */
export function formatMoney(amount: string, lang = 'en', currency = 'BDT'): string {
  const formatter = new Intl.NumberFormat(LOCALE[lang] ?? 'en-IN', {
    style: 'currency',
    currency,
    currencyDisplay: 'narrowSymbol',
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  })
  // Intl accepts numeric strings and keeps full decimal precision.
  return formatter.format(amount as unknown as number)
}

/** Formats a quantity; fractional digits only for units that allow fractions. */
export function formatQuantity(qty: string, lang = 'en', allowFraction = false): string {
  return new Intl.NumberFormat(LOCALE[lang] ?? 'en-IN', {
    maximumFractionDigits: allowFraction ? 3 : 0,
  }).format(qty as unknown as number)
}
