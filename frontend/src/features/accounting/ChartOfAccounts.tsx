import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { type LedgerAccount, useLedgerAccounts } from '../../api/accounting'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { DataTable } from '../../components/ui/DataTable'
import { Spinner } from '../../components/ui/Spinner'
import { problemMessage } from '../../lib/problem'
import { accountDepths, chartOrder } from './ledger'

/** Chart of accounts (ACC-R01): read-only list with links to each ledger. */
export function ChartOfAccounts({ tenantId }: { tenantId: string }) {
  const { t } = useTranslation()
  const accounts = useLedgerAccounts(tenantId)
  const rows = useMemo(() => chartOrder(accounts.data ?? []), [accounts.data])
  const depths = useMemo(() => accountDepths(accounts.data), [accounts.data])

  if (accounts.isPending) return <Spinner />
  if (accounts.isError) return <Alert tone="danger">{problemMessage(accounts.error, t)}</Alert>
  return (
    <DataTable<LedgerAccount>
      caption={t('accounting.chart.title')}
      rows={rows}
      rowKey={(a) => a.id}
      empty={t('common.none')}
      columns={[
        { key: 'code', header: t('accounting.chart.code'), render: (a) => <span className="font-mono text-xs">{a.code}</span> },
        {
          key: 'name',
          header: t('accounting.chart.name'),
          render: (a) => (
            <span style={{ paddingLeft: `${depths.get(a.id) ?? 0}rem` }} className={a.is_postable ? '' : 'font-semibold'}>
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
          key: 'ledger',
          header: <span className="sr-only">{t('common.actions')}</span>,
          align: 'right',
          render: (a) =>
            a.is_postable ? (
              <Link to={`/reports/general-ledger?account_id=${a.id}`} className="font-semibold text-brand-700 hover:underline" aria-label={t('accounting.chart.ledgerOf', { name: a.name })}>
                {t('accounting.chart.ledger')}
              </Link>
            ) : null,
        },
      ]}
    />
  )
}
