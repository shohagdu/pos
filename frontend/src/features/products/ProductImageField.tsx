import { type DragEvent, type ReactNode, useId, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { useObjectUrl } from '../../api/files'
import { Button } from '../../components/ui/Button'
import { IMAGE_TYPES, imageProblem } from './productImage'

/**
 * Optional product photo: a square preview you can click or drop a file on,
 * with choose / change / remove. `imageUrl` is the photo already saved (if
 * any); `onPick(null)` removes.
 */
export function ProductImageField({
  imageUrl,
  pending,
  busy = false,
  disabled = false,
  error,
  compact = false,
  onPick,
}: {
  imageUrl: string | null
  /** A chosen file not yet saved (new products upload it after creating). */
  pending?: File | null
  busy?: boolean
  disabled?: boolean
  error?: string
  /** A small square with the buttons beside it (forms that fit on one screen). */
  compact?: boolean
  onPick: (file: File | null) => void
}) {
  const { t } = useTranslation()
  const input = useRef<HTMLInputElement>(null)
  const id = useId()
  const [localError, setLocalError] = useState<string | null>(null)
  const [checking, setChecking] = useState(false)
  const [dragging, setDragging] = useState(false)
  const pendingUrl = useObjectUrl(pending)
  const shown = pendingUrl ?? imageUrl
  const message = localError ? t(localError) : error
  const locked = disabled || busy || checking

  async function choose(file: File | undefined) {
    if (input.current) input.current.value = ''
    if (!file) return
    setChecking(true)
    try {
      const problem = await imageProblem(file)
      setLocalError(problem)
      if (!problem) onPick(file)
    } finally {
      setChecking(false)
    }
  }

  function onDrop(e: DragEvent) {
    e.preventDefault()
    setDragging(false)
    if (!locked) void choose(e.dataTransfer.files[0])
  }

  const dropZone = (sizeClass: string, body: ReactNode) => (
    // Mouse/drag target only (not focusable); the "Choose image" button is the keyboard control.
    <div
      onClick={() => {
        if (!locked) input.current?.click()
      }}
      onDragOver={(e) => {
        e.preventDefault()
        if (!locked) setDragging(true)
      }}
      onDragLeave={() => setDragging(false)}
      onDrop={onDrop}
      className={`relative flex aspect-square items-center justify-center overflow-hidden rounded-xl border-2 border-dashed bg-canvas transition-colors ${sizeClass} ${
        locked ? 'cursor-not-allowed' : 'cursor-pointer'
      } ${dragging ? 'border-brand-500 bg-brand-50' : 'border-border hover:border-brand-500'}`}
    >
      {shown ? <img src={shown} alt={t('products.image.preview')} className="size-full object-cover" /> : body}
      {(busy || checking) && (
        <span className="absolute inset-0 flex items-center justify-center bg-surface/70 text-sm font-medium">{t('common.pleaseWait')}</span>
      )}
    </div>
  )
  const fileInput = (
    <input
      ref={input}
      id={id}
      type="file"
      accept={IMAGE_TYPES.join(',')}
      className="sr-only"
      aria-labelledby={`${id}-label`}
      disabled={locked}
      onChange={(e) => void choose(e.target.files?.[0])}
    />
  )
  const picture = (
    <svg viewBox="0 0 24 24" className="size-8" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true">
      <rect x="3" y="4" width="18" height="16" rx="2" />
      <circle cx="9" cy="10" r="2" />
      <path d="m21 16-5-5-9 9" />
    </svg>
  )

  if (compact) {
    return (
      <div className="flex items-start gap-3">
        {dropZone(
          'size-24 shrink-0',
          <span className="flex flex-col items-center gap-1 px-2 text-center text-xs text-ink-muted">
            {picture}
            <span>{t('products.image.dropHint')}</span>
          </span>,
        )}
        {fileInput}
        <div className="flex min-w-0 flex-col items-start gap-1.5">
          <span id={`${id}-label`} className="text-[0.8125rem] font-medium">
            {t('products.image.label')} <span className="font-normal text-ink-muted">({t('common.optional')})</span>
          </span>
          <Button
            variant="secondary"
            size="sm"
            className="whitespace-nowrap"
            disabled={locked}
            aria-describedby={`${id}-hint`}
            onClick={() => input.current?.click()}
          >
            {shown ? t('products.image.change') : t('products.image.choose')}
          </Button>
          {shown && (
            <Button
              variant="ghost"
              className="px-2"
              disabled={locked}
              onClick={() => {
                setLocalError(null)
                onPick(null)
              }}
            >
              {t('products.image.remove')}
            </Button>
          )}
          <span id={`${id}-hint`} className="text-xs text-ink-muted">
            {t('products.image.hint')}
          </span>
          {!shown && <span className="sr-only">{t('products.image.none')}</span>}
          {message && (
            <span role="alert" className="text-xs text-danger">
              {message}
            </span>
          )}
        </div>
      </div>
    )
  }

  return (
    <div className="flex flex-col gap-3">
      <span id={`${id}-label`} className="text-sm font-medium">
        {t('products.image.label')} <span className="font-normal text-ink-muted">({t('common.optional')})</span>
      </span>
      {/* Mouse/drag target only (not focusable); the "Choose image" button below is the keyboard control. */}
      <div
        onClick={() => {
          if (!locked) input.current?.click()
        }}
        onDragOver={(e) => {
          e.preventDefault()
          if (!locked) setDragging(true)
        }}
        onDragLeave={() => setDragging(false)}
        onDrop={onDrop}
        className={`relative flex aspect-square w-full max-w-64 items-center justify-center self-center overflow-hidden rounded-xl border-2 border-dashed bg-canvas transition-colors ${
          locked ? 'cursor-not-allowed' : 'cursor-pointer'
        } ${dragging ? 'border-brand-500 bg-brand-50' : 'border-border hover:border-brand-500'}`}
      >
        {shown ? (
          <img src={shown} alt={t('products.image.preview')} className="size-full object-cover" />
        ) : (
          <span className="flex flex-col items-center gap-2 px-4 text-center text-sm text-ink-muted">
            <svg viewBox="0 0 24 24" className="size-8" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true">
              <rect x="3" y="4" width="18" height="16" rx="2" />
              <circle cx="9" cy="10" r="2" />
              <path d="m21 16-5-5-9 9" />
            </svg>
            <span>{t('products.image.none')}</span>
            <span className="text-xs">{t('products.image.dropHint')}</span>
          </span>
        )}
        {(busy || checking) && (
          <span className="absolute inset-0 flex items-center justify-center bg-surface/70 text-sm font-medium">{t('common.pleaseWait')}</span>
        )}
      </div>
      <input
        ref={input}
        id={id}
        type="file"
        accept={IMAGE_TYPES.join(',')}
        className="sr-only"
        aria-labelledby={`${id}-label`}
        disabled={locked}
        onChange={(e) => void choose(e.target.files?.[0])}
      />
      <div className="flex flex-wrap justify-center gap-2">
        <Button variant="secondary" disabled={locked} aria-describedby={`${id}-hint`} onClick={() => input.current?.click()}>
          {shown ? t('products.image.change') : t('products.image.choose')}
        </Button>
        {shown && (
          <Button
            variant="ghost"
            disabled={locked}
            onClick={() => {
              setLocalError(null)
              onPick(null)
            }}
          >
            {t('products.image.remove')}
          </Button>
        )}
      </div>
      <span id={`${id}-hint`} className="text-center text-xs text-ink-muted">
        {t('products.image.hint')}
      </span>
      {message && (
        <span role="alert" className="text-center text-sm text-danger">
          {message}
        </span>
      )}
    </div>
  )
}
