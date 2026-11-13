import { type KeyboardEvent, useId, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { type PosItem, usePosCatalog, usePosCategories } from '../../api/sales'
import { Button } from '../../components/ui/Button'
import { Spinner } from '../../components/ui/Spinner'
import { dec } from '../../lib/decimal'
import { formatMoney, formatQuantity } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { ProductThumb } from '../products/ProductThumb'
import { useDebounced } from './hooks'

type StockState = 'untracked' | 'out' | 'low' | 'ok'

function stockState(item: PosItem): StockState {
  if (item.stock === null) return 'untracked'
  const stock = dec(item.stock)
  if (stock <= dec(0)) return 'out'
  if (item.reorder_level != null && stock <= dec(item.reorder_level)) return 'low'
  return 'ok'
}

const DOT: Record<StockState, string> = {
  untracked: 'bg-ink-muted/40',
  out: 'bg-danger',
  low: 'bg-warning',
  ok: 'bg-success',
}

/**
 * The POS product panel: a search box and category chips that stay put, over
 * a two-column list of compact cards that scrolls on its own: the name across
 * the card, then an optional thumbnail beside the SKU, price and stock. A tap adds one to the cart, the same as a scan; cards
 * already in the cart are ringed and show how many.
 */
export function ProductGrid({
  tenantId,
  onAdd,
  inCart,
  allowNegativeStock,
  showImages = true,
}: {
  tenantId: string
  onAdd: (item: PosItem) => void
  /** Quantity in the cart per variant id. */
  inCart: Record<string, string>
  allowNegativeStock: boolean
  /** Thumbnails on the cards (Settings → Invoice, tax & POS). */
  showImages?: boolean
}) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const searchId = useId()
  const searchRef = useRef<HTMLInputElement>(null)
  const [text, setText] = useState('')
  const q = useDebounced(text.trim(), 250)
  const [categoryId, setCategoryId] = useState<string | null>(null)
  const categories = usePosCategories(tenantId)
  const catalog = usePosCatalog(tenantId, categoryId, q)
  const items = catalog.data?.pages.flat() ?? []
  const chips = categories.data ?? []

  function stockLabel(item: PosItem, state: StockState): string {
    if (state === 'untracked' || item.stock === null) return t('pos.notTracked')
    if (state === 'out') return t('pos.outOfStock')
    const qty = formatQuantity(item.stock, lang, item.allow_fraction)
    return state === 'low' ? t('pos.lowStock', { qty, unit: item.unit_short_name }) : t('pos.inStock', { qty, unit: item.unit_short_name })
  }

  function clear() {
    setText('')
    searchRef.current?.focus()
  }

  // The POS screen listens for shortcuts on `window`. Keys that mean
  // something while editing text stay in this box: Esc clears it, and
  // Ctrl+Delete deletes a word instead of removing the selected cart line.
  // Function keys (F2, F8, …) still reach the screen.
  function onKeyDown(e: KeyboardEvent<HTMLInputElement>) {
    if (e.key === 'Escape' && text) {
      e.preventDefault()
      e.stopPropagation()
      setText('')
    } else if (e.key === 'Delete' && e.ctrlKey) {
      e.stopPropagation()
    }
  }

  return (
    <section aria-labelledby="pos-products-heading" className="flex flex-col rounded-xl border border-border bg-surface lg:min-h-0 lg:flex-1">
      <div className="flex flex-col gap-2 border-b border-border px-3 pt-2.5 pb-2">
        <div className="flex items-center gap-3">
          <h2 id="pos-products-heading" className="shrink-0 text-sm font-semibold">
            {t('pos.products')}
          </h2>
          <div className="relative min-w-0 flex-1">
            <label htmlFor={searchId} className="sr-only">
              {t('pos.filterProducts')}
            </label>
            <svg
              aria-hidden="true"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth={2}
              strokeLinecap="round"
              className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-ink-muted"
            >
              <path d="M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14zM20 20l-4-4" />
            </svg>
            <input
              ref={searchRef}
              id={searchId}
              type="search"
              autoComplete="off"
              spellCheck={false}
              value={text}
              placeholder={t('pos.filterProductsHint')}
              onChange={(e) => setText(e.target.value)}
              onKeyDown={onKeyDown}
              className="min-h-9 w-full rounded-lg border border-border bg-surface pr-9 pl-8 text-sm outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100 [&::-webkit-search-cancel-button]:hidden"
            />
            {text && (
              <button
                type="button"
                aria-label={t('pos.clearFilter')}
                onClick={clear}
                className="absolute top-1/2 right-1 flex size-8 -translate-y-1/2 items-center justify-center rounded-md text-ink-muted hover:bg-canvas hover:text-ink focus-visible:ring-2 focus-visible:ring-brand-500 focus-visible:outline-none"
              >
                <span aria-hidden="true">✕</span>
              </button>
            )}
          </div>
        </div>
        {chips.length > 0 && (
          <div role="group" aria-label={t('pos.categoryFilter')} className="-mx-3 flex gap-1.5 overflow-x-auto px-3 pb-0.5 [scrollbar-width:thin]">
            {[{ id: null, name: t('pos.allCategories'), product_count: null }, ...chips].map((c) => {
              const on = categoryId === c.id
              return (
                <button
                  key={c.id ?? 'all'}
                  type="button"
                  aria-pressed={on}
                  aria-label={c.product_count === null ? undefined : t('pos.categoryChip', { name: c.name, count: c.product_count })}
                  onClick={() => setCategoryId(c.id)}
                  className={`flex min-h-8 shrink-0 items-center gap-1.5 rounded-full border px-3 text-sm whitespace-nowrap transition-colors focus-visible:ring-2 focus-visible:ring-brand-100 focus-visible:outline-none ${
                    on ? 'border-brand-600 bg-brand-600 font-medium text-white' : 'border-border bg-surface text-ink hover:border-brand-500'
                  }`}
                >
                  {c.name}
                  {c.product_count !== null && (
                    <span className={`text-xs tabular-nums ${on ? 'text-white/80' : 'text-ink-muted'}`}>{c.product_count}</span>
                  )}
                </button>
              )
            })}
          </div>
        )}
      </div>

      <div className="@container max-h-[55svh] overflow-y-auto p-2 lg:max-h-none lg:min-h-0 lg:flex-1">
        {catalog.isPending ? (
          <ul aria-hidden="true" className={gridClass}>
            {Array.from({ length: 8 }, (_, i) => (
              <li key={i} className="flex flex-col gap-2 rounded-lg border border-border p-1.5">
                <span className="h-3 w-4/5 rounded bg-canvas motion-safe:animate-pulse" />
                <span className="flex items-center gap-2">
                  {showImages && <span className="size-12 shrink-0 rounded-md bg-canvas motion-safe:animate-pulse" />}
                  <span className="flex flex-1 flex-col gap-1.5">
                    <span className="h-2.5 w-1/2 rounded bg-canvas motion-safe:animate-pulse" />
                    <span className="h-3 w-2/3 rounded bg-canvas motion-safe:animate-pulse" />
                  </span>
                </span>
              </li>
            ))}
          </ul>
        ) : catalog.isError ? (
          <div className="flex flex-col items-center gap-3 p-6 text-center text-sm">
            <p className="text-danger">{problemMessage(catalog.error, t)}</p>
            <Button variant="secondary" onClick={() => void catalog.refetch()}>
              {t('common.retry')}
            </Button>
          </div>
        ) : items.length === 0 ? (
          <div className="flex flex-col items-center gap-2 p-8 text-center text-sm text-ink-muted">
            <p role="status">{q ? t('pos.noProductsFound') : categoryId ? t('pos.noProductsInCategory') : t('pos.noProducts')}</p>
            {q && (
              <button type="button" onClick={clear} className="font-medium text-brand-700 hover:underline">
                {t('pos.clearFilter')}
              </button>
            )}
          </div>
        ) : (
          <>
            <ul aria-busy={catalog.isPlaceholderData || undefined} className={`${gridClass} transition-opacity ${catalog.isPlaceholderData ? 'opacity-60' : ''}`}>
              {items.map((item) => {
                const name = lang === 'bn' && item.name_bn ? item.name_bn : item.name
                const state = stockState(item)
                const blocked = state === 'out' && !allowNegativeStock
                const qty = inCart[item.variant_id]
                return (
                  <li key={item.variant_id}>
                    <button
                      type="button"
                      aria-label={t('pos.addItem', { name })}
                      aria-describedby={`stock-${item.variant_id}`}
                      disabled={blocked}
                      onClick={() => onAdd(item)}
                      className={`relative flex h-full w-full flex-col gap-1.5 rounded-lg border p-1.5 text-left transition-[border-color,background-color,transform] duration-100 focus-visible:ring-2 focus-visible:ring-brand-500 focus-visible:ring-offset-1 focus-visible:outline-none enabled:active:scale-[0.97] disabled:cursor-not-allowed disabled:opacity-65 ${
                        qty ? 'border-brand-500 bg-brand-50 ring-1 ring-brand-500' : 'border-border bg-surface enabled:hover:border-brand-500'
                      }`}
                    >
                      {/* Full width, so long names never squeeze the details. */}
                      <span className={`line-clamp-2 text-[0.8125rem] leading-tight font-medium ${qty ? 'pr-5' : ''}`}>{name}</span>
                      {qty && (
                        <span className="absolute top-1.5 right-1.5 flex h-5 min-w-5 items-center justify-center rounded-full bg-brand-600 px-1 text-[0.6875rem] font-bold text-white tabular-nums">
                          <span className="sr-only">{t('pos.inCart', { qty: formatQuantity(qty, lang, item.allow_fraction) })}</span>
                          <span aria-hidden="true">{formatQuantity(qty, lang, item.allow_fraction)}</span>
                        </span>
                      )}
                      <span className="mt-auto flex items-center gap-1.5">
                        {showImages && <ProductThumb fileId={item.image_file_id} size="card" className="size-12" />}
                        <span className="flex min-w-0 flex-1 flex-col gap-0.5">
                          <span className="truncate text-[0.6875rem] leading-tight text-ink-muted">{item.sku}</span>
                          <span className="text-[0.8125rem] leading-tight font-bold whitespace-nowrap text-ink tabular-nums">
                            {formatMoney(item.selling_price, lang)}
                          </span>
                          <span
                            id={`stock-${item.variant_id}`}
                            className={`flex items-start gap-1.5 text-[0.6875rem] leading-tight ${state === 'out' ? 'font-medium text-danger' : 'text-ink-muted'}`}
                          >
                            <span aria-hidden="true" className={`mt-[0.3em] size-1.5 shrink-0 rounded-full ${DOT[state]}`} />
                            {/* Wraps rather than cutting "Out of stock" short on narrow cards. */}
                            <span className="min-w-0">{stockLabel(item, state)}</span>
                          </span>
                        </span>
                      </span>
                    </button>
                  </li>
                )
              })}
            </ul>
            {catalog.hasNextPage && (
              <div className="mt-3 flex justify-center">
                <Button variant="secondary" disabled={catalog.isFetchingNextPage} onClick={() => void catalog.fetchNextPage()}>
                  {catalog.isFetchingNextPage ? <Spinner /> : t('common.loadMore')}
                </Button>
              </div>
            )}
          </>
        )}
      </div>
    </section>
  )
}

// Two columns of cards; one when the panel is very narrow.
const gridClass = 'grid grid-cols-1 gap-2 @[17rem]:grid-cols-2'
