import { type QueryKey, useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, type ReactNode, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useParams } from 'react-router'

import {
  type Brand,
  type Category,
  MAX_CATEGORY_DEPTH,
  type TaxRate,
  type Unit,
  catalogApi,
  catalogKeys,
  categoryTree,
  useBrands,
  useCategories,
  useTaxRates,
  useUnits,
} from '../../api/catalog'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { Checkbox } from '../../components/ui/Checkbox'
import { ConfirmButton } from '../../components/ui/ConfirmButton'
import { Select } from '../../components/ui/Select'
import { Spinner } from '../../components/ui/Spinner'
import { TextField } from '../../components/ui/TextField'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { CatalogNav } from './CatalogNav'
import { categoryLabel, trimDecimal } from './labels'

const SECTIONS = ['categories', 'brands', 'units', 'tax-rates'] as const
type Section = (typeof SECTIONS)[number]

/** Catalogue masters (E2.1): `/products/setup/:section`. */
export function CatalogSetupPage() {
  const { t } = useTranslation()
  const { section = 'categories' } = useParams()
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const current = (SECTIONS.includes(section as Section) ? section : 'categories') as Section
  const editable = current === 'tax-rates' ? can('settings.manage') : can('products.update')

  return (
    <div className="flex max-w-4xl flex-col gap-6">
      <h1 className="text-2xl font-bold">{t('products.title')}</h1>
      <CatalogNav />
      {current === 'tax-rates' && !can('settings.manage') ? (
        <Alert tone="danger">{t('problems.PERMISSION_DENIED')}</Alert>
      ) : (
        <>
          {current === 'categories' && <Categories tenantId={tenantId} editable={editable} />}
          {current === 'brands' && <Brands tenantId={tenantId} editable={editable} />}
          {current === 'units' && <Units tenantId={tenantId} editable={editable} />}
          {current === 'tax-rates' && <TaxRates tenantId={tenantId} editable={editable} />}
        </>
      )}
    </div>
  )
}

/** A save mutation that refreshes `key` (and the product lists that show names). */
function useSave<A>(tenantId: string, key: QueryKey, fn: (args: A) => Promise<unknown>, onDone?: () => void) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: fn,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: key })
      await queryClient.invalidateQueries({ queryKey: catalogKeys.products(tenantId) })
      onDone?.()
    },
  })
}

function FormError({ error }: { error: unknown }) {
  const { t } = useTranslation()
  if (!error || !hasFormError(error)) return null
  return <Alert tone="danger">{problemMessage(error, t)}</Alert>
}

function Row({ children, actions }: { children: ReactNode; actions?: ReactNode }) {
  return (
    <li className="flex flex-wrap items-center justify-between gap-3 border-b border-border py-3 last:border-b-0">
      <div className="flex min-w-0 flex-col">{children}</div>
      {actions && <div className="flex flex-wrap gap-2">{actions}</div>}
    </li>
  )
}

function StatusTag({ status }: { status: string }) {
  const { t } = useTranslation()
  if (status === 'active') return null
  return <span className="ml-2 rounded bg-canvas px-2 py-0.5 text-xs font-semibold text-ink-muted">{t('products.statuses.inactive')}</span>
}

// ----------------------------------------------------------------- categories

