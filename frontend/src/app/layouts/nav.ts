import type { IconName } from './Icon'

export type NavItem = { to: string; key: string; icon: IconName; permission: string | string[] | null }

// Grouped by the job the shopkeeper is doing. Each entry names the
// permissions that show it (any of them; `null` = every member); a section
// without visible entries is hidden. Further modules add themselves here.
export const NAV: { section: string | null; items: NavItem[] }[] = [
  {
    section: null,
    items: [
      { to: '/', key: 'nav.dashboard', icon: 'dashboard', permission: null },
      { to: '/pos', key: 'nav.pos', icon: 'pos', permission: 'pos.sell' },
    ],
  },
  {
    section: 'nav.sections.selling',
    items: [
      { to: '/sales', key: 'nav.sales', icon: 'sales', permission: 'sales.view' },
      { to: '/customers', key: 'nav.customers', icon: 'customers', permission: 'customers.view' },
      { to: '/shifts', key: 'nav.shifts', icon: 'shifts', permission: 'pos.sell' },
    ],
  },
  {
    section: 'nav.sections.buying',
    items: [
      { to: '/purchases', key: 'nav.purchases', icon: 'purchases', permission: 'purchases.view' },
      { to: '/suppliers', key: 'nav.suppliers', icon: 'suppliers', permission: 'suppliers.view' },
    ],
  },
  {
    section: 'nav.sections.stock',
    items: [
      { to: '/products', key: 'nav.products', icon: 'products', permission: 'products.view' },
      { to: '/inventory', key: 'nav.inventory', icon: 'inventory', permission: 'inventory.view' },
    ],
  },
  {
    section: 'nav.sections.money',
    items: [
      { to: '/accounts', key: 'nav.money', icon: 'money', permission: 'accounts.view' },
      { to: '/expenses', key: 'nav.expenses', icon: 'expenses', permission: 'expenses.view' },
      { to: '/incomes', key: 'nav.income', icon: 'income', permission: ['income.create', 'expenses.view'] },
      {
        to: '/reports',
        key: 'nav.reports',
        icon: 'reports',
        permission: ['reports.sales', 'reports.purchases', 'reports.inventory', 'reports.financial', 'reports.parties', 'accounting.view'],
      },
      { to: '/accounting/journals', key: 'nav.accounting', icon: 'accounting', permission: 'accounting.view' },
    ],
  },
  {
    section: 'nav.sections.business',
    items: [
      { to: '/users', key: 'nav.users', icon: 'users', permission: 'users.view' },
      { to: '/audit', key: 'nav.audit', icon: 'audit', permission: 'audit.view' },
      { to: '/settings', key: 'nav.settings', icon: 'settings', permission: ['settings.manage', 'branches.manage'] },
      { to: '/plan', key: 'nav.plan', icon: 'plan', permission: null },
    ],
  },
]

/** Label key of the menu entry the path belongs to, matched on its first segment. */
export function navKeyFor(pathname: string): string | null {
  const first = (path: string) => path.split('/')[1] ?? ''
  for (const g of NAV) for (const i of g.items) if (first(i.to) === first(pathname)) return i.key
  return null
}
