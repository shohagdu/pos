import { useTranslation } from 'react-i18next'

import type { PartyStatus } from '../../api/parties'
import { Badge } from '../../components/ui/Badge'
import { dec } from '../../lib/decimal'

/** Positive balance = we owe the supplier (payable); negative = advance paid. */
export function BalanceBadge({ balance }: { balance: string }) {
  const { t } = useTranslation()
  const v = dec(balance)
  if (v === 0n) return <Badge>{t('suppliers.settled')}</Badge>
  return v > 0n ? <Badge tone="bad">{t('suppliers.payable')}</Badge> : <Badge tone="info">{t('suppliers.advance')}</Badge>
}

export function StatusBadge({ status }: { status: PartyStatus }) {
  const { t } = useTranslation()
  return <Badge tone={status === 'active' ? 'good' : 'neutral'}>{t(`suppliers.statuses.${status}`)}</Badge>
}
