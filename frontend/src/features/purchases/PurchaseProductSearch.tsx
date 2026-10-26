import { useQuery, useQueryClient } from '@tanstack/react-query'
import { type KeyboardEvent, type RefObject, useId, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { type Product, catalogApi } from '../../api/catalog'
import { formatMoney } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { useDebounced } from '../pos/hooks'

const MAX_RESULTS = 10
const filter = (q: string) => ({ q, status: 'active' as const })
const pickerKey = (tenantId: string, q: string) => ['catalog', tenantId, 'products', 'purchase-picker', q] as const

/** A scan matches one product's barcode or SKU exactly. */
const exactMatch = (items: Product[], code: string) =>
  items.find((p) => p.sku.toLowerCase() === code.toLowerCase() || p.barcodes.includes(code))

/**
 * Product search / barcode box for a purchase bill (F2), like the POS one:
 * results show as you type, arrows pick, Enter adds; a scanned barcode or
 * SKU followed by Enter adds the product at once.
 */
export function PurchaseProductSearch({
  tenantId,
  onAdd,
  inputRef,
}: {
  tenantId: string
  onAdd: (product: Product) => void
  inputRef: RefObject<HTMLInputElement | null>
}) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const queryClient = useQueryClient()
  const listId = useId()
  const [text, setText] = useState('')
  const [active, setActive] = useState(0)
  const [notFound, setNotFound] = useState<string | null>(null)
  const [error, setError] = useState<unknown>(null)
  const q = useDebounced(text.trim(), 200)
  const results = useQuery({
    queryKey: pickerKey(tenantId, q),
    queryFn: () => catalogApi.products(filter(q), null),
    enabled: q.length > 0,
  })
  const items = text.trim() && results.data ? results.data.data.slice(0, MAX_RESULTS) : []
  const open = items.length > 0

  function add(p: Product) {
    if (!p.track_inventory) return
    onAdd(p)
    setText('')
    setActive(0)
    setNotFound(null)
  }

  async function onEnter() {
    const value = text.trim()
    if (!value) return
    if (q === value && items[active]) {
      add(items[active])
      return
    }
    // A scanner types faster than the debounce: look the code up now.
    try {
      const found = (
        await queryClient.fetchQuery({
          queryKey: pickerKey(tenantId, value),
          queryFn: () => catalogApi.products(filter(value), null),
          staleTime: 10_000,
        })
      ).data
      setError(null)
      const hit = exactMatch(found, value) ?? (found.length === 1 ? found[0] : undefined)
      if (hit) add(hit)
      else if (found.length === 0) setNotFound(value)
    } catch (err) {
      setError(err)
    }
  }

  function onKeyDown(e: KeyboardEvent<HTMLInputElement>) {
    if (e.key === 'Enter') {
      e.preventDefault()
      void onEnter()
    } else if (e.key === 'ArrowDown' && open) {
      e.preventDefault()
      setActive((a) => Math.min(a + 1, items.length - 1))
    } else if (e.key === 'ArrowUp' && open) {
      e.preventDefault()
      setActive((a) => Math.max(a - 1, 0))
    } else if (e.key === 'Escape' && text) {
      e.preventDefault()
      setText('')
    }
  }

  return (
    <div className="relative">
      <label htmlFor={`${listId}-input`} className="sr-only">
        {t('purchases.form.findProduct')}
      </label>
      <input
        ref={inputRef}
        id={`${listId}-input`}
        type="search"
        role="combobox"
        aria-expanded={open}
        aria-controls={listId}
        aria-autocomplete="list"
        aria-activedescendant={open ? `${listId}-${active}` : undefined}
        autoFocus
        autoComplete="off"
        value={text}
        placeholder={t('purchases.form.findProductHint')}
        onChange={(e) => {
          setText(e.target.value)
          setActive(0)
          setNotFound(null)
        }}
        onKeyDown={onKeyDown}
        className="min-h-12 w-full rounded-lg border border-border bg-surface px-4 text-base outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100"
      />
      {notFound && <p className="mt-1 text-sm text-danger">{t('purchases.form.notFound', { code: notFound })}</p>}
      {error !== null && <p className="mt-1 text-sm text-danger">{problemMessage(error, t)}</p>}
      {text.trim() && q && results.isSuccess && results.data.data.length === 0 && !notFound && (
        <p className="mt-1 text-sm text-ink-muted">{t('common.noResults')}</p>
      )}
      {open && (
        <ul
          id={listId}
          role="listbox"
          aria-label={t('purchases.form.results')}
          className="absolute z-20 mt-1 max-h-80 w-full overflow-y-auto rounded-lg border border-border bg-surface shadow-lg"
        >
          {items.map((p, i) => (
            <li
              key={p.variant_id}
              id={`${listId}-${i}`}
              role="option"
              aria-selected={i === active}
              aria-disabled={!p.track_inventory || undefined}
              onMouseDown={(e) => e.preventDefault()}
              onMouseEnter={() => setActive(i)}
              onClick={() => add(p)}
              className={`flex items-center justify-between gap-3 px-3 py-2 text-sm ${
                p.track_inventory ? 'cursor-pointer' : 'cursor-not-allowed opacity-60'
              } ${i === active ? 'bg-brand-50' : ''}`}
            >
              <span className="min-w-0">
                <span className="block truncate font-medium">{lang === 'bn' && p.name_bn ? p.name_bn : p.name}</span>
                <span className="block truncate text-xs text-ink-muted">
                  {p.sku}
                  {p.barcodes[0] && ` · ${p.barcodes[0]}`}
                </span>
              </span>
              <span className="shrink-0 text-right text-xs text-ink-muted">
                {!p.track_inventory ? (
                  t('purchases.form.serviceNotStocked')
                ) : (
                  p.purchase_price !== null && (
                    <>
                      {t('purchases.form.currentCost')}
                      <span className="block text-sm font-semibold text-ink tabular-nums">
                        {formatMoney(p.purchase_price, lang)} / {p.unit_short_name}
                      </span>
                    </>
                  )
                )}
              </span>
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}
