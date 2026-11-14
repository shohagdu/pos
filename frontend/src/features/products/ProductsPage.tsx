import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { type ProductFilter, type ProductStatusFilter, categoryTree, useBrands, useCategories, useProducts } from '../../api/catalog'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { Select } from '../../components/ui/Select'
import { Spinner } from '../../components/ui/Spinner'
import { TextField } from '../../components/ui/TextField'
import { formatMoney } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { CatalogNav } from './CatalogNav'
import { ProductThumb } from './ProductThumb'
import { categoryLabel } from './labels'

const STATUSES: ProductStatusFilter[] = ['active', 'archived', 'all']

/** Products (E2.2): search, filters and "load more" paging. */
export function ProductsPage() {
  const { t, i18n } = useTranslation()
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const [text, setText] = useState('')
  const [filter, setFilter] = useState<ProductFilter>({ status: 'active' })
  const categories = useCategories(tenantId)
  const brands = useBrands(tenantId)
  const products = useProducts(tenantId, filter)

  const set = (patch: Partial<ProductFilter>) => setFilter((f) => ({ ...f, ...patch }))
  function onSearch(e: FormEvent) {
    e.preventDefault()
    set({ q: text.trim() || undefined })
  }

  const rows = products.data?.pages.flatMap((p) => p.data) ?? []
  const columns: Column<(typeof rows)[number]>[] = [
    {
      key: 'name',
      header: t('products.name'),
      render: (p) => (
        <div className="flex items-center gap-3">
          <ProductThumb fileId={p.image_file_id} size="md" />
          <div className="min-w-0">
            <Link to={`/products/${p.id}`} className="font-medium text-brand-700 hover:underline">
              {p.name}
            </Link>
            {p.name_bn && <span className="block text-xs text-ink-muted">{p.name_bn}</span>}
          </div>
        </div>
      ),
    },
    { key: 'sku', header: t('products.sku'), render: (p) => <span className="font-mono text-xs">{p.sku}</span> },
    { key: 'category', header: t('products.category'), render: (p) => p.category_name ?? '—' },
    {
      key: 'price',
      header: t('products.sellingPrice'),
      align: 'right',
      render: (p) => (
        <>
          {formatMoney(p.selling_price, i18n.language)}
          <span className="text-xs font-normal text-ink-muted"> / {p.unit_short_name}</span>
        </>
      ),
    },
    {
      key: 'status',
      header: t('products.status'),
      render: (p) => (
        <span className={`rounded px-2 py-0.5 text-xs font-semibold ${p.status === 'active' ? 'bg-brand-50 text-brand-700' : 'bg-canvas text-ink-muted'}`}>
          {t(`products.statuses.${p.status}`)}
        </span>
      ),
    },
    {
      key: 'actions',
      header: <span className="sr-only">{t('common.actions')}</span>,
      align: 'right',
      render: (p) => (
        <Link
          to={`/labels?variants=${p.variant_id}`}
          aria-label={t('labels.printBarcodeOf', { name: p.name })}
          className="inline-flex min-h-11 items-center text-sm font-medium whitespace-nowrap text-brand-700 hover:underline lg:min-h-0"
        >
          {t('labels.printBarcode')}
        </Link>
      ),
    },
  ]
  return (
    <div className="flex max-w-5xl flex-col gap-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h1 className="text-2xl font-bold">{t('products.title')}</h1>
        {can('products.create') && (
          <div className="flex flex-wrap gap-2">
            <Link
              to="/products/import"
              className="inline-flex min-h-11 items-center rounded-lg border border-border bg-surface px-4 text-sm font-semibold hover:bg-canvas"
            >
              {t('imports.products.nav')}
            </Link>
            <Link
              to="/products/new"
              className="inline-flex min-h-11 items-center rounded-lg bg-brand-600 px-4 text-sm font-semibold text-white hover:bg-brand-700"
            >
              {t('products.new')}
            </Link>
          </div>
        )}
      </div>
      <CatalogNav />
      <div className="flex flex-col gap-3">
        <form onSubmit={onSearch} className="flex flex-wrap items-end gap-2" role="search">
          <TextField
            className="min-w-0 flex-1"
            label={t('products.search')}
            type="search"
            value={text}
            onChange={(e) => setText(e.target.value)}
            placeholder={t('products.searchHint')}
          />
          <Button type="submit">{t('products.searchButton')}</Button>
        </form>
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
          <Select
            label={t('products.category')}
            value={filter.category_id ?? ''}
            onChange={(e) => set({ category_id: e.target.value || undefined })}
            options={[
              { value: '', label: t('products.allCategories') },
              ...categoryTree(categories.data ?? []).map((c) => ({ value: c.id, label: categoryLabel(c) })),
            ]}
          />
          <Select
            label={t('products.brand')}
            value={filter.brand_id ?? ''}
            onChange={(e) => set({ brand_id: e.target.value || undefined })}
            options={[{ value: '', label: t('products.allBrands') }, ...(brands.data ?? []).map((b) => ({ value: b.id, label: b.name }))]}
          />
          <Select
            label={t('products.statusFilter')}
            value={filter.status ?? 'active'}
            onChange={(e) => set({ status: e.target.value as ProductStatusFilter })}
            options={STATUSES.map((s) => ({ value: s, label: t(`products.statuses.${s}`) }))}
          />
        </div>
      </div>
      {products.isPending ? (
        <Spinner />
      ) : products.isError ? (
        <Alert tone="danger">{problemMessage(products.error, t)}</Alert>
      ) : rows.length === 0 ? (
        <p className="text-sm text-ink-muted">{filter.q ? t('products.noMatches') : t('products.none')}</p>
      ) : (
        <DataTable caption={t('products.title')} columns={columns} rows={rows} rowKey={(p) => p.id} />
      )}
      {products.hasNextPage && (
        <Button
          variant="secondary"
          className="self-stretch sm:self-center"
          disabled={products.isFetchingNextPage}
          onClick={() => void products.fetchNextPage()}
        >
          {products.isFetchingNextPage ? t('common.loading') : t('products.loadMore')}
        </Button>
      )}
    </div>
  )
}
