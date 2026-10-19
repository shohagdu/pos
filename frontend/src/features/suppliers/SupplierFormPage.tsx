import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate, useParams } from 'react-router'

import { ApiError } from '../../api/client'
import { type PartyStatus, type Supplier, type SupplierInput, partiesApi, partyKeys } from '../../api/parties'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { PageHeader } from '../../components/ui/PageHeader'
import { Select } from '../../components/ui/Select'
import { Spinner } from '../../components/ui/Spinner'
import { TextField } from '../../components/ui/TextField'
import { tryParse } from '../../lib/decimal'
import { optional, useFields } from '../../lib/form'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { LINK } from '../purchases/helpers'

/** `/suppliers/new` and `/suppliers/:id/edit`. */
export function SupplierFormPage() {
  const { t } = useTranslation()
  const { id } = useParams()
  const { me } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  // Same key as useSupplier, but only fetched when editing.
  const supplier = useQuery({ queryKey: partyKeys.supplier(tenantId, id ?? ''), queryFn: () => partiesApi.supplier(id ?? ''), enabled: Boolean(id) })
  if (id && supplier.isPending) return <Spinner />
  if (id && supplier.isError) return <Alert tone="danger">{problemMessage(supplier.error, t)}</Alert>
  return <SupplierForm key={id ?? 'new'} tenantId={tenantId} supplier={id ? supplier.data : undefined} />
}

function SupplierForm({ tenantId, supplier }: { tenantId: string; supplier?: Supplier }) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const { can } = useSession()
  const readOnly = !can('suppliers.manage')
  const { values, bind } = useFields({
    name: supplier?.name ?? '',
    contact_person: supplier?.contact_person ?? '',
    phone: supplier?.phone ?? '',
    email: supplier?.email ?? '',
    address: supplier?.address ?? '',
    opening_balance: '',
    notes: supplier?.notes ?? '',
    status: supplier?.status ?? 'active',
  })
  const [clientErrors, setClientErrors] = useState<Record<string, string>>({})

  const save = useMutation({
    mutationFn: () => {
      const input: SupplierInput = {
        name: values.name.trim(),
        contact_person: optional(values.contact_person),
        phone: optional(values.phone),
        email: optional(values.email),
        address: optional(values.address),
        notes: optional(values.notes),
      }
      if (supplier) return partiesApi.updateSupplier(supplier.id, supplier.version, { ...input, status: values.status as PartyStatus })
      return partiesApi.createSupplier({ ...input, opening_balance: optional(values.opening_balance) })
    },
    onSuccess: async (saved) => {
      queryClient.setQueryData(partyKeys.supplier(tenantId, saved.id), saved)
      await queryClient.invalidateQueries({ queryKey: partyKeys.suppliers(tenantId) })
      navigate(`/suppliers/${saved.id}`, { replace: !supplier })
    },
    onError: (err) => {
      if (err instanceof ApiError && err.status === 412 && supplier) {
        void queryClient.invalidateQueries({ queryKey: partyKeys.supplier(tenantId, supplier.id) })
      }
    },
  })
  const errors: Record<string, string | undefined> = { ...fieldErrors(save.error, t), ...clientErrors }

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    const found: Record<string, string> = {}
    if (values.name.trim().length < 2) found.name = t('suppliers.errors.nameRequired')
    if (!supplier && values.opening_balance.trim() && tryParse(values.opening_balance) === null) found.opening_balance = t('suppliers.errors.amountInvalid')
    setClientErrors(found)
    if (Object.keys(found).length === 0) save.mutate()
  }

  const back = supplier ? `/suppliers/${supplier.id}` : '/suppliers'
  return (
    <div className="flex max-w-3xl flex-col gap-6">
      <Link to={back} className={`text-sm ${LINK}`}>
        ← {supplier ? supplier.name : t('suppliers.title')}
      </Link>
      <PageHeader title={supplier ? t('suppliers.editTitle') : t('suppliers.new')} subtitle={supplier?.code} />
      <form onSubmit={onSubmit} noValidate className="flex flex-col gap-6">
        <fieldset disabled={readOnly} className="flex flex-col gap-6">
          <Card className="flex flex-col gap-4">
            <TextField label={t('suppliers.name')} {...bind('name')} required maxLength={200} error={errors.name} />
            <div className="grid gap-4 sm:grid-cols-2">
              <TextField label={t('suppliers.contactPerson')} {...bind('contact_person')} maxLength={200} error={errors.contact_person} />
              <TextField label={t('suppliers.phone')} type="tel" {...bind('phone')} maxLength={20} error={errors.phone} />
              <TextField label={t('suppliers.email')} type="email" {...bind('email')} maxLength={200} error={errors.email} />
              {supplier ? (
                <Select
                  label={t('common.status')}
                  {...bind('status')}
                  options={[
                    { value: 'active', label: t('suppliers.statuses.active') },
                    { value: 'inactive', label: t('suppliers.statuses.inactive') },
                  ]}
                  error={errors.status}
                />
              ) : (
                <TextField
                  label={t('suppliers.openingBalance')}
                  inputMode="decimal"
                  {...bind('opening_balance')}
                  hint={t('suppliers.openingBalanceHint')}
                  error={errors.opening_balance}
                />
              )}
            </div>
            <label className="flex flex-col gap-1.5 text-sm font-medium text-ink">
              {t('suppliers.address')}
              <textarea
                {...bind('address')}
                rows={2}
                maxLength={500}
                className="rounded-lg border border-border bg-surface px-3 py-2 text-sm font-normal outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100"
              />
              {errors.address && <span className="text-sm font-normal text-danger">{errors.address}</span>}
            </label>
            <label className="flex flex-col gap-1.5 text-sm font-medium text-ink">
              {t('suppliers.notes')}
              <textarea
                {...bind('notes')}
                rows={3}
                maxLength={2000}
                className="rounded-lg border border-border bg-surface px-3 py-2 text-sm font-normal outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100"
              />
              {errors.notes && <span className="text-sm font-normal text-danger">{errors.notes}</span>}
            </label>
          </Card>
        </fieldset>
        {save.isError && hasFormError(save.error) && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}
        {!readOnly && (
          <div className="flex justify-end gap-2">
            <Link to={back} className="inline-flex min-h-11 items-center rounded-lg px-4 text-sm font-semibold text-ink-muted hover:bg-canvas">
              {t('common.cancel')}
            </Link>
            <Button type="submit" disabled={save.isPending}>
              {save.isPending ? t('common.saving') : t('common.save')}
            </Button>
          </div>
        )}
      </form>
    </div>
  )
}
