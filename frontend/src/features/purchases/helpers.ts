import { useQueryClient } from '@tanstack/react-query'
import type { TFunction } from 'i18next'

import { purchasingKeys } from '../../api/purchasing'
import { dec, tryParse } from '../../lib/decimal'
import { optional } from '../../lib/form'

export const LINK = 'font-medium text-brand-700 hover:underline'
export const BUTTON_LINK =
  'inline-flex min-h-11 items-center justify-center rounded-lg bg-brand-600 px-4 text-sm font-semibold text-white hover:bg-brand-700'
export const SECONDARY_LINK =
  'inline-flex min-h-11 items-center justify-center rounded-lg border border-border bg-surface px-4 text-sm font-semibold text-ink hover:bg-canvas'

/** "450.5000" → "450.5" for form inputs (string only, no floats). */
export function trimDecimal(value: string | null | undefined): string {
  if (!value) return ''
  return value.includes('.') ? value.replace(/0+$/, '').replace(/\.$/, '') : value
}

/** Invalidates everything a purchase, payment or return can change. */
export function useRefreshAfterMoney(tenantId: string) {
  const queryClient = useQueryClient()
  return () =>
    Promise.all(
      [purchasingKeys.all(tenantId), ['parties', tenantId], ['inventory', tenantId], ['money', tenantId], ['reports', tenantId]].map((queryKey) =>
        queryClient.invalidateQueries({ queryKey }),
      ),
    )
}

export interface PaymentValues {
  method_id: string
  amount: string
  account_id: string
  reference: string
}

/** Client checks shared by the payment dialogs; returns field errors. */
export function checkPayment(v: PaymentValues, max: string | null, t: TFunction, requiresReference: boolean) {
  const errors: Record<string, string> = {}
  if (!v.method_id) errors.method_id = t('purchases.errors.methodRequired')
  const amount = tryParse(v.amount)
  if (amount === null || amount <= 0n) errors.amount = t('purchases.errors.amountPositive')
  else if (max !== null && amount > dec(max)) errors.amount = t('purchases.errors.amountOverDue', { max })
  if (requiresReference && !v.reference.trim()) errors.reference = t('purchases.errors.referenceRequired')
  return errors
}

export function toPayInput(v: PaymentValues) {
  return {
    method_id: v.method_id,
    amount: v.amount.trim(),
    account_id: optional(v.account_id),
    reference: optional(v.reference),
  }
}
