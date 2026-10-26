import type { RouteObject } from 'react-router'

import { SessionsPage } from '../features/account/SessionsPage'
import { AdminClaimsPage } from '../features/admin/AdminClaimsPage'
import { AdminDashboardPage } from '../features/admin/AdminDashboardPage'
import { AdminLayout } from '../features/admin/AdminLayout'
import { AdminPlansPage } from '../features/admin/AdminPlansPage'
import { AdminSettingsPage } from '../features/admin/AdminSettingsPage'
import { AdminTenantDetailPage } from '../features/admin/AdminTenantDetailPage'
import { AdminTenantsPage } from '../features/admin/AdminTenantsPage'
import { AuditLogPage } from '../features/audit/AuditLogPage'
import { JournalDetailPage } from '../features/accounting/JournalDetailPage'
import { JournalsPage } from '../features/accounting/JournalsPage'
import { PeriodsPage } from '../features/accounting/PeriodsPage'
import { CustomerDetailPage } from '../features/customers/CustomerDetailPage'
import { CustomerFormPage } from '../features/customers/CustomerFormPage'
import { CustomersPage } from '../features/customers/CustomersPage'
import { ExpenseCategoriesPage } from '../features/expenses/ExpenseCategoriesPage'
import { ExpensesPage } from '../features/expenses/ExpensesPage'
import { IncomeCategoriesPage } from '../features/income/IncomeCategoriesPage'
import { IncomesPage } from '../features/income/IncomesPage'
import { AdjustmentDetailPage } from '../features/inventory/AdjustmentDetailPage'
import { AdjustmentFormPage } from '../features/inventory/AdjustmentFormPage'
import { AdjustmentsPage } from '../features/inventory/AdjustmentsPage'
import { CountDetailPage } from '../features/inventory/CountDetailPage'
import { CountsPage } from '../features/inventory/CountsPage'
import { MovementsPage } from '../features/inventory/MovementsPage'
import { StockPage } from '../features/inventory/StockPage'
import { TransferDetailPage } from '../features/inventory/TransferDetailPage'
import { TransferFormPage } from '../features/inventory/TransferFormPage'
import { TransfersPage } from '../features/inventory/TransfersPage'
import { AccountBookPage } from '../features/money/AccountBookPage'
import { AccountsPage } from '../features/money/AccountsPage'
import { NotificationSettingsPage } from '../features/notifications/NotificationSettingsPage'
import { NotificationsPage } from '../features/notifications/NotificationsPage'
import { ReconciliationPage } from '../features/money/ReconciliationPage'
import { ReconciliationsPage } from '../features/money/ReconciliationsPage'
import { PosPage } from '../features/pos/PosPage'
import { PurchaseDetailPage } from '../features/purchases/PurchaseDetailPage'
import { PurchaseFormPage } from '../features/purchases/PurchaseFormPage'
import { PurchaseReturnDetailPage } from '../features/purchases/PurchaseReturnDetailPage'
import { PurchaseReturnPage } from '../features/purchases/PurchaseReturnPage'
import { PurchaseReturnsPage } from '../features/purchases/PurchaseReturnsPage'
import { PurchasesPage } from '../features/purchases/PurchasesPage'
import { ShiftReportPage } from '../features/registers/ShiftReportPage'
import { ShiftsPage } from '../features/registers/ShiftsPage'
import { AgingReportPage } from '../features/reports/AgingReportPage'
import { BalanceSheetPage } from '../features/reports/BalanceSheetPage'
import { DiscountReportPage } from '../features/reports/DiscountReportPage'
import { ExpenseReportPage } from '../features/reports/ExpenseReportPage'
import { GeneralLedgerPage } from '../features/reports/GeneralLedgerPage'
import { InventoryReportPage } from '../features/reports/InventoryReportPage'
import { PaymentReportPage } from '../features/reports/PaymentReportPage'
import { ProfitLossPage } from '../features/reports/ProfitLossPage'
import { ProfitReportPage } from '../features/reports/ProfitReportPage'
import { ProductMovementPage } from '../features/reports/ProductMovementPage'
import { VatReportPage } from '../features/reports/VatReportPage'
import { PurchaseReportPage } from '../features/reports/PurchaseReportPage'
import { ReportsPage } from '../features/reports/ReportsPage'
import { SalesReportPage } from '../features/reports/SalesReportPage'
import { TrialBalancePage } from '../features/reports/TrialBalancePage'
import { SaleDetailPage } from '../features/sales/SaleDetailPage'
import { SaleReturnDetailPage } from '../features/sales/SaleReturnDetailPage'
import { SaleReturnPage } from '../features/sales/SaleReturnPage'
import { SaleReturnsPage } from '../features/sales/SaleReturnsPage'
import { SalesPage } from '../features/sales/SalesPage'
import { SupplierDetailPage } from '../features/suppliers/SupplierDetailPage'
import { SupplierFormPage } from '../features/suppliers/SupplierFormPage'
import { SuppliersPage } from '../features/suppliers/SuppliersPage'
import { ChooseBusinessPage } from '../features/auth/ChooseBusinessPage'
import { ForgotPasswordPage } from '../features/auth/ForgotPasswordPage'
import { LoginPage } from '../features/auth/LoginPage'
import { RegisterPage } from '../features/auth/RegisterPage'
import { ResetPasswordPage } from '../features/auth/ResetPasswordPage'
import { VerifyPhonePage } from '../features/auth/VerifyPhonePage'
import { PlanPage } from '../features/billing/PlanPage'
import { DashboardPage } from '../features/dashboard/DashboardPage'
import { CreateBusinessPage } from '../features/onboarding/CreateBusinessPage'
import { OnboardingLayout } from '../features/onboarding/OnboardingLayout'
import { WizardStepPage } from '../features/onboarding/WizardStepPage'
import { CatalogSetupPage } from '../features/products/CatalogSetupPage'
import { ProductFormPage } from '../features/products/ProductFormPage'
import { ProductsPage } from '../features/products/ProductsPage'
import { SettingsPage } from '../features/settings/SettingsPage'
import { InvitationPage } from '../features/users/InvitationPage'
import { InvitePage } from '../features/users/InvitePage'
import { MemberPage } from '../features/users/MemberPage'
import { RoleEditorPage } from '../features/users/RoleEditorPage'
import { UsersPage } from '../features/users/UsersPage'
import { GuestOnly, RequireSession, RequireTenant } from './guards'
import { AppLayout } from './layouts/AppLayout'
import { AuthLayout } from './layouts/AuthLayout'
import { RouteError } from './RouteError'

