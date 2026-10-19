import { useQueryClient } from '@tanstack/react-query'
import { type KeyboardEvent, type RefObject, useId, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { type PosItem, salesApi, salesKeys, usePosProducts } from '../../api/sales'
import { formatMoney, formatQuantity } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { useDebounced } from './hooks'

/**
 * Product search / barcode box (POS-R02). A scan (exact barcode or SKU
 * match) followed by Enter adds the product at once and clears the box.
 */
export function ProductSearch({
  tenantId,
  onAdd,
  inputRef,
}: {
  tenantId: string
  onAdd: (item: PosItem) => void
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
  const results = usePosProducts(tenantId, q, q.length > 0)
  const items = text.trim() && results.data ? results.data : []
  const open = items.length > 0

  function add(item: PosItem) {
    onAdd(item)
    setText('')
    setActive(0)
    setNotFound(null)
    inputRef.current?.focus()
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
      const found = await queryClient.fetchQuery({
        queryKey: salesKeys.posProducts(tenantId, value),
        queryFn: () => salesApi.posProducts(value),
        staleTime: 10_000,
      })
      setError(null)
      const first = found[0]
      if (first && (first.exact || found.length === 1)) add(first)
      else if (!first) setNotFound(value)
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
        {t('pos.search')}
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
        placeholder={t('pos.searchHint')}
        onChange={(e) => {
          setText(e.target.value)
          setActive(0)
          setNotFound(null)
        }}
        onKeyDown={onKeyDown}
        className="min-h-12 w-full rounded-lg border border-border bg-surface px-4 text-base outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100"
      />
      {notFound && <p className="mt-1 text-sm text-danger">{t('pos.notFound', { code: notFound })}</p>}
      {error !== null && <p className="mt-1 text-sm text-danger">{problemMessage(error, t)}</p>}
      {text.trim() && q && results.isSuccess && results.data.length === 0 && !notFound && (
        <p className="mt-1 text-sm text-ink-muted">{t('common.noResults')}</p>
      )}
      {open && (
        <ul
          id={listId}
          role="listbox"
          aria-label={t('pos.results')}
          className="absolute z-20 mt-1 max-h-80 w-full overflow-y-auto rounded-lg border border-border bg-surface shadow-lg"
        >
          {items.map((item, i) => (
            <li
              key={item.variant_id}
              id={`${listId}-${i}`}
              role="option"
              aria-selected={i === active}
              onMouseDown={(e) => e.preventDefault()}
              onClick={() => add(item)}
              className={`flex cursor-pointer items-center justify-between gap-3 px-3 py-2 text-sm ${i === active ? 'bg-brand-50' : 'hover:bg-canvas'}`}
            >
              <span className="min-w-0">
                <span className="block truncate font-medium">{lang === 'bn' && item.name_bn ? item.name_bn : item.name}</span>
                <span className="block truncate text-xs text-ink-muted">
                  {item.sku}
                  {item.barcode && ` · ${item.barcode}`}
                </span>
              </span>
              <span className="shrink-0 text-right">
                <span className="block font-semibold tabular-nums">{formatMoney(item.selling_price, lang)}</span>
                {item.stock !== null && (
                  <span className="block text-xs text-ink-muted">
                    {t('pos.stock', { qty: formatQuantity(item.stock, lang, item.allow_fraction), unit: item.unit_short_name })}
                  </span>
                )}
              </span>
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}
