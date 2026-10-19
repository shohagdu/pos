import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'

import { type Section as ApiSection, useLedgerAccounts } from '../../api/accounting'
import { type CsvColumn } from '../../lib/csv'
import { dec, toFixed } from '../../lib/decimal'
import { accountDepths } from '../accounting/ledger'
import { money } from './format'

export interface StatementRow {
  key: string
  kind: 'heading' | 'line' | 'total' | 'grand'
  label: string
  code?: string
  accountId?: string
  amount?: string
}

/** Depth of each ledger account for indenting statement lines. */
export function useAccountDepths(tenantId: string) {
  const accounts = useLedgerAccounts(tenantId)
  return useMemo(() => accountDepths(accounts.data), [accounts.data])
}

/** Heading, account lines and a total row for one statement section. */
export function sectionRows(key: string, heading: string, section: ApiSection, totalLabel: string): StatementRow[] {
  return [
    { key: `${key}-h`, kind: 'heading', label: heading },
    ...section.lines.map((l) => ({ key: `${key}-${l.account_id}`, kind: 'line' as const, label: l.name, code: l.code, accountId: l.account_id, amount: l.amount })),
    { key: `${key}-t`, kind: 'total', label: totalLabel, amount: section.total },
  ]
}

/** "৳1,200.00 Dr" / "৳300.00 Cr" for a signed debit − credit balance. */
export function useDrCr() {
  const { t, i18n } = useTranslation()
  return (value: string) => {
    const v = dec(value)
    if (v === 0n) return money('0', i18n.language)
    return `${money(toFixed(v < 0n ? -v : v, 2), i18n.language)} ${v < 0n ? t('reports.cr') : t('reports.dr')}`
  }
}

export function statementCsv(t: (key: string) => string): CsvColumn<StatementRow>[] {
  return [
    { header: t('reports.code'), value: (r) => r.code },
    { header: t('reports.account'), value: (r) => r.label },
    { header: t('common.amount'), value: (r) => r.amount },
  ]
}
