// Money is exchanged with the API as decimal strings (Requirement Analysis
// API-08) and formatted without converting to floating point.

const LOCALE: Record<string, string> = { en: 'en-IN', bn: 'bn-BD' }

/** Currencies grouped in lakhs and crores (12,34,567); others in thousands. */
const SOUTH_ASIAN = new Set(['BDT', 'INR', 'PKR', 'NPR', 'LKR'])

// The business's currency, set when the session loads (`/auth/me`).
let businessCurrency = 'BDT'

export function setBusinessCurrency(code: string | null | undefined) {
  businessCurrency = code || 'BDT'
}

export function getBusinessCurrency(): string {
  return businessCurrency
}

function numberLocale(lang: string, currency: string): string {
  if (lang === 'bn') return 'bn-BD'
  return SOUTH_ASIAN.has(currency) ? 'en-IN' : 'en-US'
}

/** The narrow symbol of a currency: ৳, €, $ … */
export function currencySymbol(currency = businessCurrency, lang = 'en'): string {
  const part = new Intl.NumberFormat(numberLocale(lang, currency), { style: 'currency', currency, currencyDisplay: 'narrowSymbol' })
    .formatToParts(0)
    .find((p) => p.type === 'currency')
  return part?.value ?? currency
}

/**
 * Formats a decimal string in the business currency: ৳12,34,567.50 for
 * Taka (South Asian grouping), €1,234,567.50 for Euro.
 */
export function formatMoney(amount: string, lang = 'en', currency = businessCurrency): string {
  const formatter = new Intl.NumberFormat(numberLocale(lang, currency), {
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
