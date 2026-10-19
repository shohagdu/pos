import { useTranslation } from 'react-i18next'
import { NavLink } from 'react-router'

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
  ]
  return (
    <nav aria-label={t('products.tabs.label')} className="flex flex-wrap gap-1 border-b border-border">
      {tabs.map((tab) => (
        <NavLink
          key={tab.to}
          to={tab.to}
          end
          className={({ isActive }) =>
            `-mb-px border-b-2 px-3 py-2 text-sm font-medium ${isActive ? 'border-brand-600 text-brand-700' : 'border-transparent text-ink-muted hover:text-ink'}`
          }
        >
          {t(tab.key)}
        </NavLink>
      ))}
    </nav>
  )
}
