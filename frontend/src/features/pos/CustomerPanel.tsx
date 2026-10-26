import { useQuery } from '@tanstack/react-query'
import { type KeyboardEvent, type RefObject, useId, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { type Customer, partiesApi, partyKeys } from '../../api/parties'
import { Button } from '../../components/ui/Button'
import { ZERO, dec } from '../../lib/decimal'
import { formatMoney } from '../../lib/format'
import { type CartCustomer, toCartCustomer } from './cart'
import { useDebounced } from './hooks'

const MAX_RESULTS = 8

/**
 * The cart's customer: an inline search by name or phone (F4 focuses it),
 * a "New customer" button, and the chosen customer with their due.
 */
export function CustomerPanel({
  tenantId,
  customer,
  inputRef,
  onSelect,
  onNew,
}: {
  tenantId: string
  customer: CartCustomer | null
  inputRef: RefObject<HTMLInputElement | null>
  onSelect: (customer: CartCustomer | null) => void
  /** Opens the quick-add form, prefilled with what was typed. */
  onNew: (typed: string) => void
}) {
  const { t, i18n } = useTranslation()
  const listId = useId()
  const [text, setText] = useState('')
  const [open, setOpen] = useState(false)
  const [active, setActive] = useState(0)
  const blurTimer = useRef<ReturnType<typeof setTimeout> | null>(null)
  const q = useDebounced(text.trim(), 250)

  const found = useQuery({
    queryKey: [...partyKeys.customers(tenantId), 'pos-search', q],
    queryFn: () => partiesApi.customers({ q, status: 'active' }, null, MAX_RESULTS + 1),
    enabled: q !== '',
  })
  const results = (found.data?.data ?? []).filter((c) => !c.is_walk_in).slice(0, MAX_RESULTS)
  const showList = open && text.trim() !== ''
  // The last row is always "add as new customer".
  const rows = results.length + 1

  function choose(c: Customer) {
    onSelect(toCartCustomer(c))
    reset()
  }
  function addNew() {
    onNew(text.trim())
    reset()
  }
  function reset() {
    setText('')
    setOpen(false)
    setActive(0)
  }

  function onKeyDown(e: KeyboardEvent<HTMLInputElement>) {
    if (!showList) return
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      setActive((a) => (a + 1) % rows)
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      setActive((a) => (a - 1 + rows) % rows)
    } else if (e.key === 'Enter') {
      e.preventDefault()
      const c = results[active]
      if (c) choose(c)
      else addNew()
    } else if (e.key === 'Escape') {
      e.preventDefault()
      e.stopPropagation()
      reset()
    }
  }

  const due = customer && dec(customer.balance) > ZERO
  return (
    <section className="flex flex-col gap-2" aria-label={t('pos.customer')}>
      <div className="flex items-center justify-between gap-2">
        <h2 className="text-xs font-medium text-ink-muted uppercase">{t('pos.customer')}</h2>
        <kbd className="text-xs text-ink-muted">F4</kbd>
      </div>

      <div className="flex gap-2">
        <div className="relative min-w-0 flex-1">
          <input
            ref={inputRef}
            type="search"
            role="combobox"
            aria-label={t('pos.customerSearch')}
            aria-expanded={showList}
            aria-controls={listId}
            aria-autocomplete="list"
            aria-activedescendant={showList ? `${listId}-${active}` : undefined}
            placeholder={t('pos.customerSearchHint')}
            value={text}
            onChange={(e) => {
              setText(e.target.value)
              setOpen(true)
              setActive(0)
            }}
            onFocus={() => setOpen(true)}
            onBlur={() => {
              // Let a click on a result land before the list closes.
              blurTimer.current = setTimeout(() => setOpen(false), 150)
            }}
            onKeyDown={onKeyDown}
            className="min-h-10 w-full rounded-lg border border-border bg-surface px-3 text-sm outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100"
          />
          {showList && (
            <ul
              id={listId}
              role="listbox"
              aria-label={t('pos.customers')}
              className="absolute inset-x-0 top-full z-30 mt-1 max-h-80 overflow-y-auto rounded-lg border border-border bg-surface py-1 shadow-lg"
              onMouseDown={() => blurTimer.current && clearTimeout(blurTimer.current)}
            >
              {found.isFetching && results.length === 0 && <li className="px-3 py-2 text-sm text-ink-muted">{t('common.loading')}</li>}
              {!found.isFetching && q !== '' && results.length === 0 && (
                <li className="px-3 py-2 text-sm text-ink-muted">{t('pos.noCustomerFound')}</li>
              )}
              {results.map((c, i) => (
                <li
                  key={c.id}
                  id={`${listId}-${i}`}
                  role="option"
                  aria-selected={active === i}
                  onMouseEnter={() => setActive(i)}
                  onClick={() => choose(c)}
                  className={`flex cursor-pointer items-center justify-between gap-3 px-3 py-2 text-sm ${active === i ? 'bg-brand-50' : ''}`}
                >
                  <span className="min-w-0">
                    <span className="block truncate font-medium">{c.name}</span>
                    <span className="block text-xs text-ink-muted">{c.phone ?? c.code}</span>
                  </span>
                  {dec(c.current_balance) > ZERO && (
                    <span className="shrink-0 text-xs font-medium text-danger">
                      {t('pos.customerDue', { amount: formatMoney(c.current_balance, i18n.language) })}
                    </span>
                  )}
                </li>
              ))}
              <li
                id={`${listId}-${results.length}`}
                role="option"
                aria-selected={active === results.length}
                onMouseEnter={() => setActive(results.length)}
                onClick={addNew}
                className={`cursor-pointer border-t border-border px-3 py-2 text-sm font-medium text-brand-700 ${active === results.length ? 'bg-brand-50' : ''}`}
              >
                {t('pos.addAsNew', { text: text.trim() })}
              </li>
            </ul>
          )}
        </div>
        <Button variant="secondary" className="min-h-10 shrink-0" onClick={() => onNew('')}>
          + {t('pos.quickAdd')}
        </Button>
      </div>

      <div className="flex items-center justify-between gap-3 rounded-lg bg-canvas px-3 py-2">
        <div className="flex min-w-0 items-center gap-3">
          <span
            className={`flex size-9 shrink-0 items-center justify-center rounded-full text-sm font-semibold ${
              customer ? 'bg-brand-100 text-brand-700' : 'bg-border text-ink-muted'
            }`}
            aria-hidden="true"
          >
            {customer ? customer.name.trim().charAt(0).toUpperCase() : '—'}
          </span>
          <div className="min-w-0">
            <div className="truncate font-semibold" data-testid="pos-customer">
              {customer ? customer.name : t('pos.walkIn')}
            </div>
            {customer?.phone && <div className="text-xs text-ink-muted">{customer.phone}</div>}
            {due && <div className="text-xs font-medium text-danger">{t('pos.customerDue', { amount: formatMoney(customer.balance, i18n.language) })}</div>}
          </div>
        </div>
        {customer && (
          <Button variant="ghost" className="min-h-9 shrink-0 text-sm" onClick={() => onSelect(null)}>
            {t('pos.useWalkIn')}
          </Button>
        )}
      </div>
    </section>
  )
}
