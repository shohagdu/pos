import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'

import { type Customer, type PartyFilter, partiesApi, useCustomers } from '../../api/parties'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { Checkbox } from '../../components/ui/Checkbox'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { PageHeader } from '../../components/ui/PageHeader'
import { Select } from '../../components/ui/Select'
import { StatCard } from '../../components/ui/StatCard'
import { TextField } from '../../components/ui/TextField'
import { downloadCsv } from '../../lib/csv'
import { formatMoney } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { LinkButton, LoadMore, QueryStatus, TextLink } from '../money/shared'
import { fetchAll, stamp, useTenant } from '../money/utils'
import { Balance } from './Balance'

const STATUSES = ['active', 'inactive', 'all'] as const

/** Customers (`/customers`): search, dues and the customer list. */
export function CustomersPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const navigate = useNavigate()
  const { tenantId, can } = useTenant()
  const [text, setText] = useState('')
  const [filter, setFilter] = useState<PartyFilter>({ status: 'active' })
  const [exportError, setExportError] = useState<unknown>(null)
  const customers = useCustomers(tenantId, filter)
  const rows = customers.data?.pages.flatMap((p) => p.data) ?? []
  const totalDue = customers.data?.pages[0]?.total_due

  const set = (patch: Partial<PartyFilter>) => setFilter((f) => ({ ...f, ...patch }))
  function onSearch(e: FormEvent) {
    e.preventDefault()
    set({ q: text.trim() || undefined })
  }

  const columns: Column<Customer>[] = [
    { key: 'code', header: t('customers.code'), render: (c) => <span className="font-mono text-xs">{c.code}</span> },
    { key: 'name', header: t('customers.name'), render: (c) => <TextLink to={`/customers/${c.id}`}>{c.name}</TextLink> },
    { key: 'phone', header: t('customers.phone'), render: (c) => c.phone ?? '—' },
    { key: 'balance', header: t('customers.balance'), align: 'right', render: (c) => <Balance value={c.current_balance} /> },
    {
      key: 'limit',
      header: t('customers.creditLimit'),
      align: 'right',
      render: (c) => (c.credit_limit === null ? t('customers.noLimit') : formatMoney(c.credit_limit, lang)),
    },
    {
      key: 'status',
      header: t('common.status'),
      render: (c) => <Badge tone={c.status === 'active' ? 'good' : 'neutral'}>{t(`customers.statuses.${c.status}`)}</Badge>,
    },
  ]

  async function exportCsv() {
    setExportError(null)
    try {
      const all = await fetchAll((cursor) => partiesApi.customers(filter, cursor))
      downloadCsv(`customers-${stamp()}`, all, [
        { header: t('customers.code'), value: (c) => c.code },
        { header: t('customers.name'), value: (c) => c.name },
        { header: t('customers.phone'), value: (c) => c.phone },
        { header: t('customers.email'), value: (c) => c.email },
        { header: t('customers.address'), value: (c) => c.address },
        { header: t('customers.balance'), value: (c) => c.current_balance },
        { header: t('customers.creditLimit'), value: (c) => c.credit_limit },
        { header: t('common.status'), value: (c) => t(`customers.statuses.${c.status}`) },
      ])
    } catch (err) {
      setExportError(err)
    }
  }

  return (
    <div className="flex flex-col gap-6">
      <PageHeader
        title={t('customers.title')}
        subtitle={t('customers.subtitle')}
        actions={
          can('customers.manage') && (
            <LinkButton to="/customers/new" variant="primary">
              {t('customers.new')}
            </LinkButton>
          )
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
            placeholder={t('customers.searchHint')}
          />
          <Button type="submit">{t('common.search')}</Button>
        </form>
        <div className="flex flex-wrap items-end gap-4">
          <Select
            label={t('common.status')}
            value={filter.status ?? 'active'}
            onChange={(e) => set({ status: e.target.value as PartyFilter['status'] })}
            options={STATUSES.map((s) => ({ value: s, label: t(`customers.statuses.${s}`) }))}
          />
          <Checkbox className="pb-3" label={t('customers.dueOnly')} checked={filter.due_only ?? false} onChange={(e) => set({ due_only: e.target.checked || undefined })} />
          <Button variant="secondary" className="ms-auto" disabled={rows.length === 0} onClick={() => void exportCsv()}>
            {t('common.exportCsv')}
          </Button>
        </div>
      </div>
      {totalDue !== undefined && (
        <div className="grid gap-3 sm:grid-cols-3">
          <StatCard label={t('customers.totalDue')} value={formatMoney(totalDue, lang)} tone="bad" hint={t('customers.totalDueHint')} />
        </div>
      )}
      {exportError !== null && <Alert tone="danger">{problemMessage(exportError, t)}</Alert>}
      {customers.isSuccess ? (
        <>
          <DataTable
            caption={t('customers.title')}
            columns={columns}
            rows={rows}
            rowKey={(c) => c.id}
            onRowClick={(c) => navigate(`/customers/${c.id}`)}
            empty={filter.q || filter.due_only ? t('common.noResults') : t('customers.none')}
          />
          <LoadMore query={customers} />
        </>
      ) : (
        <QueryStatus query={customers} />
      )}
    </div>
  )
}
