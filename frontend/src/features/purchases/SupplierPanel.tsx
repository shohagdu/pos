import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, type KeyboardEvent, type RefObject, useId, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { type Supplier, partiesApi, partyKeys } from '../../api/parties'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Dialog } from '../../components/ui/Dialog'
import { TextField } from '../../components/ui/TextField'
import { dec } from '../../lib/decimal'
import { formatMoney } from '../../lib/format'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { useDebounced } from '../pos/hooks'

const MAX_RESULTS = 8

export interface BillSupplier {
  id: string
  name: string
  phone: string | null
  /** Positive = we owe them. `null` when not known (an edited draft). */
  balance: string | null
}

const toBillSupplier = (s: Supplier): BillSupplier => ({ id: s.id, name: s.name, phone: s.phone, balance: s.current_balance })

/**
 * The bill's supplier: an inline search by name, code or phone (F4 focuses
 * it), quick add from what was typed, and the chosen supplier with what we
 * owe them.
 */
export function SupplierPanel({
  tenantId,
  supplier,
  inputRef,
  error,
  canAdd,
  onSelect,
}: {
  tenantId: string
  supplier: BillSupplier | null
  inputRef: RefObject<HTMLInputElement | null>
  error?: string
  canAdd: boolean
  onSelect: (supplier: BillSupplier | null) => void
}) {
  const { t, i18n } = useTranslation()
  const listId = useId()
  const [text, setText] = useState('')
  const [open, setOpen] = useState(false)
  const [active, setActive] = useState(0)
  const [adding, setAdding] = useState<string | null>(null)
  const blurTimer = useRef<ReturnType<typeof setTimeout> | null>(null)
  const q = useDebounced(text.trim(), 250)

  // An empty search lists the first suppliers, so a click on the box is enough.
  const found = useQuery({
    queryKey: [...partyKeys.suppliers(tenantId), 'purchase-search', q],
    queryFn: () => partiesApi.suppliers({ q: q || undefined, status: 'active' }, null, MAX_RESULTS),
  })
  const results = found.data?.data ?? []
  const showList = open
  // With quick add, the last row is "add as new supplier".
  const addRow = canAdd && text.trim() !== ''
  const rows = results.length + (addRow ? 1 : 0)

  function choose(s: Supplier) {
    onSelect(toBillSupplier(s))
    reset()
  }
  function addNew() {
    setAdding(text.trim())
    reset()
  }
  function reset() {
    setText('')
    setOpen(false)
    setActive(0)
  }

  function onKeyDown(e: KeyboardEvent<HTMLInputElement>) {
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      setOpen(true)
      if (rows > 0) setActive((a) => (a + 1) % rows)
    } else if (e.key === 'ArrowUp' && showList && rows > 0) {
      e.preventDefault()
      setActive((a) => (a - 1 + rows) % rows)
    } else if (e.key === 'Enter') {
      e.preventDefault()
      if (!showList) return
      const s = results[active]
      if (s) choose(s)
      else if (addRow) addNew()
    } else if (e.key === 'Escape' && showList) {
      e.preventDefault()
      e.stopPropagation()
      reset()
    }
  }

  const owe = supplier?.balance != null && dec(supplier.balance) !== 0n
  return (
    <section className="flex flex-col gap-2" aria-label={t('purchases.supplier')}>
      <div className="flex items-center justify-between gap-2">
        <h2 className="text-xs font-medium text-ink-muted uppercase">{t('purchases.supplier')}</h2>
        <kbd className="text-xs text-ink-muted">F4</kbd>
      </div>

      <div className="flex gap-2">
        <div className="relative min-w-0 flex-1">
          <input
            ref={inputRef}
            type="search"
            role="combobox"
            aria-label={t('purchases.form.findSupplier')}
            aria-expanded={showList}
            aria-controls={listId}
            aria-autocomplete="list"
            aria-activedescendant={showList && rows > 0 ? `${listId}-${active}` : undefined}
            aria-invalid={error ? true : undefined}
            autoComplete="off"
            placeholder={t('purchases.form.findSupplierHint')}
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
            className="min-h-10 w-full rounded-lg border border-border bg-surface px-3 text-sm outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100 aria-invalid:border-danger"
          />
          {showList && (
            <ul
              id={listId}
              role="listbox"
              aria-label={t('purchases.form.suppliers')}
              className="absolute inset-x-0 top-full z-30 mt-1 max-h-80 overflow-y-auto rounded-lg border border-border bg-surface py-1 shadow-lg"
              onMouseDown={() => blurTimer.current && clearTimeout(blurTimer.current)}
            >
              {found.isFetching && results.length === 0 && <li className="px-3 py-2 text-sm text-ink-muted">{t('common.loading')}</li>}
              {!found.isFetching && results.length === 0 && (
                <li className="px-3 py-2 text-sm text-ink-muted">{t('purchases.form.noSupplierFound')}</li>
              )}
              {results.map((s, i) => (
                <li
                  key={s.id}
                  id={`${listId}-${i}`}
                  role="option"
                  aria-selected={active === i}
                  onMouseEnter={() => setActive(i)}
                  onClick={() => choose(s)}
                  className={`flex cursor-pointer items-center justify-between gap-3 px-3 py-2 text-sm ${active === i ? 'bg-brand-50' : ''}`}
                >
                  <span className="min-w-0">
                    <span className="block truncate font-medium">{s.name}</span>
                    <span className="block text-xs text-ink-muted">{s.phone ?? s.code}</span>
                  </span>
                  {dec(s.current_balance) > 0n && (
                    <span className="shrink-0 text-xs font-medium text-danger">
                      {t('purchases.form.payable', { amount: formatMoney(s.current_balance, i18n.language) })}
                    </span>
                  )}
                </li>
              ))}
              {addRow && (
                <li
                  id={`${listId}-${results.length}`}
                  role="option"
                  aria-selected={active === results.length}
                  onMouseEnter={() => setActive(results.length)}
                  onClick={addNew}
                  className={`cursor-pointer border-t border-border px-3 py-2 text-sm font-medium text-brand-700 ${active === results.length ? 'bg-brand-50' : ''}`}
                >
                  {t('purchases.form.addAsNew', { text: text.trim() })}
                </li>
              )}
            </ul>
          )}
        </div>
        {canAdd && (
          <Button variant="secondary" className="min-h-10 shrink-0" onClick={() => setAdding('')}>
            + {t('purchases.form.quickAdd')}
          </Button>
        )}
      </div>

      <div className="flex items-center justify-between gap-3 rounded-lg bg-canvas px-3 py-2">
        <div className="flex min-w-0 items-center gap-3">
          <span
            className={`flex size-9 shrink-0 items-center justify-center rounded-full text-sm font-semibold ${
              supplier ? 'bg-brand-100 text-brand-700' : 'bg-border text-ink-muted'
            }`}
            aria-hidden="true"
          >
            {supplier ? supplier.name.trim().charAt(0).toUpperCase() : '—'}
          </span>
          <div className="min-w-0">
            <div className={`truncate font-semibold ${supplier ? '' : 'text-ink-muted'}`} data-testid="purchase-supplier">
              {supplier ? supplier.name : t('purchases.form.noSupplier')}
            </div>
            {supplier?.phone && <div className="text-xs text-ink-muted">{supplier.phone}</div>}
            {owe && supplier.balance && (
              <div className={`text-xs font-medium ${dec(supplier.balance) > 0n ? 'text-danger' : 'text-brand-700'}`}>
                {dec(supplier.balance) > 0n
                  ? t('purchases.form.payable', { amount: formatMoney(supplier.balance, i18n.language) })
                  : t('purchases.form.advance', { amount: formatMoney(supplier.balance.replace('-', ''), i18n.language) })}
              </div>
            )}
          </div>
        </div>
        {supplier && (
          <Button variant="ghost" className="min-h-9 shrink-0 text-sm" onClick={() => onSelect(null)}>
            {t('purchases.form.change')}
          </Button>
        )}
      </div>
      {error && <p className="text-sm text-danger">{error}</p>}

      {adding !== null && (
        <NewSupplierDialog
          tenantId={tenantId}
          typed={adding}
          onClose={() => setAdding(null)}
          onCreated={(s) => {
            onSelect(toBillSupplier(s))
            setAdding(null)
          }}
        />
      )}
    </section>
  )
}

