import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Button, type ButtonProps } from './Button'

/** A destructive action that asks once inline ("Are you sure?") instead of a browser dialog. */
export function ConfirmButton({ onConfirm, children, disabled, ...props }: ButtonProps & { onConfirm: () => void }) {
  const { t } = useTranslation()
  const [asking, setAsking] = useState(false)
  if (!asking) {
    return (
      <Button variant="secondary" disabled={disabled} onClick={() => setAsking(true)} {...props}>
        {children}
      </Button>
    )
  }
  return (
    <span className="inline-flex flex-wrap items-center gap-2">
      <span className="text-sm text-ink-muted">{t('common.areYouSure')}</span>
      <Button
        className="bg-danger text-white hover:bg-danger/90"
        disabled={disabled}
        onClick={() => {
          setAsking(false)
          onConfirm()
        }}
      >
        {t('common.yes')}
      </Button>
      <Button variant="ghost" onClick={() => setAsking(false)}>
        {t('common.cancel')}
      </Button>
    </span>
  )
}
