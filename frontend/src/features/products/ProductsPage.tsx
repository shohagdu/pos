import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { type ProductFilter, type ProductStatusFilter, categoryTree, useBrands, useCategories, useProducts } from '../../api/catalog'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Select } from '../../components/ui/Select'
import { Spinner } from '../../components/ui/Spinner'
import { TextField } from '../../components/ui/TextField'
import { formatMoney } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { CatalogNav } from './CatalogNav'
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
  return (
    <div className="flex max-w-5xl flex-col gap-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h1 className="text-2xl font-bold">{t('products.title')}</h1>
        {can('products.create') && (
          <Link to="/products/new" className="inline-flex min-h-11 items-center rounded-lg bg-brand-600 px-4 text-sm font-semibold text-white hover:bg-brand-700">
            {t('products.new')}
          </Link>
        )}
      </div>
      <CatalogNav />
      <div className="flex flex-col gap-3">
        <form onSubmit={onSearch} className="flex flex-wrap items-end gap-2" role="search">
          <TextField
            className="min-w-60 flex-1"
            label={t('products.search')}
            type="search"
            value={text}
            onChange={(e) => setText(e.target.value)}
            placeholder={t('products.searchHint')}
          />
          <Button type="submit">{t('products.searchButton')}</Button>
        </form>
        <div className="grid gap-3 sm:grid-cols-3">
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
        <div className="overflow-x-auto rounded-xl border border-border bg-surface">
          <table className="w-full text-left text-sm">
            <thead className="border-b border-border bg-canvas text-xs text-ink-muted uppercase">
              <tr>
                <th scope="col" className="px-4 py-3">{t('products.name')}</th>
                <th scope="col" className="px-4 py-3">{t('products.sku')}</th>
                <th scope="col" className="px-4 py-3">{t('products.category')}</th>
                <th scope="col" className="px-4 py-3 text-right">{t('products.sellingPrice')}</th>
                <th scope="col" className="px-4 py-3">{t('products.status')}</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((p) => (
                <tr key={p.id} className="border-b border-border last:border-b-0 hover:bg-canvas">
                  <td className="px-4 py-3">
                    <Link to={`/products/${p.id}`} className="font-medium text-brand-700 hover:underline">
                      {p.name}
                    </Link>
                    {p.name_bn && <span className="block text-xs text-ink-muted">{p.name_bn}</span>}
                  </td>
                  <td className="px-4 py-3 font-mono text-xs">{p.sku}</td>
                  <td className="px-4 py-3">{p.category_name ?? '—'}</td>
                  <td className="px-4 py-3 text-right tabular-nums">
                    {formatMoney(p.selling_price, i18n.language)}
                    <span className="text-xs text-ink-muted"> / {p.unit_short_name}</span>
                  </td>
                  <td className="px-4 py-3">
                    <span className={`rounded px-2 py-0.5 text-xs font-semibold ${p.status === 'active' ? 'bg-brand-50 text-brand-700' : 'bg-canvas text-ink-muted'}`}>
                      {t(`products.statuses.${p.status}`)}
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      {products.hasNextPage && (
        <Button variant="secondary" className="self-center" disabled={products.isFetchingNextPage} onClick={() => void products.fetchNextPage()}>
          {products.isFetchingNextPage ? t('common.loading') : t('products.loadMore')}
        </Button>
      )}
    </div>
  )
}
