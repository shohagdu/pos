import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { type AccountKind, type LedgerAccount, type LedgerAccountInput, accountingApi, accountingKeys } from '../../api/accounting'
import { ApiError } from '../../api/client'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Checkbox } from '../../components/ui/Checkbox'
import { Dialog } from '../../components/ui/Dialog'
import { Select } from '../../components/ui/Select'
import { TextField } from '../../components/ui/TextField'
import { hasFormError, problemMessage } from '../../lib/problem'
import { accountLabel, chartOrder, suggestCode } from './ledger'

const KINDS: AccountKind[] = ['asset', 'liability', 'equity', 'income', 'expense']
const defaultSide = (kind: AccountKind) => (kind === 'asset' || kind === 'expense' ? 'debit' : 'credit')

/** Ids of `id` and everything under it (a group cannot move under its own branch). */
function branchOf(accounts: LedgerAccount[], id: string): Set<string> {
  const out = new Set([id])
  let grew = true
  while (grew) {
    grew = false
    for (const a of accounts) {
      if (a.parent_id && out.has(a.parent_id) && !out.has(a.id)) {
        out.add(a.id)
        grew = true
      }
    }
  }
  return out
}

/**
 * Add or edit a ledger account (ACC-R01). Under a group the type follows the
 * group; system accounts can only be renamed; once an account has entries
 * its type, side and "group" setting are fixed.
 */
