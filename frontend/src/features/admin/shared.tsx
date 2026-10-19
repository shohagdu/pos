import { type FormEvent, type ReactNode, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { Dialog } from '../../components/ui/Dialog'
import { TextField } from '../../components/ui/TextField'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { stateLabel } from './labels'

type Tone = 'neutral' | 'good' | 'warn' | 'bad' | 'info'

const STATE_TONES: Record<string, Tone> = {
  trialing: 'info',
  active: 'good',
  grace: 'warn',
  read_only: 'warn',
  suspended: 'bad',
  cancelled: 'neutral',
  terminated: 'neutral',
  business_suspended: 'bad',
}

export function SubscriptionBadge({ state }: { state: string | null }) {
  const { t } = useTranslation()
  if (!state) return <Badge>{t('admin.states.none')}</Badge>
  return <Badge tone={STATE_TONES[state] ?? 'neutral'}>{stateLabel(t, state)}</Badge>
}

export function BusinessStatusBadge({ status }: { status: string }) {
  const { t } = useTranslation()
  return <Badge tone={status === 'active' ? 'good' : 'bad'}>{t(`admin.businessStatus.${status}`, { defaultValue: status })}</Badge>
}

export function ClaimStatusBadge({ status }: { status: string }) {
  const { t } = useTranslation()
  const tone: Tone = status === 'approved' ? 'good' : status === 'rejected' ? 'bad' : 'warn'
  return <Badge tone={tone}>{t(`admin.claimStatus.${status}`, { defaultValue: status })}</Badge>
}

export function InvoiceStatusBadge({ status }: { status: string }) {
  const { t } = useTranslation()
  const tone: Tone = status === 'paid' ? 'good' : status === 'open' ? 'warn' : 'neutral'
  return <Badge tone={tone}>{t(`admin.invoiceStatus.${status}`, { defaultValue: status })}</Badge>
}

interface ReasonDialogProps {
  open: boolean
  onClose: () => void
  title: string
  hint?: string
  confirmLabel: string
  pending: boolean
  error: unknown
  onSubmit: (reason: string) => void
}

/** A dialog asking for a required reason before an action (fresh state each time it opens). */
export function ReasonDialog(props: ReasonDialogProps) {
  return props.open ? <ReasonForm {...props} /> : null
}

function ReasonForm({
  onClose,
  title,
  hint,
  confirmLabel,
  pending,
  error,
  onSubmit,
}: ReasonDialogProps) {
  const { t } = useTranslation()
  const [reason, setReason] = useState('')
  const [missing, setMissing] = useState(false)
  const formId = `reason-${title.replace(/\W+/g, '-')}`
  function submit(e: FormEvent) {
    e.preventDefault()
    const text = reason.trim()
    if (!text) {
      setMissing(true)
      return
    }
    onSubmit(text)
  }
  const fields = fieldErrors(error, t)
  return (
    <Dialog
      open
      onClose={onClose}
      title={title}
      footer={
        <>
          <Button variant="secondary" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button type="submit" form={formId} disabled={pending}>
            {pending ? t('common.saving') : confirmLabel}
          </Button>
        </>
      }
    >
      <form id={formId} onSubmit={submit} className="flex flex-col gap-3" noValidate>
        {hint && <p className="text-sm text-ink-muted">{hint}</p>}
        {Boolean(error) && hasFormError(error) && <Alert tone="danger">{problemMessage(error, t)}</Alert>}
        <TextField
          label={t('common.reason')}
          value={reason}
          required
          maxLength={500}
          onChange={(e) => {
            setReason(e.target.value)
            setMissing(false)
          }}
          error={missing ? t('problems.REQUIRED') : fields.reason}
        />
      </form>
    </Dialog>
  )
}

/** Small "label: value" pair for detail headers. */
export function Fact({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div>
      <dt className="text-xs font-medium tracking-wide text-ink-muted uppercase">{label}</dt>
      <dd className="mt-0.5 text-sm">{children}</dd>
    </div>
  )
}
