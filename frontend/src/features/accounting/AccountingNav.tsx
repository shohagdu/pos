import { useTranslation } from 'react-i18next'

import { Tabs } from '../../components/ui/Tabs'
import { useSession } from '../auth/session'
import { reportAccess } from '../reports/access'

/**
 * Journals / periods / statements tabs shared by the accounting pages and the
 * trial balance / general ledger reports, so switching tabs keeps the bar.
 * Only tabs the member may open are shown.
 */
export function AccountingNav() {
  const { t } = useTranslation()
  const { can } = useSession()
  const access = reportAccess(can)
  const items = [
    // `end: false` keeps Journals active on /accounting/journals/:id.
    access.journals && { to: '/accounting/journals', label: t('accounting.journals'), end: false },
    access.journals && { to: '/accounting/periods', label: t('accounting.periods.title') },
    access.statements && { to: '/reports/trial-balance', label: t('accounting.trialBalance') },
    access.statements && { to: '/reports/general-ledger', label: t('accounting.generalLedger') },
  ].filter((i) => !!i)
  if (items.length < 2) return null
  return (
    <div className="print:hidden">
      <Tabs label={t('nav.accounting')} items={items} />
    </div>
  )
}