function Categories({ tenantId, editable }: { tenantId: string; editable: boolean }) {
  const { t } = useTranslation()
  const categories = useCategories(tenantId)
  const [editing, setEditing] = useState<string | null>(null)
  const key = catalogKeys.categories(tenantId)
  const remove = useSave(tenantId, key, catalogApi.deleteCategory)
  if (categories.isPending) return <Spinner />
  if (categories.isError) return <Alert tone="danger">{problemMessage(categories.error, t)}</Alert>
  const tree = categoryTree(categories.data)
  // A parent must leave room below it (three levels in all).
  const parents = tree.filter((c) => c.depth < MAX_CATEGORY_DEPTH)

  return (
    <div className="flex flex-col gap-4">
      {editable && <CategoryForm tenantId={tenantId} parents={parents} />}
      <Card>
        <h2 className="font-semibold">{t('products.tabs.categories')}</h2>
        {remove.isError && <FormError error={remove.error} />}
        {tree.length === 0 ? (
          <p className="mt-2 text-sm text-ink-muted">{t('catalog.noCategories')}</p>
        ) : (
          <ul>
            {tree.map((c) =>
              editing === c.id ? (
                <li key={c.id} className="border-b border-border py-3 last:border-b-0">
                  <CategoryForm
                    tenantId={tenantId}
                    parents={parents.filter((p) => p.id !== c.id)}
                    category={c}
                    onDone={() => setEditing(null)}
                  />
                </li>
              ) : (
                <Row
                  key={c.id}
                  actions={
                    editable && (
                      <>
                        <Button variant="secondary" onClick={() => setEditing(c.id)}>
                          {t('catalog.edit')}
                        </Button>
                        <ConfirmButton disabled={remove.isPending} onConfirm={() => remove.mutate(c.id)}>
                          {t('catalog.delete')}
                        </ConfirmButton>
                      </>
                    )
                  }
                >
                  <span className="font-medium" style={{ paddingLeft: `${c.depth * 1.25}rem` }}>
                    {c.name}
                    <StatusTag status={c.status} />
                  </span>
                  <span className="text-xs text-ink-muted" style={{ paddingLeft: `${c.depth * 1.25}rem` }}>
                    {t('catalog.productCount', { count: c.product_count })}
                  </span>
                </Row>
              ),
            )}
          </ul>
        )}
      </Card>
    </div>
  )
}

function CategoryForm({ tenantId, parents, category, onDone }: { tenantId: string; parents: Category[]; category?: Category; onDone?: () => void }) {
  const { t } = useTranslation()
  const [name, setName] = useState(category?.name ?? '')
  const [parentId, setParentId] = useState(category?.parent_id ?? '')
  const [active, setActive] = useState((category?.status ?? 'active') === 'active')
  const save = useSave(
    tenantId,
    catalogKeys.categories(tenantId),
    () => {
      const input = {
        name,
        parent_id: parentId || null,
        sort_order: category?.sort_order ?? 0,
        status: active ? ('active' as const) : ('inactive' as const),
      }
      return category ? catalogApi.updateCategory(category.id, input) : catalogApi.createCategory(input)
    },
    () => {
      if (!category) setName('')
      onDone?.()
    },
  )
  const errors = fieldErrors(save.error, t)
  function onSubmit(e: FormEvent) {
    e.preventDefault()
    save.mutate(undefined)
  }
  const fields = (
    <form onSubmit={onSubmit} className="flex flex-col gap-3" noValidate>
      {!category && <h2 className="font-semibold">{t('catalog.newCategory')}</h2>}
      <div className="grid gap-3 sm:grid-cols-2">
        <TextField label={t('catalog.name')} value={name} onChange={(e) => setName(e.target.value)} maxLength={100} required error={errors.name} />
        <Select
          label={t('catalog.parent')}
          value={parentId}
          onChange={(e) => setParentId(e.target.value)}
          options={[{ value: '', label: t('catalog.topLevel') }, ...parents.map((p) => ({ value: p.id, label: categoryLabel(p) }))]}
          error={errors.parent_id}
        />
      </div>
      {category && <Checkbox label={t('catalog.active')} checked={active} onChange={(e) => setActive(e.target.checked)} />}
      <FormError error={save.error} />
      <div className="flex gap-2">
        <Button type="submit" disabled={save.isPending}>
          {category ? t('common.save') : t('catalog.add')}
        </Button>
        {onDone && (
          <Button variant="ghost" onClick={onDone}>
            {t('common.cancel')}
          </Button>
        )}
      </div>
    </form>
  )
  return category ? fields : <Card>{fields}</Card>
}

// --------------------------------------------------------------------- brands

