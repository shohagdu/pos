import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type ChangeEvent, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { type Product, catalogApi, catalogKeys, useBrands, useCategories, useTaxRates, useUnits } from '../../api/catalog'
import { ApiError } from '../../api/client'
import { inventoryKeys } from '../../api/inventory'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { Spinner } from '../../components/ui/Spinner'
import { csvRecords, downloadCsvRows, readText } from '../../lib/csv'
import { problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { taxLabel, unitLabel } from '../products/labels'
import {
  type ImportRow,
  MAX_IMPORT_ROWS,
  type Masters,
  type RowField,
  blankRow,
  checkRow,
  findBrand,
  findCategory,
  openingStock,
  readRows,
  sampleRows,
  serverField,
  toInput,
} from './productImport'

const CELL =
  'min-h-9 w-full rounded-md border border-border bg-surface px-2 text-sm outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100 aria-invalid:border-danger aria-invalid:bg-danger/5'

type ServerErrors = Record<string, Partial<Record<RowField | 'row', string>>>

/** `/products/import`: products from a CSV file, checked and edited on screen before saving. */
export function ProductImportPage() {
  const { me } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const queries = [useUnits(tenantId), useTaxRates(tenantId), useCategories(tenantId), useBrands(tenantId)] as const
  if (queries.some((q) => q.isPending)) return <Spinner />
  const failed = queries.find((q) => q.isError)
  if (failed) return <Alert tone="danger">{failed.error?.message}</Alert>
  const [units, taxRates, categories, brands] = queries
  return (
    <ProductImport
      tenantId={tenantId}
      masters={{ units: units.data ?? [], taxRates: taxRates.data ?? [], categories: categories.data ?? [], brands: brands.data ?? [] }}
    />
  )
}

function ProductImport({ tenantId, masters }: { tenantId: string; masters: Masters }) {
  const { t } = useTranslation()
  const { can } = useSession()
  const queryClient = useQueryClient()
  const canViewCost = can('products.view_cost')
  const canStock = can('inventory.opening_stock')
  const fileRef = useRef<HTMLInputElement>(null)
  const [rows, setRows] = useState<ImportRow[]>([])
  const [fileName, setFileName] = useState('')
  const [notice, setNotice] = useState<{ tone: 'warning' | 'danger'; text: string } | null>(null)
  const [serverErrors, setServerErrors] = useState<ServerErrors>({})
  // Rows listed while "only problems" is on; fixed rows stay until it is turned off.
  const [problemKeys, setProblemKeys] = useState<Set<string> | null>(null)
  const [done, setDone] = useState<{ products: Product[]; stocked: number } | null>(null)

  const localErrors = Object.fromEntries(rows.map((r) => [r.key, checkRow(r, canViewCost, masters.units)]))
  const errorOf = (row: ImportRow, field: RowField): string | undefined => {
    const local = localErrors[row.key]?.[field]
    if (local) return t(`imports.errors.${local.key}`, local.values)
    return serverErrors[row.key]?.[field]
  }
  const rowHasProblem = (row: ImportRow) =>
    Object.keys(localErrors[row.key] ?? {}).length > 0 || Object.keys(serverErrors[row.key] ?? {}).length > 0
  const problemRows = rows.filter(rowHasProblem)
  const shown = problemKeys ? rows.filter((r) => problemKeys.has(r.key)) : rows

  const newCategories = unique(rows.map((r) => r.category).filter((c) => c.trim() && !findCategory(masters.categories, c)))
  const newBrands = unique(rows.map((r) => r.brand).filter((b) => b.trim() && !findBrand(masters.brands, b)))

  function downloadSample() {
    downloadCsvRows('products-sample', sampleRows(masters, canViewCost, canStock))
  }

  async function onFile(e: ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0]
    e.target.value = ''
    if (!file) return
    setDone(null)
    setServerErrors({})
    setProblemKeys(null)
    const { headers, records } = csvRecords(await readText(file))
    const result = readRows(headers, records, masters, canStock, canViewCost)
    if (result.missing.length > 0) {
      setNotice({ tone: 'danger', text: t('imports.missingColumns', { columns: result.missing.join(', ') }) })
      return
    }
    if (result.rows.length === 0) {
      setNotice({ tone: 'danger', text: t('imports.emptyFile') })
      return
    }
    if (result.rows.length > MAX_IMPORT_ROWS) {
      setNotice({ tone: 'danger', text: t('imports.tooManyRows', { max: MAX_IMPORT_ROWS, count: result.rows.length }) })
      return
    }
    setFileName(file.name)
    setRows(result.rows)
    setNotice(result.ignored.length > 0 ? { tone: 'warning', text: t('imports.ignoredColumns', { columns: result.ignored.join(', ') }) } : null)
  }

  function update(key: string, patch: Partial<ImportRow>) {
    setRows((rs) => rs.map((r) => (r.key === key ? { ...r, ...patch } : r)))
    // An edited cell is checked again on the next save.
    setServerErrors((all) => {
      const mine = all[key]
      if (!mine) return all
      const next = { ...mine }
      for (const f of Object.keys(patch)) delete next[f as RowField]
      delete next.row
      return { ...all, [key]: next }
    })
  }

  const save = useMutation({
    mutationFn: async () => {
      // Categories and brands named in the file but not set up yet are added first.
      const [cats, brs] = await Promise.all([catalogApi.categories(), catalogApi.brands()])
      for (const name of newCategories) {
        if (!findCategory(cats, name)) cats.push(await catalogApi.createCategory({ name: name.trim() }))
      }
      for (const name of newBrands) {
        if (!findBrand(brs, name)) brs.push(await catalogApi.createBrand({ name: name.trim() }))
      }
      const ids = { category: (n: string) => findCategory(cats, n)?.id, brand: (n: string) => findBrand(brs, n)?.id }
      const stock = canStock ? openingStock(rows, canViewCost) : []
      const products = await catalogApi.importProducts(rows.map((r) => toInput(r, ids, canViewCost)), stock)
      return { products, stocked: stock.length }
    },
    onSuccess: (created) => {
      setDone(created)
      setRows([])
      setServerErrors({})
      setNotice(null)
    },
    onError: (err) => {
      if (!(err instanceof ApiError)) return
      const next: ServerErrors = {}
      for (const e of err.problem.errors ?? []) {
        const at = serverField(e.field)
        const row = at ? rows[at.index] : undefined
        if (!at || !row) continue
        next[row.key] = { ...next[row.key], [at.field]: t(`problems.${e.code}`, { defaultValue: e.message }) }
      }
      setServerErrors(next)
      if (Object.keys(next).length > 0) setProblemKeys(new Set(Object.keys(next)))
    },
    onSettled: () => {
      void queryClient.invalidateQueries({ queryKey: catalogKeys.all(tenantId) })
      void queryClient.invalidateQueries({ queryKey: inventoryKeys.all(tenantId) })
    },
  })

  function onSave() {
    if (problemRows.length > 0) {
      setProblemKeys(new Set(problemRows.map((r) => r.key)))
      return
    }
    save.mutate()
  }

  const serverRowProblems = Object.values(serverErrors).filter((e) => Object.keys(e).length > 0).length
  const unmappedError = save.error && !(save.error instanceof ApiError && (save.error.problem.errors?.length ?? 0) > 0)

  return (
    <div className="flex flex-col gap-6">
      <Link to="/products" className="text-sm font-medium text-brand-700 hover:underline">
        ← {t('products.title')}
      </Link>
      <div>
        <h1 className="text-2xl font-bold">{t('imports.products.title')}</h1>
        <p className="text-sm text-ink-muted">{t('imports.products.subtitle')}</p>
      </div>

      {done && (
        <Alert tone="success">
          <div className="flex flex-wrap items-center gap-3">
            <span className="font-semibold">
              {t('imports.products.done', { count: done.products.length })}
              {done.stocked > 0 && ` ${t('imports.products.stocked', { count: done.stocked })}`}
            </span>
            <Link to="/products" className="font-semibold underline">
              {t('imports.products.viewProducts')}
            </Link>
            {done.products.some((p) => p.track_inventory) && canStock && (
              <Link to="/inventory/adjustments/new?reason=opening" className="font-semibold underline">
                {t('imports.products.enterOpeningStock')}
              </Link>
            )}
          </div>
        </Alert>
      )}

      <Card className="flex flex-col gap-4">
        <ol className="flex flex-col gap-1 text-sm text-ink-muted">
          <li>{t('imports.steps.download')}</li>
          <li>{t('imports.steps.fill')}</li>
          <li>{t('imports.steps.upload')}</li>
        </ol>
        <div className="flex flex-wrap gap-2">
          <Button variant="secondary" onClick={downloadSample}>
            {t('imports.downloadSample')}
          </Button>
          <Button onClick={() => fileRef.current?.click()}>{t('imports.chooseFile')}</Button>
          <input ref={fileRef} type="file" accept=".csv,text/csv" className="hidden" aria-label={t('imports.chooseFile')} onChange={(e) => void onFile(e)} />
        </div>
        <details className="text-sm">
          <summary className="cursor-pointer font-medium text-brand-700">{t('imports.products.columnsTitle')}</summary>
          <ul className="mt-2 flex list-disc flex-col gap-1 pl-5 text-ink-muted">
            <li>{t('imports.products.columns.required')}</li>
            <li>{t('imports.products.columns.sku')}</li>
            <li>{t('imports.products.columns.barcode')}</li>
            <li>{t('imports.products.columns.masters')}</li>
            <li>{t('imports.products.columns.tax')}</li>
            <li>{t('imports.products.columns.type')}</li>
            {canStock && <li>{t('imports.products.columns.openingStock')}</li>}
            {canStock && canViewCost && <li>{t('imports.products.columns.openingCost')}</li>}
          </ul>
        </details>
      </Card>

      {notice && <Alert tone={notice.tone}>{notice.text}</Alert>}

      {rows.length > 0 && (
        <Card className="flex flex-col gap-4">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div>
              <h2 className="font-semibold">{t('imports.preview', { name: fileName })}</h2>
              <p className="text-sm text-ink-muted">{t('imports.products.rowCount', { count: rows.length })}</p>
            </div>
            <div className="flex flex-wrap items-center gap-2">
              {(problemRows.length > 0 || problemKeys) && (
                <label className="flex items-center gap-2 text-sm">
                  <input
                    type="checkbox"
                    checked={problemKeys !== null}
                    onChange={(e) => setProblemKeys(e.target.checked ? new Set(problemRows.map((r) => r.key)) : null)}
                  />
                  {t('imports.onlyProblems', { count: problemKeys?.size ?? problemRows.length })}
                </label>
              )}
              <Button variant="ghost" className="min-h-9" onClick={() => {
                  setProblemKeys(null)
                  setRows((rs) => [...rs, blankRow(masters)])
                }}>
                {t('imports.addRow')}
              </Button>
            </div>
          </div>

          {problemRows.length > 0 && <Alert tone="warning">{t('imports.fixProblems', { count: problemRows.length })}</Alert>}
          {serverRowProblems > 0 && <Alert tone="danger">{t('imports.serverProblems', { count: serverRowProblems })}</Alert>}
          {unmappedError && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}
          {(newCategories.length > 0 || newBrands.length > 0) && (
            <Alert>
              {[
                newCategories.length > 0 ? t('imports.products.newCategories', { names: newCategories.join(', ') }) : '',
                newBrands.length > 0 ? t('imports.products.newBrands', { names: newBrands.join(', ') }) : '',
              ]
                .filter(Boolean)
                .join(' ')}
            </Alert>
          )}

          <datalist id="import-categories">
            {masters.categories.filter((c) => c.status === 'active').map((c) => (
              <option key={c.id} value={c.name} />
            ))}
          </datalist>
          <datalist id="import-brands">
            {masters.brands.filter((b) => b.status === 'active').map((b) => (
              <option key={b.id} value={b.name} />
            ))}
          </datalist>

          <div className="max-h-[70vh] overflow-auto rounded-lg border border-border">
            <table className="min-w-max text-sm">
              <thead className="sticky top-0 z-10 bg-canvas text-xs text-ink-muted uppercase">
                <tr>
                  <th scope="col" className="px-2 py-2 text-left">{t('imports.line')}</th>
                  <th scope="col" className="px-2 py-2 text-left">{t('products.name')} *</th>
                  <th scope="col" className="px-2 py-2 text-left">{t('imports.products.nameBn')}</th>
                  <th scope="col" className="px-2 py-2 text-left">{t('products.sku')}</th>
                  <th scope="col" className="px-2 py-2 text-left">{t('products.barcodes')}</th>
                  <th scope="col" className="px-2 py-2 text-left">{t('products.category')}</th>
                  <th scope="col" className="px-2 py-2 text-left">{t('products.brand')}</th>
                  <th scope="col" className="px-2 py-2 text-left">{t('products.unit')} *</th>
                  <th scope="col" className="px-2 py-2 text-left">{t('products.taxRate')}</th>
                  {canViewCost && <th scope="col" className="px-2 py-2 text-left">{t('imports.products.cost')}</th>}
                  <th scope="col" className="px-2 py-2 text-left">{t('imports.products.price')} *</th>
                  <th scope="col" className="px-2 py-2 text-left">{t('imports.products.wholesale')}</th>
                  <th scope="col" className="px-2 py-2 text-left">{t('imports.products.minPrice')}</th>
                  <th scope="col" className="px-2 py-2 text-left">{t('imports.products.reorder')}</th>
                  <th scope="col" className="px-2 py-2 text-left">{t('products.type')}</th>
                  <th scope="col" className="px-2 py-2 text-left">{t('imports.products.track')}</th>
                  {canStock && <th scope="col" className="px-2 py-2 text-left">{t('imports.products.openingStock')}</th>}
                  {canStock && canViewCost && <th scope="col" className="px-2 py-2 text-left">{t('imports.products.openingCost')}</th>}
                  <th scope="col" className="px-2 py-2">
                    <span className="sr-only">{t('imports.remove')}</span>
                  </th>
                </tr>
              </thead>
              <tbody>
                {shown.map((row) => {
                  const label = (col: string) => t('imports.cellLabel', { column: col, line: row.line || t('imports.newRow') })
                  const text = (field: RowField, col: string, extra: { placeholder?: string; list?: string; width?: string; inputMode?: 'decimal' } = {}) => {
                    const error = errorOf(row, field)
                    return (
                      <td className="px-1 py-1 align-top">
                        <input
                          aria-label={label(col)}
                          aria-invalid={error ? true : undefined}
                          title={error}
                          value={String(row[field])}
                          placeholder={extra.placeholder}
                          list={extra.list}
                          inputMode={extra.inputMode}
                          onChange={(e) => update(row.key, { [field]: e.target.value })}
                          className={`${CELL} ${extra.width ?? 'w-28'}`}
                        />
                        {error && <p className="mt-0.5 max-w-40 text-xs text-danger">{error}</p>}
                      </td>
                    )
                  }
                  const unitError = errorOf(row, 'unit_id')
                  const taxError = errorOf(row, 'tax_rate_id')
                  const rowError = serverErrors[row.key]?.row
                  return (
                    <tr key={row.key} className={`border-t border-border ${rowHasProblem(row) ? 'bg-danger/5' : ''}`}>
                      <td className="px-2 py-1 align-top text-ink-muted tabular-nums">
                        {row.line || '+'}
                        {rowError && <p className="max-w-40 text-xs text-danger">{rowError}</p>}
                      </td>
                      {text('name', t('products.name'), { width: 'w-52' })}
                      {text('name_bn', t('imports.products.nameBn'), { width: 'w-40' })}
                      {text('sku', t('products.sku'), { placeholder: t('imports.auto') })}
                      {text('barcodes', t('products.barcodes'), { placeholder: t('imports.auto'), width: 'w-36' })}
                      {text('category', t('products.category'), { list: 'import-categories' })}
                      {text('brand', t('products.brand'), { list: 'import-brands' })}
                      <td className="px-1 py-1 align-top">
                        <select
                          aria-label={label(t('products.unit'))}
                          aria-invalid={unitError ? true : undefined}
                          value={row.unit_id}
                          onChange={(e) => update(row.key, { unit_id: e.target.value })}
                          className={`${CELL} w-28`}
                        >
                          {!row.unit_id && <option value="">{t('products.chooseUnit')}</option>}
                          {masters.units.filter((u) => u.status === 'active').map((u) => (
                            <option key={u.id} value={u.id}>
                              {unitLabel(u)}
                            </option>
                          ))}
                        </select>
                        {unitError && <p className="mt-0.5 max-w-40 text-xs text-danger">{unitError}</p>}
                      </td>
                      <td className="px-1 py-1 align-top">
                        <select
                          aria-label={label(t('products.taxRate'))}
                          aria-invalid={taxError ? true : undefined}
                          value={row.tax_rate_id}
                          onChange={(e) => update(row.key, { tax_rate_id: e.target.value, unknownTax: undefined })}
                          className={`${CELL} w-32`}
                        >
                          <option value="">{t('products.defaultTax')}</option>
                          {masters.taxRates.filter((r) => r.status === 'active').map((r) => (
                            <option key={r.id} value={r.id}>
                              {taxLabel(r)}
                            </option>
                          ))}
                        </select>
                        {taxError && <p className="mt-0.5 max-w-40 text-xs text-danger">{taxError}</p>}
                      </td>
                      {canViewCost && text('purchase_price', t('imports.products.cost'), { inputMode: 'decimal', width: 'w-24' })}
                      {text('selling_price', t('imports.products.price'), { inputMode: 'decimal', width: 'w-24' })}
                      {text('wholesale_price', t('imports.products.wholesale'), { inputMode: 'decimal', width: 'w-24' })}
                      {text('min_selling_price', t('imports.products.minPrice'), { inputMode: 'decimal', width: 'w-24' })}
                      {text('reorder_level', t('imports.products.reorder'), { inputMode: 'decimal', width: 'w-20' })}
                      <td className="px-1 py-1 align-top">
                        <select
                          aria-label={label(t('products.type'))}
                          value={row.service ? 'service' : 'standard'}
                          onChange={(e) => {
                            const service = e.target.value === 'service'
                            update(row.key, { service, track_inventory: !service })
                          }}
                          className={`${CELL} w-32`}
                        >
                          <option value="standard">{t('products.types.standard')}</option>
                          <option value="service">{t('products.types.service')}</option>
                        </select>
                      </td>
                      <td className="px-2 py-1 text-center align-top">
                        <input
                          type="checkbox"
                          aria-label={label(t('imports.products.track'))}
                          checked={row.track_inventory}
                          disabled={row.service}
                          onChange={(e) => update(row.key, { track_inventory: e.target.checked })}
                          className="mt-2.5 size-4"
                        />
                      </td>
                      {canStock && text('opening_stock', t('imports.products.openingStock'), { inputMode: 'decimal', width: 'w-24' })}
                      {canStock &&
                        canViewCost &&
                        text('opening_cost', t('imports.products.openingCost'), {
                          inputMode: 'decimal',
                          width: 'w-24',
                          placeholder: row.purchase_price.trim() || undefined,
                        })}
                      <td className="px-1 py-1 align-top">
                        <button
                          type="button"
                          aria-label={t('imports.removeRow', { line: row.line || t('imports.newRow') })}
                          onClick={() => {
                            setRows((rs) => rs.filter((r) => r.key !== row.key))
                            setServerErrors(({ [row.key]: _gone, ...rest }) => rest)
                          }}
                          className="min-h-9 rounded-md px-2 text-ink-muted hover:bg-canvas hover:text-danger"
                        >
                          ✕
                        </button>
                      </td>
                    </tr>
                  )
                })}
              </tbody>
            </table>
          </div>

          <div className="flex flex-wrap items-center justify-between gap-3">
            <Button
              variant="ghost"
              onClick={() => {
                setRows([])
                setServerErrors({})
                setNotice(null)
              }}
            >
              {t('imports.clear')}
            </Button>
            <Button onClick={onSave} disabled={save.isPending}>
              {save.isPending ? t('imports.saving') : t('imports.products.save', { count: rows.length })}
            </Button>
          </div>
        </Card>
      )}
    </div>
  )
}

function unique(names: string[]): string[] {
  const seen = new Map<string, string>()
  for (const n of names) {
    const key = n.trim().toLowerCase()
    if (!seen.has(key)) seen.set(key, n.trim())
  }
  return [...seen.values()]
}
