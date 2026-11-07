import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import type { TFunction } from 'i18next'
import { useTranslation } from 'react-i18next'
import { Link, useLocation, useNavigate, useParams } from 'react-router'

import {
  type Product,
  type ProductInput,
  PRODUCT_TYPES,
  type ProductType,
  type WarrantyUnit,
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
import { useFileUrl } from '../../api/files'
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
import { ProductImageField } from './ProductImageField'
import { dateLocale } from '../../lib/dates'

/** `/products/new` and `/products/:id`. */
export function ProductFormPage() {
  const { t } = useTranslation()
  const { id } = useParams()
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const units = useUnits(tenantId)
  const taxRates = useTaxRates(tenantId)
  const categories = useCategories(tenantId)
  const brands = useBrands(tenantId)
  const product = useProduct(tenantId, id ?? '')
  const lists = [units, taxRates, categories, brands]
  // A new product starts with the next automatic SKU and a new in-store barcode
  // filled in; so does a saved product that has no barcode yet.
  const needsCodes = id ? product.data?.barcodes.length === 0 && can('products.update') : can('products.create')
  const nextCodes = useQuery({
    queryKey: [...catalogKeys.products(tenantId), 'next-codes'],
    queryFn: catalogApi.nextCodes,
    enabled: needsCodes,
    staleTime: 0,
    gcTime: 0,
  })

  if (lists.some((q) => q.isPending) || (id && product.isPending) || nextCodes.isLoading) return <Spinner />
  const failed = lists.find((q) => q.isError) ?? (id && product.isError ? product : undefined)
  if (failed) return <Alert tone="danger">{problemMessage(failed.error, t)}</Alert>
  return (
    <ProductForm
      key={id ?? 'new'}
      product={id ? product.data : undefined}
      suggestedSku={id ? undefined : (nextCodes.data?.sku ?? undefined)}
      suggestedBarcode={needsCodes ? nextCodes.data?.barcode : undefined}
      tenantId={tenantId}
    />
  )
}

/** An upload error: the server names the `file` field (wrong kind, damaged, hidden code). */
function imageError(err: unknown, t: TFunction): string {
  return fieldErrors(err, t).file ?? problemMessage(err, t)
}

function decimal(value: string | null | undefined): string {
  return value ? trimDecimal(value) : ''
}

/** Fields the form checks before sending (the server checks them too). */
const REQUIRED_FIELDS = ['product_type', 'name', 'sku', 'barcodes', 'unit_id', 'selling_price'] as const

function ProductForm({
  product,
  suggestedSku,
  suggestedBarcode,
  tenantId,
}: {
  product?: Product
  suggestedSku?: string
  suggestedBarcode?: string
  tenantId: string
}) {
  const { t, i18n } = useTranslation()
  const navigate = useNavigate()
  const location = useLocation()
  const queryClient = useQueryClient()
  const { can } = useSession()
  // Uploading a photo is a change to the product (`products.update`).
  const canSetImage = can('products.update')
  const imageUrl = useFileUrl(tenantId, product?.image_file_id)
  // New products keep the chosen photo here and upload it once created.
  const [newImage, setNewImage] = useState<File | null>(null)
  const imageFailed = (location.state as { imageFailed?: string } | null)?.imageFailed
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
    sku: product?.sku ?? suggestedSku ?? '',
    barcodes: product?.barcodes.length ? product.barcodes.join(', ') : (suggestedBarcode ?? ''),
    category_id: product?.category_id ?? '',
    brand_id: product?.brand_id ?? '',
    unit_id: product?.unit_id ?? defaultUnit?.id ?? '',
    tax_rate_id: product?.tax_rate_id ?? '',
    track_inventory: product?.track_inventory ?? true,
    track_batches: product?.track_batches ?? false,
    has_warranty: product?.warranty_period != null,
    warranty_period: product?.warranty_period?.toString() ?? '',
    warranty_unit: product?.warranty_unit ?? 'month',
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
      // Blank required fields are flagged on the field; a blank price would otherwise be a raw 400.
      const barcodes = values.barcodes
        .split(/[\s,]+/)
        .map((b) => b.trim())
        .filter(Boolean)
      const missing: string[] = REQUIRED_FIELDS.filter((f) => (f === 'barcodes' ? barcodes.length === 0 : !values[f].trim()))
      if (values.has_warranty && !values.warranty_period.trim()) missing.push('warranty_period')
      if (missing.length > 0) {
        throw new ApiError({
          type: 'urn:pos:problem:validation_failed',
          title: 'Bad Request',
          status: 400,
          code: 'VALIDATION_FAILED',
          detail: '',
          errors: missing.map((field) => ({ field, code: 'REQUIRED', message: '' })),
        })
      }
      const input: ProductInput = {
        product_type: values.product_type as ProductType,
        name: values.name.trim(),
        name_bn: optional(values.name_bn),
        description: optional(values.description),
        // The suggested code is left to the server, which takes the next free one when saving.
        sku: !product && values.sku.trim() === suggestedSku ? undefined : optional(values.sku),
        // The suggested barcode is left to the server, which takes the next free in-store code when saving.
        barcodes: barcodes.filter((b) => b !== suggestedBarcode),
        generate_barcode: suggestedBarcode !== undefined && barcodes.includes(suggestedBarcode) ? true : undefined,
        category_id: optional(values.category_id),
        brand_id: optional(values.brand_id),
        unit_id: values.unit_id,
        tax_rate_id: optional(values.tax_rate_id),
        track_inventory: isService ? false : values.track_inventory,
        track_batches: !isService && values.track_inventory ? values.track_batches : false,
        // A malformed length is sent as-is so the server flags the field.
        warranty_period: values.has_warranty ? Number(values.warranty_period.trim()) : undefined,
        warranty_unit: values.has_warranty ? (values.warranty_unit as WarrantyUnit) : undefined,
        // Without view_cost the field is hidden and must not be sent (403).
        purchase_price: canViewCost ? optional(values.purchase_price) : undefined,
        selling_price: values.selling_price.trim(),
        wholesale_price: optional(values.wholesale_price),
        min_selling_price: optional(values.min_selling_price),
        reorder_level: optional(values.reorder_level),
      }
      if (product) return catalogApi.updateProduct(product.id, product.version, input).then((saved) => ({ saved, imageError: null }))
      return catalogApi.createProduct(input).then(async (created) => {
        if (!newImage) return { saved: created, imageError: null }
        // The product exists now, so a failed upload must not fail the save (saving again would duplicate it).
        try {
          return { saved: await catalogApi.setProductImage(created.id, newImage), imageError: null }
        } catch (err) {
          return { saved: created, imageError: imageError(err, t) }
        }
      })
    },
    onSuccess: async ({ saved, imageError }) => {
      queryClient.setQueryData(catalogKeys.product(tenantId, saved.id), saved)
      await refresh()
      if (!product) navigate(`/products/${saved.id}`, { replace: true, state: imageError ? { imageFailed: imageError } : null })
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
  // A saved product's photo changes at once, apart from the form's Save.
  const image = useMutation({
    mutationFn: (file: File | null) =>
      file ? catalogApi.setProductImage(product?.id ?? '', file) : catalogApi.removeProductImage(product?.id ?? ''),
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
    <div className="flex max-w-6xl flex-col gap-4">
      <Link to="/products" className="text-sm font-medium text-brand-700 hover:underline">
        ← {t('products.title')}
      </Link>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h1 className="text-2xl font-bold">{product ? product.name : t('products.new')}</h1>
        <div className="flex flex-wrap items-center gap-2">
          {product?.status === 'archived' && <span className="rounded bg-canvas px-2 py-0.5 text-xs font-semibold text-ink-muted">{t('products.statuses.archived')}</span>}
          {product && (
            <Link
              to={`/labels?variants=${product.variant_id}`}
              className="inline-flex min-h-11 items-center rounded-lg border border-border bg-surface px-4 text-sm font-semibold hover:bg-canvas"
            >
              {t('labels.printBarcode')}
            </Link>
          )}
        </div>
      </div>
      {product?.status === 'archived' && <Alert>{t('products.archivedHint')}</Alert>}
      {save.isSuccess && <Alert tone="success">{t('products.saved')}</Alert>}
      {imageFailed && <Alert tone="warning">{t('products.image.uploadFailed', { reason: imageFailed })}</Alert>}
      <form onSubmit={onSubmit} className="flex flex-col gap-4" noValidate>
        <fieldset disabled={readOnly} className="grid min-w-0 gap-4 lg:grid-cols-[minmax(0,1fr)_20rem] lg:items-start">
          <div className="flex min-w-0 flex-col gap-4">
            <Card className="flex flex-col gap-4 p-5">
              <h2 className="font-semibold">{t('products.details')}</h2>
              <div className="grid gap-4 sm:grid-cols-[minmax(0,2fr)_minmax(0,1fr)]">
                <TextField label={t('products.name')} {...bind('name')} required maxLength={200} error={errors.name} />
                <Select
                  label={t('products.type')}
                  {...bind('product_type')}
                  required
                  options={PRODUCT_TYPES.map((p) => ({ value: p, label: t(`products.types.${p}`) }))}
                  error={errors.product_type}
                />
              </div>
              <TextField label={t('products.nameBn')} {...bind('name_bn')} lang="bn" maxLength={200} error={errors.name_bn} />
              <label className="flex flex-col gap-1.5 text-sm font-medium text-ink">
                {t('products.description')}
                <textarea
                  {...bind('description')}
                  rows={2}
                  maxLength={2000}
                  className="rounded-lg border border-border bg-surface px-3 py-2 text-sm font-normal outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100"
                />
              </label>
            </Card>
            <Card className="flex flex-col gap-4 p-5">
              <h2 className="font-semibold">{t('products.codes')}</h2>
              <div className="grid gap-4 sm:grid-cols-2">
                <TextField
                  label={t('products.sku')}
                  {...bind('sku')}
                  required
                  hint={product ? undefined : t('products.skuHint')}
                  maxLength={64}
                  error={errors.sku}
                />
                <TextField
                  label={t('products.barcodes')}
                  {...bind('barcodes')}
                  required
                  hint={suggestedBarcode && values.barcodes.includes(suggestedBarcode) ? t('products.barcodeGenerated') : t('products.barcodesHint')}
                  error={errors.barcodes}
                />
              </div>
            </Card>
            <Card className="flex flex-col gap-4 p-5">
              <h2 className="font-semibold">{t('products.classification')}</h2>
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
                  required
                  options={[
                    ...(values.unit_id ? [] : [{ value: '', label: t('products.chooseUnit') }]),
                    ...active(units, values.unit_id).map((u) => ({ value: u.id, label: unitLabel(u) })),
                  ]}
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
            </Card>
            <Card className="flex flex-col gap-4 p-5">
              <h2 className="font-semibold">{t('products.stockWarranty')}</h2>
              {!isService && <Checkbox label={t('products.trackInventory')} hint={t('products.trackInventoryHint')} {...bindCheck('track_inventory')} />}
              {!isService && values.track_inventory && (
                <Checkbox label={t('products.trackBatches')} hint={t('products.trackBatchesHint')} {...bindCheck('track_batches')} />
              )}
              {!isService && (
                <TextField
                  label={t('products.reorderLevel')}
                  inputMode="decimal"
                  {...bind('reorder_level')}
                  hint={t('products.reorderLevelHint')}
                  error={errors.reorder_level}
                  className="sm:max-w-xs"
                />
              )}
              <Checkbox label={t('products.hasWarranty')} hint={t('products.hasWarrantyHint')} {...bindCheck('has_warranty')} />
              {values.has_warranty && (
                <div className="grid gap-4 sm:max-w-md sm:grid-cols-2">
                  <TextField
                    label={t('products.warrantyPeriod')}
                    type="number"
                    inputMode="numeric"
                    min={1}
                    step={1}
                    {...bind('warranty_period')}
                    required
                    error={errors.warranty_period}
                  />
                  <Select
                    label={t('products.warrantyUnit')}
                    {...bind('warranty_unit')}
                    options={(['day', 'month', 'year'] as const).map((u) => ({ value: u, label: t(`products.warrantyUnits.${u}`) }))}
                    error={errors.warranty_unit}
                  />
                </div>
              )}
            </Card>
          </div>
          <div className="flex min-w-0 flex-col gap-4 lg:sticky lg:top-4">
            {canSetImage && (
              <Card className="p-5">
                {product ? (
                  <ProductImageField
                    imageUrl={imageUrl}
                    busy={image.isPending}
                    error={image.isError ? imageError(image.error, t) : undefined}
                    onPick={(file) => image.mutate(file)}
                  />
                ) : (
                  <ProductImageField imageUrl={null} pending={newImage} onPick={setNewImage} />
                )}
              </Card>
            )}
            <Card className="flex flex-col gap-4 p-5">
              <h2 className="font-semibold">{t('products.prices')}</h2>
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
              <div className="grid grid-cols-2 gap-3">
                <TextField label={t('products.wholesalePrice')} inputMode="decimal" {...bind('wholesale_price')} error={errors.wholesale_price} />
                <TextField label={t('products.minSellingPrice')} inputMode="decimal" {...bind('min_selling_price')} error={errors.min_selling_price} />
              </div>
            </Card>
          </div>
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
