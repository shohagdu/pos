import type { TFunction } from 'i18next'

import { COUNTRIES, type CountryCode } from '../../api/tenancy'

/** Country choices for the business profile. */
export function countryOptions(t: TFunction) {
  return COUNTRIES.map((c) => ({ value: c, label: t(`countries.${c}`) }))
}

/** A new business in the Italian UI is most likely in Italy. */
export function defaultCountry(lang: string): CountryCode {
  return lang === 'it' ? 'IT' : 'BD'
}

/**
 * Labels for the address and registration fields, which differ by country:
 * Italy has provinces, the Partita IVA, codice fiscale and R.E.A. instead of
 * districts, the BIN and a trade licence.
 */
export function profileLabels(t: TFunction, country: string) {
  const italy = country === 'IT'
  return {
    italy,
    district: italy ? t('onboarding.business.province') : t('onboarding.business.district'),
    bin: italy ? t('onboarding.business.partitaIva') : t('onboarding.business.bin'),
  }
}
