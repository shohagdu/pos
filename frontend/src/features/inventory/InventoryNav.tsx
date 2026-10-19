import { useTranslation } from 'react-i18next'

import { Tabs } from '../../components/ui/Tabs'
import { useSession } from '../auth/session'

/** Stock / Movements / Adjustments / Transfers / Counts sub-navigation. */
export function InventoryNav() {
  const { t } = useTranslation()
  const { can } = useSession()
  return (
    <div className="print:hidden">
      <Tabs
        label={t('inventory.sections')}
        items={[
          { to: '/inventory', label: t('inventory.stock') },
          { to: '/inventory/movements', label: t('inventory.movements') },
          { to: '/inventory/adjustments', label: t('inventory.adjustments'), end: false },
          { to: '/inventory/transfers', label: t('inventory.transfers'), end: false },
          ...(can('inventory.count') ? [{ to: '/inventory/counts', label: t('inventory.counts'), end: false }] : []),
        ]}
      />
    </div>
  )
}
