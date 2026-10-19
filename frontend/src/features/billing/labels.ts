import type { TFunction } from 'i18next'

/** Plan feature and limit names in the UI language, falling back to the API's English name. */
export function featureName(t: TFunction, f: { code: string; name: string }): string {
  return t(`planFeatures.${f.code.replace('.', '_')}`, { defaultValue: f.name })
}
