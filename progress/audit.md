    # Responsive audit

Checked in Bengali at 360, 390 (mobile), 768, 1024 (tablet), 1366×768 (laptop), 1440, 1920 (desktop). Device columns show the **current** result (after fixes when verified; per-page files keep the before/after detail).

**Fail**: page scrolls sideways, a table scrolls sideways on laptop/desktop, or amounts cut off with "…". **Warn**: a table scrolls sideways inside its frame on a phone/tablet, or a work screen scrolls at 1366×768.

| Route | Page name | Page type | Mobile | Tablet | Laptop | Desktop | Status | Notes |
|---|---|---|---|---|---|---|---|---|
| `/login` | [login](pages/login.md) | Auth form | Pass | Pass | Pass | Pass | Passed |  |
| `/register` | [register](pages/register.md) | Auth form | Pass | Pass | Pass | Pass | Passed |  |
| `/forgot-password` | [forgot-password](pages/forgot-password.md) | Auth form | Pass | Pass | Pass | Pass | Passed |  |
| `/reset-password` | [reset-password](pages/reset-password.md) | Auth form | Pass | Pass | Pass | Pass | Passed |  |
| `/verify` | [verify](pages/verify.md) | Auth form | Pass | Pass | Pass | Pass | Passed |  |
| `/invite` | [invite](pages/invite.md) | Auth form | Pass | Pass | Pass | Pass | Passed |  |
| `/choose-business` | [choose-business](pages/choose-business.md) | Auth form | Pass | Pass | Pass | Pass | Passed |  |
| `/` | [dashboard](pages/dashboard.md) | Dashboard | Pass | Pass | Pass | Pass | Fixed | KPI amounts wrap instead of "…", 6 per row only from 2xl; lists in half-width panels become cards when they do not fit. |
| `/pos` | [pos](pages/pos.md) | POS sale screen | Pass | Pass | Pass | Pass | Fixed | Products/Cart switch + sticky total/Pay bar below lg; customer & discount bottom sheet (F4/F7 open it); 44px targets; serial-number badges; product images setting also covers cart and search. |
| `/settings/regional` | [settings-regional](pages/settings-regional.md) | Settings form | Pass | Pass | Pass | Pass | Passed |  |
| `/settings/branch` | [settings-branch](pages/settings-branch.md) | Settings form | Pass | Pass | Pass | Pass | Passed |  |
| `/settings/invoice` | [settings-invoice](pages/settings-invoice.md) | Settings form | Pass | Pass | Pass | Pass | Passed |  |
| `/settings/codes` | [settings-codes](pages/settings-codes.md) | Settings form | Pass | Pass | Pass | Pass | Passed |  |
| `/settings/branches` | [settings-branches](pages/settings-branches.md) | Settings form | Pass | Pass | Pass | Pass | Fixed |  |
| `/settings/registers` | [settings-registers](pages/settings-registers.md) | Settings form | Pass | Pass | Pass | Pass | Fixed |  |
| `/settings/data` | [settings-data](pages/settings-data.md) | Settings form | Pass | Pass | Pass | Pass | Passed |  |
| `/account/sessions` | [account-sessions](pages/account-sessions.md) | List / table | Pass | Pass | Pass | Pass | Passed |  |
| `/account/password` | [account-password](pages/account-password.md) | Auth form | Pass | Pass | Pass | Pass | Passed |  |
| `/account/notifications` | [account-notifications](pages/account-notifications.md) | Settings form | Pass | Pass | Pass | Pass | Passed |  |
| `/notifications` | [notifications](pages/notifications.md) | List / table | Pass | Pass | Pass | Pass | Passed |  |
| `/plan` | [plan](pages/plan.md) | List / table | Pass | Pass | Pass | Pass | Fixed |  |
| `/users` | [users](pages/users.md) | List / table | Pass | Pass | Pass | Pass | Passed | Tabs use the shared Tabs. |
| `/users/invitations` | [users-invitations](pages/users-invitations.md) | List / table | Pass | Pass | Pass | Pass | Passed |  |
| `/users/roles` | [users-roles](pages/users-roles.md) | List / table | Pass | Pass | Pass | Pass | Passed |  |
| `/users/invite` | [users-invite](pages/users-invite.md) | Form | Pass | Pass | Pass | Pass | Passed |  |
| `/users/members/:id` | [users-members-detail](pages/users-members-detail.md) | Detail | Pass | Pass | Pass | Pass | Passed |  |
| `/users/roles/new` | [users-roles-new](pages/users-roles-new.md) | Form | Pass | Pass | Pass | Pass | Passed |  |
| `/users/roles/:id` | [users-roles-detail](pages/users-roles-detail.md) | Form | Pass | Pass | Pass | Pass | Passed |  |
| `/products` | [products](pages/products.md) | List / table | Pass | Pass | Pass | Pass | Fixed | Hand-written table moved to the shared DataTable (cards below lg, barcode action in the card footer); search field no longer forces 240px; tabs use the shared Tabs. |
| `/products/new` | [products-new](pages/products-new.md) | Form | Pass | Pass | Pass | Pass | Fixed | Was 46px too wide at every size: hidden ⓘ tooltips (InfoTip) kept their width. Sticky save bar margins follow the new page padding. |
| `/products/import` | [products-import](pages/products-import.md) | Form | Pass | Pass | Pass | Pass | Passed |  |
| `/products/:id` | [products-detail](pages/products-detail.md) | Form | Pass | Pass | Pass | Pass | Fixed | Same as /products/new. |
| `/products/setup/categories` | [products-setup-categories](pages/products-setup-categories.md) | List / table | Pass | Pass | Pass | Pass | Passed |  |
| `/products/setup/brands` | [products-setup-brands](pages/products-setup-brands.md) | List / table | Pass | Pass | Pass | Pass | Passed |  |
| `/products/setup/units` | [products-setup-units](pages/products-setup-units.md) | List / table | Pass | Pass | Pass | Pass | Passed |  |
| `/products/setup/tax-rates` | [products-setup-tax-rates](pages/products-setup-tax-rates.md) | List / table | Pass | Pass | Pass | Pass | Passed |  |
| `/labels` | [labels](pages/labels.md) | Form | Pass | Pass | Pass | Pass | Passed | Grid got a grid-cols-1 base (was 11px too wide at 360). |
| `/sales` | [sales](pages/sales.md) | List / table | Pass | Pass | Pass | Pass | Fixed | Hand-made cards below xl, filters fold behind a Filters button below lg, Paid/Cashier columns from 2xl. |
| `/sales/returns` | [sales-returns](pages/sales-returns.md) | List / table | Pass | Pass | Pass | Pass | Fixed |  |
| `/sales/returns/:id` | [sales-returns-detail](pages/sales-returns-detail.md) | Detail | Pass | Pass | Pass | Pass | Fixed |  |
| `/sales/warranties` | [sales-warranties](pages/sales-warranties.md) | List / table | Pass | Pass | Pass | Pass | Fixed |  |
| `/sales/:id` | [sales-detail](pages/sales-detail.md) | Detail | Pass | Pass | Pass | Pass | Fixed | Hand-made item and payment cards below lg, totals under payments below xl, unit cost column from xl, "Print without returned items". |
| `/sales/:id/return` | [sales-detail-return](pages/sales-detail-return.md) | Form | Pass | Pass | Pass | Pass | Fixed | Line table uses table-stack (one card per line with labelled sold / returnable / quantity / condition / refund). |
| `/purchases` | [purchases](pages/purchases.md) | List / table | Pass | Pass | Pass | Pass | Fixed |  |
| `/purchases/new` | [purchases-new](pages/purchases-new.md) | Form | Pass | Pass | Pass | Pass | Passed |  |
| `/purchases/returns` | [purchases-returns](pages/purchases-returns.md) | List / table | Pass | Pass | Pass | Pass | Fixed |  |
| `/purchases/returns/:id` | [purchases-returns-detail](pages/purchases-returns-detail.md) | Detail | Pass | Pass | Pass | Pass | Passed |  |
| `/purchases/:id` | [purchases-detail](pages/purchases-detail.md) | Detail | Pass | Pass | Pass | Pass | Fixed |  |
| `/purchases/:id/edit` | [purchases-detail-edit](pages/purchases-detail-edit.md) | Form | Pass | Pass | Pass | Pass | Passed |  |
| `/purchases/:id/return` | [purchases-detail-return](pages/purchases-detail-return.md) | Form | Pass | Pass | Pass | Pass | Fixed | Line table uses table-stack. |
| `/suppliers` | [suppliers](pages/suppliers.md) | List / table | Pass | Pass | Pass | Pass | Fixed |  |
| `/suppliers/new` | [suppliers-new](pages/suppliers-new.md) | Form | Pass | Pass | Pass | Pass | Passed |  |
| `/suppliers/:id` | [suppliers-detail](pages/suppliers-detail.md) | Detail | Pass | Pass | Pass | Pass | Fixed |  |
| `/suppliers/:id/edit` | [suppliers-detail-edit](pages/suppliers-detail-edit.md) | Form | Pass | Pass | Pass | Pass | Passed |  |
| `/customers` | [customers](pages/customers.md) | List / table | Pass | Pass | Pass | Pass | Fixed | Card title is the name, balance on the right. |
| `/customers/new` | [customers-new](pages/customers-new.md) | Form | Pass | Pass | Pass | Pass | Passed |  |
| `/customers/:id` | [customers-detail](pages/customers-detail.md) | Detail | Pass | Pass | Pass | Pass | Fixed | Statement table uses table-stack; name is the card title in lists. |
| `/customers/:id/edit` | [customers-detail-edit](pages/customers-detail-edit.md) | Form | Pass | Pass | Pass | Pass | Passed |  |
| `/inventory` | [inventory](pages/inventory.md) | List / table | Pass | Pass | Pass | Pass | Fixed | Quantity is the card headline; table becomes cards whenever it does not fit (1024 with sidebar). |
| `/inventory/movements` | [inventory-movements](pages/inventory-movements.md) | List / table | Pass | Pass | Pass | Pass | Fixed |  |
| `/inventory/expiry` | [inventory-expiry](pages/inventory-expiry.md) | List / table | Pass | Pass | Pass | Pass | Passed |  |
| `/inventory/adjustments` | [inventory-adjustments](pages/inventory-adjustments.md) | List / table | Pass | Pass | Pass | Pass | Fixed |  |
| `/inventory/adjustments/new` | [inventory-adjustments-new](pages/inventory-adjustments-new.md) | Form | Pass | Pass | Pass | Pass | Passed | Line table uses table-stack (once lines are added). |
| `/inventory/adjustments/:id` | [inventory-adjustments-detail](pages/inventory-adjustments-detail.md) | Detail | Pass | Pass | Pass | Pass | Fixed |  |
| `/inventory/transfers` | [inventory-transfers](pages/inventory-transfers.md) | List / table | Pass | Pass | Pass | Pass | Fixed |  |
| `/inventory/transfers/new` | [inventory-transfers-new](pages/inventory-transfers-new.md) | Form | Pass | Pass | Pass | Pass | Passed |  |
| `/inventory/transfers/:id` | [inventory-transfers-detail](pages/inventory-transfers-detail.md) | Detail | Pass | Pass | Pass | Pass | Passed |  |
| `/inventory/counts` | [inventory-counts](pages/inventory-counts.md) | List / table | Pass | Pass | Pass | Pass | Passed |  |
| `/shifts` | [shifts](pages/shifts.md) | List / table | Pass | Pass | Pass | Pass | Fixed |  |
| `/shifts/:id` | [shifts-detail](pages/shifts-detail.md) | Detail | Pass | Pass | Pass | Pass | Fixed |  |
| `/accounts` | [accounts](pages/accounts.md) | List / table | Pass | Pass | Pass | Pass | Fixed | Payment methods table uses table-stack. |
| `/accounts/reconciliations` | [accounts-reconciliations](pages/accounts-reconciliations.md) | List / table | Pass | Pass | Pass | Pass | Passed |  |
| `/accounts/:id` | [accounts-detail](pages/accounts-detail.md) | Detail | Pass | Pass | Pass | Pass | Fixed | Account book uses table-stack (opening balance and totals stay label + amount). |
| `/expenses` | [expenses](pages/expenses.md) | List / table | Pass | Pass | Pass | Pass | Passed |  |
| `/expenses/categories` | [expenses-categories](pages/expenses-categories.md) | List / table | Pass | Pass | Pass | Pass | Fixed |  |
| `/incomes` | [incomes](pages/incomes.md) | List / table | Pass | Pass | Pass | Pass | Passed |  |
| `/incomes/categories` | [incomes-categories](pages/incomes-categories.md) | List / table | Pass | Pass | Pass | Pass | Fixed |  |
| `/reports` | [reports](pages/reports.md) | Dashboard | Pass | Pass | Pass | Pass | Passed |  |
| `/reports/sales` | [reports-sales](pages/reports-sales.md) | Report | Pass | Pass | Pass | Pass | Fixed |  |
| `/reports/purchases` | [reports-purchases](pages/reports-purchases.md) | Report | Pass | Pass | Pass | Pass | Fixed |  |
| `/reports/inventory` | [reports-inventory](pages/reports-inventory.md) | Report | Pass | Pass | Pass | Pass | Fixed |  |
| `/reports/expenses` | [reports-expenses](pages/reports-expenses.md) | Report | Pass | Pass | Pass | Pass | Passed |  |
| `/reports/payments` | [reports-payments](pages/reports-payments.md) | Report | Pass | Pass | Pass | Pass | Fixed |  |
| `/reports/profit` | [reports-profit](pages/reports-profit.md) | Report | Pass | Pass | Pass | Pass | Passed |  |
| `/reports/trial-balance` | [reports-trial-balance](pages/reports-trial-balance.md) | Report | Pass | Pass | Pass | Pass | Fixed |  |
| `/reports/profit-and-loss` | [reports-profit-and-loss](pages/reports-profit-and-loss.md) | Report | Pass | Pass | Pass | Pass | Passed |  |
| `/reports/balance-sheet` | [reports-balance-sheet](pages/reports-balance-sheet.md) | Report | Pass | Pass | Pass | Pass | Passed |  |
| `/reports/general-ledger` | [reports-general-ledger](pages/reports-general-ledger.md) | Report | Pass | Pass | Pass | Pass | Passed |  |
| `/reports/aging` | [reports-aging](pages/reports-aging.md) | Report | Pass | Pass | Pass | Pass | Passed |  |
| `/reports/customer-dues` | [reports-customer-dues](pages/reports-customer-dues.md) | Report | Pass | Pass | Pass | Pass | Passed |  |
| `/reports/supplier-dues` | [reports-supplier-dues](pages/reports-supplier-dues.md) | Report | Pass | Pass | Pass | Pass | Fixed |  |
| `/reports/vat` | [reports-vat](pages/reports-vat.md) | Report | Pass | Pass | Pass | Pass | Passed |  |
| `/reports/discounts` | [reports-discounts](pages/reports-discounts.md) | Report | Pass | Pass | Pass | Pass | Fixed |  |
| `/reports/product-movement` | [reports-product-movement](pages/reports-product-movement.md) | Report | Pass | Pass | Pass | Pass | Fixed |  |
| `/accounting/journals` | [accounting-journals](pages/accounting-journals.md) | List / table | Pass | Pass | Pass | Pass | Fixed |  |
| `/accounting/journals/:id` | [accounting-journals-detail](pages/accounting-journals-detail.md) | Detail | Pass | Pass | Pass | Pass | Fixed |  |
| `/accounting/periods` | [accounting-periods](pages/accounting-periods.md) | List / table | Pass | Pass | Pass | Pass | Passed |  |
| `/audit` | [audit](pages/audit.md) | List / table | Pass | Pass | Pass | Pass | Fixed | Audit table uses table-stack; expand button 44px. |
| `/onboarding/business` | [onboarding-business](pages/onboarding-business.md) | Onboarding form | Pass | Pass | Pass | Pass | Passed |  |
| `/onboarding/account` | [onboarding-account](pages/onboarding-account.md) | Onboarding form | Pass | Pass | Pass | Pass | Fixed | Redirects to /. |
| `/onboarding/regional` | [onboarding-regional](pages/onboarding-regional.md) | Onboarding form | Pass | Pass | Pass | Pass | Passed |  |
| `/onboarding/branch` | [onboarding-branch](pages/onboarding-branch.md) | Onboarding form | Pass | Pass | Pass | Pass | Passed |  |
| `/onboarding/invoice` | [onboarding-invoice](pages/onboarding-invoice.md) | Onboarding form | Pass | Pass | Pass | Pass | Passed |  |
| `/admin` | [admin](pages/admin.md) | Dashboard | Pass | Pass | Pass | Pass | Passed |  |
| `/admin/tenants` | [admin-tenants](pages/admin-tenants.md) | List / table | Pass | Pass | Pass | Pass | Fixed |  |
| `/admin/tenants/:id` | [admin-tenants-detail](pages/admin-tenants-detail.md) | Detail | Pass | Pass | Pass | Pass | Fixed |  |
| `/admin/plans` | [admin-plans](pages/admin-plans.md) | List / table | Pass | Pass | Pass | Pass | Passed |  |
| `/admin/claims` | [admin-claims](pages/admin-claims.md) | List / table | Pass | Pass | Pass | Pass | Passed |  |
| `/admin/settings` | [admin-settings](pages/admin-settings.md) | Settings form | Pass | Pass | Pass | Pass | Passed |  |
