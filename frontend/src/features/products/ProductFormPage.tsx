import { useMutation, useQueryClient } from '@tanstack/react-query'
import type { FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate, useParams } from 'react-router'

import {
  type Product,
  type ProductInput,
  PRODUCT_TYPES,
  type ProductType,
  catalogApi,
  catalogKeys,
  categoryTree,
  useBrands,
  useCategories,
  usePriceHistory,
  useProduct,
  useTaxRates,
  useUnits,
} from '../../api/catalog'
import { ApiError } from '../../api/client'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { Checkbox } from '../../components/ui/Checkbox'
import { ConfirmButton } from '../../components/ui/ConfirmButton'
import { Select } from '../../components/ui/Select'
import { Spinner } from '../../components/ui/Spinner'
import { TextField } from '../../components/ui/TextField'
import { formatMoney } from '../../lib/format'
import { optional, useFields } from '../../lib/form'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { categoryLabel, taxLabel, trimDecimal, unitLabel } from './labels'
import { dateLocale } from '../../lib/dates'

/** `/products/new` and `/products/:id`. */
export function ProductFormPage() {
  const { t } = useTranslation()
  const { id } = useParams()
  const { me } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const units = useUnits(tenantId)
  const taxRates = useTaxRates(tenantId)
  const categories = useCategories(tenantId)
  const brands = useBrands(tenantId)
  const product = useProduct(tenantId, id ?? '')
  const lists = [units, taxRates, categories, brands]

  if (lists.some((q) => q.isPending) || (id && product.isPending)) return <Spinner />
  const failed = lists.find((q) => q.isError) ?? (id && product.isError ? product : undefined)
  if (failed) return <Alert tone="danger">{problemMessage(failed.error, t)}</Alert>
  return <ProductForm key={id ?? 'new'} product={id ? product.data : undefined} tenantId={tenantId} />
}

function decimal(value: string | null | undefined): string {
  return value ? trimDecimal(value) : ''
}

