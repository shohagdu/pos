import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { HeldCart } from '../../api/sales'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { ConfirmButton } from '../../components/ui/ConfirmButton'
import { Dialog } from '../../components/ui/Dialog'
import { TextField } from '../../components/ui/TextField'
import { formatDateTime } from '../../lib/dates'
import { problemMessage } from '../../lib/problem'

/** Hold cart (F9) with an optional label. */
export function HoldDialog({
  pending,
  error,
  onClose,
  onHold,
}: {
  pending: boolean
  error: unknown
  onClose: () => void
  onHold: (label: string) => void
}) {
  const { t } = useTranslation()
  const [label, setLabel] = useState('')
  function submit(e: FormEvent) {
    e.preventDefault()
    if (!pending) onHold(label.trim())
  }
  return (
    <Dialog
      open
      onClose={onClose}
      title={t('pos.holdTitle')}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button type="submit" form="pos-hold" disabled={pending}>
            {pending ? t('common.saving') : t('pos.hold')}
          </Button>
        </>
      }
    >
      <form id="pos-hold" onSubmit={submit} className="flex flex-col gap-3">
        <TextField
          label={`${t('pos.holdLabel')} (${t('common.optional')})`}
          value={label}
          maxLength={100}
          placeholder={t('pos.holdLabelHint')}
          onChange={(e) => setLabel(e.target.value)}
          autoFocus
        />
        {error !== null && error !== undefined && <Alert tone="danger">{problemMessage(error, t)}</Alert>}
      </form>
    </Dialog>
  )
}

/** Held carts (F10): resume or discard. */
export function HeldCartsDialog({
  carts,
  loading,
  error,
  cartBusy,
  pendingId,
  onClose,
  onResume,
  onDiscard,
}: {
  carts: HeldCart[]
  loading: boolean
  error: unknown
  /** The current cart has lines: resuming would replace it. */
  cartBusy: boolean
  pendingId: string | null
  onClose: () => void
  onResume: (cart: HeldCart) => void
  onDiscard: (cart: HeldCart) => void
}) {
  const { t, i18n } = useTranslation()
  return (
    <Dialog open onClose={onClose} title={t('pos.heldTitle')} wide>
      <div className="flex flex-col gap-3">
        {cartBusy && <Alert tone="warning">{t('pos.holdCurrentFirst')}</Alert>}
        {error !== null && error !== undefined && <Alert tone="danger">{problemMessage(error, t)}</Alert>}
        {loading ? (
          <p className="text-sm text-ink-muted">{t('common.loading')}</p>
        ) : carts.length === 0 ? (
          <p className="text-sm text-ink-muted">{t('pos.noHeld')}</p>
        ) : (
          <ul className="flex flex-col divide-y divide-border rounded-lg border border-border">
            {carts.map((c) => {
              const count = Array.isArray(c.cart.lines) ? c.cart.lines.length : 0
              return (
                <li key={c.id} className="flex flex-wrap items-center justify-between gap-3 px-3 py-2 text-sm">
                  <div className="min-w-0">
                    <div className="font-medium">{c.label || t('pos.unnamedCart')}</div>
                    <div className="text-xs text-ink-muted">
                      {[
                        c.customer_name ?? t('pos.walkIn'),
                        t('pos.itemCount', { count }),
                        c.held_by_name,
                        formatDateTime(c.created_at, i18n.language),
                      ]
                        .filter(Boolean)
                        .join(' · ')}
                    </div>
                  </div>
                  <div className="flex gap-2">
                    <Button disabled={cartBusy || pendingId !== null} onClick={() => onResume(c)}>
                      {t('pos.resume')}
                    </Button>
                    <ConfirmButton disabled={pendingId !== null} onConfirm={() => onDiscard(c)}>
                      {t('pos.discard')}
                    </ConfirmButton>
                  </div>
                </li>
              )
            })}
          </ul>
        )}
      </div>
    </Dialog>
  )
}
