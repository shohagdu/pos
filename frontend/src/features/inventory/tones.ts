// Badge tones for transfer and count statuses (list and detail pages).

import type { CountStatus, TransferStatus } from '../../api/inventory'

type Tone = 'neutral' | 'good' | 'warn' | 'bad' | 'info'

export const TRANSFER_TONES: Record<TransferStatus, Tone> = {
  draft: 'neutral',
  dispatched: 'warn',
  received: 'good',
  cancelled: 'bad',
}

export const COUNT_TONES: Record<CountStatus, Tone> = {
  in_progress: 'warn',
  posted: 'good',
  cancelled: 'bad',
}