function ProductForm({ product, tenantId }: { product?: Product; tenantId: string }) {
  const { t, i18n } = useTranslation()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const { can } = useSession()
  const units = useUnits(tenantId).data ?? []
  const taxRates = useTaxRates(tenantId).data ?? []
  const categories = useCategories(tenantId).data ?? []
  const brands = useBrands(tenantId).data ?? []
  const canViewCost = can('products.view_cost')
  const readOnly = product ? !can('products.update') : !can('products.create')
  const defaultUnit = units.find((u) => u.short_name === 'pcs' && u.status === 'active') ?? units.find((u) => u.status === 'active')

  const { values, bind, bindCheck } = useFields({
    product_type: product?.product_type ?? 'standard',
    name: product?.name ?? '',
    name_bn: product?.name_bn ?? '',
    description: product?.description ?? '',
    sku: product?.sku ?? '',
    barcodes: product?.barcodes.join(', ') ?? '',
    category_id: product?.category_id ?? '',
    brand_id: product?.brand_id ?? '',
    unit_id: product?.unit_id ?? defaultUnit?.id ?? '',
    tax_rate_id: product?.tax_rate_id ?? '',
    track_inventory: product?.track_inventory ?? true,
    purchase_price: decimal(product?.purchase_price),
    selling_price: decimal(product?.selling_price),
    wholesale_price: decimal(product?.wholesale_price),
    min_selling_price: decimal(product?.min_selling_price),
    reorder_level: decimal(product?.reorder_level),
  })
  const isService = values.product_type === 'service'

  const refresh = () => queryClient.invalidateQueries({ queryKey: catalogKeys.products(tenantId) })
  const save = useMutation({
    mutationFn: () => {
      const input: ProductInput = {
        product_type: values.product_type as ProductType,
        name: values.name.trim(),
        name_bn: optional(values.name_bn),
        description: optional(values.description),
        sku: optional(values.sku),
        barcodes: values.barcodes
          .split(/[\s,]+/)
          .map((b) => b.trim())
          .filter(Boolean),
        category_id: optional(values.category_id),
        brand_id: optional(values.brand_id),
        unit_id: values.unit_id,
        tax_rate_id: optional(values.tax_rate_id),
        track_inventory: isService ? false : values.track_inventory,
        // Without view_cost the field is hidden and must not be sent (403).
        purchase_price: canViewCost ? optional(values.purchase_price) : undefined,
        selling_price: values.selling_price.trim(),
        wholesale_price: optional(values.wholesale_price),
        min_selling_price: optional(values.min_selling_price),
        reorder_level: optional(values.reorder_level),
      }
      return product ? catalogApi.updateProduct(product.id, product.version, input) : catalogApi.createProduct(input)
    },
    onSuccess: async (saved) => {
      queryClient.setQueryData(catalogKeys.product(tenantId, saved.id), saved)
      await refresh()
      if (!product) navigate(`/products/${saved.id}`, { replace: true })
    },
    onError: (err) => {
      // Someone else saved first: load their version; the next save uses it.
      if (err instanceof ApiError && err.status === 412) void refresh()
    },
  })
  const status = useMutation({
    mutationFn: () => (product?.status === 'active' ? catalogApi.archiveProduct : catalogApi.restoreProduct)(product?.id ?? ''),
    onSuccess: async (saved) => {
      queryClient.setQueryData(catalogKeys.product(tenantId, saved.id), saved)
      await refresh()
    },
  })
  const remove = useMutation({
    mutationFn: () => catalogApi.deleteProduct(product?.id ?? ''),
    onSuccess: async () => {
      await refresh()
      navigate('/products', { replace: true })
    },
  })
  const errors = fieldErrors(save.error, t)
  // Duplicates come back as 409s without field details; show them on the field.
  const conflict = save.error instanceof ApiError ? save.error.problem.code : ''
  if (conflict === 'DUPLICATE_BARCODE') errors.barcodes = problemMessage(save.error, t)
  if (conflict === 'DUPLICATE_SKU') errors.sku = problemMessage(save.error, t)
  const shownInline = conflict === 'DUPLICATE_BARCODE' || conflict === 'DUPLICATE_SKU'

  // Inactive choices stay listed only when the product already uses them.
  const active = <T extends { id: string; status: string }>(list: T[], current: string) =>
    list.filter((x) => x.status === 'active' || x.id === current)

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    save.mutate()
  }

  return (
    <div className="flex max-w-3xl flex-col gap-6">
      <Link to="/products" className="text-sm font-medium text-brand-700 hover:underline">
        ← {t('products.title')}
      </Link>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h1 className="text-2xl font-bold">{product ? product.name : t('products.new')}</h1>
        {product?.status === 'archived' && <span className="rounded bg-canvas px-2 py-0.5 text-xs font-semibold text-ink-muted">{t('products.statuses.archived')}</span>}
      </div>
      {product?.status === 'archived' && <Alert>{t('products.archivedHint')}</Alert>}
      {save.isSuccess && <Alert tone="success">{t('products.saved')}</Alert>}
      <form onSubmit={onSubmit} className="flex flex-col gap-6" noValidate>
        <fieldset disabled={readOnly} className="flex flex-col gap-6">
          <Card className="flex flex-col gap-4">
            <h2 className="font-semibold">{t('products.details')}</h2>
            <Select
              label={t('products.type')}
              {...bind('product_type')}
              options={PRODUCT_TYPES.map((p) => ({ value: p, label: t(`products.types.${p}`) }))}
              error={errors.product_type}
            />
            <TextField label={t('products.name')} {...bind('name')} required maxLength={200} error={errors.name} />
            <TextField label={t('products.nameBn')} {...bind('name_bn')} lang="bn" maxLength={200} error={errors.name_bn} />
            <div className="grid gap-4 sm:grid-cols-2">
              <TextField
                label={t('products.sku')}
                {...bind('sku')}
                hint={product ? undefined : t('products.skuHint')}
                maxLength={64}
                error={errors.sku}
              />
              <TextField label={t('products.barcodes')} {...bind('barcodes')} hint={t('products.barcodesHint')} error={errors.barcodes} />
            </div>
            <div className="grid gap-4 sm:grid-cols-2">
              <Select
                label={t('products.category')}
                {...bind('category_id')}
                options={[
                  { value: '', label: t('products.noCategory') },
                  ...categoryTree(active(categories, values.category_id)).map((c) => ({ value: c.id, label: categoryLabel(c) })),
                ]}
                error={errors.category_id}
              />
              <Select
                label={t('products.brand')}
                {...bind('brand_id')}
                options={[{ value: '', label: t('products.noBrand') }, ...active(brands, values.brand_id).map((b) => ({ value: b.id, label: b.name }))]}
                error={errors.brand_id}
              />
              <Select
                label={t('products.unit')}
                {...bind('unit_id')}
                options={active(units, values.unit_id).map((u) => ({ value: u.id, label: unitLabel(u) }))}
                error={errors.unit_id}
              />
              <Select
                label={t('products.taxRate')}
                {...bind('tax_rate_id')}
                options={[
                  { value: '', label: t('products.defaultTax') },
                  ...active(taxRates, values.tax_rate_id).map((r) => ({ value: r.id, label: taxLabel(r) })),
                ]}
                error={errors.tax_rate_id}
              />
            </div>
            <label className="flex flex-col gap-1.5 text-sm font-medium text-ink">
              {t('products.description')}
              <textarea
                {...bind('description')}
                rows={3}
                maxLength={2000}
                className="rounded-lg border border-border bg-surface px-3 py-2 text-sm font-normal outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100"
              />
            </label>
            {!isService && <Checkbox label={t('products.trackInventory')} hint={t('products.trackInventoryHint')} {...bindCheck('track_inventory')} />}
          </Card>
          <Card className="flex flex-col gap-4">
            <h2 className="font-semibold">{t('products.prices')}</h2>
            <div className="grid gap-4 sm:grid-cols-2">
              <TextField label={t('products.sellingPrice')} inputMode="decimal" {...bind('selling_price')} required error={errors.selling_price} />
              {canViewCost && (
                <TextField
                  label={t('products.purchasePrice')}
                  inputMode="decimal"
                  {...bind('purchase_price')}
                  hint={t('products.purchasePriceHint')}
                  error={errors.purchase_price}
                />
              )}
              <TextField label={t('products.wholesalePrice')} inputMode="decimal" {...bind('wholesale_price')} error={errors.wholesale_price} />
              <TextField label={t('products.minSellingPrice')} inputMode="decimal" {...bind('min_selling_price')} error={errors.min_selling_price} />
              {!isService && (
                <TextField
                  label={t('products.reorderLevel')}
                  inputMode="decimal"
                  {...bind('reorder_level')}
                  hint={t('products.reorderLevelHint')}
                  error={errors.reorder_level}
                />
              )}
            </div>
          </Card>
        </fieldset>
        {save.isError && hasFormError(save.error) && !shownInline && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}
        {(status.isError || remove.isError) && <Alert tone="danger">{problemMessage(status.error ?? remove.error, t)}</Alert>}
        <div className="flex flex-wrap items-center justify-between gap-3">
          {product && can('products.archive') ? (
            <div className="flex flex-wrap gap-2">
              {product.status === 'active' ? (
                <ConfirmButton disabled={status.isPending} onConfirm={() => status.mutate()}>
                  {t('products.archive')}
                </ConfirmButton>
              ) : (
                <Button variant="secondary" disabled={status.isPending} onClick={() => status.mutate()}>
                  {t('products.restore')}
                </Button>
              )}
              <ConfirmButton disabled={remove.isPending} onConfirm={() => remove.mutate()}>
                {t('products.delete')}
              </ConfirmButton>
            </div>
          ) : (
            <span />
          )}
          {!readOnly && (
            <Button type="submit" disabled={save.isPending}>
              {save.isPending ? t('common.saving') : t('common.save')}
            </Button>
          )}
        </div>
      </form>
      {product && <PriceHistory product={product} tenantId={tenantId} lang={i18n.language} />}
    </div>
  )
}

