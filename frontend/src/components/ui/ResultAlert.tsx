import { useEffect, useId, useRef } from 'react'

import { Button } from './Button'

type Tone = 'success' | 'warning'

const ICON: Record<Tone, { ring: string; path: string }> = {
  success: { ring: 'bg-success/12 text-success', path: 'M7 12.5l3.2 3.2L17 9' },
  warning: { ring: 'bg-warning/15 text-warning', path: 'M12 7.5v5.5M12 16.5h.01' },
}

/**
 * A centred result pop-up (SweetAlert style) on the native `<dialog>`: a
 * large icon that draws itself, a title, a message and one button. With
 * `autoCloseMs` it closes by itself, showing the time left as a bar. Enter,
 * Esc and the button all call `onClose`. Motion is skipped when reduced
 * motion is preferred.
 */
export function ResultAlert({
  tone = 'success',
  title,
  message,
  confirmLabel,
  autoCloseMs,
  onClose,
}: {
  tone?: Tone
  title: string
  message?: string
  confirmLabel: string
  /** Close by itself after this long; leave out to wait for the button. */
  autoCloseMs?: number
  onClose: () => void
}) {
  const ref = useRef<HTMLDialogElement>(null)
  const titleId = useId()
  const messageId = useId()
  const closeRef = useRef(onClose)
  useEffect(() => {
    closeRef.current = onClose
  })

  useEffect(() => {
    const el = ref.current
    if (!el || el.open) return
    // jsdom has no showModal(); fall back to the open attribute.
    if (typeof el.showModal === 'function') el.showModal()
    else el.setAttribute('open', '')
    el.querySelector<HTMLElement>('[data-autofocus]')?.focus()
  }, [])

  useEffect(() => {
    if (!autoCloseMs) return
    const timer = setTimeout(() => closeRef.current(), autoCloseMs)
    return () => clearTimeout(timer)
  }, [autoCloseMs])

  const icon = ICON[tone]
  return (
    <dialog
      ref={ref}
      role="alertdialog"
      aria-labelledby={titleId}
      aria-describedby={message ? messageId : undefined}
      onCancel={(e) => {
        e.preventDefault()
        onClose()
      }}
      className="m-auto w-[calc(100%-2rem)] max-w-sm overflow-hidden rounded-2xl border border-border bg-surface p-0 text-center text-ink shadow-xl backdrop:bg-ink/40 motion-safe:animate-alert-pop"
    >
      <div className="flex flex-col items-center gap-3 px-6 pt-8 pb-6">
        <span aria-hidden="true" className={`flex size-16 items-center justify-center rounded-full ${icon.ring}`}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2.4} strokeLinecap="round" strokeLinejoin="round" className="size-9">
            <path d={icon.path} pathLength={1} strokeDasharray={1} className="motion-safe:animate-alert-draw" />
          </svg>
        </span>
        <h2 id={titleId} className="text-xl font-semibold">
          {title}
        </h2>
        {message && (
          <p id={messageId} className="max-w-[34ch] text-sm text-ink-muted">
            {message}
          </p>
        )}
        <Button data-autofocus className="mt-2 min-w-32" onClick={onClose}>
          {confirmLabel}
        </Button>
      </div>
      {autoCloseMs ? (
        <span
          aria-hidden="true"
          className="block h-1 origin-left bg-brand-500/70 motion-safe:animate-alert-countdown motion-reduce:hidden"
          style={{ animationDuration: `${autoCloseMs}ms` }}
        />
      ) : null}
    </dialog>
  )
}
