import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { type AccountKind, type LedgerAccount, useLedgerAccounts } from '../../api/accounting'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { DataTable } from '../../components/ui/DataTable'
import { Select } from '../../components/ui/Select'
import { Spinner } from '../../components/ui/Spinner'
import { TextField } from '../../components/ui/TextField'
import { problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { LedgerAccountDialog } from './LedgerAccountDialog'
import { accountDepths, chartOrder } from './ledger'

const KINDS: AccountKind[] = ['asset', 'liability', 'equity', 'income', 'expense']

type Editing = { account: LedgerAccount } | { parentId?: string } | null

/**
 * Chart of accounts (ACC-R01) with links to each ledger. With
 * `accounting.coa_manage`, accounts and groups can be added and edited.
 */
export function ChartOfAccounts({ tenantId }: { tenantId: string }) {
  const { t } = useTranslation()
  const { can } = useSession()
  const canManage = can('accounting.coa_manage')
  const accounts = useLedgerAccounts(tenantId)
  const [q, setQ] = useState('')
  const [kind, setKind] = useState('')
  const [editing, setEditing] = useState<Editing>(null)
  const all = accounts.data ?? []
  const ordered = useMemo(() => chartOrder(accounts.data ?? []), [accounts.data])
  const depths = useMemo(() => accountDepths(accounts.data), [accounts.data])
  const needle = q.trim().toLowerCase()
  const rows = ordered.filter(
    (a) => (!kind || a.account_type === kind) && (!needle || a.code.toLowerCase().includes(needle) || a.name.toLowerCase().includes(needle)),
  )
  // While searching, indenting a lone match under a hidden group would mislead.
  const indent = !needle && !kind

  if (accounts.isPending) return <Spinner />
  if (accounts.isError) return <Alert tone="danger">{problemMessage(accounts.error, t)}</Alert>
  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-end gap-3 print:hidden">
        <TextField
          className="min-w-56 flex-1"
          label={t('accounting.chart.search')}
          type="search"
          value={q}
          onChange={(e) => setQ(e.target.value)}
          placeholder={t('accounting.chart.searchHint')}
        />
        <Select
          label={t('accounting.chart.type')}
          value={kind}
          onChange={(e) => setKind(e.target.value)}
          options={[{ value: '', label: t('accounting.chart.allTypes') }, ...KINDS.map((k) => ({ value: k, label: t(`accounting.types.${k}`) }))]}
        />
        {canManage && <Button onClick={() => setEditing({})}>+ {t('accounting.chart.add')}</Button>}
      </div>

      <DataTable<LedgerAccount>
        caption={t('accounting.chart.title')}
        rows={rows}
        rowKey={(a) => a.id}
        empty={t('common.noResults')}
        columns={[
          { key: 'code', header: t('accounting.chart.code'), render: (a) => <span className="font-mono text-xs">{a.code}</span> },
          {
            key: 'name',
            header: t('accounting.chart.name'),
            render: (a) => (
              <span style={{ paddingLeft: `${indent ? (depths.get(a.id) ?? 0) : 0}rem` }} className={a.is_postable ? '' : 'font-semibold'}>
                {a.name}
              </span>
            ),
          },
          { key: 'type', header: t('accounting.chart.type'), render: (a) => t(`accounting.types.${a.account_type}`) },
          { key: 'side', header: t('accounting.chart.normalSide'), render: (a) => (a.normal_side === 'debit' ? t('accounting.debit') : t('accounting.credit')) },
          {
            key: 'flags',
            header: t('common.status'),
            render: (a) => (
              <span className="flex flex-wrap gap-1">
                {a.system_key && <Badge tone="info">{t('accounting.chart.system')}</Badge>}
                {!a.is_postable && <Badge>{t('accounting.chart.group')}</Badge>}
                {a.status !== 'active' && <Badge tone="warn">{t('accounting.chart.inactive')}</Badge>}
              </span>
            ),
          },
          {
            key: 'actions',
            header: <span className="sr-only">{t('common.actions')}</span>,
            align: 'right',
            render: (a) => (
              <span className="flex flex-wrap items-center justify-end gap-x-3 gap-y-1 text-sm whitespace-nowrap print:hidden">
                {a.is_postable && (
                  <Link to={`/reports/general-ledger?account_id=${a.id}`} className="font-semibold text-brand-700 hover:underline" aria-label={t('accounting.chart.ledgerOf', { name: a.name })}>
                    {t('accounting.chart.ledger')}
                  </Link>
                )}
                {canManage && !a.is_postable && a.status === 'active' && (
                  <button type="button" className="font-semibold text-brand-700 hover:underline" onClick={() => setEditing({ parentId: a.id })} aria-label={t('accounting.chart.addSubTo', { name: a.name })}>
                    + {t('accounting.chart.addSub')}
                  </button>
                )}
                {canManage && (
                  <button type="button" className="font-semibold text-brand-700 hover:underline" onClick={() => setEditing({ account: a })} aria-label={t('accounting.chart.editOf', { name: a.name })}>
                    {t('common.edit')}
                  </button>
                )}
              </span>
            ),
          },
        ]}
      />

      {editing && (
        <LedgerAccountDialog
          tenantId={tenantId}
          accounts={all}
          account={'account' in editing ? editing.account : undefined}
          parentId={'parentId' in editing ? editing.parentId : undefined}
          onClose={() => setEditing(null)}
        />
      )}
    </div>
  )
}