export function LedgerAccountDialog({
  tenantId,
  accounts,
  account,
  parentId,
  onClose,
}: {
  tenantId: string
  accounts: LedgerAccount[]
  /** The account to edit; absent to add one. */
  account?: LedgerAccount
  /** Preset group for "add sub-account". */
  parentId?: string
  onClose: () => void
}) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const byId = useMemo(() => new Map(accounts.map((a) => [a.id, a])), [accounts])
  const initialParent = (account ? account.parent_id : parentId) ?? ''
  const [parent, setParent] = useState(initialParent)
  const [code, setCode] = useState(account?.code ?? suggestCode(accounts, byId.get(initialParent) ?? null))
  const [codeTouched, setCodeTouched] = useState(Boolean(account))
  const [name, setName] = useState(account?.name ?? '')
  const [kind, setKind] = useState<AccountKind>(account?.account_type ?? byId.get(initialParent)?.account_type ?? 'expense')
  const [side, setSide] = useState<'debit' | 'credit'>(account?.normal_side ?? defaultSide(kind))
  const [isGroup, setIsGroup] = useState(account ? !account.is_postable : false)
  const [active, setActive] = useState(account ? account.status === 'active' : true)

  const system = Boolean(account?.system_key)
  const inUse = account?.has_entries ?? false
  const hasChildren = account ? accounts.some((a) => a.parent_id === account.id) : false
  const parentAccount = parent ? (byId.get(parent) ?? null) : null
  const effectiveKind = parentAccount?.account_type ?? kind
  const excluded = useMemo(() => (account ? branchOf(accounts, account.id) : new Set<string>()), [accounts, account])
  const groups = chartOrder(accounts).filter((a) => !a.is_postable && (a.status === 'active' || a.id === initialParent) && !excluded.has(a.id))
  const kindLocked = system || Boolean(parentAccount) || (account && (inUse || hasChildren))
  const sideLocked = system || inUse

  function chooseParent(id: string) {
    setParent(id)
    const p = id ? byId.get(id) : undefined
    if (p) {
      setKind(p.account_type)
      if (!sideLocked) setSide(defaultSide(p.account_type))
    }
    if (!codeTouched) setCode(suggestCode(accounts, p ?? null))
  }

  const save = useMutation({
    mutationFn: () => {
      const input: LedgerAccountInput = {
        code: code.trim(),
        name: name.trim(),
        account_type: effectiveKind,
        parent_id: parent || null,
        normal_side: side,
        is_postable: !isGroup,
      }
      if (account) input.status = active ? 'active' : 'inactive'
      return account ? accountingApi.updateLedgerAccount(account.id, account.version, input) : accountingApi.createLedgerAccount(input)
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: accountingKeys.ledgerAccounts(tenantId) })
      onClose()
    },
    onError: (err) => {
      if (err instanceof ApiError && err.status === 412) void queryClient.invalidateQueries({ queryKey: accountingKeys.ledgerAccounts(tenantId) })
    },
  })

  // Codes like IN_USE mean different things per screen; say it for accounts.
  const errors: Record<string, string> = {}
  if (save.error instanceof ApiError) {
    for (const e of save.error.problem.errors ?? []) {
      errors[e.field] = t(`accounting.chart.errors.${e.code}`, { defaultValue: t(`problems.${e.code}`, { defaultValue: e.message }) })
    }
  }

  function submit(e: FormEvent) {
    e.preventDefault()
    if (code.trim() && name.trim() && !save.isPending) save.mutate()
  }

  return (
    <Dialog
      open
      onClose={onClose}
      title={account ? t('accounting.chart.editTitle') : t('accounting.chart.addTitle')}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button type="submit" form="ledger-account" disabled={save.isPending || !code.trim() || !name.trim()}>
            {save.isPending ? t('common.saving') : t('common.save')}
          </Button>
        </>
      }
    >
      <form id="ledger-account" onSubmit={submit} noValidate className="flex flex-col gap-4">
        {system && <Alert tone="info">{t('accounting.chart.systemNote')}</Alert>}
        {!system && inUse && <Alert tone="info">{t('accounting.chart.inUseNote')}</Alert>}

        <Select
          label={t('accounting.chart.parent')}
          value={parent}
          onChange={(e) => chooseParent(e.target.value)}
          disabled={system}
          options={[{ value: '', label: t('accounting.chart.topLevel') }, ...groups.map((g) => ({ value: g.id, label: accountLabel(g) }))]}
          error={errors.parent_id}
        />
        <div className="grid gap-4 sm:grid-cols-[9rem_1fr]">
          <TextField
            label={t('accounting.chart.code')}
            value={code}
            maxLength={20}
            autoComplete="off"
            disabled={system}
            onChange={(e) => {
              setCode(e.target.value)
              setCodeTouched(true)
            }}
            error={errors.code}
            hint={!account && parentAccount ? t('accounting.chart.codeHint') : undefined}
          />
          <TextField
            label={t('accounting.chart.name')}
            value={name}
            maxLength={150}
            autoComplete="off"
            onChange={(e) => setName(e.target.value)}
            error={errors.name}
            required
            autoFocus
            data-autofocus
          />
        </div>
        <div className="grid gap-4 sm:grid-cols-2">
          <Select
            label={t('accounting.chart.type')}
            value={effectiveKind}
            onChange={(e) => {
              const k = e.target.value as AccountKind
              setKind(k)
              if (!sideLocked) setSide(defaultSide(k))
            }}
            disabled={Boolean(kindLocked)}
            options={KINDS.map((k) => ({ value: k, label: t(`accounting.types.${k}`) }))}
            error={errors.account_type}
          />
          <Select
            label={t('accounting.chart.normalSide')}
            value={side}
            onChange={(e) => setSide(e.target.value as 'debit' | 'credit')}
            disabled={sideLocked}
            options={[
              { value: 'debit', label: t('accounting.debit') },
              { value: 'credit', label: t('accounting.credit') },
            ]}
            error={errors.normal_side}
          />
        </div>
        {side !== defaultSide(effectiveKind) && <p className="-mt-2 text-xs text-ink-muted">{t('accounting.chart.contraHint')}</p>}

        <Checkbox
          label={t('accounting.chart.isGroup')}
          hint={t('accounting.chart.isGroupHint')}
          checked={isGroup}
          onChange={(e) => setIsGroup(e.target.checked)}
          disabled={system || (account && (isGroup ? hasChildren : inUse))}
        />
        {errors.is_postable && <p className="-mt-2 text-sm text-danger">{errors.is_postable}</p>}
        {account && !system && (
          <Checkbox label={t('accounting.chart.active')} hint={t('accounting.chart.activeHint')} checked={active} onChange={(e) => setActive(e.target.checked)} />
        )}
        {errors.status && <p className="-mt-2 text-sm text-danger">{errors.status}</p>}

        {save.isError && hasFormError(save.error) && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}
      </form>
    </Dialog>
  )
}