function Brands({ tenantId, editable }: { tenantId: string; editable: boolean }) {
  const { t } = useTranslation()
  const brands = useBrands(tenantId)
  const [editing, setEditing] = useState<string | null>(null)
  const remove = useSave(tenantId, catalogKeys.brands(tenantId), catalogApi.deleteBrand)
  if (brands.isPending) return <Spinner />
  if (brands.isError) return <Alert tone="danger">{problemMessage(brands.error, t)}</Alert>
  return (
    <div className="flex flex-col gap-4">
      {editable && (
        <Card>
          <BrandForm tenantId={tenantId} />
        </Card>
      )}
      <Card>
        <h2 className="font-semibold">{t('products.tabs.brands')}</h2>
        {remove.isError && <FormError error={remove.error} />}
        {brands.data.length === 0 ? (
          <p className="mt-2 text-sm text-ink-muted">{t('catalog.noBrands')}</p>
        ) : (
          <ul>
            {brands.data.map((b) =>
              editing === b.id ? (
                <li key={b.id} className="border-b border-border py-3 last:border-b-0">
                  <BrandForm tenantId={tenantId} brand={b} onDone={() => setEditing(null)} />
                </li>
              ) : (
                <Row
                  key={b.id}
                  actions={
                    editable && (
                      <>
                        <Button variant="secondary" onClick={() => setEditing(b.id)}>
                          {t('catalog.edit')}
                        </Button>
                        <ConfirmButton disabled={remove.isPending} onConfirm={() => remove.mutate(b.id)}>
                          {t('catalog.delete')}
                        </ConfirmButton>
                      </>
                    )
                  }
                >
                  <span className="font-medium">
                    {b.name}
                    <StatusTag status={b.status} />
                  </span>
                  <span className="text-xs text-ink-muted">{t('catalog.productCount', { count: b.product_count })}</span>
                </Row>
              ),
            )}
          </ul>
        )}
      </Card>
    </div>
  )
}

function BrandForm({ tenantId, brand, onDone }: { tenantId: string; brand?: Brand; onDone?: () => void }) {
  const { t } = useTranslation()
  const [name, setName] = useState(brand?.name ?? '')
  const [active, setActive] = useState((brand?.status ?? 'active') === 'active')
  const save = useSave(
    tenantId,
    catalogKeys.brands(tenantId),
    () => {
      const input = { name, status: active ? ('active' as const) : ('inactive' as const) }
      return brand ? catalogApi.updateBrand(brand.id, input) : catalogApi.createBrand(input)
    },
    () => {
      if (!brand) setName('')
      onDone?.()
    },
  )
  const errors = fieldErrors(save.error, t)
  function onSubmit(e: FormEvent) {
    e.preventDefault()
    save.mutate(undefined)
  }
  return (
    <form onSubmit={onSubmit} className="flex flex-col gap-3" noValidate>
      {!brand && <h2 className="font-semibold">{t('catalog.newBrand')}</h2>}
      <TextField label={t('catalog.name')} value={name} onChange={(e) => setName(e.target.value)} maxLength={100} required error={errors.name} />
      {brand && <Checkbox label={t('catalog.active')} checked={active} onChange={(e) => setActive(e.target.checked)} />}
      <FormError error={save.error} />
      <div className="flex gap-2">
        <Button type="submit" disabled={save.isPending}>
          {brand ? t('common.save') : t('catalog.add')}
        </Button>
        {onDone && (
          <Button variant="ghost" onClick={onDone}>
            {t('common.cancel')}
          </Button>
        )}
      </div>
    </form>
  )
}

// ---------------------------------------------------------------------- units

