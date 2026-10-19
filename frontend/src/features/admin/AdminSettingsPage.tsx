import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useId, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { canAdminister, platformApi, platformKeys, usePaymentInstructions } from '../../api/platform'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { PageHeader } from '../../components/ui/PageHeader'
import { Spinner } from '../../components/ui/Spinner'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { useStaffRole } from './labels'

const KNOWN = ['bkash', 'nagad', 'rocket', 'bank_transfer', 'note'] as const

/** Platform settings (ADM-R08): manual payment instructions shown to owners. */
export function AdminSettingsPage() {
  const { t } = useTranslation()
  const instructions = usePaymentInstructions()

  return (
    <div className="flex max-w-3xl flex-col gap-6">
      <PageHeader title={t('admin.settings.title')} />
      {instructions.isPending ? (
        <Spinner />
      ) : instructions.isError ? (
        <Alert tone="danger">{problemMessage(instructions.error, t)}</Alert>
      ) : (
        <InstructionsForm initial={instructions.data} />
      )}
    </div>
  )
}

function InstructionsForm({ initial }: { initial: Record<string, string> }) {
  const { t } = useTranslation()
  const role = useStaffRole()
  const editable = canAdminister(role)
  const queryClient = useQueryClient()
  const [values, setValues] = useState<Record<string, string>>(() => ({ ...initial }))
  const extra = Object.keys(initial).filter((k) => !(KNOWN as readonly string[]).includes(k))
  const save = useMutation({
    mutationFn: (value: Record<string, string>) => platformApi.updatePaymentInstructions(value),
    onSuccess: (data) => {
      queryClient.setQueryData(platformKeys.paymentInstructions, data)
      return queryClient.invalidateQueries({ queryKey: platformKeys.all })
    },
  })
  const errors = fieldErrors(save.error, t)

  function submit(e: FormEvent) {
    e.preventDefault()
    const out: Record<string, string> = {}
    for (const [k, v] of Object.entries(values)) {
      const text = v.trim()
      // Keep existing extra keys even when blank; drop blank known keys.
      if (text || extra.includes(k)) out[k] = text
    }
    save.mutate(out)
  }

  return (
    <Card>
      <form onSubmit={submit} className="flex flex-col gap-4">
        <h2 className="text-lg font-semibold">{t('admin.settings.paymentInstructions')}</h2>
        <p className="text-sm text-ink-muted">{t('admin.settings.intro')}</p>
        {!editable && <Alert>{t('admin.settings.readOnly')}</Alert>}
        {save.isError && hasFormError(save.error) && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}
        {save.isSuccess && <Alert tone="success">{t('admin.settings.saved')}</Alert>}
        {[...KNOWN, ...extra].map((key) => (
          <TextArea
            key={key}
            label={t(`admin.settings.keys.${key}`, { defaultValue: key })}
            value={values[key] ?? ''}
            disabled={!editable}
            error={errors[key]}
            onChange={(v) => {
              save.reset()
              setValues((s) => ({ ...s, [key]: v }))
            }}
          />
        ))}
        {editable && (
          <Button type="submit" className="self-start" disabled={save.isPending}>
            {save.isPending ? t('common.saving') : t('common.save')}
          </Button>
        )}
      </form>
    </Card>
  )
}

function TextArea({
  label,
  value,
  disabled,
  error,
  onChange,
}: {
  label: string
  value: string
  disabled: boolean
  error?: string
  onChange: (value: string) => void
}) {
  const id = useId()
  return (
    <div className="flex flex-col gap-1.5">
      <label htmlFor={id} className="text-sm font-medium text-ink">
        {label}
      </label>
      <textarea
        id={id}
        rows={3}
        value={value}
        disabled={disabled}
        aria-invalid={error ? true : undefined}
        aria-describedby={error ? `${id}-error` : undefined}
        onChange={(e) => onChange(e.target.value)}
        className="rounded-lg border border-border bg-surface px-3 py-2 text-sm outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100 aria-invalid:border-danger disabled:bg-canvas disabled:text-ink-muted"
      />
      {error && (
        <p id={`${id}-error`} className="text-sm text-danger">
          {error}
        </p>
      )}
    </div>
  )
}
