import { useTranslation } from 'react-i18next'

import type { PromotionState } from '../../api/promotions'
import { Badge } from '../../components/ui/Badge'

const STATE_TONE: Record<PromotionState, 'neutral' | 'good' | 'warn' | 'bad' | 'info'> = {
  draft: 'neutral',
  scheduled: 'info',
  running: 'good',
  paused: 'warn',
  expired: 'neutral',
  sold_out: 'bad',
  ended: 'neutral',
}

export function StateBadge({ state }: { state: PromotionState }) {
  const { t } = useTranslation()
  return <Badge tone={STATE_TONE[state]}>{t(`promotions.states.${state}`)}</Badge>
}
