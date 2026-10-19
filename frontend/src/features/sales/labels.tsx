import type { ComponentProps } from 'react'
import { useTranslation } from 'react-i18next'

import type { PaymentStatus, SaleStatus } from '../../api/sales'
import { Badge } from '../../components/ui/Badge'

type Tone = NonNullable<ComponentProps<typeof Badge>['tone']>

const STATUS_TONE: Record<SaleStatus, Tone> = {
  completed: 'good',
  partially_returned: 'warn',
  returned: 'neutral',
  cancelled: 'bad',
}

const PAYMENT_TONE: Record<PaymentStatus, Tone> = {
  paid: 'good',
  partial: 'warn',
  unpaid: 'bad',
  refunded: 'neutral',
}

export function SaleStatusBadge({ status }: { status: SaleStatus }) {
  const { t } = useTranslation()
  return <Badge tone={STATUS_TONE[status]}>{t(`sales.statuses.${status}`)}</Badge>
}

export function PaymentStatusBadge({ status }: { status: PaymentStatus }) {
  const { t } = useTranslation()
  return <Badge tone={PAYMENT_TONE[status]}>{t(`sales.paymentStatuses.${status}`)}</Badge>
}