export const routes: RouteObject[] = [
  {
    errorElement: <RouteError />,
    children: [
      {
        element: <AuthLayout />,
        children: [
          {
            element: <GuestOnly />,
            children: [
              { path: '/login', element: <LoginPage /> },
              { path: '/register', element: <RegisterPage /> },
              { path: '/forgot-password', element: <ForgotPasswordPage /> },
              { path: '/reset-password', element: <ResetPasswordPage /> },
            ],
          },
          // Reached right after sign-up (no session yet) and by signed-in, unverified users.
          { path: '/verify', element: <VerifyPhonePage /> },
          // Public: works signed out (sign in / sign up first) and signed in.
          { path: '/invite', element: <InvitationPage /> },
        ],
      },
      {
        element: <RequireSession />,
        children: [
          { element: <AuthLayout />, children: [{ path: '/choose-business', element: <ChooseBusinessPage /> }] },
          // Super Admin console: platform staff only (checked by AdminLayout and the API).
          {
            element: <AdminLayout />,
            children: [
              { path: '/admin', element: <AdminDashboardPage /> },
              { path: '/admin/tenants', element: <AdminTenantsPage /> },
              { path: '/admin/tenants/:id', element: <AdminTenantDetailPage /> },
              { path: '/admin/plans', element: <AdminPlansPage /> },
              { path: '/admin/claims', element: <AdminClaimsPage /> },
              { path: '/admin/settings', element: <AdminSettingsPage /> },
            ],
          },
          {
            element: <OnboardingLayout />,
            children: [
              { path: '/onboarding/business', element: <CreateBusinessPage /> },
              { element: <RequireTenant />, children: [{ path: '/onboarding/:step', element: <WizardStepPage /> }] },
            ],
          },
          {
            element: <RequireTenant />,
            children: [
              // The POS is full-screen, without the sidebar.
              { path: '/pos', element: <PosPage /> },
              {
                element: <AppLayout />,
                children: [
                  { path: '/', element: <DashboardPage /> },
                  { path: '/settings', element: <SettingsPage /> },
                  { path: '/settings/:section', element: <SettingsPage /> },
                  { path: '/account/sessions', element: <SessionsPage /> },
                  { path: '/account/notifications', element: <NotificationSettingsPage /> },
                  { path: '/notifications', element: <NotificationsPage /> },
                  { path: '/plan', element: <PlanPage /> },
                  { path: '/users', element: <UsersPage /> },
                  { path: '/users/:tab', element: <UsersPage /> },
                  { path: '/users/invite', element: <InvitePage /> },
                  { path: '/users/members/:id', element: <MemberPage /> },
                  { path: '/users/roles/new', element: <RoleEditorPage /> },
                  { path: '/users/roles/:id', element: <RoleEditorPage /> },
                  { path: '/products', element: <ProductsPage /> },
                  { path: '/products/new', element: <ProductFormPage /> },
                  { path: '/products/:id', element: <ProductFormPage /> },
                  { path: '/products/setup/:section', element: <CatalogSetupPage /> },
                  { path: '/sales', element: <SalesPage /> },
                  { path: '/sales/returns', element: <SaleReturnsPage /> },
                  { path: '/sales/returns/:id', element: <SaleReturnDetailPage /> },
                  { path: '/sales/:id', element: <SaleDetailPage /> },
                  { path: '/sales/:id/return', element: <SaleReturnPage /> },
                  { path: '/purchases', element: <PurchasesPage /> },
                  { path: '/purchases/new', element: <PurchaseFormPage /> },
                  { path: '/purchases/returns', element: <PurchaseReturnsPage /> },
                  { path: '/purchases/returns/:id', element: <PurchaseReturnDetailPage /> },
                  { path: '/purchases/:id', element: <PurchaseDetailPage /> },
                  { path: '/purchases/:id/edit', element: <PurchaseFormPage /> },
                  { path: '/purchases/:id/return', element: <PurchaseReturnPage /> },
                  { path: '/suppliers', element: <SuppliersPage /> },
                  { path: '/suppliers/new', element: <SupplierFormPage /> },
                  { path: '/suppliers/:id', element: <SupplierDetailPage /> },
                  { path: '/suppliers/:id/edit', element: <SupplierFormPage /> },
                  { path: '/customers', element: <CustomersPage /> },
                  { path: '/customers/new', element: <CustomerFormPage /> },
                  { path: '/customers/:id', element: <CustomerDetailPage /> },
                  { path: '/customers/:id/edit', element: <CustomerFormPage /> },
                  { path: '/inventory', element: <StockPage /> },
                  { path: '/inventory/movements', element: <MovementsPage /> },
                  { path: '/inventory/adjustments', element: <AdjustmentsPage /> },
                  { path: '/inventory/adjustments/new', element: <AdjustmentFormPage /> },
                  { path: '/inventory/adjustments/:id', element: <AdjustmentDetailPage /> },
                  { path: '/inventory/transfers', element: <TransfersPage /> },
                  { path: '/inventory/transfers/new', element: <TransferFormPage /> },
                  { path: '/inventory/transfers/:id', element: <TransferDetailPage /> },
                  { path: '/inventory/counts', element: <CountsPage /> },
                  { path: '/inventory/counts/:id', element: <CountDetailPage /> },
                  { path: '/shifts', element: <ShiftsPage /> },
                  { path: '/shifts/:id', element: <ShiftReportPage /> },
                  { path: '/accounts', element: <AccountsPage /> },
                  { path: '/accounts/reconciliations', element: <ReconciliationsPage /> },
                  { path: '/accounts/reconciliations/:id', element: <ReconciliationPage /> },
                  { path: '/accounts/:id', element: <AccountBookPage /> },
                  { path: '/expenses', element: <ExpensesPage /> },
                  { path: '/expenses/categories', element: <ExpenseCategoriesPage /> },
                  { path: '/incomes', element: <IncomesPage /> },
                  { path: '/incomes/categories', element: <IncomeCategoriesPage /> },
                  { path: '/reports', element: <ReportsPage /> },
                  { path: '/reports/sales', element: <SalesReportPage /> },
                  { path: '/reports/purchases', element: <PurchaseReportPage /> },
                  { path: '/reports/inventory', element: <InventoryReportPage /> },
                  { path: '/reports/expenses', element: <ExpenseReportPage /> },
                  { path: '/reports/payments', element: <PaymentReportPage /> },
                  { path: '/reports/profit', element: <ProfitReportPage /> },
                  { path: '/reports/trial-balance', element: <TrialBalancePage /> },
                  { path: '/reports/profit-and-loss', element: <ProfitLossPage /> },
                  { path: '/reports/balance-sheet', element: <BalanceSheetPage /> },
                  { path: '/reports/general-ledger', element: <GeneralLedgerPage /> },
                  { path: '/reports/aging', element: <AgingReportPage /> },
                  { path: '/reports/vat', element: <VatReportPage /> },
                  { path: '/reports/discounts', element: <DiscountReportPage /> },
                  { path: '/reports/product-movement', element: <ProductMovementPage /> },
                  { path: '/accounting/journals', element: <JournalsPage /> },
                  { path: '/accounting/journals/:id', element: <JournalDetailPage /> },
                  { path: '/accounting/periods', element: <PeriodsPage /> },
                  { path: '/audit', element: <AuditLogPage /> },
                ],
              },
            ],
          },
        ],
      },
    ],
  },
]