function PriceHistory({ product, tenantId, lang }: { product: Product; tenantId: string; lang: string }) {
  const { t } = useTranslation()
  const history = usePriceHistory(tenantId, product.id)
  const date = (iso: string) => new Intl.DateTimeFormat(dateLocale(lang), { dateStyle: 'medium', timeStyle: 'short' }).format(new Date(iso))
  const money = (v: string | null) => (v === null ? '—' : formatMoney(v, lang))
  return (
    <Card className="flex flex-col gap-3">
      <h2 className="font-semibold">{t('products.priceHistory')}</h2>
      {history.isPending ? (
        <Spinner />
      ) : history.isError ? (
        <Alert tone="danger">{problemMessage(history.error, t)}</Alert>
      ) : history.data.length === 0 ? (
        <p className="text-sm text-ink-muted">{t('products.noPriceChanges')}</p>
      ) : (
        <ul className="flex flex-col gap-2 text-sm">
          {history.data.map((h) => (
            <li key={`${h.price_field}-${h.changed_at}`} className="flex flex-wrap justify-between gap-2 border-b border-border pb-2 last:border-b-0">
              <span>
                {t(`products.priceFields.${h.price_field}`)}: {money(h.old_value)} → {money(h.new_value)}
              </span>
              <span className="text-ink-muted">{date(h.changed_at)}</span>
            </li>
          ))}
        </ul>
      )}
    </Card>
  )
}
