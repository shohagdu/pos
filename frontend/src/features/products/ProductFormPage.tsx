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
import { ResultAlert } from '../../components/ui/ResultAlert'
import { Select } from '../../components/ui/Select'
import { Spinner } from '../../components/ui/Spinner'
import { TextField } from '../../components/ui/TextField'
import { dec, toFixed } from '../../lib/decimal'
import { currencySymbol, formatMoney } from '../../lib/format'
import { optional, useFields } from '../../lib/form'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { categoryLabel, taxLabel, trimDecimal, unitLabel } from './labels'
import { ProductImageField } from './ProductImageField'
import { useDebounced } from '../pos/hooks'
import { dateLocale } from '../../lib/dates'

/** `/products/new` and `/products/:id`. */
export function ProductFormPage() {
  const { t } = useTranslation()
  const { id } = useParams()
  const location = useLocation()
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
      // Each visit to /products/new is a new, empty form (Save & Add Product
      // comes back to the same address).
      key={id ?? `new-${location.key}`}
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
  const queryClient = useQueryClient()
  const { can } = useSession()
  // Uploading a photo is a change to the product (`products.update`).
  const canSetImage = can('products.update')
  const imageUrl = useFileUrl(tenantId, product?.image_file_id)
  // New products keep the chosen photo here and upload it once created.
  const [newImage, setNewImage] = useState<File | null>(null)
  // Description is optional: folded away unless the product already has one.
  const [showDescription, setShowDescription] = useState(Boolean(product?.description))
  // Set once the product is saved (added or changed): the pop-up shows, then the list opens.
  const [done, setDone] = useState<{ kind: 'created' | 'updated'; name: string; imageError: string | null; addAnother: boolean } | null>(null)
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
  const currency = currencySymbol()
  // Profit per unit while both prices are filled in (display only).
  const margin = (() => {
    const number = /^\d+(\.\d+)?$/
    const sell = values.selling_price.trim()
    const cost = values.purchase_price.trim()
    if (!canViewCost || !number.test(sell) || !number.test(cost) || Number(sell) <= 0) return null
    const diff = dec(sell) - dec(cost)
    const pct = ((Number(sell) - Number(cost)) / Number(sell)) * 100
    return {
      loss: diff < 0n,
      amount: formatMoney(toFixed(diff < 0n ? -diff : diff, 2), i18n.language),
      pct: `${Math.abs(pct).toFixed(1)}%`,
    }
  })()

  const refresh = () => queryClient.invalidateQueries({ queryKey: catalogKeys.products(tenantId) })
  // The variable says which button sent the form: Save (then the list) or
  // Save & Add Product (then a new, empty form).
  const save = useMutation({
    mutationFn: (_addAnother: boolean) => {
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
    onSuccess: async ({ saved, imageError }, addAnother) => {
      queryClient.setQueryData(catalogKeys.product(tenantId, saved.id), saved)
      await refresh()
      setDone({ kind: product ? 'updated' : 'created', name: saved.name, imageError, addAnother: !product && addAnother })
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
    mutationFn: (file: File | null) => (file ? catalogApi.setProductImage(product?.id ?? '', file) : catalogApi.removeProductImage(product?.id ?? '')),
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

  // Typed codes are checked against other products while typing; saving
  // checks again (two people could type the same code at once).
  const typedSku = useDebounced(values.sku.trim(), 300)
  const typedBarcodes = useDebounced(values.barcodes.trim(), 300)
  const codeCheck = useQuery({
    queryKey: [...catalogKeys.products(tenantId), 'check-codes', product?.id ?? null, typedSku, typedBarcodes],
    queryFn: () => catalogApi.checkCodes({ sku: typedSku, barcodes: typedBarcodes, product_id: product?.id }),
    enabled: !readOnly && (typedSku !== '' || typedBarcodes !== ''),
    staleTime: 10_000,
  })
  // Only for what is in the box now, not for an older value still on its way.
  const takenSku = typedSku === values.sku.trim() ? codeCheck.data?.sku : null
  const takenBarcodes = typedBarcodes === values.barcodes.trim() ? (codeCheck.data?.barcodes ?? []) : []
  if (!errors.sku && takenSku) errors.sku = t('products.skuTaken', { name: takenSku.product_name })
  if (!errors.barcodes && takenBarcodes.length === 1) {
    errors.barcodes = t('products.barcodeTaken', { code: takenBarcodes[0]?.code, name: takenBarcodes[0]?.product_name })
  } else if (!errors.barcodes && takenBarcodes.length > 1) {
    errors.barcodes = t('products.barcodesTaken', { list: takenBarcodes.map((b) => `${b.code} (${b.product_name})`).join(', ') })
  }

  // Inactive choices stay listed only when the product already uses them.
  const active = <T extends { id: string; status: string }>(list: T[], current: string) => list.filter((x) => x.status === 'active' || x.id === current)

  function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault()
    // Enter in a field counts as the first button, Save.
    const submitter = (e.nativeEvent as SubmitEvent).submitter
    save.mutate(submitter?.dataset.action === 'add-another')
  }

  // Dense cards so the whole form fits on one laptop screen (1366×768).
  const card = 'flex flex-col gap-2.5 p-3.5'
  const cardTitle = 'text-sm font-semibold'
  return (
    <div className="flex max-w-[100rem] flex-col gap-3">
      <div className="flex flex-wrap items-center justify-between gap-x-4 gap-y-2">
        <div className="flex min-w-0 items-center gap-3">
          <Link to="/products" className="shrink-0 text-sm font-medium text-brand-700 hover:underline">
            ← {t('products.title')}
          </Link>
          <span aria-hidden="true" className="h-5 w-px bg-border" />
          <h1 className="truncate text-xl font-bold">{product ? product.name : t('products.new')}</h1>
          {product?.status === 'archived' && (
            <span className="rounded bg-canvas px-2 py-0.5 text-xs font-semibold text-ink-muted">{t('products.statuses.archived')}</span>
          )}
        </div>
        {product && (
          <div className="flex flex-wrap items-center gap-2">
            <Link
              to={`/labels?variants=${product.variant_id}`}
              className="inline-flex min-h-9 items-center rounded-lg border border-border bg-surface px-3 text-sm font-semibold hover:bg-canvas"
            >
              {t('labels.printBarcode')}
            </Link>
            {can('products.archive') &&
              (product.status === 'active' ? (
                <ConfirmButton size="sm" disabled={status.isPending} onConfirm={() => status.mutate()}>
                  {t('products.archive')}
                </ConfirmButton>
              ) : (
                <Button size="sm" variant="secondary" disabled={status.isPending} onClick={() => status.mutate()}>
                  {t('products.restore')}
                </Button>
              ))}
            {can('products.archive') && (
              <ConfirmButton size="sm" disabled={remove.isPending} onConfirm={() => remove.mutate()}>
                {t('products.delete')}
              </ConfirmButton>
            )}
          </div>
        )}
      </div>
      {product?.status === 'archived' && <Alert>{t('products.archivedHint')}</Alert>}
      {done && (
        <ResultAlert
          tone={done.imageError ? 'warning' : 'success'}
          title={done.kind === 'created' ? t('products.createdTitle') : t('products.updatedTitle')}
          message={
            done.imageError
              ? t('products.createdImageFailed', { name: done.name, reason: done.imageError })
              : done.kind === 'created'
                ? t('products.createdMessage', { name: done.name })
                : t('products.updatedMessage', { name: done.name })
          }
          confirmLabel={done.addAnother ? t('products.addAnotherOk') : t('products.createdOk')}
          // A failed photo needs reading, so that one waits for the button.
          autoCloseMs={done.imageError ? undefined : 2500}
          onClose={() => navigate(done.addAnother ? '/products/new' : '/products', { replace: true })}
        />
      )}
      <form onSubmit={onSubmit} className="flex flex-col gap-3" noValidate>
        {/* Desktop: the form cards across two columns' width plus a narrow
            sidebar (photo, stock, actions). Tablet/phone: one column, with
            the actions in a bar fixed to the bottom of the screen. */}
        <fieldset disabled={readOnly} className="grid grid-cols-1 min-w-0 gap-3 xl:grid-cols-[minmax(0,1fr)_19rem] xl:grid-rows-[auto_1fr] xl:items-start">
          <div className="flex min-w-0 flex-col gap-3 xl:row-span-2">
            <Card className={card}>
              <h2 className={cardTitle}>{t('products.details')}</h2>
              <div className="grid grid-cols-1 items-end gap-x-3 gap-y-2.5 sm:grid-cols-2 lg:grid-cols-[minmax(0,5fr)_minmax(0,4fr)_minmax(0,3fr)]">
                <TextField size="sm" label={t('products.name')} {...bind('name')} required maxLength={200} error={errors.name} />
                <TextField size="sm" label={t('products.nameBn')} {...bind('name_bn')} lang="bn" maxLength={200} error={errors.name_bn} />
                <Select
                  size="sm"
                  label={t('products.type')}
                  {...bind('product_type')}
                  required
                  options={PRODUCT_TYPES.map((p) => ({ value: p, label: t(`products.types.${p}`) }))}
                  error={errors.product_type}
                />
              </div>
              {/* Optional and rarely needed: folded away until asked for. */}
              <div className="flex flex-col gap-1">
                <button
                  type="button"
                  aria-expanded={showDescription}
                  aria-controls="product-description"
                  onClick={() => {
                    const open = !showDescription
                    setShowDescription(open)
                    if (open) setTimeout(() => document.getElementById('product-description')?.focus(), 0)
                  }}
                  className="flex min-h-7 items-center gap-1.5 self-start rounded-md text-[0.8125rem] font-medium text-brand-700 hover:underline focus-visible:outline-2 focus-visible:outline-brand-600"
                >
                  <svg
                    viewBox="0 0 20 20"
                    className={`size-4 transition-transform motion-reduce:transition-none ${showDescription ? 'rotate-90' : ''}`}
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="1.8"
                    aria-hidden="true"
                  >
                    <path d="m7.5 5 5 5-5 5" strokeLinecap="round" strokeLinejoin="round" />
                  </svg>
                  {t('products.description')}
                  {!showDescription && values.description.trim() && (
                    <span className="max-w-[40ch] truncate font-normal text-ink-muted">— {values.description.trim()}</span>
                  )}
                </button>
                {showDescription && (
                  <textarea
                    id="product-description"
                    aria-label={t('products.description')}
                    {...bind('description')}
                    rows={2}
                    maxLength={2000}
                    placeholder={t('products.descriptionHint')}
                    className="resize-y rounded-lg border border-border bg-surface px-3 py-1 text-sm outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100"
                  />
                )}
              </div>
            </Card>
            <Card className={card}>
              <h2 className={cardTitle}>{t('products.classification')}</h2>
              <div className="grid grid-cols-1 gap-x-4 gap-y-3 sm:grid-cols-2">
                <Select
                  size="sm"
                  label={t('products.category')}
                  {...bind('category_id')}
                  options={[
                    { value: '', label: t('products.noCategory') },
                    ...categoryTree(active(categories, values.category_id)).map((c) => ({ value: c.id, label: categoryLabel(c) })),
                  ]}
                  error={errors.category_id}
                />
                <Select
                  size="sm"
                  label={t('products.brand')}
                  {...bind('brand_id')}
                  options={[{ value: '', label: t('products.noBrand') }, ...active(brands, values.brand_id).map((b) => ({ value: b.id, label: b.name }))]}
                  error={errors.brand_id}
                />
                <Select
                  size="sm"
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
                  size="sm"
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
            <Card className={card}>
              <div className="flex flex-wrap items-baseline justify-between gap-x-3">
                <h2 className={cardTitle}>{t('products.prices')}</h2>
                {/* Live while both prices are filled in; in the title row so it adds no height. */}
                <p aria-live="polite" className={`text-xs ${margin?.loss ? 'font-medium text-danger' : 'text-ink-muted'}`}>
                  {margin && t(margin.loss ? 'products.lossPerUnit' : 'products.profitPerUnit', { amount: margin.amount, pct: margin.pct })}
                </p>
              </div>
              <div className="grid grid-cols-1 gap-x-4 gap-y-3 sm:grid-cols-2">
                <TextField
                  size="sm"
                  prefix={currency}
                  label={t('products.sellingPrice')}
                  inputMode="decimal"
                  {...bind('selling_price')}
                  required
                  error={errors.selling_price}
                />
                {canViewCost && (
                  <TextField
                    size="sm"
                    prefix={currency}
                    label={t('products.purchasePrice')}
                    inputMode="decimal"
                    {...bind('purchase_price')}
                    tip={t('products.purchasePriceHint')}
                    error={errors.purchase_price}
                  />
                )}
                <TextField
                  size="sm"
                  prefix={currency}
                  label={t('products.wholesalePrice')}
                  inputMode="decimal"
                  {...bind('wholesale_price')}
                  error={errors.wholesale_price}
                />
                <TextField
                  size="sm"
                  prefix={currency}
                  label={t('products.minSellingPrice')}
                  inputMode="decimal"
                  {...bind('min_selling_price')}
                  error={errors.min_selling_price}
                />
              </div>

              {product && <PriceHistory product={product} tenantId={tenantId} lang={i18n.language} />}
            </Card>
          </div>

          <div className="grid grid-cols-1 min-w-0 gap-3 sm:grid-cols-2 xl:col-start-2 xl:row-start-1 xl:flex xl:flex-col">
            {canSetImage && (
              <Card className={card}>
                {product ? (
                  <ProductImageField
                    compact
                    imageUrl={imageUrl}
                    busy={image.isPending}
                    error={image.isError ? imageError(image.error, t) : undefined}
                    onPick={(file) => image.mutate(file)}
                  />
                ) : (
                  <ProductImageField compact imageUrl={null} pending={newImage} onPick={setNewImage} />
                )}
              </Card>
            )}
            <Card className={card}>
              <h2 className={cardTitle}>{t('products.codes')}</h2>
              <div className="flex flex-col gap-2.5">
                <TextField
                  size="sm"
                  inline="wide"
                  label={t('products.sku')}
                  {...bind('sku')}
                  required
                  tip={product ? undefined : t('products.skuHint')}
                  maxLength={64}
                  error={errors.sku}
                />
                <TextField
                  size="sm"
                  inline="wide"
                  label={t('products.barcodes')}
                  {...bind('barcodes')}
                  required
                  tip={suggestedBarcode && values.barcodes.includes(suggestedBarcode) ? t('products.barcodeGenerated') : t('products.barcodesHint')}
                  error={errors.barcodes}
                />
              </div>
            </Card>
            <Card className={card}>
              <h2 className={cardTitle}>{t('products.stockWarranty')}</h2>
              {!isService && (
                <Checkbox label={t('products.trackInventory')} tip={t('products.trackInventoryHint')} tipAlign="end" {...bindCheck('track_inventory')} />
              )}
              {!isService && values.track_inventory && (
                <Checkbox label={t('products.trackBatches')} tip={t('products.trackBatchesHint')} tipAlign="end" {...bindCheck('track_batches')} />
              )}
              <Checkbox label={t('products.hasWarranty')} tip={t('products.hasWarrantyHint')} tipAlign="end" {...bindCheck('has_warranty')} />
              {values.has_warranty && (
                // Length and unit on one row under the tick: the number keeps its
                // visible label, the unit list shows its value and is named for
                // screen readers.
                <div className="flex flex-col gap-1">
                  <div className="flex items-center justify-between gap-2">
                    <label
                      htmlFor="warranty-period"
                      className="text-[0.8125rem] font-medium whitespace-nowrap text-ink after:ml-0.5 after:text-danger after:content-['*']"
                    >
                      {t('products.warrantyPeriod')}
                    </label>
                    <div className="flex gap-1.5">
                      <input
                        id="warranty-period"
                        type="number"
                        inputMode="numeric"
                        min={1}
                        step={1}
                        required
                        {...bind('warranty_period')}
                        aria-invalid={errors.warranty_period ? true : undefined}
                        aria-describedby={errors.warranty_period ? 'warranty-period-error' : undefined}
                        className="min-h-9 w-14 rounded-lg border border-border bg-surface px-2 text-sm outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100 aria-invalid:border-danger"
                      />
                      <select
                        aria-label={t('products.warrantyUnit')}
                        {...bind('warranty_unit')}
                        aria-invalid={errors.warranty_unit ? true : undefined}
                        className="min-h-9 w-[5.5rem] rounded-lg border border-border bg-surface px-2 text-sm outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100 aria-invalid:border-danger"
                      >
                        {(['day', 'month', 'year'] as const).map((u) => (
                          <option key={u} value={u}>
                            {t(`products.warrantyUnits.${u}`)}
                          </option>
                        ))}
                      </select>
                    </div>
                  </div>
                  {(errors.warranty_period || errors.warranty_unit) && (
                    <p id="warranty-period-error" className="text-xs text-danger">
                      {errors.warranty_period ?? errors.warranty_unit}
                    </p>
                  )}
                </div>
              )}
              {!isService && (
                <TextField
                  size="sm"
                  inline
                  label={t('products.reorderLevel')}
                  inputMode="decimal"
                  {...bind('reorder_level')}
                  tip={t('products.reorderLevelHint')}
                  error={errors.reorder_level}
                  className="border-t border-border pt-2.5"
                />
              )}
            </Card>
          </div>
          {/* Always in reach. A child of the form grid (not of the sidebar),
              so on tablets and phones it sticks to the bottom of the screen
              while the form scrolls; on desktop it sits under the sidebar. */}
          {!readOnly && (
            <div className="sticky bottom-0 z-10 -mx-4 flex gap-2 border-t border-border bg-surface/95 px-4 py-3 backdrop-blur sm:-mx-6 sm:px-6 xl:static xl:col-start-2 xl:row-start-2 xl:mx-0 xl:self-start xl:border-0 xl:bg-transparent xl:p-0 xl:backdrop-blur-none print:hidden">
              <Button type="submit" className="min-h-11 flex-1" disabled={save.isPending}>
                {save.isPending && !save.variables ? t('common.saving') : t('common.save')}
              </Button>
              {!product && (
                <Button type="submit" variant="dark" className="min-h-11 flex-1 px-3 xl:flex-none xl:px-4" data-action="add-another" disabled={save.isPending}>
                  {save.isPending && save.variables ? t('common.saving') : t('products.saveAndAdd')}
                </Button>
              )}
            </div>
          )}
        </fieldset>
        {save.isError && hasFormError(save.error) && !shownInline && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}
        {(status.isError || remove.isError) && <Alert tone="danger">{problemMessage(status.error ?? remove.error, t)}</Alert>}
      </form>
    </div>
  )
}

function PriceHistory({ product, tenantId, lang }: { product: Product; tenantId: string; lang: string }) {
  const { t } = useTranslation()
  const [open, setOpen] = useState(false)
  const history = usePriceHistory(tenantId, product.id)
  const date = (iso: string) => new Intl.DateTimeFormat(dateLocale(lang), { dateStyle: 'medium', timeStyle: 'short' }).format(new Date(iso))
  const money = (v: string | null) => (v === null ? '—' : formatMoney(v, lang))
  const count = history.data?.length ?? 0
  // A fold-out at the foot of the Prices card: one line until opened, and the
  // list scrolls inside when long, so the page keeps its height.
  return (
    <div className="flex flex-col gap-1.5 border-t border-border pt-2">
      <button
        type="button"
        aria-expanded={open}
        aria-controls="price-history"
        onClick={() => setOpen((o) => !o)}
        className="flex min-h-7 items-center gap-1.5 self-start rounded-md text-[0.8125rem] font-medium text-brand-700 hover:underline focus-visible:outline-2 focus-visible:outline-brand-600"
      >
        <svg
          viewBox="0 0 20 20"
          className={`size-4 transition-transform motion-reduce:transition-none ${open ? 'rotate-90' : ''}`}
          fill="none"
          stroke="currentColor"
          strokeWidth="1.8"
          aria-hidden="true"
        >
          <path d="m7.5 5 5 5-5 5" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
        {t('products.priceHistory')}
        {history.isSuccess && <span className="font-normal text-ink-muted">({count === 0 ? t('products.noPriceChanges') : count})</span>}
      </button>
      {open && (
        <div id="price-history">
          {history.isPending ? (
            <Spinner />
          ) : history.isError ? (
            <Alert tone="danger">{problemMessage(history.error, t)}</Alert>
          ) : count === 0 ? (
            <p className="text-xs text-ink-muted">{t('products.noPriceChanges')}</p>
          ) : (
            <ul className="flex max-h-32 flex-col gap-1.5 overflow-y-auto pr-1 text-[0.8125rem]">
              {history.data.map((h) => (
                <li key={`${h.price_field}-${h.changed_at}`} className="flex flex-wrap justify-between gap-x-2 border-b border-border pb-1.5 last:border-b-0">
                  <span>
                    {t(`products.priceFields.${h.price_field}`)}: {money(h.old_value)} → {money(h.new_value)}
                  </span>
                  <span className="text-ink-muted">{date(h.changed_at)}</span>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </div>
  )
}
