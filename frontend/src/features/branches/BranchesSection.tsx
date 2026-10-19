import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { ApiError } from '../../api/client'
import { type Branch, tenancyApi, tenancyKeys, useBranches } from '../../api/tenancy'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { ConfirmButton } from '../../components/ui/ConfirmButton'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { Dialog } from '../../components/ui/Dialog'
import { TextField } from '../../components/ui/TextField'
import { optional, useFields } from '../../lib/form'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { QueryStatus } from '../money/shared'

/** Settings → Branches: every branch of the business, add / edit / (de)activate. */
export function BranchesSection({ tenantId }: { tenantId: string }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const branches = useBranches(tenantId)
  const [editing, setEditing] = useState<Branch | 'new' | null>(null)

  const toggle = useMutation({
    mutationFn: (b: Branch) => (b.status === 'active' ? tenancyApi.deactivateBranch(b.id) : tenancyApi.activateBranch(b.id)),
    onSettled: () => queryClient.invalidateQueries({ queryKey: tenancyKeys.branches(tenantId) }),
  })

  const columns: Column<Branch>[] = [
    { key: 'code', header: t('branches.code'), render: (b) => <span className="font-mono">{b.code}</span> },
    {
      key: 'name',
      header: t('branches.name'),
      render: (b) => (
        <span className="inline-flex flex-wrap items-center gap-2 font-medium">
          {b.name}
          {b.is_default && <Badge tone="info">{t('branches.main')}</Badge>}
        </span>
      ),
    },
    { key: 'phone', header: t('branches.phone'), render: (b) => b.phone ?? '—' },
    { key: 'address', header: t('branches.address'), render: (b) => <span className="line-clamp-2 max-w-60">{b.address ?? '—'}</span> },
    {
      key: 'status',
      header: t('common.status'),
      render: (b) => <Badge tone={b.status === 'active' ? 'good' : 'neutral'}>{t(`branches.statuses.${b.status}`, { defaultValue: b.status })}</Badge>,
    },
    {
      key: 'actions',
      header: <span className="sr-only">{t('common.actions')}</span>,
      render: (b) => (
        <span className="inline-flex flex-wrap justify-end gap-2">
          <Button variant="ghost" className="min-h-9" onClick={() => setEditing(b)} aria-label={t('branches.editNamed', { name: b.name })}>
            {t('common.edit')}
          </Button>
          {!b.is_default &&
            (b.status === 'active' ? (
              <ConfirmButton className="min-h-9" disabled={toggle.isPending} onConfirm={() => toggle.mutate(b)} aria-label={t('branches.deactivateNamed', { name: b.name })}>
                {t('branches.deactivate')}
              </ConfirmButton>
            ) : (
              <Button variant="secondary" className="min-h-9" disabled={toggle.isPending} onClick={() => toggle.mutate(b)} aria-label={t('branches.activateNamed', { name: b.name })}>
                {t('branches.activate')}
              </Button>
            ))}
        </span>
      ),
    },
  ]

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <p className="text-sm text-ink-muted">{t('branches.intro')}</p>
        <Button onClick={() => setEditing('new')}>{t('branches.add')}</Button>
      </div>
      {toggle.isError && <Alert tone="danger">{problemMessage(toggle.error, t)}</Alert>}
      {branches.isSuccess ? (
        <DataTable caption={t('settings.sections.branches')} columns={columns} rows={branches.data} rowKey={(b) => b.id} empty={t('branches.none')} />
      ) : (
        <QueryStatus query={branches} />
      )}
      {editing && <BranchDialog tenantId={tenantId} branch={editing === 'new' ? null : editing} onClose={() => setEditing(null)} />}
    </div>
  )
}

function BranchDialog({ tenantId, branch, onClose }: { tenantId: string; branch: Branch | null; onClose: () => void }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const { values, bind } = useFields({
    code: branch?.code ?? '',
    name: branch?.name ?? '',
    address: branch?.address ?? '',
    phone: branch?.phone ?? '',
    receipt_header: branch?.receipt_header ?? '',
  })
  const [localErrors, setLocalErrors] = useState<Record<string, string>>({})

  const save = useMutation({
    mutationFn: () => {
      if (branch) {
        // Blank strings clear a field; `null` would leave it unchanged.
        const patch = { name: values.name.trim(), address: values.address.trim(), phone: values.phone.trim(), receipt_header: values.receipt_header.trim() }
        return tenancyApi.updateBranch(branch.id, branch.version, patch)
      }
      return tenancyApi.createBranch({
        code: values.code.trim().toUpperCase(),
        name: values.name.trim(),
        address: optional(values.address),
        phone: optional(values.phone),
        receipt_header: optional(values.receipt_header),
      })
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: tenancyKeys.branches(tenantId) })
      onClose()
    },
    onError: (err) => {
      // Someone else changed the branch: reload it so the next save uses the new version.
      if (err instanceof ApiError && err.status === 412) void queryClient.invalidateQueries({ queryKey: tenancyKeys.branches(tenantId) })
    },
  })

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    const errs: Record<string, string> = {}
    if (!branch && !/^[A-Za-z0-9]{2,10}$/.test(values.code.trim())) errs.code = t('branches.errors.code')
    if (!values.name.trim()) errs.name = t('branches.errors.name')
    setLocalErrors(errs)
    if (Object.keys(errs).length === 0) save.mutate()
  }

  const errors = { ...fieldErrors(save.error, t), ...localErrors }
  return (
    <Dialog open onClose={onClose} title={branch ? t('branches.editNamed', { name: branch.name }) : t('branches.add')}>
      <form onSubmit={onSubmit} noValidate className="flex flex-col gap-4">
        {save.isError && hasFormError(save.error) && (
          <Alert tone="danger">
            {save.error instanceof ApiError && save.error.status === 412 ? t('branches.changed') : problemMessage(save.error, t)}
          </Alert>
        )}
        {!branch && <TextField label={t('branches.code')} required maxLength={10} hint={t('branches.codeHint')} error={errors.code} {...bind('code')} />}
        <TextField label={t('branches.name')} required maxLength={150} error={errors.name} {...bind('name')} />
        <TextField label={`${t('branches.address')} (${t('common.optional')})`} maxLength={500} error={errors.address} {...bind('address')} />
        <TextField label={`${t('branches.phone')} (${t('common.optional')})`} type="tel" error={errors.phone} {...bind('phone')} />
        <TextField
          label={`${t('branches.receiptHeader')} (${t('common.optional')})`}
          hint={t('branches.receiptHeaderHint')}
          maxLength={500}
          error={errors.receipt_header}
          {...bind('receipt_header')}
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
