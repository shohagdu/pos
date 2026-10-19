# SaaS POS Software --- Feature List & Development Roadmap

**Project type:** Multi-tenant SaaS POS, inventory, accounting, and
business management platform\
**Reference:** [Hisab Nikash](https://hisabnikash.com/)\
**Initial target market:** Bangladesh, with potential for international
expansion\
**Suggested stack:** React + TypeScript, Tailwind CSS, Rust + Actix Web,
PostgreSQL

> This document is a proposed product plan inspired by common POS/ERP
> capabilities. It is not an exhaustive or independently verified audit
> of the reference website. Build original branding, UI, workflows, and
> implementation.

## 1. Product Vision

Build a subscription-based cloud platform that lets businesses manage
sales, purchases, inventory, customers, suppliers, payments, branches,
and financial reports in one place. The platform should support
different business types through optional industry modules.

## 2. Module Overview

  \#   Module                                 Suggested priority
  ---- -------------------------------------- --------------------
  1    SaaS Super Admin                       P0
  2    Business registration and onboarding   P0
  3    Subscription and billing               P0
  4    Business dashboard                     P0
  5    Product and category management        P0
  6    POS and sales                          P0
  7    Purchase management                    P0
  8    Inventory and stock                    P0
  9    Customer management (CRM)              P0
  10   Supplier management                    P0
  11   Cash, bank, and payment management     P0
  12   Expense and other income               P1
  13   Returns, refunds, and exchanges        P1
  14   Employee and role management           P0
  15   Barcode and label printing             P1
  16   Reports and analytics                  P1
  17   Branch and warehouse management        P1
  18   Accounting                             P1
  19   Loyalty and promotions                 P2
  20   Restaurant POS and kitchen             P2
  21   Pharmacy management                    P3
  22   E-commerce and API integrations        P3
  23   HRM and payroll                        P3
  24   Notifications and messaging            P2
  25   Audit logs and security                P0
  26   Backup, export, and recovery           P0

## 3. SaaS Super Admin

-   Super Admin dashboard
-   Registered, active, trial, suspended, and cancelled business counts
-   Subscription and plan management
-   Business activation, suspension, and reactivation
-   MRR and ARR reporting
-   Trial monitoring and renewal overview
-   Payment and failed-payment monitoring
-   Feature entitlements and plan limits
-   Platform announcements
-   Support request management
-   Global configuration
-   Platform activity and audit logs
-   Usage analytics

## 4. Business Registration & Onboarding

-   Business registration and contact verification
-   Secure login, logout, and password recovery
-   Business name, address, logo, business type, timezone, currency, and
    language
-   VAT/tax configuration
-   First branch creation
-   Initial users and roles
-   Invoice settings and numbering
-   Payment method setup
-   Product, customer, and supplier import from CSV/Excel
-   Guided setup wizard and progress checklist
-   Optional sample/demo data
-   Free trial activation where applicable

**Suggested onboarding flow:** Register → verify contact → enter
business details → create branch → configure invoice and payment
settings → import products/opening stock → create cashier account →
start selling.

## 5. Subscription & Billing

-   Free trial, monthly, quarterly, and annual plans
-   Custom enterprise plans
-   Plan upgrades, downgrades, renewals, and cancellation
-   Grace periods and expiry handling
-   Feature-based access and usage limits
-   Coupons and promotional pricing
-   Payment history and subscription invoices
-   Renewal and failed-payment reminders
-   Manual payment entry and verification
-   Payment gateway integration, subject to merchant access

Potential Bangladesh payment options include bKash, Nagad, SSLCommerz,
bank transfer, and manually verified payments. Use supported merchant
integrations and record each transaction.

## 6. Product & Category Management

-   Create, update, view, archive, and search products
-   SKU, product code, barcode, image, description
-   Categories, subcategories, brands, and units
-   Multiple units and conversion rules
-   Purchase, retail, wholesale, and minimum selling prices
-   Tax/VAT settings and discounts
-   Opening stock and reorder thresholds
-   Variants such as size and color
-   Batch, lot, expiry, and serial-number tracking where applicable
-   Product price and stock movement history
-   Last purchase price, date, supplier, and quantity
-   Bulk import/export and duplicate validation
-   Product status and archive controls

### Industry-specific fields

-   **Clothing:** size, color, design, brand, variant barcode
-   **Electronics:** serial/IMEI where applicable, warranty duration
-   **Grocery:** fractional quantities, weight-based products, expiry
-   **Pharmacy:** manufacturer, batch, expiry, medicine-specific data

## 7. POS & Sales

### Core POS

-   Fast search and barcode scanning
-   Product grid and category filters
-   Cart, quantity adjustment, unit selection
-   Item-level and invoice-level discounts
-   Customer selection and walk-in customer
-   Tax/VAT and total calculation
-   Cash, bank/card, mobile financial service, and split payments
-   Partial payment and due tracking
-   Hold and resume sale
-   Permission-controlled cancellation
-   Invoice printing, reprinting, and email/SMS delivery where
    integrated
-   Thermal and A4 print layouts
-   Cashier and salesperson tracking
-   Sales history and keyboard shortcuts

### Advanced POS

-   Retail/wholesale price levels
-   Customer-specific pricing
-   Bundles and combo offers
-   Promotions and loyalty points
-   Gift cards and store credit
-   Returns, refunds, and exchanges
-   Cash drawer and shift opening/closing
-   Cash reconciliation
-   Offline transaction queue only if designed and tested for safe
    synchronization

### Recommended sales workflow

Scan/search products → adjust cart → select customer → calculate totals
→ take payment → commit sale → update stock and financial records →
issue receipt.

Sales, payment, stock movement, and accounting updates must be
consistent. Use database transactions and idempotency protections where
relevant.

## 8. Purchase Management

-   Purchase orders and supplier selection
-   Multiple products and quantities
-   Purchase unit and unit conversion
-   Purchase price, discounts, tax/VAT, shipping, and other costs
-   Approval workflow
-   Full or partial goods receiving
-   Purchase invoice
-   Supplier payment and due tracking
-   Purchase returns and cancellation controls
-   Printable purchase documents and purchase reports
-   Purchase price history and supplier comparison

### Purchase price intelligence

When a product is selected, show:

-   Last purchase price
-   Last purchase date
-   Last supplier
-   Previous purchase quantity
-   Recent purchase prices
-   Current price difference and percentage change
-   Previous purchase invoice
-   Average or weighted-average cost, according to the configured
    costing method

Illustrative example: if the previous purchase price was ৳1,450 and the
current price is ৳1,475, display the ৳25 increase and the percentage
change. Actual values must come from recorded transactions.

## 9. Inventory & Stock

-   Real-time stock by business, branch, and warehouse
-   Opening stock and stock valuation
-   Stock adjustments with reason and approval
-   Stock transfers and receiving confirmation
-   Damaged, expired, lost, and quarantined goods
-   Stock reservation where needed
-   Low-stock and out-of-stock alerts
-   Physical stock count and reconciliation
-   Stock movement ledger
-   Inventory valuation
-   Fast-, slow-, and non-moving stock reports
-   Expiry and batch tracking where applicable

**Inventory rules:** 1. Every stock movement has a source, destination
where applicable, reason, user, timestamp, and reference. 2. A completed
sale reduces available stock. 3. Received purchases increase stock. 4.
Transfers decrease source stock and increase destination stock
consistently. 5. Returns update stock based on returned-item condition.
6. Historical movements must not be silently overwritten. 7. Use a stock
ledger as the audit trail; any cached current quantity must be
reconcilable to the ledger.

## 10. Customer Management (CRM)

-   Customer profile, contact details, address, and code
-   Customer groups and price levels
-   Credit limit and opening balance
-   Current outstanding balance
-   Purchase and transaction history
-   Payment collection and customer ledger
-   Customer statements and due reports
-   Customer notes and segmentation
-   CSV/Excel import and export
-   Optional loyalty points, rewards, promotions, and feedback

## 11. Supplier Management

-   Supplier profile and contact information
-   Opening and outstanding balances
-   Purchase and payment history
-   Supplier ledger and statements
-   Due tracking and payment reminders
-   Purchase returns
-   Supplier-specific product prices
-   Supplier performance reports
-   Import/export

## 12. Cash, Bank & Payments

-   Cash, bank, and mobile financial service accounts
-   Account opening balances
-   Deposits, withdrawals, and transfers
-   Customer collections and supplier payments
-   Expense payments and other income receipts
-   Multiple payment methods and split payments
-   Transaction history and reconciliation
-   Daily cash balance and account reports

A sale and its payment are separate concepts. For a ৳10,000 sale with
৳7,000 received, record ৳10,000 sales, ৳7,000 received, and ৳3,000
receivable.

## 13. Accounting & Finance

-   Chart of accounts: assets, liabilities, equity, income, and expenses
-   Journal entries and general ledger
-   Cash book and bank book
-   Accounts receivable and payable
-   Customer and supplier ledgers
-   Trial balance
-   Profit and loss statement
-   Balance sheet
-   Cash flow statement
-   Financial periods, opening balances, closing entries, and period
    locking
-   Controlled reversal/correction of posted transactions
-   Automated accounting entries for sales, purchases, payments,
    returns, taxes, and cost of goods sold

Accounting examples must be adapted to the final accounting policy. Have
Bangladesh VAT/tax workflows reviewed by a qualified professional.

## 14. Branch & Warehouse Management

-   Multiple branches and warehouses
-   Branch managers and branch-level permissions
-   Branch-specific inventory and optional pricing
-   Stock transfer request, approval, dispatch, and receiving
-   Branch-specific invoice numbering
-   Branch-wise sales, purchases, stock, and expenses
-   Consolidated business reporting
-   Configurable location-specific tax settings where legally applicable

## 15. Employee & User Management

-   User invitations and account lifecycle
-   Roles and granular permissions
-   Business and branch access restrictions
-   Module- and action-level access
-   Login history and session management
-   Cashier, salesperson, store manager, accountant, branch manager, and
    business admin roles
-   Optional commission configuration
-   Activity logs
-   Approval permissions for refunds, stock adjustments, price changes,
    and sensitive edits

## 16. Expense & Other Income

-   Expense categories, dates, amounts, payees, and payment accounts
-   Reference numbers and supporting attachments
-   Approval workflows and recurring expenses
-   Branch-wise expenses and expense reports
-   Other income source, category, receiving account, and supporting
    documents
-   Income reports

Examples include rent, utilities, internet, transportation, maintenance,
marketing, and packaging costs.

## 17. Returns, Refunds & Exchanges

-   Search original invoice
-   Full or partial product returns
-   Refund calculation and payment method
-   Store credit where supported
-   Exchanges
-   Restock, quarantine, or disposal decision based on item condition
-   Return reason and approval
-   Return receipt and history
-   Inventory, customer balance, payment, and accounting updates

Do not edit completed sales directly to represent returns. Use linked
return/refund transactions.

## 18. Barcode & Label Printing

-   Barcode generation and scanning
-   SKU lookup
-   Individual and bulk label printing
-   Product name and price on labels
-   Batch/expiry data where relevant
-   Thermal label printer support
-   Barcode format configuration
-   Duplicate barcode validation

## 19. Reports & Analytics

### Sales

-   Daily, weekly, monthly, and annual sales
-   Invoice-, product-, category-, customer-, cashier-, salesperson-,
    branch-, and payment-method reports
-   Returns and discounts

### Purchases

-   Purchase history and supplier-wise purchases
-   Product-wise purchase history
-   Purchase price trends and returns
-   Supplier balances

### Inventory

-   Current stock and stock movements
-   Low/out-of-stock and near-expiry reports
-   Stock valuation, adjustments, damaged goods
-   Fast-, slow-, and non-moving stock

### Finance

-   Cash flow and account transactions
-   Customer receivables and supplier payables
-   Expense and income summaries
-   Gross profit, net profit, P&L, balance sheet, and trial balance

Provide date, branch, product, category, customer, supplier,
payment-method, and employee filters where relevant. Support PDF, CSV,
Excel, and print-friendly output.

## 20. Restaurant POS & Kitchen (Phase 2)

-   Dine-in, takeaway, and delivery orders
-   Table and reservation management
-   Menu categories, variants, and add-ons
-   Kitchen tickets and real-time kitchen display
-   Order status tracking
-   Split/merge bills and table transfers
-   Service and delivery charges
-   Waiter management
-   Recipe and ingredient inventory
-   Recipe costing and wastage tracking
-   Restaurant-specific reports

## 21. Pharmacy Management (Phase 3)

-   Medicine-specific product data
-   Batch and expiry tracking
-   Manufacturer and supplier
-   Batch-wise stock and selling
-   Near-expiry and expired-stock alerts
-   Purchase and sales history
-   Returns and barcode support
-   Prescription references where appropriate
-   Compliance workflows for regulated medicines as required

## 22. E-commerce & API Integration (Phase 3)

-   REST API and OpenAPI documentation
-   API credentials, scopes, rate limits, and logs
-   Webhooks
-   Product, price, and stock synchronization
-   Customer synchronization
-   Online order import and status updates
-   Refund synchronization
-   WooCommerce and custom website integration where feasible

Use idempotency keys or equivalent controls to prevent duplicate orders
and payments from retried requests.

## 23. HRM & Payroll (Phase 3)

-   Employee profiles, departments, designations, and documents
-   Branch assignment and employment status
-   Attendance, shifts, and leave
-   Salary structure, allowances, deductions, and bonuses
-   Payroll runs and payslips
-   Salary payment records
-   Optional biometric integration, commissions, and expense claims

## 24. Notifications

-   Low-stock and expiry alerts
-   Subscription renewal and failed-payment alerts
-   Customer due and supplier payment reminders
-   Purchase and transfer approvals
-   Daily sales summaries
-   Security notifications
-   In-app, email, SMS, and supported WhatsApp integrations
-   Per-user notification preferences

## 25. Security, Audit & Recovery

-   Secure authentication and authorization
-   Tenant isolation enforced server-side
-   Role-based access control
-   Optional multi-factor authentication
-   Login and session controls
-   Rate limiting and API access control
-   Audit logs for sensitive actions
-   Encrypted network communication
-   Protected files and secrets
-   Automated backups, retention policies, and tested restoration
-   Business data exports and documented retention/deletion policies

Audit logs should capture actor, tenant, action, timestamp, affected
record, and relevant before/after values where appropriate. Financial
records should be corrected through controlled reversals rather than
silent deletion.

## 26. SaaS Differentiators

### Multi-tenancy

Each business has isolated products, customers, suppliers, sales,
purchases, inventory, users, branches, and financial records. Enforce
tenant scope in database access, APIs, background jobs, caching, and
file storage.

### Feature-based plans

Manage plan entitlements and usage limits through configuration instead
of hardcoding plan names throughout the application.

### White-label options

Custom logo, invoice branding, email branding, custom domain, and
optional branded login for eligible plans.

### Data migration

Provide templates, column mapping, validation, import previews,
duplicate detection, error reports, and opening stock/balance imports.

### Business health dashboard

Show sales, purchases, cash received, receivables, payables, gross
profit, expenses, low stock, near-expiry stock, and product performance.
Clearly distinguish revenue, cash received, gross profit, and net
profit.

## 27. Recommended Technology Architecture

  Layer             Suggested technology
  ----------------- ----------------------------------------
  Frontend          React + TypeScript
  UI                Tailwind CSS
  Backend           Rust + Actix Web
  Database          PostgreSQL
  Authentication    Secure session or token-based approach
  Authorization     Role-based access control
  Real-time         WebSockets where needed
  Background jobs   Dedicated workers
  Cache             Redis where justified
  File storage      S3-compatible object storage
  Reverse proxy     Nginx
  Deployment        Docker-based deployment
  Documentation     OpenAPI and Markdown
  Monitoring        Structured logs, metrics, and alerts

Start with a modular monolith and separate modules internally. Split
services only when scale or operational needs justify it.

### Suggested modules

-   Authentication
-   Tenants
-   Subscriptions
-   Businesses and branches
-   Users and permissions
-   Products
-   Inventory
-   Purchases
-   Sales and POS
-   Customers and suppliers
-   Payments
-   Expenses
-   Accounting
-   Reports
-   Notifications
-   Audit logs

### Database tenancy options

**Shared database and tables:** Tenant ID on tenant-owned records, with
server-enforced tenant scope and PostgreSQL row-level security where
appropriate. Lower operational complexity, but isolation must be
implemented rigorously.

**Separate database per tenant:** Stronger physical separation and
flexible per-business backup, but more complex migrations, connection
management, and operations.

Suggested starting point: shared PostgreSQL with tested tenant isolation
and an architecture that can support dedicated databases for enterprise
customers later.

## 28. Development Roadmap

### Phase 1 --- MVP

-   Tenant isolation and authentication
-   Business registration and onboarding
-   Basic subscription plans
-   Users and permissions
-   Products and categories
-   POS, sales, invoices, and payments
-   Purchases and stock ledger
-   Customers and suppliers
-   Expenses
-   Essential reports
-   Audit logs and backups

**Outcome:** A small shop can register, enter products, receive stock,
sell items, print invoices, collect payments, and review basic reports.

### Phase 2 --- Business Management

-   Multiple branches and warehouses
-   Stock transfers and approvals
-   Purchase orders and partial receiving
-   Returns and refunds
-   Customer dues and supplier payables
-   Cash/bank reconciliation
-   Accounting
-   Advanced reporting
-   Notifications and subscription lifecycle improvements

### Phase 3 --- Industry Expansion

-   Restaurant and kitchen management
-   Pharmacy module
-   Loyalty and CRM
-   HRM and payroll
-   E-commerce integrations
-   API and webhooks
-   Advanced analytics

### Phase 4 --- Enterprise

-   White-label branding
-   Custom domains
-   Enterprise permissions and audit reporting
-   Dedicated database option
-   Data migration tools
-   Integration marketplace
-   Advanced support and recovery workflows

## 29. Priority Definitions

  -----------------------------------------------------------------------
  Priority                Meaning                 Examples
  ----------------------- ----------------------- -----------------------
  P0                      Required for safe       Tenant isolation, auth,
                          initial launch          products, POS, stock,
                                                  audit, backups

  P1                      Important for           Purchases, returns,
                          commercial operations   payments, core reports,
                                                  branches

  P2                      Growth features         Advanced accounting,
                                                  loyalty, notifications,
                                                  restaurant

  P3                      Industry expansion      Pharmacy, HRM,
                                                  e-commerce

  P4                      Enterprise              White-label, dedicated
                          differentiation         databases, advanced
                                                  integrations
  -----------------------------------------------------------------------

Security and tenant isolation are P0 and must not be postponed.

## 30. Key Success Criteria

-   A new business can complete onboarding without technical assistance.
-   A cashier can complete a normal sale quickly.
-   Stock movements reconcile to the inventory ledger.
-   Sales, payments, and due balances remain consistent.
-   Users cannot access another tenant's data.
-   Financial reports can be traced back to source transactions.
-   Backups can be restored in a tested procedure.
-   Subscription limits are enforced consistently.
-   Common reports can be filtered and exported.
-   The system provides useful audit trails for sensitive actions.

## 31. Recommended Next Documents

1.  Software Requirements Specification (SRS)
2.  Database Design Document
3.  API Specification
4.  UI/UX screen and workflow specification
5.  Test plan and acceptance criteria
6.  Deployment and operations runbook

## 32. Final Recommendation

Start with a dependable multi-tenant core: POS, products, purchases,
inventory, payments, customers, suppliers, roles, reporting, and
subscription management. Add industry-specific modules only after the
core workflows and data consistency are reliable.

The product's advantage should be ease of onboarding, fast checkout,
accurate stock and financial records, clear reporting, and strong tenant
isolation---not simply the number of features.
