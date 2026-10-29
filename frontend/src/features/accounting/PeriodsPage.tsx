import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { type FiscalPeriod, accountingApi, accountingKeys, usePeriods } from '../../api/accounting'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { ConfirmButton } from '../../components/ui/ConfirmButton'
import { DataTable } from '../../components/ui/DataTable'
import { PageHeader } from '../../components/ui/PageHeader'
import { Spinner } from '../../components/ui/Spinner'
import { TextField } from '../../components/ui/TextField'
import { formatDate, formatDateTime, isoDate, today, dateLocale } from '../../lib/dates'
import { problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { AccountingNav } from './AccountingNav'

/** "2026-09" for the month before today. */
function lastMonth(): string {
  const d = new Date()
  d.setDate(1)
  d.setMonth(d.getMonth() - 1)
  return isoDate(d).slice(0, 7)
}

/** Fiscal periods (ACC-R07): lock ended months, reopen locked ones. */
export function PeriodsPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const queryClient = useQueryClient()
  const allowed = can('accounting.view')
  const canLock = can('accounting.period_lock')
  const periods = usePeriods(tenantId)
  const [month, setMonth] = useState(lastMonth())
  const [done, setDone] = useState('')

  const onSuccess = (message: string) => (p: FiscalPeriod) => {
    setDone(t(message, { month: formatMonth(p.start_date.slice(0, 7)) }))
    void queryClient.invalidateQueries({ queryKey: accountingKeys.all(tenantId) })
    void queryClient.invalidateQueries({ queryKey: ['reports', tenantId] })
  }
  const lock = useMutation({ mutationFn: (m: string) => accountingApi.lockPeriod(m), onSuccess: onSuccess('accounting.periods.locked') })
  const reopen = useMutation({ mutationFn: (m: string) => accountingApi.reopenPeriod(m), onSuccess: onSuccess('accounting.periods.reopened') })
  const busy = lock.isPending || reopen.isPending
  const error = lock.error ?? reopen.error

  function formatMonth(m: string) {
    return new Intl.DateTimeFormat(dateLocale(lang), { month: 'long', year: 'numeric' }).format(new Date(`${m}-01T00:00:00`))
  }
  const run = (action: 'lock' | 'reopen', m: string) => {
    setDone('')
    lock.reset()
    reopen.reset()
    if (action === 'lock') lock.mutate(m)
    else reopen.mutate(m)
  }

  return (
    <div className="flex max-w-5xl flex-col gap-6">
      <PageHeader title={t('accounting.periods.title')} subtitle={t('accounting.periods.subtitle')} />
      <AccountingNav />
      {!allowed ? (
        <Alert tone="warning">{t('reports.noAccess')}</Alert>
      ) : (
        <>
          {canLock && (
            <Card className="flex flex-col gap-3 p-4">
              <h2 className="font-semibold">{t('accounting.periods.lockTitle')}</h2>
              <div className="flex flex-wrap items-end gap-2">
                <TextField label={t('accounting.periods.month')} type="month" value={month} max={lastMonth()} onChange={(e) => setMonth(e.target.value)} />
                <Button disabled={!month || busy} onClick={() => run('lock', month)}>
                  {t('accounting.periods.lock')}
                </Button>
                <ConfirmButton disabled={!month || busy} onConfirm={() => run('reopen', month)}>
                  {t('accounting.periods.reopen')}
                </ConfirmButton>
              </div>
              <p className="text-xs text-ink-muted">{t('accounting.periods.help')}</p>
              {done && <Alert tone="success">{done}</Alert>}
              {error && <Alert tone="danger">{problemMessage(error, t)}</Alert>}
            </Card>
          )}
          {periods.isPending ? (
            <Spinner />
          ) : periods.isError ? (
            <Alert tone="danger">{problemMessage(periods.error, t)}</Alert>
          ) : (
            <DataTable<FiscalPeriod>
              caption={t('accounting.periods.title')}
              rows={periods.data}
              rowKey={(p) => p.id}
              empty={t('accounting.periods.none')}
              columns={[
                { key: 'month', header: t('accounting.periods.month'), render: (p) => <span className="font-medium">{formatMonth(p.start_date.slice(0, 7))}</span> },
                { key: 'start', header: t('accounting.periods.start'), render: (p) => formatDate(p.start_date, lang) },
                { key: 'end', header: t('accounting.periods.end'), render: (p) => formatDate(p.end_date, lang) },
                {
                  key: 'status',
                  header: t('common.status'),
                  render: (p) => <Badge tone={p.status === 'locked' ? 'warn' : 'good'}>{t(`accounting.periods.statuses.${p.status}`)}</Badge>,
                },
                { key: 'lockedAt', header: t('accounting.periods.lockedAt'), render: (p) => (p.locked_at ? formatDateTime(p.locked_at, lang) : '—') },
                ...(canLock
                  ? [
                      {
                        key: 'action',
                        header: <span className="sr-only">{t('common.actions')}</span>,
                        align: 'right' as const,
                        render: (p: FiscalPeriod) =>
                          p.status === 'locked' ? (
                            <ConfirmButton disabled={busy} onConfirm={() => run('reopen', p.start_date.slice(0, 7))}>
                              {t('accounting.periods.reopen')}
                            </ConfirmButton>
                          ) : p.end_date < today() ? (
                            <Button variant="ghost" disabled={busy} onClick={() => run('lock', p.start_date.slice(0, 7))}>
                              {t('accounting.periods.lock')}
                            </Button>
                          ) : null,
                      },
                    ]
                  : []),
              ]}
            />
          )}
        </>
      )}
    </div>
  )
}
