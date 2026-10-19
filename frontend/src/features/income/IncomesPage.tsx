import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { type Income, type IncomeFilter, moneyApi, moneyKeys, useIncomeCategories, useIncomes, usePaymentAccounts } from '../../api/money'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { DateRangeFilter } from '../../components/ui/DateRangeFilter'
import { PageHeader } from '../../components/ui/PageHeader'
import { Select } from '../../components/ui/Select'
import { StatCard } from '../../components/ui/StatCard'
import { downloadCsv } from '../../lib/csv'
import { formatDate, startOfMonth, today } from '../../lib/dates'
import { formatMoney } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { ReasonDialog } from '../money/ReasonDialog'
import { LinkButton, LoadMore, QueryStatus } from '../money/shared'
import { fetchAll, stamp, useTenant } from '../money/utils'
import { IncomeDialog } from './IncomeDialog'

const STATUSES = ['posted', 'voided', 'all'] as const

/** Other income (`/incomes`): filters, total, list, record and void. */
export function IncomesPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const queryClient = useQueryClient()
  const { tenantId, can } = useTenant()
  const [filter, setFilter] = useState<IncomeFilter>({ status: 'posted', from: startOfMonth(), to: today() })
  const [creating, setCreating] = useState(false)
  const [voiding, setVoiding] = useState<Income | null>(null)
  const [exportError, setExportError] = useState<unknown>(null)
  const incomes = useIncomes(tenantId, filter)
  const categories = useIncomeCategories(tenantId)
  const accounts = usePaymentAccounts(tenantId)
  const rows = incomes.data?.pages.flatMap((p) => p.data) ?? []
  const total = incomes.data?.pages[0]?.total_amount
  const canVoid = can('payments.void')
  const set = (patch: Partial<IncomeFilter>) => setFilter((f) => ({ ...f, ...patch }))

  const voidIncome = useMutation({
    mutationFn: ({ id, reason }: { id: string; reason: string }) => moneyApi.voidIncome(id, reason),
    onSuccess: async () => {
      setVoiding(null)
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: moneyKeys.all(tenantId) }),
        queryClient.invalidateQueries({ queryKey: ['reports', tenantId] }),
      ])
    },
  })

  const columns: Column<Income>[] = [
    { key: 'no', header: t('income.no'), render: (x) => <span className="font-medium">{x.income_no}</span> },
    { key: 'date', header: t('common.date'), render: (x) => <span className="whitespace-nowrap">{formatDate(x.business_date, lang)}</span> },
    { key: 'category', header: t('income.category'), render: (x) => x.category_name },
    {
      key: 'source',
      header: t('income.source'),
      render: (x) => (
        <div className="flex flex-col">
          <span>{x.source ?? '—'}</span>
          {x.note && <span className="text-xs text-ink-muted">{x.note}</span>}
        </div>
      ),
    },
    { key: 'account', header: t('income.account'), render: (x) => x.account_name },
    {
      key: 'amount',
      header: t('common.amount'),
      align: 'right',
      render: (x) => <span className={x.status === 'voided' ? 'text-ink-muted line-through' : ''}>{formatMoney(x.amount, lang)}</span>,
    },
    {
      key: 'status',
      header: t('common.status'),
      render: (x) => (
        <span title={x.void_reason ?? undefined}>
          <Badge tone={x.status === 'posted' ? 'good' : 'bad'}>{t(`income.statuses.${x.status}`)}</Badge>
        </span>
      ),
    },
    ...(canVoid
      ? ([
          {
            key: 'actions',
            header: <span className="sr-only">{t('common.actions')}</span>,
            render: (x) =>
              x.status === 'posted' ? (
                <Button
                  variant="ghost"
                  className="min-h-9 text-danger print:hidden"
                  aria-label={t('income.voidNo', { no: x.income_no })}
                  onClick={() => {
                    voidIncome.reset()
                    setVoiding(x)
                  }}
                >
                  {t('income.void')}
                </Button>
              ) : null,
          },
        ] satisfies Column<Income>[])
      : []),
  ]

  async function exportCsv() {
    setExportError(null)
    try {
      const all = await fetchAll((cursor) => moneyApi.incomes(filter, cursor))
      downloadCsv(`income-${stamp()}`, all, [
        { header: t('income.no'), value: (x) => x.income_no },
        { header: t('common.date'), value: (x) => x.business_date },
        { header: t('income.category'), value: (x) => x.category_name },
        { header: t('income.source'), value: (x) => x.source },
        { header: t('income.account'), value: (x) => x.account_name },
        { header: t('common.amount'), value: (x) => x.amount },
        { header: t('income.reference'), value: (x) => x.reference },
        { header: t('common.note'), value: (x) => x.note },
        { header: t('common.status'), value: (x) => t(`income.statuses.${x.status}`) },
      ])
    } catch (err) {
      setExportError(err)
    }
  }

  return (
    <div className="flex flex-col gap-6">
      <PageHeader
        title={t('income.title')}
        subtitle={t('income.subtitle')}
        actions={
          <>
            <LinkButton to="/incomes/categories">{t('income.categories')}</LinkButton>
            {can('income.create') && <Button onClick={() => setCreating(true)}>{t('income.new')}</Button>}
          </>
        }
      />
      <div className="flex flex-wrap items-end gap-3 print:hidden">
        <DateRangeFilter from={filter.from ?? ''} to={filter.to ?? ''} onChange={(r) => set({ from: r.from || undefined, to: r.to || undefined })} />
        <Select
          label={t('income.category')}
          value={filter.category_id ?? ''}
          onChange={(e) => set({ category_id: e.target.value || undefined })}
          options={[{ value: '', label: t('income.allCategories') }, ...(categories.data ?? []).map((c) => ({ value: c.id, label: c.name }))]}
        />
        <Select
          label={t('common.status')}
          value={filter.status ?? 'posted'}
          onChange={(e) => set({ status: e.target.value as IncomeFilter['status'] })}
          options={STATUSES.map((s) => ({ value: s, label: t(`income.statuses.${s}`) }))}
        />
        <Button variant="secondary" className="ms-auto" disabled={rows.length === 0} onClick={() => void exportCsv()}>
          {t('common.exportCsv')}
        </Button>
      </div>
      {total !== undefined && (
        <div className="grid gap-3 sm:grid-cols-3">
          <StatCard label={t('income.total')} value={formatMoney(total, lang)} hint={t('income.totalHint')} tone="good" />
        </div>
      )}
      {exportError !== null && <Alert tone="danger">{problemMessage(exportError, t)}</Alert>}
      {incomes.isSuccess ? (
        <>
          <DataTable caption={t('income.title')} columns={columns} rows={rows} rowKey={(x) => x.id} empty={t('income.none')} />
          <LoadMore query={incomes} />
        </>
      ) : (
        <QueryStatus query={incomes} />
      )}
      {creating && <IncomeDialog tenantId={tenantId} categories={categories.data ?? []} accounts={accounts.data ?? []} onClose={() => setCreating(false)} />}
      {voiding && (
        <ReasonDialog
          title={t('income.voidNo', { no: voiding.income_no })}
          description={t('income.voidHint', { amount: formatMoney(voiding.amount, lang) })}
          confirmLabel={t('income.void')}
          pending={voidIncome.isPending}
          error={voidIncome.error}
          onConfirm={(reason) => voidIncome.mutate({ id: voiding.id, reason })}
          onClose={() => setVoiding(null)}
        />
      )}
    </div>
  )
}
