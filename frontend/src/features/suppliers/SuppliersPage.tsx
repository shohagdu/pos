import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate } from 'react-router'

import { type PartyFilter, type Supplier, useSuppliers } from '../../api/parties'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Checkbox } from '../../components/ui/Checkbox'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { PageHeader } from '../../components/ui/PageHeader'
import { Select } from '../../components/ui/Select'
import { Spinner } from '../../components/ui/Spinner'
import { StatCard } from '../../components/ui/StatCard'
import { TextField } from '../../components/ui/TextField'
import { downloadCsv } from '../../lib/csv'
import { formatMoney } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { BUTTON_LINK, LINK } from '../purchases/helpers'
import { BalanceBadge, StatusBadge } from './shared'

const STATUSES = ['active', 'inactive', 'all'] as const

/** `/suppliers`: search, filters, total payable and CSV. */
export function SuppliersPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const navigate = useNavigate()
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const [text, setText] = useState('')
  const [filter, setFilter] = useState<PartyFilter>({ status: 'active' })
  const suppliers = useSuppliers(tenantId, filter)
  const set = (patch: Partial<PartyFilter>) => setFilter((f) => ({ ...f, ...patch }))
  const rows = suppliers.data?.pages.flatMap((p) => p.data) ?? []
  const totalPayable = suppliers.data?.pages[0]?.total_payable

  function onSearch(e: FormEvent) {
    e.preventDefault()
    set({ q: text.trim() || undefined })
  }

  const columns: Column<Supplier>[] = [
    { key: 'code', header: t('suppliers.code'), render: (s) => <span className="font-mono text-xs">{s.code}</span> },
    {
      key: 'name',
      header: t('suppliers.name'),
      render: (s) => (
        <Link to={`/suppliers/${s.id}`} className={LINK}>
          {s.name}
        </Link>
      ),
    },
    { key: 'contact', header: t('suppliers.contactPerson'), render: (s) => s.contact_person ?? '—' },
    { key: 'phone', header: t('suppliers.phone'), render: (s) => s.phone ?? '—' },
    {
      key: 'balance',
      header: t('suppliers.balance'),
      align: 'right',
      render: (s) => (
        <span className="inline-flex items-center gap-2">
          {formatMoney(s.current_balance.replace(/^-/, ''), lang)}
          <BalanceBadge balance={s.current_balance} />
        </span>
      ),
    },
    { key: 'status', header: t('common.status'), render: (s) => <StatusBadge status={s.status} /> },
  ]

  return (
    <div className="flex max-w-6xl flex-col gap-6">
      <PageHeader
        title={t('suppliers.title')}
        actions={
          can('suppliers.manage') && (
            <Link to="/suppliers/new" className={BUTTON_LINK}>
              {t('suppliers.new')}
            </Link>
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
            placeholder={t('suppliers.searchHint')}
          />
          <Button type="submit">{t('common.search')}</Button>
        </form>
        <div className="flex flex-wrap items-end gap-4">
          <Select
            label={t('common.status')}
            value={filter.status ?? 'active'}
            onChange={(e) => set({ status: e.target.value as PartyFilter['status'] })}
            options={STATUSES.map((s) => ({ value: s, label: t(`suppliers.statuses.${s}`) }))}
          />
          <Checkbox label={t('suppliers.dueOnly')} checked={Boolean(filter.due_only)} onChange={(e) => set({ due_only: e.target.checked || undefined })} className="pb-3" />
          <Button
            variant="secondary"
            disabled={rows.length === 0}
            onClick={() =>
              downloadCsv('suppliers', rows, [
                { header: t('suppliers.code'), value: (s) => s.code },
                { header: t('suppliers.name'), value: (s) => s.name },
                { header: t('suppliers.contactPerson'), value: (s) => s.contact_person },
                { header: t('suppliers.phone'), value: (s) => s.phone },
                { header: t('suppliers.email'), value: (s) => s.email },
                { header: t('suppliers.balance'), value: (s) => s.current_balance },
                { header: t('common.status'), value: (s) => t(`suppliers.statuses.${s.status}`) },
              ])
            }
          >
            {t('common.exportCsv')}
          </Button>
        </div>
      </div>
      {totalPayable !== undefined && (
        <div className="grid gap-3 sm:grid-cols-3">
          <StatCard label={t('suppliers.totalPayable')} value={formatMoney(totalPayable, lang)} tone="bad" hint={t('suppliers.totalPayableHint')} />
        </div>
      )}
      {suppliers.isPending ? (
        <Spinner />
      ) : suppliers.isError ? (
        <Alert tone="danger">{problemMessage(suppliers.error, t)}</Alert>
      ) : (
        <DataTable
          caption={t('suppliers.title')}
          columns={columns}
          rows={rows}
          rowKey={(s) => s.id}
          onRowClick={(s) => navigate(`/suppliers/${s.id}`)}
          empty={filter.q || filter.due_only ? t('common.noResults') : t('suppliers.none')}
        />
      )}
      {suppliers.hasNextPage && (
        <Button variant="secondary" className="self-center print:hidden" disabled={suppliers.isFetchingNextPage} onClick={() => void suppliers.fetchNextPage()}>
          {suppliers.isFetchingNextPage ? t('common.loading') : t('common.loadMore')}
        </Button>
      )}
    </div>
  )
}
