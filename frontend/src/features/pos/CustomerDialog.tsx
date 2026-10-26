import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { partiesApi, partyKeys } from '../../api/parties'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Dialog } from '../../components/ui/Dialog'
import { TextField } from '../../components/ui/TextField'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { type CartCustomer, toCartCustomer } from './cart'

/** What was typed in the customer search goes to the phone field when it looks like a number. */
function splitTyped(typed: string): { name: string; phone: string } {
  const value = typed.trim()
  return /^[+\d][\d\s-]*$/.test(value) ? { name: '', phone: value } : { name: value, phone: '' }
}

/** Quick add from the POS: name, phone and address, then the customer is put on the cart. */
export function NewCustomerDialog({
  tenantId,
  typed,
  onClose,
  onCreated,
}: {
  tenantId: string
  typed: string
  onClose: () => void
  onCreated: (customer: CartCustomer) => void
}) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const initial = splitTyped(typed)
  const [name, setName] = useState(initial.name)
  const [phone, setPhone] = useState(initial.phone)
  const [address, setAddress] = useState('')
  const create = useMutation({
    mutationFn: () =>
      partiesApi.createCustomer({ name: name.trim(), phone: phone.trim() || undefined, address: address.trim() || undefined }),
    onSuccess: (c) => {
      void queryClient.invalidateQueries({ queryKey: partyKeys.customers(tenantId) })
      onCreated(toCartCustomer(c))
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
      title={t('pos.newCustomerTitle')}
      footer={
        <>
          <Button variant="secondary" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button type="submit" form="pos-new-customer" disabled={create.isPending || !name.trim()}>
            {create.isPending ? t('common.saving') : t('pos.addCustomer')}
          </Button>
        </>
      }
    >
      <form id="pos-new-customer" onSubmit={submit} className="flex flex-col gap-3" noValidate>
        {create.isError && hasFormError(create.error) && <Alert tone="danger">{problemMessage(create.error, t)}</Alert>}
        <TextField
          label={t('pos.customerName')}
          value={name}
          onChange={(e) => setName(e.target.value)}
          error={errors.name}
          required
          autoFocus={initial.name === ''}
          data-autofocus={initial.name === '' || undefined}
        />
        <TextField
          label={t('pos.customerPhone')}
          type="tel"
          inputMode="tel"
          value={phone}
          onChange={(e) => setPhone(e.target.value)}
          error={errors.phone}
          hint={t('pos.customerPhoneHint')}
          autoFocus={initial.name !== '' && initial.phone === ''}
          data-autofocus={(initial.name !== '' && initial.phone === '') || undefined}
        />
        <TextField
          label={`${t('pos.customerAddress')} (${t('common.optional')})`}
          value={address}
          onChange={(e) => setAddress(e.target.value)}
          error={errors.address}
        />
      </form>
    </Dialog>
  )
}
