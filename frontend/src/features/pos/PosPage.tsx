import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type KeyboardEvent as ReactKeyboardEvent, useCallback, useEffect, useMemo, useReducer, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { ApiError } from '../../api/client'
import { usePaymentMethods } from '../../api/money'
import { newKey } from '../../api/query'
import {
  type HeldCart,
  type PosContext,
  type PosItem,
  type SaleDetail,
  type SaleInput,
  salesApi,
  salesKeys,
  useHeldCarts,
  usePosContext,
} from '../../api/sales'
import { LanguageSwitcher } from '../../components/LanguageSwitcher'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { ConfirmButton } from '../../components/ui/ConfirmButton'
import { Spinner } from '../../components/ui/Spinner'
import { ZERO, toFixed } from '../../lib/decimal'
import { formatMoney } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { ShiftIndicator } from '../registers/ShiftIndicator'
import { PrintOnly } from '../sales/print'
import { SaleReceipt } from '../sales/Receipt'
import { usePrint } from '../sales/usePrint'
import { type Cart, cartReducer, deserializeCart, emptyCart, priceCart, refreshLine, saleInput, serializeCart } from './cart'
import { CartTable } from './CartTable'
import { CustomerPanel } from './CustomerPanel'
import { NewCustomerDialog } from './CustomerDialog'
import { useMediaQuery, useOnline } from './hooks'
import { HeldCartsDialog, HoldDialog } from './HoldDialogs'
import { PaymentDialog } from './PaymentDialog'
import { ProductGrid } from './ProductGrid'
import { ProductSearch } from './ProductSearch'
import { ReceiptDialog } from './ReceiptDialog'
import { Shortcuts } from './Shortcuts'
import { TotalsPanel } from './TotalsPanel'

type DialogName = 'none' | 'customer' | 'pay' | 'hold' | 'held' | 'receipt'

/** The POS screen (Requirement Analysis §7.6), full screen without the sidebar. */
export function PosPage() {
  const { t } = useTranslation()
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const ctx = usePosContext(tenantId)

  if (!can('pos.sell')) {
    return (
      <div className="flex min-h-svh flex-col items-center justify-center gap-4 p-6">
        <Alert tone="danger">{t('problems.PERMISSION_DENIED')}</Alert>
        <Link to="/" className="font-medium text-brand-700 hover:underline">
          {t('common.backHome')}
        </Link>
      </div>
    )
  }
  if (ctx.isPending) {
    return (
      <div className="flex min-h-svh items-center justify-center">
        <Spinner />
      </div>
    )
  }
  if (ctx.isError) {
    return (
      <div className="flex min-h-svh flex-col items-center justify-center gap-4 p-6">
        <Alert tone="danger">{problemMessage(ctx.error, t)}</Alert>
        <Button variant="secondary" onClick={() => void ctx.refetch()}>
          {t('common.retry')}
        </Button>
      </div>
    )
  }
  return <PosScreen ctx={ctx.data} tenantId={tenantId} />
}

/** Re-reads products (current price and stock) for the cart lines. */
async function refreshCart(cart: Cart): Promise<Cart> {
  const lines = await Promise.all(
    cart.lines.map(async (line) => {
      try {
        const found = await salesApi.posProducts(line.item.sku, 5)
        const fresh = found.find((f) => f.variant_id === line.item.variant_id)
        return fresh ? refreshLine(line, fresh) : line
      } catch {
        return line
      }
    }),
  )
  return { ...cart, lines }
}

