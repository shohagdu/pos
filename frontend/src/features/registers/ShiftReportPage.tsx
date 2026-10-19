import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useParams } from 'react-router'

import { type ShiftReport, registerKeys, registersApi, useShiftReport } from '../../api/registers'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { DataTable } from '../../components/ui/DataTable'
import { PageHeader } from '../../components/ui/PageHeader'
import { StatCard } from '../../components/ui/StatCard'
import { TextField } from '../../components/ui/TextField'
import { formatDateTime } from '../../lib/dates'
import { dec, toFixed, tryParse } from '../../lib/decimal'
import { formatMoney } from '../../lib/format'
import { optional } from '../../lib/form'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { QueryStatus } from '../money/shared'
import { useTenant } from '../money/utils'
import { ShiftDifference } from './ShiftDifference'

/** X report of an open shift, Z report of a closed one (`/shifts/:id`); the cashier closes the shift here. */
export function ShiftReportPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { id = '' } = useParams()
  const { tenantId, me, can } = useTenant()
  const report = useShiftReport(tenantId, id)
  const r = report.data
  if (!r) return <QueryStatus query={report} />

  const s = r.shift
  const mayClose = s.status === 'open' && (s.opened_by === me?.user.id || can('sales.view_all_users'))
  const money = (v: string) => formatMoney(v, lang)
  const rows: [string, string][] = [
    [t('shifts.openingCash'), money(s.opening_cash)],
    [t('shifts.cashIn'), money(r.cash_in)],
    [t('shifts.cashOut'), money(r.cash_out)],
    [t('shifts.expectedCash'), money(r.expected_cash)],
  ]

  return (
    <div className="flex max-w-4xl flex-col gap-6">
      <Link to="/shifts" className="text-sm font-medium text-brand-700 hover:underline print:hidden">
        ← {t('shifts.title')}
      </Link>
      <PageHeader
        title={`${r.kind === 'Z' ? t('shifts.zReport') : t('shifts.xReport')} · ${s.register_name} · ${t('shifts.shiftNo', { no: s.shift_no })}`}
        subtitle={r.kind === 'Z' ? t('shifts.zHint') : t('shifts.xHint')}
        actions={
          <Button variant="secondary" className="print:hidden" onClick={() => window.print()}>
            {t('common.print')}
          </Button>
        }
      />
      <Card>
        <dl className="grid gap-4 text-sm sm:grid-cols-3">
          <div>
            <dt className="text-ink-muted">{t('shifts.cashier')}</dt>
            <dd className="font-medium">{s.opened_by_name ?? '—'}</dd>
          </div>
          <div>
            <dt className="text-ink-muted">{t('shifts.openedAt')}</dt>
            <dd className="font-medium">{formatDateTime(s.opened_at, lang)}</dd>
          </div>
          <div>
            <dt className="text-ink-muted">{t('common.status')}</dt>
            <dd>
              <Badge tone={s.status === 'open' ? 'info' : 'neutral'}>{t(`shifts.statuses.${s.status}`)}</Badge>
            </dd>
          </div>
          {s.closed_at && (
            <div>
              <dt className="text-ink-muted">{t('shifts.closedAt')}</dt>
              <dd className="font-medium">
                {formatDateTime(s.closed_at, lang)}
                {s.closed_by_name ? ` · ${s.closed_by_name}` : ''}
              </dd>
            </div>
          )}
          {s.note && (
            <div className="sm:col-span-2">
              <dt className="text-ink-muted">{t('common.note')}</dt>
              <dd className="whitespace-pre-line">{s.note}</dd>
            </div>
          )}
        </dl>
      </Card>
      <div className="grid gap-3 sm:grid-cols-3">
        <StatCard label={t('shifts.sales')} value={money(r.sales_total)} hint={t('shifts.invoiceCount', { count: r.sales_count })} />
        <StatCard label={t('shifts.returns')} value={money(r.returns_total)} hint={t('shifts.returnCount', { count: r.returns_count })} />
        <StatCard label={t('shifts.expectedCash')} value={money(r.expected_cash)} />
        <StatCard label={t('shifts.discount')} value={money(r.discount_total)} />
        <StatCard label={t('shifts.vat')} value={money(r.tax_total)} />
        <StatCard label={t('shifts.due')} value={money(r.due_total)} hint={t('shifts.cancelledCount', { count: r.cancelled_count })} />
      </div>
      <section className="flex flex-col gap-3">
        <h2 className="text-lg font-semibold">{t('shifts.byMethod')}</h2>
        <DataTable
          caption={t('shifts.byMethod')}
          rows={r.payments}
          rowKey={(p) => p.method_name}
          empty={t('shifts.noPayments')}
          columns={[
            { key: 'method', header: t('shifts.method'), render: (p) => p.method_name },
            { key: 'count', header: t('shifts.count'), align: 'right', render: (p) => p.count },
            { key: 'in', header: t('shifts.moneyIn'), align: 'right', render: (p) => money(p.money_in) },
            { key: 'out', header: t('shifts.moneyOut'), align: 'right', render: (p) => money(p.money_out) },
          ]}
        />
      </section>
      <section className="flex flex-col gap-3 print:break-inside-avoid">
        <h2 className="text-lg font-semibold">{t('shifts.drawer')}</h2>
        <Card>
          <dl className="flex flex-col gap-2 text-sm">
            {rows.map(([label, value]) => (
              <div key={label} className="flex justify-between gap-4">
                <dt className="text-ink-muted">{label}</dt>
                <dd className="font-medium tabular-nums">{value}</dd>
              </div>
            ))}
            {s.status === 'closed' && (
              <>
                <div className="flex justify-between gap-4 border-t border-border pt-2">
                  <dt className="text-ink-muted">{t('shifts.countedCash')}</dt>
                  <dd className="font-medium tabular-nums">{s.counted_cash === null ? '—' : money(s.counted_cash)}</dd>
                </div>
                <div className="flex justify-between gap-4">
                  <dt className="text-ink-muted">{t('shifts.difference')}</dt>
                  <dd className="tabular-nums">
                    <ShiftDifference value={s.difference} />
                  </dd>
                </div>
              </>
            )}
          </dl>
        </Card>
      </section>
      {mayClose && <CloseShiftForm tenantId={tenantId} report={r} />}
    </div>
  )
}

