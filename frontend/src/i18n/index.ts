import i18n from 'i18next'
import { initReactI18next } from 'react-i18next'

import bn from './bn.json'
import en from './en.json'
import it from './it.json'

export const LANGUAGES = [
  { code: 'en', label: 'English' },
  { code: 'bn', label: 'বাংলা' },
  { code: 'it', label: 'Italiano' },
] as const

export type LanguageCode = (typeof LANGUAGES)[number]['code']

const STORAGE_KEY = 'pos.lang'

function initialLanguage(): LanguageCode {
  try {
    const saved = localStorage.getItem(STORAGE_KEY)
    if (LANGUAGES.some((l) => l.code === saved)) return saved as LanguageCode
  } catch {
    // storage unavailable (private mode): fall back to default
  }
  return 'en'
}

type Messages = { [key: string]: string | Messages }

// Each feature keeps its strings in `features/<name>/i18n/{en,bn}.json`
// under its own top-level key; they are merged into the base files.
const featureMessages = import.meta.glob<{ default: Messages }>('/src/features/*/i18n/*.json', { eager: true })

function merge(target: Messages, source: Messages): Messages {
  for (const [key, value] of Object.entries(source)) {
    const current = target[key]
    target[key] =
      typeof value === 'object' && typeof current === 'object' ? merge({ ...current }, value) : value
  }
  return target
}

function messages(base: Messages, lang: string): Messages {
  const out = merge({}, base)
  for (const [path, module] of Object.entries(featureMessages)) {
    if (path.endsWith(`/${lang}.json`)) merge(out, module.default)
  }
  return out
}

void i18n.use(initReactI18next).init({
  resources: {
    en: { translation: messages(en, 'en') },
    bn: { translation: messages(bn, 'bn') },
    it: { translation: messages(it, 'it') },
  },
  lng: initialLanguage(),
  fallbackLng: 'en',
  interpolation: { escapeValue: false },
})

i18n.on('languageChanged', (lng) => {
  document.documentElement.lang = lng
  try {
    localStorage.setItem(STORAGE_KEY, lng)
  } catch {
    // ignore
  }
})
document.documentElement.lang = i18n.language

export default i18n
