import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { partiesApi, partyKeys } from '../../api/parties'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Dialog } from '../../components/ui/Dialog'
import { TextField } from '../../components/ui/TextField'
import { ZERO, dec } from '../../lib/decimal'
import { formatMoney } from '../../lib/format'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { type CartCustomer, toCartCustomer } from './cart'
import { useDebounced } from './hooks'

/** Customer selector (F4): walk-in, search by name/phone, quick add. */
export function CustomerDialog({
  tenantId,
  onClose,
  onSelect,
}: {
  tenantId: string
  onClose: () => void
  onSelect: (customer: CartCustomer | null) => void
}) {
  const { t, i18n } = useTranslation()
  const queryClient = useQueryClient()
  const [text, setText] = useState('')
  const q = useDebounced(text.trim(), 250)
  const [adding, setAdding] = useState(false)
  const [name, setName] = useState('')
  const [phone, setPhone] = useState('')

  const found = useQuery({
    queryKey: [...partyKeys.customers(tenantId), 'pos-search', q],
    queryFn: () => partiesApi.customers({ q: q || undefined, status: 'active' }, null, 10),
  })
  const create = useMutation({
    mutationFn: () => partiesApi.createCustomer({ name: name.trim(), phone: phone.trim() || undefined }),
    onSuccess: (c) => {
      void queryClient.invalidateQueries({ queryKey: partyKeys.customers(tenantId) })
      onSelect(toCartCustomer(c))
    },
  })

  function submitNew(e: FormEvent) {
    e.preventDefault()
    if (name.trim()) create.mutate()
  }

  const errors = fieldErrors(create.error, t)
  const customers = (found.data?.data ?? []).filter((c) => !c.is_walk_in)
  return (
    <Dialog open onClose={onClose} title={t('pos.chooseCustomer')} wide>
      <div className="flex flex-col gap-4">
        <div className="flex flex-wrap items-end gap-2">
          <TextField
            className="min-w-56 flex-1"
            label={t('pos.customerSearch')}
            type="search"
            value={text}
            placeholder={t('pos.customerSearchHint')}
            onChange={(e) => setText(e.target.value)}
            autoFocus
          />
          <Button variant="secondary" onClick={() => onSelect(null)}>
            {t('pos.walkIn')}
          </Button>
          <Button variant="secondary" onClick={() => setAdding((a) => !a)} aria-expanded={adding}>
            {t('pos.quickAdd')}
          </Button>
        </div>

        {adding && (
          <form onSubmit={submitNew} className="flex flex-col gap-3 rounded-lg border border-border p-3">
            <div className="grid gap-3 sm:grid-cols-2">
              <TextField label={t('pos.customerName')} value={name} onChange={(e) => setName(e.target.value)} error={errors.name} required />
              <TextField label={t('pos.customerPhone')} type="tel" value={phone} onChange={(e) => setPhone(e.target.value)} error={errors.phone} />
            </div>
            {create.isError && hasFormError(create.error) && <Alert tone="danger">{problemMessage(create.error, t)}</Alert>}
            <Button type="submit" className="self-start" disabled={create.isPending || !name.trim()}>
              {create.isPending ? t('common.saving') : t('pos.addCustomer')}
            </Button>
          </form>
        )}

        {found.isError ? (
          <Alert tone="danger">{problemMessage(found.error, t)}</Alert>
        ) : found.isPending ? (
          <p className="text-sm text-ink-muted">{t('common.loading')}</p>
        ) : customers.length === 0 ? (
          <p className="text-sm text-ink-muted">{t('common.noResults')}</p>
        ) : (
          <ul className="flex flex-col divide-y divide-border rounded-lg border border-border" aria-label={t('pos.customers')}>
            {customers.map((c) => (
              <li key={c.id}>
                <button
                  type="button"
                  onClick={() => onSelect(toCartCustomer(c))}
                  className="flex w-full items-center justify-between gap-3 px-3 py-2 text-left text-sm hover:bg-canvas"
                >
                  <span>
                    <span className="block font-medium">{c.name}</span>
                    <span className="block text-xs text-ink-muted">{c.phone ?? c.code}</span>
                  </span>
                  {dec(c.current_balance) > ZERO && (
                    <span className="text-xs font-medium text-danger">
                      {t('pos.customerDue', { amount: formatMoney(c.current_balance, i18n.language) })}
                    </span>
                  )}
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </Dialog>
  )
}