function Units({ tenantId, editable }: { tenantId: string; editable: boolean }) {
  const { t } = useTranslation()
  const units = useUnits(tenantId)
  const [editing, setEditing] = useState<string | null>(null)
  if (units.isPending) return <Spinner />
  if (units.isError) return <Alert tone="danger">{problemMessage(units.error, t)}</Alert>
  return (
    <div className="flex flex-col gap-4">
      {editable && (
        <Card>
          <UnitForm tenantId={tenantId} />
        </Card>
      )}
      <Card>
        <h2 className="font-semibold">{t('products.tabs.units')}</h2>
        <ul>
          {units.data.map((u) =>
            editing === u.id ? (
              <li key={u.id} className="border-b border-border py-3 last:border-b-0">
                <UnitForm tenantId={tenantId} unit={u} onDone={() => setEditing(null)} />
              </li>
            ) : (
              <Row
                key={u.id}
                actions={
                  editable && (
                    <Button variant="secondary" onClick={() => setEditing(u.id)}>
                      {t('catalog.edit')}
                    </Button>
                  )
                }
              >
                <span className="font-medium">
                  {u.name} <span className="font-mono text-xs text-ink-muted">{u.short_name}</span>
                  <StatusTag status={u.status} />
                </span>
                <span className="text-xs text-ink-muted">{u.allow_fraction ? t('catalog.fractions') : t('catalog.wholeNumbers')}</span>
              </Row>
            ),
          )}
        </ul>
      </Card>
    </div>
  )
}

function UnitForm({ tenantId, unit, onDone }: { tenantId: string; unit?: Unit; onDone?: () => void }) {
  const { t } = useTranslation()
  const [name, setName] = useState(unit?.name ?? '')
  const [shortName, setShortName] = useState(unit?.short_name ?? '')
  const [fraction, setFraction] = useState(unit?.allow_fraction ?? false)
  const [active, setActive] = useState((unit?.status ?? 'active') === 'active')
  const save = useSave(
    tenantId,
    catalogKeys.units(tenantId),
    () => {
      const input = { name, short_name: shortName, allow_fraction: fraction, status: active ? ('active' as const) : ('inactive' as const) }
      return unit ? catalogApi.updateUnit(unit.id, input) : catalogApi.createUnit(input)
    },
    () => {
      if (!unit) {
        setName('')
        setShortName('')
        setFraction(false)
      }
      onDone?.()
    },
  )
  const errors = fieldErrors(save.error, t)
  function onSubmit(e: FormEvent) {
    e.preventDefault()
    save.mutate(undefined)
  }
  return (
    <form onSubmit={onSubmit} className="flex flex-col gap-3" noValidate>
      {!unit && <h2 className="font-semibold">{t('catalog.newUnit')}</h2>}
      <div className="grid gap-3 sm:grid-cols-2">
        <TextField label={t('catalog.name')} value={name} onChange={(e) => setName(e.target.value)} maxLength={50} required error={errors.name} />
        <TextField label={t('catalog.shortName')} value={shortName} onChange={(e) => setShortName(e.target.value)} maxLength={10} required error={errors.short_name} />
      </div>
      <Checkbox label={t('catalog.allowFraction')} hint={t('catalog.allowFractionHint')} checked={fraction} onChange={(e) => setFraction(e.target.checked)} />
      {unit && <Checkbox label={t('catalog.active')} checked={active} onChange={(e) => setActive(e.target.checked)} />}
      <FormError error={save.error} />
      <div className="flex gap-2">
        <Button type="submit" disabled={save.isPending}>
          {unit ? t('common.save') : t('catalog.add')}
        </Button>
        {onDone && (
          <Button variant="ghost" onClick={onDone}>
            {t('common.cancel')}
          </Button>
        )}
      </div>
    </form>
  )
}

// ------------------------------------------------------------------ tax rates

