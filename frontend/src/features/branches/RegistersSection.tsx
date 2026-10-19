import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { type PaymentAccount, usePaymentAccounts } from '../../api/money'
import { type Register, registerKeys, registersApi, useRegisters } from '../../api/registers'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { Dialog } from '../../components/ui/Dialog'
import { Select } from '../../components/ui/Select'
import { TextField } from '../../components/ui/TextField'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { QueryStatus } from '../money/shared'
import { useBranch } from './useBranch'

/** Settings → Registers: cash drawers of the working branch (all branches when none is picked). */
export function RegistersSection({ tenantId }: { tenantId: string }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const { branch } = useBranch()
  const registers = useRegisters(tenantId)
  const accounts = usePaymentAccounts(tenantId)
  const [editing, setEditing] = useState<Register | 'new' | null>(null)

  const toggle = useMutation({
    mutationFn: (r: Register) => registersApi.updateRegister(r.id, { name: r.name, status: r.status === 'active' ? 'inactive' : 'active' }),
    onSettled: () => queryClient.invalidateQueries({ queryKey: registerKeys.all(tenantId) }),
  })

  const columns: Column<Register>[] = [
    { key: 'name', header: t('branches.registerName'), render: (r) => <span className="font-medium">{r.name}</span> },
    { key: 'branch', header: t('branches.branch'), render: (r) => r.branch_name },
    { key: 'account', header: t('branches.cashAccount'), render: (r) => r.cash_account_name },
    {
      key: 'shift',
      header: t('branches.openShift'),
      render: (r) => (r.open_shift_id ? <Badge tone="info">{r.open_shift_user ?? t('branches.shiftOpen')}</Badge> : '—'),
    },
    {
      key: 'status',
      header: t('common.status'),
      render: (r) => <Badge tone={r.status === 'active' ? 'good' : 'neutral'}>{t(`branches.statuses.${r.status}`)}</Badge>,
    },
    {
      key: 'actions',
      header: <span className="sr-only">{t('common.actions')}</span>,
      render: (r) => (
        <span className="inline-flex flex-wrap justify-end gap-2">
          <Button variant="ghost" className="min-h-9" onClick={() => setEditing(r)} aria-label={t('branches.editNamed', { name: r.name })}>
            {t('common.edit')}
          </Button>
          <Button
            variant="secondary"
            className="min-h-9"
            disabled={toggle.isPending || (r.status === 'active' && r.open_shift_id !== null)}
            onClick={() => toggle.mutate(r)}
            aria-label={(r.status === 'active' ? t('branches.deactivateNamed', { name: r.name }) : t('branches.activateNamed', { name: r.name }))}
          >
            {r.status === 'active' ? t('branches.deactivate') : t('branches.activate')}
          </Button>
        </span>
      ),
    },
  ]

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <p className="text-sm text-ink-muted">
          {branch ? t('branches.registersIntroBranch', { branch: branch.name }) : t('branches.registersIntro')}
        </p>
        <Button onClick={() => setEditing('new')}>{t('branches.addRegister')}</Button>
      </div>
      {toggle.isError && <Alert tone="danger">{problemMessage(toggle.error, t)}</Alert>}
      {registers.isSuccess ? (
        <DataTable caption={t('settings.sections.registers')} columns={columns} rows={registers.data} rowKey={(r) => r.id} empty={t('branches.noRegisters')} />
      ) : (
        <QueryStatus query={registers} />
      )}
      {editing && (
        <RegisterDialog
          tenantId={tenantId}
          register={editing === 'new' ? null : editing}
          accounts={accounts.data ?? []}
          branchName={branch?.name ?? null}
          onClose={() => setEditing(null)}
        />
      )}
    </div>
  )
}

function RegisterDialog({
  tenantId,
  register,
  accounts,
  branchName,
  onClose,
}: {
  tenantId: string
  register: Register | null
  accounts: PaymentAccount[]
  branchName: string | null
  onClose: () => void
}) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const cash = accounts.filter((a) => a.account_type === 'cash' && (a.status === 'active' || a.id === register?.cash_account_id))
  const [name, setName] = useState(register?.name ?? '')
  const [accountId, setAccountId] = useState(register?.cash_account_id ?? '')
  const [localErrors, setLocalErrors] = useState<Record<string, string>>({})

  const save = useMutation({
    mutationFn: () => {
      const input = { name: name.trim(), cash_account_id: accountId || undefined }
      return register ? registersApi.updateRegister(register.id, input) : registersApi.createRegister(input)
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: registerKeys.all(tenantId) })
      onClose()
    },
  })

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    const errs: Record<string, string> = {}
    if (!name.trim()) errs.name = t('branches.errors.name')
    setLocalErrors(errs)
    if (Object.keys(errs).length === 0) save.mutate()
  }

  const errors = { ...fieldErrors(save.error, t), ...localErrors }
  return (
    <Dialog open onClose={onClose} title={register ? t('branches.editNamed', { name: register.name }) : t('branches.addRegister')}>
      <form onSubmit={onSubmit} noValidate className="flex flex-col gap-4">
        {save.isError && hasFormError(save.error) && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}
        {!register && (
          <p className="text-sm text-ink-muted">{branchName ? t('branches.registerGoesTo', { branch: branchName }) : t('branches.registerGoesToMain')}</p>
        )}
        <TextField label={t('branches.registerName')} required maxLength={50} value={name} onChange={(e) => setName(e.target.value)} error={errors.name} />
        <Select
          label={t('branches.cashAccount')}
          value={accountId}
          onChange={(e) => setAccountId(e.target.value)}
          error={errors.cash_account_id}
          options={[
            ...(register ? [] : [{ value: '', label: t('branches.defaultCash') }]),
            ...cash.map((a) => ({ value: a.id, label: a.name })),
          ]}
        />
        <div className="flex flex-wrap justify-end gap-2">
          <Button variant="ghost" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button type="submit" disabled={save.isPending}>
            {save.isPending ? t('common.saving') : t('common.save')}
          </Button>
        </div>
      </form>
    </Dialog>
  )
}