function PosScreen({ ctx, tenantId }: { ctx: PosContext; tenantId: string }) {
  const { t, i18n } = useTranslation()
  const { me, can } = useSession()
  const queryClient = useQueryClient()
  const [cart, dispatch] = useReducer(cartReducer, emptyCart)
  const [selected, setSelected] = useState<string | null>(null)
  const [dialog, setDialog] = useState<DialogName>('none')
  // What was typed in the customer search when "new customer" was chosen.
  const [newCustomer, setNewCustomer] = useState('')
  // One Idempotency-Key per cart, kept until that sale succeeds (POS-R06).
  const [saleKey, setSaleKey] = useState(newKey)
  const [lastSale, setLastSale] = useState<SaleDetail | null>(null)
  const [flash, setFlash] = useState<{ id: string; n: number } | null>(null)
  const searchRef = useRef<HTMLInputElement>(null)
  const customerRef = useRef<HTMLInputElement>(null)
  const discountRef = useRef<HTMLInputElement>(null)
  // Below lg (phones, tablets in portrait) one view shows at a time: the
  // product panel, or the cart with customer and totals.
  const [view, setView] = useState<'products' | 'cart'>('products')
  // Below lg, customer and invoice discount live in a bottom sheet.
  const wide = useMediaQuery('(min-width: 64rem)')
  const [sheetOpen, setSheetOpen] = useState(false)
  const sheetModal = sheetOpen && !wide
  const sheetModalRef = useRef(false)
  useEffect(() => {
    sheetModalRef.current = sheetModal
  })
  const sheetRef = useRef<HTMLDivElement>(null)
  const sheetTriggerRef = useRef<HTMLButtonElement>(null)
  const online = useOnline()
  const print = usePrint()
  const methods = usePaymentMethods(tenantId)
  const held = useHeldCarts(tenantId)
  const canDiscountOverLimit = can('pos.discount_over_limit')

  // Off in Settings → Invoice, tax & POS; missing on older API builds = shown.
  const showProducts = ctx.show_product_panel !== false
  // Product photos on the cards, cart lines and search results; missing = shown.
  const showImages = ctx.show_product_images !== false
  const inCart = useMemo(() => Object.fromEntries(cart.lines.map((l) => [l.item.variant_id, l.quantity])), [cart.lines])
  const priced = useMemo(() => priceCart(cart, ctx, canDiscountOverLimit), [cart, ctx, canDiscountOverLimit])
  const activeMethods = useMemo(() => (methods.data ?? []).filter((m) => m.is_active).sort((a, b) => a.sort_order - b.sort_order), [methods.data])
  const canPay = cart.lines.length > 0 && priced.totals !== null && Object.keys(priced.lineProblems).length === 0 && priced.invoiceProblem === null

  const close = useCallback(() => {
    setDialog('none')
    setTimeout(() => (sheetModalRef.current ? customerRef : searchRef).current?.focus(), 0)
  }, [])

  const sale = useMutation({
    mutationFn: ({ input, key }: { input: SaleInput; key: string }) => salesApi.createSale(input, key),
    onSuccess: (detail) => {
      setLastSale(detail)
      dispatch({ type: 'clear' })
      setSelected(null)
      setSaleKey(newKey())
      setDialog('receipt')
      for (const key of ['sales', 'inventory', 'parties', 'money', 'reports']) void queryClient.invalidateQueries({ queryKey: [key, tenantId] })
    },
    onError: async (err) => {
      // Prices changed on the server: show the current ones.
      if (err instanceof ApiError && err.problem.code === 'TOTAL_MISMATCH') dispatch({ type: 'replace', cart: await refreshCart(cart) })
    },
  })

  const hold = useMutation({
    mutationFn: (label: string) => salesApi.holdCart({ label: label || undefined, customer_id: cart.customer?.id, cart: serializeCart(cart) }),
    onSuccess: () => {
      dispatch({ type: 'clear' })
      setSelected(null)
      setSaleKey(newKey())
      void queryClient.invalidateQueries({ queryKey: salesKeys.heldCarts(tenantId) })
      close()
    },
  })

  const resume = useMutation({
    mutationFn: async (h: HeldCart) => refreshCart(deserializeCart((await salesApi.removeHeldCart(h.id)).cart)),
    onSuccess: (restored) => {
      dispatch({ type: 'replace', cart: restored })
      setSelected(restored.lines[0]?.item.variant_id ?? null)
      setSaleKey(newKey())
      void queryClient.invalidateQueries({ queryKey: salesKeys.heldCarts(tenantId) })
      close()
    },
  })

  const discard = useMutation({
    mutationFn: (h: HeldCart) => salesApi.removeHeldCart(h.id),
    onSettled: () => void queryClient.invalidateQueries({ queryKey: salesKeys.heldCarts(tenantId) }),
  })

  function addItem(item: PosItem) {
    dispatch({ type: 'add', item })
    setSelected(item.variant_id)
    setFlash((f) => ({ id: item.variant_id, n: (f?.n ?? 0) + 1 }))
  }

  function openPay() {
    if (!canPay) return
    setSheetOpen(false)
    sale.reset()
    setDialog('pay')
  }

  function openHold() {
    if (cart.lines.length === 0) return
    hold.reset()
    setDialog('hold')
  }

  // Keyboard shortcuts (§7.6): F2 search, F4 customer, F7 discount, F8 pay,
  // F9 hold, F10 held carts, Ctrl+Delete removes the selected line.
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (dialog !== 'none') return
      const handled = () => e.preventDefault()
      if (e.key === 'Escape' && sheetModal) {
        handled()
        closeSheet()
      } else if (e.key === 'F2') {
        handled()
        searchRef.current?.focus()
      } else if (e.key === 'F4') {
        handled()
        setView('cart')
        if (!wide) setSheetOpen(true)
        setTimeout(() => customerRef.current?.focus(), 0)
      } else if (e.key === 'F7') {
        handled()
        setView('cart')
        if (!wide) setSheetOpen(true)
        setTimeout(() => {
          discountRef.current?.focus()
          discountRef.current?.select()
        }, 0)
      } else if (e.key === 'F8') {
        handled()
        openPay()
      } else if (e.key === 'F9') {
        handled()
        openHold()
      } else if (e.key === 'F10') {
        handled()
        resume.reset()
        setDialog('held')
      } else if (e.key === 'Delete' && e.ctrlKey && selected) {
        handled()
        dispatch({ type: 'remove', variantId: selected })
        setSelected(null)
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  })

  // While open, the sheet keeps the page behind it from scrolling.
  useEffect(() => {
    if (!sheetModal) return
    const before = document.body.style.overflow
    document.body.style.overflow = 'hidden'
    return () => {
      document.body.style.overflow = before
    }
  }, [sheetModal])

  // Phone browsers draw the on-screen keyboard over fixed elements instead of
  // shrinking the page, which would hide the sheet's inputs. Keep the sheet
  // above the keyboard and within the part of the screen still visible.
  useEffect(() => {
    const vv = window.visualViewport
    const sheet = sheetRef.current
    if (!sheetModal || !vv || !sheet) return
    const fit = () => {
      const covered = Math.max(0, window.innerHeight - (vv.height + vv.offsetTop))
      sheet.style.bottom = `${covered}px`
      sheet.style.maxHeight = `${Math.round(vv.height * 0.85)}px`
      const focused = document.activeElement
      if (covered > 0 && focused instanceof HTMLElement && sheet.contains(focused)) focused.scrollIntoView({ block: 'nearest' })
    }
    fit()
    vv.addEventListener('resize', fit)
    vv.addEventListener('scroll', fit)
    return () => {
      vv.removeEventListener('resize', fit)
      vv.removeEventListener('scroll', fit)
      sheet.style.bottom = ''
      sheet.style.maxHeight = ''
    }
  }, [sheetModal])

  function openSheet() {
    setSheetOpen(true)
    setTimeout(() => sheetRef.current?.querySelector<HTMLElement>('[data-sheet-close]')?.focus(), 0)
  }

  function closeSheet() {
    setSheetOpen(false)
    setTimeout(() => sheetTriggerRef.current?.focus(), 0)
  }

  // Tab stays inside the sheet while it covers the page (Esc: see the shortcuts).
  function onSheetKeyDown(e: ReactKeyboardEvent<HTMLDivElement>) {
    if (!sheetModal) return
    if (e.key === 'Tab' && sheetRef.current) {
      const focusable = [...sheetRef.current.querySelectorAll<HTMLElement>('button, input, select, textarea, a[href], [tabindex]:not([tabindex="-1"])')].filter(
        (el) => !el.hasAttribute('disabled') && el.offsetParent !== null,
      )
      const first = focusable[0]
      const last = focusable.at(-1)
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault()
        last?.focus()
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault()
        first?.focus()
      }
    }
  }

  const heldCount = held.data?.length ?? 0
  const invoiceDiscount = priced.totals?.invoiceDiscount
  // The view switch only exists with the product panel; without it the cart is the only view.
  const tabbed = showProducts
  const onCartView = !tabbed || view === 'cart'
  const hideBelowLg = 'max-lg:hidden'
  return (
    <div className="flex min-h-svh flex-col bg-canvas text-ink lg:h-svh print:block print:min-h-0 print:bg-white print:[&_dialog]:hidden">
      <div className="flex min-h-0 flex-1 flex-col print:hidden">
        <header className="flex flex-wrap items-center gap-x-3 gap-y-2 border-b border-border bg-surface px-3 py-2 lg:gap-x-4">
          <Link to="/" className="flex min-h-11 items-center text-sm font-medium text-brand-700 hover:underline lg:min-h-0">
            ← {t('pos.dashboard')}
          </Link>
          <div className="flex min-w-0 flex-1 flex-col sm:flex-row sm:items-baseline sm:gap-2">
            <span className="truncate font-bold">{ctx.receipt.business_name || me?.tenant?.name}</span>
            {ctx.receipt.branch_name && <span className="truncate text-sm text-ink-muted">{ctx.receipt.branch_name}</span>}
          </div>
          <Button
            variant="secondary"
            className="min-h-11 lg:min-h-9"
            onClick={() => {
              resume.reset()
              setDialog('held')
            }}
          >
            {t('pos.heldCarts', { count: heldCount })} <kbd className="hidden text-xs text-ink-muted lg:inline">F10</kbd>
          </Button>
          <ShiftIndicator tenantId={tenantId} />
          <span className="hidden text-sm text-ink-muted xl:inline">{me?.user.full_name}</span>
          <LanguageSwitcher />
        </header>
        {!online && (
          <div role="alert" className="bg-danger px-3 py-1.5 text-center text-sm font-medium text-white">
            {t('pos.offline')}
          </div>
        )}

        {/* Phones and tablets in portrait leave room for the sticky total bar. */}
        <main className="grid flex-1 grid-cols-1 content-start gap-3 p-3 pb-24 lg:min-h-0 lg:grid-cols-[minmax(0,1fr)_20rem] lg:content-stretch lg:pb-3 xl:grid-cols-[minmax(0,1fr)_22rem]">
          <div className="flex min-w-0 flex-col gap-3 lg:min-h-0">
            <ProductSearch tenantId={tenantId} inputRef={searchRef} onAdd={addItem} showImages={showImages} />
            {tabbed && (
              <div role="group" aria-label={t('pos.viewSwitch')} className="grid grid-cols-2 gap-1 rounded-xl border border-border bg-surface p-1 lg:hidden">
                {(['products', 'cart'] as const).map((v) => (
                  <button
                    key={v}
                    type="button"
                    aria-pressed={view === v}
                    onClick={() => setView(v)}
                    className={`flex min-h-11 items-center justify-center gap-2 rounded-lg text-sm font-semibold transition-colors focus-visible:ring-2 focus-visible:ring-brand-500 focus-visible:outline-none ${
                      view === v ? 'bg-brand-600 text-white' : 'text-ink hover:bg-canvas'
                    }`}
                  >
                    {v === 'products' ? t('pos.products') : t('pos.cart')}
                    {v === 'cart' && cart.lines.length > 0 && (
                      <span className={`min-w-5 rounded-full px-1.5 text-xs tabular-nums ${view === v ? 'bg-white/20' : 'bg-brand-600 text-white'}`}>
                        {cart.lines.length}
                      </span>
                    )}
                  </button>
                ))}
              </div>
            )}
            {/* Products 35% / cart 65% side by side from xl; stacked (products
                on top, capped) on tablets and phones. Without the panel the
                cart takes the whole width. */}
            <div className={`flex flex-col gap-3 lg:min-h-0 lg:flex-1 ${showProducts ? 'xl:grid xl:grid-cols-[minmax(0,35fr)_minmax(0,65fr)]' : ''}`}>
              {showProducts && (
                <div className={`flex flex-col lg:max-h-[45%] lg:min-h-0 xl:max-h-none ${view === 'products' ? '' : hideBelowLg}`}>
                  <ProductGrid tenantId={tenantId} onAdd={addItem} inCart={inCart} allowNegativeStock={ctx.allow_negative_stock} showImages={showImages} />
                </div>
              )}
              <div className={`flex flex-col lg:min-h-0 lg:flex-1 ${onCartView ? '' : hideBelowLg}`}>
                <CartTable
                  lines={cart.lines}
                  results={priced.totals?.lines ?? null}
                  problems={priced.lineProblems}
                  selected={selected}
                  onSelect={setSelected}
                  dispatch={dispatch}
                  canOverridePrice={can('pos.price_override')}
                  businessDate={ctx.business_date}
                  flash={flash}
                  showImages={showImages}
                />
              </div>
            </div>
          </div>

          <aside className={`flex min-w-0 flex-col gap-4 rounded-xl border border-border bg-surface p-4 lg:overflow-y-auto ${onCartView ? '' : hideBelowLg}`}>
            {/* Below lg: a summary of customer and discount that opens the sheet. */}
            <button
              ref={sheetTriggerRef}
              type="button"
              aria-haspopup="dialog"
              aria-expanded={sheetOpen}
              onClick={openSheet}
              className="flex min-h-14 items-center gap-3 rounded-lg bg-canvas px-3 py-2 text-left focus-visible:ring-2 focus-visible:ring-brand-500 focus-visible:outline-none lg:hidden"
            >
              <span className="min-w-0 flex-1">
                <span className="block text-xs font-medium text-ink-muted uppercase">{t('pos.customerAndDiscount')}</span>
                <span className="block truncate font-semibold">{cart.customer ? cart.customer.name : t('pos.walkIn')}</span>
                {invoiceDiscount !== undefined && invoiceDiscount !== ZERO && (
                  <span className="block truncate text-sm text-brand-700 tabular-nums">
                    {t('pos.discountAmount', { amount: formatMoney(toFixed(invoiceDiscount, 2), i18n.language) })}
                  </span>
                )}
                {priced.invoiceProblem && <span className="block text-sm text-danger">{t(`pos.errors.${priced.invoiceProblem}`)}</span>}
              </span>
              <span aria-hidden="true" className="text-xl text-ink-muted">
                ›
              </span>
            </button>

            {sheetModal && <div aria-hidden="true" onClick={closeSheet} className="fixed inset-0 z-30 bg-black/40 motion-safe:animate-fade-in lg:hidden" />}
            {/* On lg this is plain content of the side panel; below lg it is the bottom sheet. */}
            <div
              ref={sheetRef}
              role={sheetModal ? 'dialog' : undefined}
              aria-modal={sheetModal || undefined}
              aria-labelledby={sheetModal ? 'pos-sheet-title' : undefined}
              onKeyDown={onSheetKeyDown}
              className={`flex flex-col gap-4 ${
                sheetOpen
                  ? 'max-lg:fixed max-lg:inset-x-0 max-lg:bottom-0 max-lg:z-40 max-lg:mx-auto max-lg:max-h-[85svh] max-lg:max-w-2xl max-lg:overflow-y-auto max-lg:rounded-t-2xl max-lg:bg-surface max-lg:px-4 max-lg:pt-2 max-lg:pb-[max(1rem,env(safe-area-inset-bottom))] max-lg:shadow-2xl max-lg:motion-safe:animate-sheet-in'
                  : 'max-lg:hidden'
              }`}
            >
              <div className="sticky top-0 z-10 -mx-4 flex flex-col bg-surface px-4 pb-1 lg:hidden">
                <span aria-hidden="true" className="mx-auto mb-1 h-1 w-10 rounded-full bg-border" />
                <div className="flex items-center justify-between gap-3">
                  <h2 id="pos-sheet-title" className="font-semibold">
                    {t('pos.customerAndDiscount')}
                  </h2>
                  <button
                    type="button"
                    data-sheet-close
                    aria-label={t('common.close')}
                    onClick={closeSheet}
                    className="flex size-11 items-center justify-center rounded-lg text-ink-muted hover:bg-canvas hover:text-ink focus-visible:ring-2 focus-visible:ring-brand-500 focus-visible:outline-none"
                  >
                    <span aria-hidden="true">✕</span>
                  </button>
                </div>
              </div>
              <CustomerPanel
                tenantId={tenantId}
                customer={cart.customer}
                inputRef={customerRef}
                onSelect={(customer) => dispatch({ type: 'customer', customer })}
                onNew={(typed) => {
                  setNewCustomer(typed)
                  setDialog('customer')
                }}
              />
              <TotalsPanel
                totals={priced.totals}
                pricesIncludeTax={ctx.prices_include_tax}
                discountType={cart.invoiceDiscountType}
                discountValue={cart.invoiceDiscountValue}
                discountProblem={priced.invoiceProblem}
                onDiscount={(discountType, value) => dispatch({ type: 'invoiceDiscount', discountType, value })}
                discountRef={discountRef}
              />
              <Button className="min-h-12 lg:hidden" onClick={closeSheet}>
                {t('common.close')}
              </Button>
            </div>
            {/* Below lg the sticky bar at the bottom carries Pay. */}
            <Button className="min-h-14 text-lg max-lg:hidden" disabled={!canPay} onClick={openPay}>
              {t('pos.pay')} <kbd className="text-sm opacity-80">F8</kbd>
            </Button>
            <div className="flex flex-wrap gap-2">
              <Button variant="secondary" className="min-h-11 flex-1 lg:min-h-0" disabled={cart.lines.length === 0} onClick={openHold}>
                {t('pos.hold')} <kbd className="hidden text-xs text-ink-muted lg:inline">F9</kbd>
              </Button>
              {cart.lines.length > 0 && (
                <ConfirmButton
                  onConfirm={() => {
                    dispatch({ type: 'clear' })
                    setSelected(null)
                  }}
                >
                  {t('pos.clearCart')}
                </ConfirmButton>
              )}
            </div>
            <Shortcuts />
          </aside>
        </main>

        {/* Phones and tablets in portrait: total, line count and Pay always in reach. */}
        <div className="fixed inset-x-0 bottom-0 z-20 flex items-center gap-3 border-t border-border bg-surface px-3 pt-2 pb-[max(0.5rem,env(safe-area-inset-bottom))] shadow-[0_-4px_12px_rgb(0_0_0/0.06)] lg:hidden">
          <button
            type="button"
            onClick={() => setView('cart')}
            className="flex min-h-11 min-w-0 flex-1 flex-col items-start justify-center rounded-lg text-left focus-visible:ring-2 focus-visible:ring-brand-500 focus-visible:outline-none"
          >
            <span className="truncate text-xs text-ink-muted">
              {t('pos.grandTotal')} · {t('pos.lineCount', { count: cart.lines.length })}
            </span>
            <span className="truncate text-xl font-bold text-brand-700 tabular-nums">
              {priced.totals ? formatMoney(toFixed(priced.totals.grandTotal, 2), i18n.language) : '—'}
            </span>
          </button>
          <Button className="min-h-12 shrink-0 px-8 text-base" disabled={!canPay} onClick={openPay}>
            {t('pos.pay')}
          </Button>
        </div>

        {dialog === 'customer' && (
          <NewCustomerDialog
            tenantId={tenantId}
            typed={newCustomer}
            onClose={close}
            onCreated={(customer) => {
              dispatch({ type: 'customer', customer })
              close()
            }}
          />
        )}
        {dialog === 'pay' &&
          priced.totals &&
          (methods.isPending ? null : (
            <PaymentDialog
              methods={activeMethods}
              grandTotal={priced.totals.grandTotal}
              customer={cart.customer}
              pending={sale.isPending}
              error={sale.error}
              onClose={close}
              onConfirm={(payments, note) => {
                if (priced.totals) sale.mutate({ input: saleInput(cart, priced.totals, payments, note), key: saleKey })
              }}
            />
          ))}
        {dialog === 'hold' && <HoldDialog pending={hold.isPending} error={hold.error} onClose={close} onHold={(label) => hold.mutate(label)} />}
        {dialog === 'held' && (
          <HeldCartsDialog
            carts={held.data ?? []}
            loading={held.isPending}
            error={held.error ?? resume.error ?? discard.error}
            cartBusy={cart.lines.length > 0}
            pendingId={resume.isPending ? (resume.variables?.id ?? null) : discard.isPending ? (discard.variables?.id ?? null) : null}
            onClose={close}
            onResume={(h) => resume.mutate(h)}
            onDiscard={(h) => discard.mutate(h)}
          />
        )}
        {dialog === 'receipt' && lastSale && <ReceiptDialog sale={lastSale} onPrint={print} onNewSale={close} />}
      </div>
      {lastSale && (
        <PrintOnly>
          <SaleReceipt sale={lastSale} />
        </PrintOnly>
      )}
    </div>
  )
}
