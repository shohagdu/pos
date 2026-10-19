import { useTranslation } from 'react-i18next'

import { LANGUAGES } from '../i18n'

export function LanguageSwitcher() {
  const { t, i18n } = useTranslation()
  return (
    <label className="inline-flex items-center gap-2 text-sm text-ink-muted">
      <span className="sr-only">{t('common.language')}</span>
      <select
        value={i18n.language}
        onChange={(e) => void i18n.changeLanguage(e.target.value)}
        className="min-h-9 rounded-lg border border-border bg-surface px-2 text-sm text-ink"
      >
        {LANGUAGES.map((l) => (
          <option key={l.code} value={l.code}>
            {l.label}
          </option>
        ))}
      </select>
    </label>
  )
}