/** Counted cash and note; closing returns the Z report, shown in place. */
function CloseShiftForm({ tenantId, report }: { tenantId: string; report: ShiftReport }) {
  const { t, i18n } = useTranslation()
  const queryClient = useQueryClient()
  const [counted, setCounted] = useState('')
  const [note, setNote] = useState('')
  const [localError, setLocalError] = useState<string | undefined>()
  const id = report.shift.id

  const close = useMutation({
    mutationFn: () => registersApi.closeShift(id, counted.trim(), optional(note)),
    onSuccess: async (z) => {
      queryClient.setQueryData(registerKeys.report(tenantId, id), z)
      queryClient.setQueryData(registerKeys.current(tenantId), null)
      await queryClient.invalidateQueries({ queryKey: registerKeys.all(tenantId), predicate: (q) => q.queryKey[2] !== 'report' })
    },
  })

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    const value = tryParse(counted)
    const bad = value === null || value < 0n || !counted.trim()
    setLocalError(bad ? t('shifts.errors.cash') : undefined)
    if (!bad) close.mutate()
  }

  const errors = fieldErrors(close.error, t)
  const value = tryParse(counted)
  const preview = value === null || !counted.trim() ? null : toFixed(value - dec(report.expected_cash), 2)
  return (
    <Card className="print:hidden">
      <form onSubmit={onSubmit} noValidate className="flex flex-col gap-4">
        <h2 className="text-lg font-semibold">{t('shifts.close')}</h2>
        <p className="text-sm text-ink-muted">{t('shifts.closeHint', { amount: formatMoney(report.expected_cash, i18n.language) })}</p>
        {close.isError && hasFormError(close.error) && <Alert tone="danger">{problemMessage(close.error, t)}</Alert>}
        <div className="grid gap-4 sm:grid-cols-2">
          <TextField
            label={t('shifts.countedCash')}
            value={counted}
            onChange={(e) => setCounted(e.target.value)}
            inputMode="decimal"
            required
            error={localError ?? errors.counted_cash}
          />
          <TextField label={`${t('common.note')} (${t('common.optional')})`} value={note} onChange={(e) => setNote(e.target.value)} maxLength={500} error={errors.note} />
        </div>
        {preview !== null && (
          <p className="text-sm" aria-live="polite">
            {t('shifts.difference')}: <ShiftDifference value={preview} />
          </p>
        )}
        <Button type="submit" disabled={close.isPending} className="sm:self-end">
          {close.isPending ? t('common.saving') : t('shifts.closeAndZ')}
        </Button>
      </form>
    </Card>
  )
}
