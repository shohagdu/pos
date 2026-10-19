import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { type Shift, useCurrentShift, useRegisters, useShifts } from '../../api/registers'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { PageHeader } from '../../components/ui/PageHeader'
import { formatDateTime } from '../../lib/dates'
import { formatMoney } from '../../lib/format'
import { useBranch } from '../branches/useBranch'
import { LinkButton, QueryStatus, TextLink } from '../money/shared'
import { useTenant } from '../money/utils'
import { OpenShiftDialog } from './OpenShiftDialog'
import { ShiftDifference } from './ShiftDifference'

/** Register shifts (`/shifts`): the cashier's open shift and the shift history. */
export function ShiftsPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { tenantId } = useTenant()
  const { branch } = useBranch()
  const [opening, setOpening] = useState(false)
  const current = useCurrentShift(tenantId)
  const registers = useRegisters(tenantId)
  const shifts = useShifts(tenantId, {})
  const shift = current.data

  const columns: Column<Shift>[] = [
    {
      key: 'no',
      header: t('shifts.shift'),
      render: (s) => <TextLink to={`/shifts/${s.id}`}>{t('shifts.shiftNo', { no: s.shift_no })}</TextLink>,
    },
    { key: 'register', header: t('shifts.register'), render: (s) => s.register_name },
    { key: 'cashier', header: t('shifts.cashier'), render: (s) => s.opened_by_name ?? '—' },
    { key: 'opened', header: t('shifts.openedAt'), render: (s) => <span className="whitespace-nowrap">{formatDateTime(s.opened_at, lang)}</span> },
    {
      key: 'closed',
      header: t('shifts.closedAt'),
      render: (s) => <span className="whitespace-nowrap">{s.closed_at ? formatDateTime(s.closed_at, lang) : '—'}</span>,
    },
    { key: 'opening', header: t('shifts.openingCash'), align: 'right', render: (s) => formatMoney(s.opening_cash, lang) },
    { key: 'expected', header: t('shifts.expectedCash'), align: 'right', render: (s) => (s.expected_cash === null ? '—' : formatMoney(s.expected_cash, lang)) },
    { key: 'counted', header: t('shifts.countedCash'), align: 'right', render: (s) => (s.counted_cash === null ? '—' : formatMoney(s.counted_cash, lang)) },
    { key: 'difference', header: t('shifts.difference'), align: 'right', render: (s) => <ShiftDifference value={s.difference} /> },
    {
      key: 'status',
      header: t('common.status'),
      render: (s) => <Badge tone={s.status === 'open' ? 'info' : 'neutral'}>{t(`shifts.statuses.${s.status}`)}</Badge>,
    },
  ]

  return (
    <div className="flex flex-col gap-6">
      <PageHeader title={t('shifts.title')} subtitle={branch ? t('shifts.subtitleBranch', { branch: branch.name }) : t('shifts.subtitle')} />
      <Card className="flex flex-col gap-3">
        <h2 className="font-semibold">{t('shifts.current')}</h2>
        {current.isSuccess ? (
          shift ? (
            <div className="flex flex-wrap items-center justify-between gap-3">
              <dl className="grid gap-x-6 gap-y-1 text-sm sm:grid-cols-3">
                <div>
                  <dt className="text-ink-muted">{t('shifts.register')}</dt>
                  <dd className="font-medium">
                    {shift.register_name} · {t('shifts.shiftNo', { no: shift.shift_no })}
                  </dd>
                </div>
                <div>
                  <dt className="text-ink-muted">{t('shifts.openedAt')}</dt>
                  <dd className="font-medium">{formatDateTime(shift.opened_at, lang)}</dd>
                </div>
                <div>
                  <dt className="text-ink-muted">{t('shifts.openingCash')}</dt>
                  <dd className="font-medium">{formatMoney(shift.opening_cash, lang)}</dd>
                </div>
              </dl>
              <LinkButton to={`/shifts/${shift.id}`} variant="primary">
                {t('shifts.viewXClose')}
              </LinkButton>
            </div>
          ) : (
            <div className="flex flex-wrap items-center justify-between gap-3">
              <p className="text-sm text-ink-muted">{t('shifts.noneOpenHint')}</p>
              <Button onClick={() => setOpening(true)} disabled={!registers.isSuccess}>
                {t('shifts.open')}
              </Button>
            </div>
          )
        ) : (
          <QueryStatus query={current} />
        )}
      </Card>
      <section className="flex flex-col gap-3">
        <h2 className="text-lg font-semibold">{t('shifts.history')}</h2>
        {shifts.isSuccess ? (
          <DataTable caption={t('shifts.history')} columns={columns} rows={shifts.data} rowKey={(s) => s.id} empty={t('shifts.none')} />
        ) : (
          <QueryStatus query={shifts} />
        )}
      </section>
      {opening && registers.data && <OpenShiftDialog tenantId={tenantId} registers={registers.data} onClose={() => setOpening(false)} />}
    </div>
  )
}
