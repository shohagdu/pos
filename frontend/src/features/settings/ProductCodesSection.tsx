import { useMutation, useQueryClient } from '@tanstack/react-query'
import type { FormEvent } from 'react'
import { useTranslation } from 'react-i18next'

import { ApiError } from '../../api/client'
import { catalogKeys } from '../../api/catalog'
import { type BusinessSettings, tenancyApi, tenancyKeys, useSettings } from '../../api/tenancy'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { Select } from '../../components/ui/Select'
import { Spinner } from '../../components/ui/Spinner'
import { TextField } from '../../components/ui/TextField'
import { useFields } from '../../lib/form'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'

const DIGITS = [3, 4, 5, 6, 7, 8, 9, 10]
const PREFIX = /^[A-Z0-9]{1,10}$/

/** What a code with this numbering looks like: `MIR` + 4 digits → `MIR0001`. */
function exampleCode(prefix: string, digits: number, n = 1): string {
  return `${prefix.trim().toUpperCase()}${String(n).padStart(digits, '0')}`
}

/** An all-digit barcode 8, 12 or 13 long reads as a retail EAN/UPC (needs a check digit). */
function retailLength(prefix: string, digits: number): boolean {
  const p = prefix.trim()
  return /^[0-9]*$/.test(p) && [8, 12, 13].includes(p.length + digits)
}

/** Settings → Product codes: how automatic SKUs and barcodes are numbered. */
export function ProductCodesSection({ tenantId }: { tenantId: string }) {
  const { t } = useTranslation()
  const settings = useSettings(tenantId)
  if (settings.isPending) return <Spinner />
  if (settings.isError) return <Alert tone="danger">{problemMessage(settings.error, t)}</Alert>
  return <ProductCodesForm settings={settings.data} tenantId={tenantId} />
}

function ProductCodesForm({ settings, tenantId }: { settings: BusinessSettings; tenantId: string }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const { values, bind } = useFields({
    sku_prefix: settings.sku_prefix,
    sku_digits: String(settings.sku_digits),
    barcode_prefix: settings.barcode_prefix,
    barcode_digits: String(settings.barcode_digits),
  })
  const skuDigits = Number(values.sku_digits)
  const barcodeDigits = Number(values.barcode_digits)

  // The same rules the server applies, shown before saving.
  const local: Record<string, string> = {}
  if (!PREFIX.test(values.sku_prefix.trim().toUpperCase())) local.sku_prefix = t('settings.codes.prefixInvalid')
  if (!PREFIX.test(values.barcode_prefix.trim().toUpperCase())) local.barcode_prefix = t('settings.codes.prefixInvalid')
  else if (retailLength(values.barcode_prefix, barcodeDigits)) local.barcode_digits = t('settings.codes.retailLength')

  const save = useMutation({
    mutationFn: () =>
      tenancyApi.updateSettings(settings.version, {
        sku_prefix: values.sku_prefix,
        sku_digits: skuDigits,
        barcode_prefix: values.barcode_prefix,
        barcode_digits: barcodeDigits,
      }),
    onSuccess: async (saved) => {
      queryClient.setQueryData(tenancyKeys.settings(tenantId), saved)
      // Product forms suggest the next codes in the new style.
      await queryClient.invalidateQueries({ queryKey: catalogKeys.products(tenantId) })
    },
    onError: (err) => {
      if (err instanceof ApiError && err.status === 412) void queryClient.invalidateQueries({ queryKey: tenancyKeys.settings(tenantId) })
    },
  })
  const errors = { ...fieldErrors(save.error, t), ...local }

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    if (Object.keys(local).length === 0) save.mutate()
  }

  const digitOptions = DIGITS.map((d) => ({ value: String(d), label: t('settings.codes.digitsOption', { count: d }) }))
  return (
    <Card>
      <form onSubmit={onSubmit} className="flex flex-col gap-6" noValidate>
        <p className="text-sm text-ink-muted">{t('settings.codes.intro')}</p>

        <fieldset className="flex flex-col gap-3">
          <legend className="mb-1 font-semibold">{t('settings.codes.sku')}</legend>
          <div className="grid gap-4 sm:grid-cols-2">
            <TextField
              label={t('settings.codes.prefix')}
              maxLength={10}
              autoCapitalize="characters"
              error={errors.sku_prefix}
              hint={t('settings.codes.prefixHint')}
              {...bind('sku_prefix')}
            />
            <Select label={t('settings.codes.digits')} options={digitOptions} error={errors.sku_digits} {...bind('sku_digits')} />
          </div>
          <p className="text-sm" aria-live="polite">
            {t('settings.codes.example')} <code className="rounded bg-canvas px-1.5 py-0.5 font-semibold">{exampleCode(values.sku_prefix, skuDigits)}</code>
          </p>
        </fieldset>

        <fieldset className="flex flex-col gap-3">
          <legend className="mb-1 font-semibold">{t('settings.codes.barcode')}</legend>
          <div className="grid gap-4 sm:grid-cols-2">
            <TextField
              label={t('settings.codes.prefix')}
              maxLength={10}
              autoCapitalize="characters"
              error={errors.barcode_prefix}
              hint={t('settings.codes.prefixHint')}
              {...bind('barcode_prefix')}
            />
            <Select label={t('settings.codes.digits')} options={digitOptions} error={errors.barcode_digits} {...bind('barcode_digits')} />
          </div>
          <p className="text-sm" aria-live="polite">
            {t('settings.codes.example')}{' '}
            <code className="rounded bg-canvas px-1.5 py-0.5 font-semibold">{exampleCode(values.barcode_prefix, barcodeDigits)}</code>
          </p>
          <p className="text-sm text-ink-muted">{t('settings.codes.barcodeHint')}</p>
        </fieldset>

        <p className="text-sm text-ink-muted">{t('settings.codes.carryOn')}</p>
        {save.isError && hasFormError(save.error) && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}
        {save.isSuccess && <Alert tone="success">{t('settings.saved')}</Alert>}
        <Button type="submit" disabled={save.isPending} className="sm:self-end">
          {save.isPending ? t('common.saving') : t('common.save')}
        </Button>
      </form>
    </Card>
  )
}
