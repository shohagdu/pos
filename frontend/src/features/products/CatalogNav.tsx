import { useTranslation } from 'react-i18next'
import { Tabs } from '../../components/ui/Tabs'

import { useSession } from '../auth/session'

/** Tabs shared by the products list and the catalogue setup pages. */
export function CatalogNav() {
  const { t } = useTranslation()
  const { can } = useSession()
  const tabs = [
    { to: '/products', key: 'products.tabs.products' },
    { to: '/products/setup/categories', key: 'products.tabs.categories' },
    { to: '/products/setup/brands', key: 'products.tabs.brands' },
    { to: '/products/setup/units', key: 'products.tabs.units' },
    ...(can('settings.manage') ? [{ to: '/products/setup/tax-rates', key: 'products.tabs.taxRates' }] : []),
    { to: '/labels', key: 'labels.nav' },
  ]
  return <Tabs label={t('products.tabs.label')} items={tabs.map((tab) => ({ to: tab.to, label: t(tab.key) }))} />
}
