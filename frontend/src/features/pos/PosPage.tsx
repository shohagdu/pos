import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useCallback, useEffect, useMemo, useReducer, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { ApiError } from '../../api/client'
import { usePaymentMethods } from '../../api/money'
import { newKey } from '../../api/query'
import { type HeldCart, type PosContext, type SaleDetail, type SaleInput, salesApi, salesKeys, useHeldCarts, usePosContext } from '../../api/sales'
import { LanguageSwitcher } from '../../components/LanguageSwitcher'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { ConfirmButton } from '../../components/ui/ConfirmButton'
import { Spinner } from '../../components/ui/Spinner'
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
import { useOnline } from './hooks'
import { HeldCartsDialog, HoldDialog } from './HoldDialogs'
import { PaymentDialog } from './PaymentDialog'
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
  const { t } = useTranslation()
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
  const searchRef = useRef<HTMLInputElement>(null)
  const customerRef = useRef<HTMLInputElement>(null)
  const discountRef = useRef<HTMLInputElement>(null)
  const online = useOnline()
  const print = usePrint()
  const methods = usePaymentMethods(tenantId)
  const held = useHeldCarts(tenantId)
  const canDiscountOverLimit = can('pos.discount_over_limit')

  const priced = useMemo(() => priceCart(cart, ctx, canDiscountOverLimit), [cart, ctx, canDiscountOverLimit])
  const activeMethods = useMemo(
    () => (methods.data ?? []).filter((m) => m.is_active).sort((a, b) => a.sort_order - b.sort_order),
    [methods.data],
  )
  const canPay =
    cart.lines.length > 0 && priced.totals !== null && Object.keys(priced.lineProblems).length === 0 && priced.invoiceProblem === null

  const close = useCallback(() => {
    setDialog('none')
    setTimeout(() => searchRef.current?.focus(), 0)
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
    mutationFn: (label: string) =>
      salesApi.holdCart({ label: label || undefined, customer_id: cart.customer?.id, cart: serializeCart(cart) }),
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

  function openPay() {
    if (!canPay) return
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
      if (e.key === 'F2') {
        handled()
        searchRef.current?.focus()
      } else if (e.key === 'F4') {
        handled()
        customerRef.current?.focus()
      } else if (e.key === 'F7') {
        handled()
        discountRef.current?.focus()
        discountRef.current?.select()
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

  const heldCount = held.data?.length ?? 0
  return (
    <div className="flex min-h-svh flex-col bg-canvas text-ink lg:h-svh print:block print:min-h-0 print:bg-white print:[&_dialog]:hidden">
      <div className="flex min-h-0 flex-1 flex-col print:hidden">
        <header className="flex flex-wrap items-center gap-x-4 gap-y-2 border-b border-border bg-surface px-3 py-2">
          <Link to="/" className="text-sm font-medium text-brand-700 hover:underline">
            ← {t('pos.dashboard')}
          </Link>
          <div className="min-w-0 flex-1">
            <span className="font-bold">{ctx.receipt.business_name || me?.tenant?.name}</span>
            {ctx.receipt.branch_name && <span className="ml-2 text-sm text-ink-muted">{ctx.receipt.branch_name}</span>}
          </div>
          <Button
            variant="secondary"
            className="min-h-9"
            onClick={() => {
              resume.reset()
              setDialog('held')
            }}
          >
            {t('pos.heldCarts', { count: heldCount })} <kbd className="text-xs text-ink-muted">F10</kbd>
          </Button>
          <ShiftIndicator tenantId={tenantId} />
          <span className="hidden text-sm text-ink-muted sm:inline">{me?.user.full_name}</span>
          <LanguageSwitcher />
        </header>
        {!online && (
          <div role="alert" className="bg-danger px-3 py-1.5 text-center text-sm font-medium text-white">
            {t('pos.offline')}
          </div>
        )}

        <main className="grid flex-1 gap-3 p-3 lg:min-h-0 lg:grid-cols-[minmax(0,1fr)_22rem]">
          <section className="flex flex-col gap-3 lg:min-h-0" aria-label={t('pos.cart')}>
            <ProductSearch
              tenantId={tenantId}
              inputRef={searchRef}
              onAdd={(item) => {
                dispatch({ type: 'add', item })
                setSelected(item.variant_id)
              }}
            />
            <div className="flex flex-col lg:min-h-0 lg:flex-1 lg:overflow-y-auto">
              <CartTable
                lines={cart.lines}
                results={priced.totals?.lines ?? null}
                problems={priced.lineProblems}
                selected={selected}
                onSelect={setSelected}
                dispatch={dispatch}
                canOverridePrice={can('pos.price_override')}
              />
            </div>
          </section>

          <aside className="flex flex-col gap-4 rounded-xl border border-border bg-surface p-4 lg:overflow-y-auto">
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
            <Button className="min-h-14 text-lg" disabled={!canPay} onClick={openPay}>
              {t('pos.pay')} <kbd className="text-sm opacity-80">F8</kbd>
            </Button>
            <div className="flex flex-wrap gap-2">
              <Button variant="secondary" className="flex-1" disabled={cart.lines.length === 0} onClick={openHold}>
                {t('pos.hold')} <kbd className="text-xs text-ink-muted">F9</kbd>
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
        {dialog === 'pay' && priced.totals && (
          methods.isPending ? null : (
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
          )
        )}
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
