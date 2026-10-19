import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { type Expense, type ExpenseFilter, moneyApi, moneyKeys, useExpenseCategories, useExpenses, usePaymentAccounts } from '../../api/money'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { DateRangeFilter } from '../../components/ui/DateRangeFilter'
import { PageHeader } from '../../components/ui/PageHeader'
import { Select } from '../../components/ui/Select'
import { StatCard } from '../../components/ui/StatCard'
import { TextField } from '../../components/ui/TextField'
import { downloadCsv } from '../../lib/csv'
import { dec } from '../../lib/decimal'
import { formatDate, startOfMonth, today } from '../../lib/dates'
import { formatMoney } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { ReasonDialog } from '../money/ReasonDialog'
import { LinkButton, LoadMore, QueryStatus } from '../money/shared'
import { fetchAll, stamp, useTenant } from '../money/utils'
import { ExpenseDialog } from './ExpenseDialog'

const STATUSES = ['posted', 'voided', 'all'] as const

/** Expenses (`/expenses`): filters, total, list, new expense and void. */
export function ExpensesPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const queryClient = useQueryClient()
  const { tenantId, can } = useTenant()
  const [text, setText] = useState('')
  const [filter, setFilter] = useState<ExpenseFilter>({ status: 'posted', from: startOfMonth(), to: today() })
  const [creating, setCreating] = useState(false)
  const [voiding, setVoiding] = useState<Expense | null>(null)
  const [exportError, setExportError] = useState<unknown>(null)
  const expenses = useExpenses(tenantId, filter)
  const categories = useExpenseCategories(tenantId)
  const accounts = usePaymentAccounts(tenantId)
  const rows = expenses.data?.pages.flatMap((p) => p.data) ?? []
  const total = expenses.data?.pages[0]?.total_amount
  const canVoid = can('expenses.approve') || can('payments.void')
  const set = (patch: Partial<ExpenseFilter>) => setFilter((f) => ({ ...f, ...patch }))

  const voidExpense = useMutation({
    mutationFn: ({ id, reason }: { id: string; reason: string }) => moneyApi.voidExpense(id, reason),
    onSuccess: async () => {
      setVoiding(null)
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: moneyKeys.all(tenantId) }),
        queryClient.invalidateQueries({ queryKey: ['reports', tenantId] }),
      ])
    },
  })

  function onSearch(e: FormEvent) {
    e.preventDefault()
    set({ q: text.trim() || undefined })
  }

  const columns: Column<Expense>[] = [
    { key: 'no', header: t('expenses.no'), render: (x) => <span className="font-medium">{x.expense_no}</span> },
    { key: 'date', header: t('common.date'), render: (x) => <span className="whitespace-nowrap">{formatDate(x.business_date, lang)}</span> },
    { key: 'category', header: t('expenses.category'), render: (x) => x.category_name },
    {
      key: 'payee',
      header: t('expenses.payee'),
      render: (x) => (
        <div className="flex flex-col">
          <span>{x.payee ?? '—'}</span>
          {x.note && <span className="text-xs text-ink-muted">{x.note}</span>}
        </div>
      ),
    },
    { key: 'account', header: t('expenses.account'), render: (x) => x.account_name },
    {
      key: 'amount',
      header: t('common.amount'),
      align: 'right',
      render: (x) => <span className={x.status === 'voided' ? 'text-ink-muted line-through' : ''}>{formatMoney(x.amount, lang)}</span>,
    },
    { key: 'vat', header: t('expenses.vat'), align: 'right', render: (x) => (dec(x.tax_amount) === 0n ? '' : formatMoney(x.tax_amount, lang)) },
    {
      key: 'status',
      header: t('common.status'),
      render: (x) => (
        <span title={x.void_reason ?? undefined}>
          <Badge tone={x.status === 'posted' ? 'good' : 'bad'}>{t(`expenses.statuses.${x.status}`)}</Badge>
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
                  aria-label={t('expenses.voidNo', { no: x.expense_no })}
                  onClick={() => {
                    voidExpense.reset()
                    setVoiding(x)
                  }}
                >
                  {t('expenses.void')}
                </Button>
              ) : null,
          },
        ] satisfies Column<Expense>[])
      : []),
  ]

  async function exportCsv() {
    setExportError(null)
    try {
      const all = await fetchAll((cursor) => moneyApi.expenses(filter, cursor))
      downloadCsv(`expenses-${stamp()}`, all, [
        { header: t('expenses.no'), value: (x) => x.expense_no },
        { header: t('common.date'), value: (x) => x.business_date },
        { header: t('expenses.category'), value: (x) => x.category_name },
        { header: t('expenses.payee'), value: (x) => x.payee },
        { header: t('expenses.account'), value: (x) => x.account_name },
        { header: t('common.amount'), value: (x) => x.amount },
        { header: t('expenses.vat'), value: (x) => x.tax_amount },
        { header: t('expenses.reference'), value: (x) => x.reference },
        { header: t('common.note'), value: (x) => x.note },
        { header: t('common.status'), value: (x) => t(`expenses.statuses.${x.status}`) },
      ])
    } catch (err) {
      setExportError(err)
    }
  }

  return (
    <div className="flex flex-col gap-6">
      <PageHeader
        title={t('expenses.title')}
        subtitle={t('expenses.subtitle')}
        actions={
          <>
            <LinkButton to="/expenses/categories">{t('expenses.categories')}</LinkButton>
            {can('expenses.create') && <Button onClick={() => setCreating(true)}>{t('expenses.new')}</Button>}
          </>
        }
      />
      <div className="flex flex-col gap-3 print:hidden">
        <form onSubmit={onSearch} className="flex flex-wrap items-end gap-2" role="search">
          <TextField
            className="min-w-60 flex-1"
            label={t('common.search')}
            type="search"
            value={text}
            onChange={(e) => setText(e.target.value)}
            placeholder={t('expenses.searchHint')}
          />
          <Button type="submit">{t('common.search')}</Button>
        </form>
        <div className="flex flex-wrap items-end gap-3">
          <DateRangeFilter from={filter.from ?? ''} to={filter.to ?? ''} onChange={(r) => set({ from: r.from || undefined, to: r.to || undefined })} />
          <Select
            label={t('expenses.category')}
            value={filter.category_id ?? ''}
            onChange={(e) => set({ category_id: e.target.value || undefined })}
            options={[{ value: '', label: t('expenses.allCategories') }, ...(categories.data ?? []).map((c) => ({ value: c.id, label: c.name }))]}
          />
          {accounts.isSuccess && (
            <Select
              label={t('expenses.account')}
              value={filter.account_id ?? ''}
              onChange={(e) => set({ account_id: e.target.value || undefined })}
              options={[{ value: '', label: t('expenses.allAccounts') }, ...accounts.data.map((a) => ({ value: a.id, label: a.name }))]}
            />
          )}
          <Select
            label={t('common.status')}
            value={filter.status ?? 'posted'}
            onChange={(e) => set({ status: e.target.value as ExpenseFilter['status'] })}
            options={STATUSES.map((s) => ({ value: s, label: t(`expenses.statuses.${s}`) }))}
          />
          <Button variant="secondary" className="ms-auto" disabled={rows.length === 0} onClick={() => void exportCsv()}>
            {t('common.exportCsv')}
          </Button>
        </div>
      </div>
      {total !== undefined && (
        <div className="grid gap-3 sm:grid-cols-3">
          <StatCard label={t('expenses.total')} value={formatMoney(total, lang)} hint={t('expenses.totalHint')} />
        </div>
      )}
      {exportError !== null && <Alert tone="danger">{problemMessage(exportError, t)}</Alert>}
      {expenses.isSuccess ? (
        <>
          <DataTable caption={t('expenses.title')} columns={columns} rows={rows} rowKey={(x) => x.id} empty={t('expenses.none')} />
          <LoadMore query={expenses} />
        </>
      ) : (
        <QueryStatus query={expenses} />
      )}
      {creating && <ExpenseDialog tenantId={tenantId} categories={categories.data ?? []} accounts={accounts.data ?? []} onClose={() => setCreating(false)} />}
      {voiding && (
        <ReasonDialog
          title={t('expenses.voidNo', { no: voiding.expense_no })}
          description={t('expenses.voidHint', { amount: formatMoney(voiding.amount, lang) })}
          confirmLabel={t('expenses.void')}
          pending={voidExpense.isPending}
          error={voidExpense.error}
          onConfirm={(reason) => voidExpense.mutate({ id: voiding.id, reason })}
          onClose={() => setVoiding(null)}
        />
      )}
    </div>
  )
}