/** What was typed in the supplier search goes to the phone field when it looks like a number. */
function splitTyped(typed: string): { name: string; phone: string } {
  const value = typed.trim()
  return /^[+\d][\d\s-]*$/.test(value) ? { name: '', phone: value } : { name: value, phone: '' }
}

/** Quick add from the bill: name, contact and phone; the full form is on the Suppliers page. */
function NewSupplierDialog({
  tenantId,
  typed,
  onClose,
  onCreated,
}: {
  tenantId: string
  typed: string
  onClose: () => void
  onCreated: (supplier: Supplier) => void
}) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const initial = splitTyped(typed)
  const [name, setName] = useState(initial.name)
  const [contact, setContact] = useState('')
  const [phone, setPhone] = useState(initial.phone)
  const create = useMutation({
    mutationFn: () =>
      partiesApi.createSupplier({ name: name.trim(), contact_person: contact.trim() || undefined, phone: phone.trim() || undefined }),
    onSuccess: (s) => {
      void queryClient.invalidateQueries({ queryKey: partyKeys.suppliers(tenantId) })
      onCreated(s)
    },
  })
  const errors = fieldErrors(create.error, t)

  function submit(e: FormEvent) {
    e.preventDefault()
    if (name.trim()) create.mutate()
  }

  return (
    <Dialog
      open
      onClose={onClose}
      title={t('purchases.form.newSupplierTitle')}
      footer={
        <>
          <Button variant="secondary" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button type="submit" form="purchase-new-supplier" disabled={create.isPending || !name.trim()}>
            {create.isPending ? t('common.saving') : t('purchases.form.addSupplier')}
          </Button>
        </>
      }
    >
      <form id="purchase-new-supplier" onSubmit={submit} className="flex flex-col gap-4">
        {create.isError && hasFormError(create.error) && <Alert tone="danger">{problemMessage(create.error, t)}</Alert>}
        <TextField
          label={t('purchases.form.supplierName')}
          value={name}
          maxLength={150}
          onChange={(e) => setName(e.target.value)}
          error={errors.name}
          required
          autoFocus={initial.name === ''}
          data-autofocus={initial.name === '' || undefined}
        />
        <TextField
          label={`${t('purchases.form.contactPerson')} (${t('common.optional')})`}
          value={contact}
          maxLength={150}
          onChange={(e) => setContact(e.target.value)}
          error={errors.contact_person}
        />
        <TextField
          label={`${t('purchases.form.phone')} (${t('common.optional')})`}
          type="tel"
          inputMode="tel"
          value={phone}
          maxLength={20}
          onChange={(e) => setPhone(e.target.value)}
          error={errors.phone}
          autoFocus={initial.name !== '' && initial.phone === ''}
          data-autofocus={(initial.name !== '' && initial.phone === '') || undefined}
        />
      </form>
    </Dialog>
  )
}
