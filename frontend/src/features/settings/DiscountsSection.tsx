import { useMutation, useQueryClient } from '@tanstack/react-query'
import type { FormEvent } from 'react'
import { useTranslation } from 'react-i18next'

import { type BusinessSettings, tenancyApi, tenancyKeys, useSettings } from '../../api/tenancy'
import { salesKeys } from '../../api/sales'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { Checkbox } from '../../components/ui/Checkbox'
import { Spinner } from '../../components/ui/Spinner'
import { TextField } from '../../components/ui/TextField'
import { dec, toPlain } from '../../lib/decimal'
import { useFields } from '../../lib/form'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'

/** Settings → Discounts (Final_docs/05 §5): how promotions, coupons and manual discounts combine. */
export function DiscountsSection({ tenantId }: { tenantId: string }) {
  const { t } = useTranslation()
  const settings = useSettings(tenantId)
  if (settings.isPending) return <Spinner />
  if (settings.isError) return <Alert tone="danger">{problemMessage(settings.error, t)}</Alert>
  return <DiscountsForm key={settings.data.version} tenantId={tenantId} settings={settings.data} />
}

function DiscountsForm({ tenantId, settings }: { tenantId: string; settings: BusinessSettings }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const { values, bind, bindCheck } = useFields({
    promotions_stack: settings.promotions_stack ?? false,
    coupon_stacks: settings.coupon_stacks ?? false,
    manual_discount_with_promotions: settings.manual_discount_with_promotions ?? true,
    restore_coupon_on_full_return: settings.restore_coupon_on_full_return ?? false,
    cashier_max_discount_pct: toPlain(dec(settings.cashier_max_discount_pct ?? '10')),
  })
  const save = useMutation({
    mutationFn: () =>
      tenancyApi.updateSettings(settings.version, {
        promotions_stack: values.promotions_stack,
        coupon_stacks: values.coupon_stacks,
        manual_discount_with_promotions: values.manual_discount_with_promotions,
        restore_coupon_on_full_return: values.restore_coupon_on_full_return,
        cashier_max_discount_pct: values.cashier_max_discount_pct.trim(),
      }),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: tenancyKeys.settings(tenantId) })
      // The POS prices with these settings.
      void queryClient.invalidateQueries({ queryKey: salesKeys.posContext(tenantId) })
    },
  })
  const errors = fieldErrors(save.error, t)
  return (
    <Card className="flex flex-col gap-4">
      <form
        className="flex flex-col gap-4"
        onSubmit={(e: FormEvent) => {
          e.preventDefault()
          save.mutate()
        }}
      >
        <p className="text-sm text-ink-muted">{t('settings.discounts.intro')}</p>
        <Checkbox label={t('settings.discounts.promotionsStack')} hint={t('settings.discounts.promotionsStackHint')} {...bindCheck('promotions_stack')} />
        <Checkbox label={t('settings.discounts.couponStacks')} hint={t('settings.discounts.couponStacksHint')} {...bindCheck('coupon_stacks')} />
        <Checkbox
          label={t('settings.discounts.manualWithPromotions')}
          hint={t('settings.discounts.manualWithPromotionsHint')}
          {...bindCheck('manual_discount_with_promotions')}
        />
        <Checkbox
          label={t('settings.discounts.restoreCoupon')}
          hint={t('settings.discounts.restoreCouponHint')}
          {...bindCheck('restore_coupon_on_full_return')}
        />
        <TextField
          className="max-w-60"
          label={t('settings.discounts.maxManual')}
          hint={t('settings.discounts.maxManualHint')}
          inputMode="decimal"
          {...bind('cashier_max_discount_pct')}
          error={errors.cashier_max_discount_pct}
        />
        <p className="text-xs text-ink-muted">{t('settings.discounts.couponsPerInvoice')}</p>
        {save.isError && hasFormError(save.error) && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}
        {save.isSuccess && <Alert tone="success">{t('settings.saved')}</Alert>}
        <Button type="submit" className="self-start" disabled={save.isPending}>
          {save.isPending ? t('common.loading') : t('common.save')}
        </Button>
      </form>
    </Card>
  )
}
