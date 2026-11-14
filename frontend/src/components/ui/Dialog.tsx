import { type ReactNode, useEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'

/**
 * Modal dialog on the native `<dialog>` element: focus is trapped, `Esc`
 * closes it (POS shortcut), and the page behind is inert. Mark the field to
 * focus on open with `data-autofocus`.
 */
export function Dialog({
  open,
  onClose,
  title,
  children,
  footer,
  wide = false,
}: {
  open: boolean
  onClose: () => void
  title: string
  children: ReactNode
  footer?: ReactNode
  wide?: boolean
}) {
  const { t } = useTranslation()
  const ref = useRef<HTMLDialogElement>(null)

  useEffect(() => {
    const el = ref.current
    if (!el) return
    if (open && !el.open) {
      // jsdom has no showModal(); fall back to the open attribute.
      if (typeof el.showModal === 'function') el.showModal()
      else el.setAttribute('open', '')
      // React's autoFocus fires while the dialog is still hidden, and
      // showModal() then focuses the header ✕; honour data-autofocus instead.
      el.querySelector<HTMLElement>('[data-autofocus]')?.focus()
    } else if (!open && el.open) {
      if (typeof el.close === 'function') el.close()
      else el.removeAttribute('open')
    }
  }, [open])

  if (!open) return null
  return (
    <dialog
      ref={ref}
      aria-label={title}
      onCancel={(e) => {
        e.preventDefault()
        onClose()
      }}
      className={`m-auto w-[calc(100%-2rem)] rounded-xl border border-border bg-surface p-0 text-ink shadow-xl backdrop:bg-ink/40 max-sm:mb-0 max-sm:w-full max-sm:max-w-none max-sm:rounded-b-none ${wide ? 'max-w-3xl' : 'max-w-lg'}`}
    >
      <div className="flex items-center justify-between gap-4 border-b border-border px-4 py-2 sm:px-5 sm:py-3">
        <h2 className="text-lg font-semibold">{title}</h2>
        <button
          type="button"
          onClick={onClose}
          className="-mr-2 flex size-11 shrink-0 items-center justify-center rounded-lg text-ink-muted hover:bg-canvas lg:size-9"
          aria-label={t('common.close')}
        >
          ✕
        </button>
      </div>
      <div className="dialog-body max-h-[70svh] overflow-y-auto px-4 py-4 sm:px-5">{children}</div>
      {footer && (
        <div className="flex flex-wrap justify-end gap-2 border-t border-border px-4 py-3 pb-[max(0.75rem,env(safe-area-inset-bottom))] sm:px-5">{footer}</div>
      )}
    </dialog>
  )
}
