import { useTranslation } from 'react-i18next'

import { Tabs } from '../../components/ui/Tabs'

/** Journals / periods / statements links shared by the accounting pages. */
export function AccountingNav() {
  const { t } = useTranslation()
  return (
    <div className="print:hidden">
      <Tabs
        label={t('nav.accounting')}
        items={[
          { to: '/accounting/journals', label: t('accounting.journals') },
          { to: '/accounting/periods', label: t('accounting.periods.title') },
          { to: '/reports/trial-balance', label: t('accounting.trialBalance') },
          { to: '/reports/general-ledger', label: t('accounting.generalLedger') },
        ]}
      />
    </div>
  )
}