function TaxRates({ tenantId, editable }: { tenantId: string; editable: boolean }) {
  const { t } = useTranslation()
  const rates = useTaxRates(tenantId)
  const [editing, setEditing] = useState<string | null>(null)
  const makeDefault = useSave(tenantId, catalogKeys.taxRates(tenantId), (r: TaxRate) =>
    catalogApi.updateTaxRate(r.id, { name: r.name, rate_pct: r.rate_pct, status: r.status, is_default: true }),
  )
  if (rates.isPending) return <Spinner />
  if (rates.isError) return <Alert tone="danger">{problemMessage(rates.error, t)}</Alert>
  return (
    <div className="flex flex-col gap-4">
      <Alert>{t('catalog.taxHint')}</Alert>
      {editable && (
        <Card>
          <TaxRateForm tenantId={tenantId} />
        </Card>
      )}
      <Card>
        <h2 className="font-semibold">{t('products.tabs.taxRates')}</h2>
        {makeDefault.isError && <FormError error={makeDefault.error} />}
        <ul>
          {rates.data.map((r) =>
            editing === r.id ? (
              <li key={r.id} className="border-b border-border py-3 last:border-b-0">
                <TaxRateForm tenantId={tenantId} rate={r} onDone={() => setEditing(null)} />
              </li>
            ) : (
              <Row
                key={r.id}
                actions={
                  editable && (
                    <>
                      {!r.is_default && r.status === 'active' && (
                        <Button variant="secondary" disabled={makeDefault.isPending} onClick={() => makeDefault.mutate(r)}>
                          {t('catalog.makeDefault')}
                        </Button>
                      )}
                      <Button variant="secondary" onClick={() => setEditing(r.id)}>
                        {t('catalog.edit')}
                      </Button>
                    </>
                  )
                }
              >
                <span className="font-medium">
                  {r.name}
                  {r.is_default && <span className="ml-2 rounded bg-brand-50 px-2 py-0.5 text-xs font-semibold text-brand-700">{t('catalog.default')}</span>}
                  <StatusTag status={r.status} />
                </span>
                <span className="text-xs text-ink-muted">{trimDecimal(r.rate_pct)}%</span>
              </Row>
            ),
          )}
        </ul>
      </Card>
    </div>
  )
}

function TaxRateForm({ tenantId, rate, onDone }: { tenantId: string; rate?: TaxRate; onDone?: () => void }) {
  const { t } = useTranslation()
  const [name, setName] = useState(rate?.name ?? '')
  const [pct, setPct] = useState(rate ? trimDecimal(rate.rate_pct) : '')
  const [active, setActive] = useState((rate?.status ?? 'active') === 'active')
  const save = useSave(
    tenantId,
    catalogKeys.taxRates(tenantId),
    () => {
      const input = { name, rate_pct: pct.trim(), is_default: rate?.is_default ?? false, status: active ? ('active' as const) : ('inactive' as const) }
      return rate ? catalogApi.updateTaxRate(rate.id, input) : catalogApi.createTaxRate(input)
    },
    () => {
      if (!rate) {
        setName('')
        setPct('')
      }
      onDone?.()
    },
  )
  const errors = fieldErrors(save.error, t)
  function onSubmit(e: FormEvent) {
    e.preventDefault()
    save.mutate(undefined)
  }
  return (
    <form onSubmit={onSubmit} className="flex flex-col gap-3" noValidate>
      {!rate && <h2 className="font-semibold">{t('catalog.newTaxRate')}</h2>}
      <div className="grid gap-3 sm:grid-cols-2">
        <TextField label={t('catalog.name')} value={name} onChange={(e) => setName(e.target.value)} maxLength={50} required error={errors.name} />
        <TextField label={t('catalog.ratePct')} inputMode="decimal" value={pct} onChange={(e) => setPct(e.target.value)} required error={errors.rate_pct} />
      </div>
      {rate && !rate.is_default && <Checkbox label={t('catalog.active')} checked={active} onChange={(e) => setActive(e.target.checked)} />}
      <FormError error={save.error} />
      <div className="flex gap-2">
        <Button type="submit" disabled={save.isPending}>
          {rate ? t('common.save') : t('catalog.add')}
        </Button>
        {onDone && (
          <Button variant="ghost" onClick={onDone}>
            {t('common.cancel')}
          </Button>
        )}
      </div>
    </form>
  )
}
