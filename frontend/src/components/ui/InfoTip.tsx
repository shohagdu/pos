import { useTranslation } from 'react-i18next'

/**
 * A small ⓘ next to a field label that shows help text on hover or keyboard
 * focus. The text stays in the page with `id`, so the field can point at it
 * with `aria-describedby` and screen readers read it with the field.
 */
export function InfoTip({ id, text, align = 'start' }: { id: string; text: string; align?: 'start' | 'end' }) {
  const { t } = useTranslation()
  return (
    <span className="group relative inline-flex">
      <button
        type="button"
        aria-label={t('common.moreInfo')}
        aria-describedby={id}
        className="inline-flex size-5 items-center justify-center rounded-full text-ink-muted hover:text-brand-700 focus-visible:text-brand-700 focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-brand-600"
      >
        <svg viewBox="0 0 20 20" className="size-3.5" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true">
          <circle cx="10" cy="10" r="7.5" />
          <path d="M10 9v4.5M10 6.5h.01" strokeLinecap="round" />
        </svg>
      </button>
      <span
        id={id}
        role="tooltip"
        className={`pointer-events-none invisible absolute bottom-full z-30 mb-1.5 w-60 rounded-lg bg-ink px-3 py-2 text-xs leading-snug font-normal text-white opacity-0 shadow-lg transition-opacity group-focus-within:visible group-focus-within:opacity-100 group-hover:visible group-hover:opacity-100 motion-reduce:transition-none ${
          align === 'end' ? 'right-0' : '-left-2'
        }`}
      >
        {text}
      </span>
    </span>
  )
}
