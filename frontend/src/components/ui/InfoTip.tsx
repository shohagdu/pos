import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

/**
 * A small ⓘ next to a field label that shows help text on hover or keyboard
 * focus. The text stays in the page with `id`, so the field can point at it
 * with `aria-describedby` and screen readers read it with the field.
 *
 * While hidden the bubble is `display: none`, so it never widens the page;
 * when shown it opens towards the side with room (`align` is the preference).
 */
export function InfoTip({ id, text, align = 'start' }: { id: string; text: string; align?: 'start' | 'end' }) {
  const { t } = useTranslation()
  const anchor = useRef<HTMLSpanElement>(null)
  const [side, setSide] = useState(align)

  // 15rem bubble plus a margin; flip when the preferred side would run off-screen.
  function place() {
    const r = anchor.current?.getBoundingClientRect()
    if (!r || typeof window === 'undefined') return
    const room = 15 * 16 + 16
    if (align === 'start') setSide(r.left + room > window.innerWidth ? 'end' : 'start')
    else setSide(r.right - room < 0 ? 'start' : 'end')
  }

  return (
    <span ref={anchor} className="group relative inline-flex" onPointerEnter={place} onFocus={place}>
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
        className={`pointer-events-none absolute bottom-full z-30 mb-1.5 hidden w-60 max-w-[calc(100vw-2rem)] rounded-lg bg-ink px-3 py-2 text-xs leading-snug font-normal text-white shadow-lg group-focus-within:block group-hover:block ${
          side === 'end' ? 'right-0' : '-left-2'
        }`}
      >
        {text}
      </span>
    </span>
  )
}
