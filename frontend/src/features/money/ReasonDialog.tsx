import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Dialog } from '../../components/ui/Dialog'
import { TextField } from '../../components/ui/TextField'
import { fieldErrors, problemMessage } from '../../lib/problem'

/** Asks for a reason before a void (payments, expenses). Mount it only while open. */
export function ReasonDialog({
  title,
  description,
  confirmLabel,
  pending,
  error,
  onConfirm,
  onClose,
}: {
  title: string
  description?: string
  confirmLabel: string
  pending: boolean
  error: unknown
  onConfirm: (reason: string) => void
  onClose: () => void
}) {
  const { t } = useTranslation()
  const [reason, setReason] = useState('')
  const [missing, setMissing] = useState(false)
  const errors = fieldErrors(error, t)

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    if (!reason.trim()) {
      setMissing(true)
      return
    }
    onConfirm(reason.trim())
  }

  return (
    <Dialog open onClose={onClose} title={title}>
      <form id="reason-form" onSubmit={onSubmit} noValidate className="flex flex-col gap-4">
        {description && <p className="text-sm text-ink-muted">{description}</p>}
        {error !== null && error !== undefined && !errors.reason && <Alert tone="danger">{problemMessage(error, t)}</Alert>}
        <TextField
          label={t('common.reason')}
          value={reason}
          onChange={(e) => setReason(e.target.value)}
          maxLength={500}
          required
          autoFocus
          error={errors.reason ?? (missing ? t('money.reasonRequired') : undefined)}
        />
        <div className="flex flex-wrap justify-end gap-2">
          <Button variant="ghost" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button type="submit" className="bg-danger hover:bg-danger/90" disabled={pending}>
            {pending ? t('common.saving') : confirmLabel}
          </Button>
        </div>
      </form>
    </Dialog>
  )
}
