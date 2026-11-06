import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { type WarrantyFilter, useWarranties } from '../../api/sales'
import { Button } from '../../components/ui/Button'
import { PageHeader } from '../../components/ui/PageHeader'
import { Select } from '../../components/ui/Select'
import { TextField } from '../../components/ui/TextField'
import { useSession } from '../auth/session'
import { LoadMore, QueryStatus } from '../money/shared'
import { WarrantyTable } from './WarrantyTable'

type Status = NonNullable<WarrantyFilter['status']>

/** All / active / expired. */
export function WarrantyStatusFilter({ value, onChange }: { value: Status; onChange: (status: Status) => void }) {
  const { t } = useTranslation()
  return (
    <Select
      className="sm:max-w-xs"
      label={t('common.status')}
      value={value}
      onChange={(e) => onChange(e.target.value as Status)}
      options={[
        { value: '', label: t('common.all') },
        { value: 'active', label: t('sales.warrantyTable.active') },
        { value: 'expired', label: t('sales.warrantyTable.expired') },
      ]}
    />
  )
}

/** Warranty lookup for claims: find a sold item by invoice, phone, product, SKU or barcode. */
export function WarrantyLookupPage() {
  const { t } = useTranslation()
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const [text, setText] = useState('')
  const [filter, setFilter] = useState<WarrantyFilter>({})
  const warranties = useWarranties(tenantId, filter)
  const pages = warranties.data?.pages ?? []

  function onSearch(e: FormEvent) {
    e.preventDefault()
    setFilter((f) => ({ ...f, q: text.trim() || undefined }))
  }

  return (
    <div className="flex max-w-6xl flex-col gap-6">
      <PageHeader
        title={t('sales.warrantyLookup.title')}
        subtitle={can('sales.view_all_users') ? t('sales.warrantyLookup.subtitle') : t('sales.ownOnly')}
        actions={
          <Link
            to="/sales"
            className="inline-flex min-h-11 items-center rounded-lg border border-border bg-surface px-4 text-sm font-semibold text-ink hover:bg-canvas"
          >
            {t('sales.title')}
          </Link>
        }
      />
      <div className="flex flex-col gap-3 sm:flex-row sm:items-end">
        <form onSubmit={onSearch} role="search" className="flex flex-1 items-end gap-2">
          <TextField
            className="flex-1"
            label={t('common.search')}
            type="search"
            autoFocus
            value={text}
            placeholder={t('sales.warrantyLookup.searchHint')}
            onChange={(e) => setText(e.target.value)}
          />
          <Button type="submit">{t('common.search')}</Button>
        </form>
        <WarrantyStatusFilter value={filter.status ?? ''} onChange={(status) => setFilter((f) => ({ ...f, status }))} />
      </div>
      {warranties.isSuccess ? (
        <>
          <WarrantyTable
            showCustomer
            rows={pages.flatMap((p) => p.data)}
            today={pages[0]?.today ?? ''}
            caption={t('sales.warrantyLookup.title')}
            empty={filter.q ? t('sales.warrantyLookup.noMatch', { q: filter.q }) : t('sales.warrantyLookup.none')}
          />
          <LoadMore query={warranties} />
        </>
      ) : (
        <QueryStatus query={warranties} />
      )}
    </div>
  )
}
