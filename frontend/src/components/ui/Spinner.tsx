import { useTranslation } from 'react-i18next'

export function Spinner({ className = '' }: { className?: string }) {
  const { t } = useTranslation()
  return (
    <span role="status" className={`inline-flex items-center gap-2 text-sm text-ink-muted ${className}`}>
      <span aria-hidden className="size-4 animate-spin rounded-full border-2 border-border border-t-brand-600" />
      <span className="sr-only">{t('common.loading')}</span>
    </span>
  )
}
