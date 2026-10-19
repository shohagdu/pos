import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { PageHeader } from '../../components/ui/PageHeader'
import { useSession } from '../auth/session'
import { type ReportAccess, reportAccess } from './access'

interface HubCard {
  key: string
  to: string
  access: keyof ReportAccess
}

const GROUPS: { key: string; cards: HubCard[] }[] = [
  {
    key: 'business',
    cards: [
      { key: 'sales', to: '/reports/sales', access: 'sales' },
      { key: 'purchases', to: '/reports/purchases', access: 'purchases' },
      { key: 'inventory', to: '/reports/inventory', access: 'inventory' },
      { key: 'expenses', to: '/reports/expenses', access: 'expenses' },
      { key: 'payments', to: '/reports/payments', access: 'payments' },
      { key: 'profit', to: '/reports/profit', access: 'profit' },
    ],
  },
  {
    key: 'analysis',
    cards: [
      { key: 'aging', to: '/reports/aging', access: 'parties' },
      { key: 'productMovement', to: '/reports/product-movement', access: 'movement' },
      { key: 'discounts', to: '/reports/discounts', access: 'discounts' },
      { key: 'vat', to: '/reports/vat', access: 'vat' },
    ],
  },
  {
    key: 'accounts',
    cards: [
      { key: 'trialBalance', to: '/reports/trial-balance', access: 'statements' },
      { key: 'profitLoss', to: '/reports/profit-and-loss', access: 'statements' },
      { key: 'balanceSheet', to: '/reports/balance-sheet', access: 'statements' },
      { key: 'generalLedger', to: '/reports/general-ledger', access: 'statements' },
      { key: 'journals', to: '/accounting/journals', access: 'journals' },
      { key: 'periods', to: '/accounting/periods', access: 'journals' },
    ],
  },
]

/** Reports hub: one card per report the member may open. */
export function ReportsPage() {
  const { t } = useTranslation()
  const { can } = useSession()
  const access = reportAccess(can)
  const groups = GROUPS.map((g) => ({ ...g, cards: g.cards.filter((c) => access[c.access]) })).filter((g) => g.cards.length > 0)

  return (
    <div className="flex max-w-5xl flex-col gap-6">
      <PageHeader title={t('reports.title')} subtitle={t('reports.subtitle')} />
      {groups.length === 0 && <p className="text-sm text-ink-muted">{t('reports.noReports')}</p>}
      {groups.map((g) => (
        <section key={g.key} className="flex flex-col gap-3" aria-labelledby={`reports-${g.key}`}>
          <h2 id={`reports-${g.key}`} className="text-lg font-semibold">
            {t(`reports.groups.${g.key}`)}
          </h2>
          <ul className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
            {g.cards.map((c) => (
              <li key={c.key}>
                <Link
                  to={c.to}
                  className="flex h-full flex-col gap-1 rounded-xl border border-border bg-surface p-4 shadow-sm hover:border-brand-500 focus-visible:outline-2 focus-visible:outline-brand-600"
                >
                  <span className="font-semibold text-brand-700">{t(`reports.cards.${c.key}.title`)}</span>
                  <span className="text-sm text-ink-muted">{t(`reports.cards.${c.key}.description`)}</span>
                </Link>
              </li>
            ))}
          </ul>
        </section>
      ))}
    </div>
  )
}
