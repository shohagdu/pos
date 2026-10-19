import { useMutation, useQueryClient } from '@tanstack/react-query'
import type { FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate, useParams } from 'react-router'

import { ApiError } from '../../api/client'
import { type Customer, type CustomerInput, type PartyStatus, partiesApi, partyKeys, useCustomer } from '../../api/parties'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { Select } from '../../components/ui/Select'
import { TextField } from '../../components/ui/TextField'
import { optional, useFields } from '../../lib/form'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { QueryStatus } from '../money/shared'
import { trimDecimal, useTenant } from '../money/utils'

/** `/customers/new` and `/customers/:id/edit`. */
export function CustomerFormPage() {
  const { t } = useTranslation()
  const { id } = useParams()
  const { tenantId, can } = useTenant()
  if (!can('customers.manage')) return <Alert tone="warning">{t('customers.noPermission')}</Alert>
  return id ? <EditCustomer key={id} id={id} tenantId={tenantId} /> : <CustomerForm tenantId={tenantId} />
}

function EditCustomer({ id, tenantId }: { id: string; tenantId: string }) {
  const { t } = useTranslation()
  const customer = useCustomer(tenantId, id)
  if (!customer.data) return <QueryStatus query={customer} />
  if (customer.data.is_walk_in) return <Alert tone="warning">{t('customers.walkInReadOnly')}</Alert>
  return <CustomerForm customer={customer.data} tenantId={tenantId} />
}

function CustomerForm({ customer, tenantId }: { customer?: Customer; tenantId: string }) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const { values, bind } = useFields({
    name: customer?.name ?? '',
    phone: customer?.phone ?? '',
    email: customer?.email ?? '',
    address: customer?.address ?? '',
    credit_limit: trimDecimal(customer?.credit_limit),
    opening_balance: '',
    notes: customer?.notes ?? '',
    status: customer?.status ?? 'active',
  })

  const save = useMutation({
    mutationFn: () => {
      const input: CustomerInput = {
        name: values.name.trim(),
        phone: optional(values.phone),
        email: optional(values.email),
        address: optional(values.address),
        credit_limit: optional(values.credit_limit),
        notes: optional(values.notes),
      }
      if (customer) return partiesApi.updateCustomer(customer.id, customer.version, { ...input, status: values.status as PartyStatus })
      return partiesApi.createCustomer({ ...input, opening_balance: optional(values.opening_balance) })
    },
    onSuccess: async (saved) => {
      queryClient.setQueryData(partyKeys.customer(tenantId, saved.id), saved)
      await queryClient.invalidateQueries({ queryKey: partyKeys.customers(tenantId) })
      navigate(`/customers/${saved.id}`, { replace: true })
    },
    onError: (err) => {
      if (err instanceof ApiError && err.status === 412) void queryClient.invalidateQueries({ queryKey: partyKeys.customer(tenantId, customer?.id ?? '') })
    },
  })
  const errors = fieldErrors(save.error, t)
  const conflict = save.error instanceof ApiError ? save.error.problem.code : ''
  if (conflict === 'DUPLICATE_PHONE') errors.phone = problemMessage(save.error, t)

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    save.mutate()
  }

  const back = customer ? `/customers/${customer.id}` : '/customers'
  return (
    <div className="flex max-w-3xl flex-col gap-6">
      <Link to={back} className="text-sm font-medium text-brand-700 hover:underline">
        ← {customer ? customer.name : t('customers.title')}
      </Link>
      <h1 className="text-2xl font-bold">{customer ? t('customers.edit') : t('customers.new')}</h1>
      {save.isError && hasFormError(save.error) && conflict !== 'DUPLICATE_PHONE' && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}
      <form onSubmit={onSubmit} noValidate className="flex flex-col gap-6">
        <Card className="flex flex-col gap-4">
          <TextField label={t('customers.name')} {...bind('name')} required maxLength={200} error={errors.name} />
          <div className="grid gap-4 sm:grid-cols-2">
            <TextField label={t('customers.phone')} {...bind('phone')} type="tel" inputMode="tel" placeholder="01XXXXXXXXX" error={errors.phone} />
            <TextField label={t('customers.email')} {...bind('email')} type="email" error={errors.email} />
          </div>
          <TextField label={t('customers.address')} {...bind('address')} maxLength={500} error={errors.address} />
          <div className="grid gap-4 sm:grid-cols-2">
            <TextField
              label={t('customers.creditLimit')}
              {...bind('credit_limit')}
              inputMode="decimal"
              hint={t('customers.creditLimitHint')}
              error={errors.credit_limit}
            />
            {!customer && (
              <TextField
                label={t('customers.openingBalance')}
                {...bind('opening_balance')}
                inputMode="decimal"
                hint={t('customers.openingBalanceHint')}
                error={errors.opening_balance}
              />
            )}
            {customer && (
              <Select
                label={t('common.status')}
                {...bind('status')}
                error={errors.status}
                options={(['active', 'inactive'] as const).map((s) => ({ value: s, label: t(`customers.statuses.${s}`) }))}
              />
            )}
          </div>
          <div className="flex flex-col gap-1.5">
            <label htmlFor="customer-notes" className="text-sm font-medium text-ink">
              {t('customers.notes')}
            </label>
            <textarea
              id="customer-notes"
              {...bind('notes')}
              rows={3}
              className="rounded-lg border border-border bg-surface px-3 py-2 text-sm outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100"
            />
            {errors.notes && <p className="text-sm text-danger">{errors.notes}</p>}
          </div>
        </Card>
        <div className="flex flex-wrap gap-2">
          <Button type="submit" disabled={save.isPending}>
            {save.isPending ? t('common.saving') : t('common.save')}
          </Button>
          <Link to={back} className="inline-flex min-h-11 items-center px-4 text-sm font-semibold text-ink-muted hover:text-ink">
            {t('common.cancel')}
          </Link>
        </div>
      </form>
    </div>
  )
}
