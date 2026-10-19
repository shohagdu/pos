# Software Requirements Specification (SRS)

## Multi-Tenant SaaS POS & Business Management Platform

**Document version:** 1.0\
**Status:** Initial draft for review\
**Prepared:** 17 October 2026\
**Product category:** SaaS POS, inventory, sales, purchasing, and
business management\
**Initial market:** Bangladesh\
**Proposed technology:** React + TypeScript, Tailwind CSS, Rust + Actix
Web, PostgreSQL

------------------------------------------------------------------------

## Document Control

  -----------------------------------------------------------------------
  Field                               Value
  ----------------------------------- -----------------------------------
  Document title                      Software Requirements Specification
                                      --- SaaS POS

  Version                             1.0

  Status                              Draft

  Intended audience                   Product owner, developers, QA,
                                      UI/UX designers, DevOps, business
                                      stakeholders

  Approval                            Pending stakeholder review
  -----------------------------------------------------------------------

## 1. Introduction

### 1.1 Purpose

This document defines the functional and non-functional requirements for
a multi-tenant SaaS POS platform. It is intended to guide product
planning, architecture, implementation, testing, and acceptance.

### 1.2 Scope

The platform will allow multiple independent businesses to manage
products, sales, purchases, inventory, customers, suppliers, payments,
users, and reports through a cloud-based application. The SaaS operator
will manage tenants, plans, subscriptions, and platform configuration
through a Super Admin panel.

The initial release focuses on general retail. Branch management, full
accounting, restaurant, pharmacy, HRM, and e-commerce integrations may
be delivered in later phases.

### 1.3 Objectives

-   Provide a fast and user-friendly point-of-sale experience.
-   Maintain reliable inventory and transaction records.
-   Isolate each tenant's data.
-   Support subscription-based monetization.
-   Provide accurate sales, stock, receivable, payable, and financial
    reports.
-   Support expansion to multi-branch and industry-specific workflows.
-   Provide a maintainable API and modular architecture.

### 1.4 Intended Users

-   SaaS Super Admin
-   Business owner
-   Business administrator
-   Branch manager
-   Cashier
-   Accountant
-   Store/warehouse staff
-   Salesperson
-   Support staff with explicitly authorized access

### 1.5 Definitions

  -----------------------------------------------------------------------
  Term                                Definition
  ----------------------------------- -----------------------------------
  SaaS                                Software delivered as a service,
                                      generally through a subscription

  Tenant                              A business workspace whose data
                                      must be isolated from other
                                      businesses

  POS                                 Point of Sale interface used to
                                      complete sales

  SKU                                 Stock Keeping Unit, an internal
                                      product identifier

  RBAC                                Role-Based Access Control

  MRR                                 Monthly Recurring Revenue

  ARR                                 Annual Recurring Revenue

  Stock ledger                        Auditable sequence of stock
                                      movements

  Idempotency                         Processing a repeated request
                                      without creating duplicate effects

  MVP                                 Minimum Viable Product

  RLS                                 PostgreSQL Row-Level Security
  -----------------------------------------------------------------------

## 2. Overall Description

### 2.1 Product Perspective

The product consists of: 1. A tenant-facing web application. 2. A POS
interface optimized for checkout. 3. A Super Admin panel. 4. A backend
API. 5. A PostgreSQL database. 6. Optional background workers,
notification providers, payment gateways, and object storage.

### 2.2 Product Functions

At a high level, the platform will: - Register businesses and initialize
workspaces. - Authenticate users and authorize actions. - Manage
subscription plans and entitlements. - Maintain products and prices. -
Record sales, payments, purchases, and returns. - Maintain stock
movements and balances. - Manage customers, suppliers, and outstanding
balances. - Record expenses and other income. - Generate operational and
financial reports. - Maintain audit logs and backups. - Support future
branch and industry modules.

### 2.3 User Classes

  -----------------------------------------------------------------------
  User class                          Typical permissions
  ----------------------------------- -----------------------------------
  Super Admin                         Platform-wide tenant and
                                      subscription administration

  Business Owner                      Full access to their own tenant,
                                      subject to platform policy

  Business Admin                      Configurable business
                                      administration

  Branch Manager                      Authorized branch operations and
                                      reports

  Cashier                             POS and permitted payment/sales
                                      operations

  Accountant                          Financial records and reports

  Store Manager                       Product, purchase, and inventory
                                      operations

  Warehouse Staff                     Receiving, issuing, counting, and
                                      transfers

  Salesperson                         Sales and permitted customer
                                      operations
  -----------------------------------------------------------------------

A user's permissions must be limited by both role and tenant/branch
scope.

### 2.4 Operating Environment

-   Modern desktop and mobile web browsers
-   Responsive web UI
-   HTTPS access
-   PostgreSQL-backed backend
-   Server-side authorization
-   Deployment through a controlled hosting environment
-   Optional receipt/label printers and barcode scanners

### 2.5 Constraints

-   Tenant isolation is mandatory.
-   The first release should prioritize general retail.
-   Payment gateway availability depends on approved merchant access and
    provider APIs.
-   Offline POS support is not assumed for MVP; if added, it requires a
    separate synchronization design and acceptance tests.
-   Legal, tax, and VAT behavior must be confirmed for the relevant
    jurisdiction.
-   Final subscription prices and package limits are business decisions.

### 2.6 Assumptions and Dependencies

-   The SaaS operator can provision and maintain hosting, database,
    backup, and monitoring.
-   Businesses provide accurate product, opening stock, customer, and
    supplier data.
-   Third-party integrations may require accounts, credentials,
    approval, and usage fees.
-   Email/SMS/WhatsApp delivery depends on provider configuration.
-   The product owner will approve workflows, tax rules, invoice
    formats, and package entitlements before release.

## 3. System Architecture Requirements

### 3.1 Proposed Architecture

-   Frontend: React + TypeScript and Tailwind CSS
-   Backend: Rust + Actix Web
-   Database: PostgreSQL
-   Optional cache: Redis
-   Background workers for long-running tasks
-   Object storage for files and attachments
-   Nginx or equivalent reverse proxy
-   OpenAPI documentation for the API
-   Structured application logs and metrics

A modular monolith is recommended for the initial implementation.
Modules should have clear boundaries so they can be separated later if
justified.

### 3.2 Tenant Isolation

**SRS-TEN-001:** Every tenant-owned record must be associated with a
tenant identifier or be reachable only through a tenant-scoped parent
record.

**SRS-TEN-002:** The server must derive the active tenant from an
authenticated and authorized context. It must not trust a tenant ID
supplied by the client without verification.

**SRS-TEN-003:** All tenant-owned reads and writes must enforce tenant
scope, including APIs, background jobs, file access, cache keys,
exports, and reports.

**SRS-TEN-004:** A user must not access another tenant's records by
changing an identifier in a URL, request body, query parameter, or API
call.

**SRS-TEN-005:** Tenant-isolation tests must cover normal endpoints,
nested resources, reports, exports, file access, and background jobs.

**SRS-TEN-006:** If PostgreSQL RLS is used, application connection and
transaction handling must set tenant context safely and reset it
correctly between requests.

### 3.3 Data Integrity

**SRS-DATA-001:** Financial transactions must be recorded within
database transactions where multiple records must succeed or fail
together.

**SRS-DATA-002:** Completed financial records must not be silently
overwritten or permanently deleted through ordinary UI workflows.

**SRS-DATA-003:** Corrections to posted transactions must use controlled
reversal, refund, or adjustment workflows.

**SRS-DATA-004:** Critical operations must be protected against
duplicate processing caused by retries or repeated submissions.

**SRS-DATA-005:** Stock balances must be reconcilable to stock movement
records.

## 4. Functional Requirements

Requirement identifiers are intended for traceability in implementation
and testing. MVP requirements are marked **P0**; later-phase
requirements are marked **P1**, **P2**, or **P3**.

### 4.1 Authentication and User Management

-   **SRS-AUTH-001 (P0):** The system shall allow authorized users to
    sign in and sign out securely.
-   **SRS-AUTH-002 (P0):** The system shall provide a secure password
    reset process.
-   **SRS-AUTH-003 (P0):** The system shall support user activation and
    deactivation.
-   **SRS-AUTH-004 (P0):** The system shall enforce role-based
    permissions on the server.
-   **SRS-AUTH-005 (P0):** The system shall record relevant login and
    security events.
-   **SRS-AUTH-006 (P0):** The system shall restrict users to authorized
    tenant and branch scopes.
-   **SRS-AUTH-007 (P1):** The system should support multi-factor
    authentication.
-   **SRS-AUTH-008 (P0):** Sensitive actions such as refunds, price
    overrides, stock adjustments, and user permission changes shall
    require appropriate permissions.

### 4.2 Business Registration and Onboarding

-   **SRS-ONB-001 (P0):** A new business owner shall be able to register
    a business account.
-   **SRS-ONB-002 (P0):** The system shall collect required business
    profile fields and validate them.
-   **SRS-ONB-003 (P0):** The system shall create an isolated tenant
    workspace after successful registration.
-   **SRS-ONB-004 (P0):** The onboarding flow shall support initial
    branch, invoice, and payment settings.
-   **SRS-ONB-005 (P0):** The system shall allow authorized users to
    import products from a supported template.
-   **SRS-ONB-006 (P1):** The system should support customer, supplier,
    and opening-stock imports.
-   **SRS-ONB-007 (P1):** The interface should show onboarding progress
    and identify incomplete setup steps.

### 4.3 SaaS Super Admin

-   **SRS-ADM-001 (P0):** The Super Admin shall be able to list and
    search tenants.
-   **SRS-ADM-002 (P0):** The Super Admin shall be able to activate,
    suspend, and reactivate tenant accounts with an audit record.
-   **SRS-ADM-003 (P0):** The Super Admin shall be able to create and
    update subscription plans.
-   **SRS-ADM-004 (P0):** The Super Admin shall be able to configure
    plan features and usage limits.
-   **SRS-ADM-005 (P1):** The system should display subscription,
    renewal, trial, and payment status summaries.
-   **SRS-ADM-006 (P1):** The system should provide MRR and ARR
    summaries based on clearly documented calculation rules.
-   **SRS-ADM-007 (P0):** Platform-wide access shall be restricted to
    authorized Super Admin users and audited.

### 4.4 Subscription and Billing

-   **SRS-SUB-001 (P0):** The system shall associate each tenant with a
    subscription state.
-   **SRS-SUB-002 (P0):** The system shall support configurable plans
    and feature entitlements.
-   **SRS-SUB-003 (P0):** The system shall enforce plan limits on the
    server, not only in the UI.
-   **SRS-SUB-004 (P0):** The system shall record subscription start,
    renewal, expiry, cancellation, and status changes.
-   **SRS-SUB-005 (P1):** The system should support trials, grace
    periods, coupons, and manual payment verification.
-   **SRS-SUB-006 (P1):** The system should notify authorized users
    before renewal or expiry.
-   **SRS-SUB-007 (P1):** Payment gateway callbacks shall be verified
    and processed idempotently.
-   **SRS-SUB-008 (P0):** Subscription expiry behavior shall be
    configurable and must preserve data according to the approved
    retention policy.

### 4.5 Product and Category Management

-   **SRS-PROD-001 (P0):** Authorized users shall be able to create,
    update, view, search, and archive products.
-   **SRS-PROD-002 (P0):** Each product shall have a tenant-scoped
    identifier and a validated SKU or product code according to
    configuration.
-   **SRS-PROD-003 (P0):** Products shall support name, category, unit,
    purchase price, selling price, and active status.
-   **SRS-PROD-004 (P0):** The system shall prevent unintended duplicate
    identifiers within the applicable tenant scope.
-   **SRS-PROD-005 (P1):** The system should support barcode generation
    and scanning.
-   **SRS-PROD-006 (P1):** The system should support variants, multiple
    units, and unit conversion where configured.
-   **SRS-PROD-007 (P1):** The system should retain purchase price
    history and show the latest purchase information.
-   **SRS-PROD-008 (P1):** The system should support bulk import/export
    with validation and error reporting.
-   **SRS-PROD-009 (P2):** Batch, expiry, and serial-number tracking
    shall be available for applicable industry modules.

### 4.6 POS and Sales

-   **SRS-POS-001 (P0):** Authorized cashiers shall be able to find
    products by search and, when configured, barcode scanning.
-   **SRS-POS-002 (P0):** The POS shall support adding/removing items
    and changing quantities before completion.
-   **SRS-POS-003 (P0):** The system shall calculate line totals,
    discounts, taxes, and invoice totals using consistent rounding
    rules.
-   **SRS-POS-004 (P0):** The system shall support configured payment
    methods.
-   **SRS-POS-005 (P0):** The system shall support full and partial
    payment where credit sales are enabled.
-   **SRS-POS-006 (P0):** The system shall generate a unique invoice
    number within the configured numbering scope.
-   **SRS-POS-007 (P0):** On sale completion, the system shall record
    the sale and related stock/payment effects consistently.
-   **SRS-POS-008 (P0):** The system shall support invoice viewing and
    printing.
-   **SRS-POS-009 (P1):** The system should support held sales and later
    resumption.
-   **SRS-POS-010 (P1):** The system should support A4 and thermal
    receipt layouts.
-   **SRS-POS-011 (P1):** The system should support controlled
    discounts, price overrides, and invoice reprints.
-   **SRS-POS-012 (P1):** The system should support shift opening, shift
    closing, and cash reconciliation.
-   **SRS-POS-013 (P2):** The system may support offline sales only
    after conflict handling, durable local storage, synchronization, and
    duplicate prevention are specified and tested.

### 4.7 Purchase Management

-   **SRS-PUR-001 (P0):** Authorized users shall be able to create
    purchase records with supplier, items, quantities, prices, and
    totals.
-   **SRS-PUR-002 (P0):** The system shall validate quantities, prices,
    discounts, and applicable taxes.
-   **SRS-PUR-003 (P0):** Received goods shall create corresponding
    stock movements.
-   **SRS-PUR-004 (P0):** The system shall record supplier payments and
    outstanding balances.
-   **SRS-PUR-005 (P1):** The system should support purchase orders,
    approval, and partial receiving.
-   **SRS-PUR-006 (P1):** The product selector should show last purchase
    price, date, supplier, and quantity where available.
-   **SRS-PUR-007 (P1):** The system should show recent purchase prices
    and price variance.
-   **SRS-PUR-008 (P1):** Purchase returns shall be linked to the
    original purchase when possible and shall update stock and balances
    consistently.

### 4.8 Inventory

-   **SRS-STK-001 (P0):** The system shall maintain stock quantities by
    tenant and configured stock location.
-   **SRS-STK-002 (P0):** Completed sales shall reduce stock according
    to the configured inventory policy.
-   **SRS-STK-003 (P0):** Received purchases shall increase stock.
-   **SRS-STK-004 (P0):** The system shall record each stock movement
    with a reference, reason, user, timestamp, and quantity.
-   **SRS-STK-005 (P0):** Stock adjustments shall require authorization
    and a reason.
-   **SRS-STK-006 (P0):** The system shall prevent or explicitly control
    negative stock according to tenant configuration.
-   **SRS-STK-007 (P1):** The system should support stock counts and
    reconciliation.
-   **SRS-STK-008 (P1):** The system should support stock transfers with
    dispatch and receiving states.
-   **SRS-STK-009 (P1):** The system should provide low-stock and
    out-of-stock alerts.
-   **SRS-STK-010 (P1):** The system shall provide stock valuation using
    an explicitly selected costing method.
-   **SRS-STK-011 (P2):** Batch/expiry and damaged-stock workflows shall
    be supported by relevant industry modules.

### 4.9 Customers and Suppliers

-   **SRS-CRM-001 (P0):** Authorized users shall be able to create and
    update customer profiles.
-   **SRS-CRM-002 (P0):** The system shall track customer payments and
    outstanding balances.
-   **SRS-CRM-003 (P0):** The system shall provide customer transaction
    history and statements.
-   **SRS-SUP-001 (P0):** Authorized users shall be able to create and
    update supplier profiles.
-   **SRS-SUP-002 (P0):** The system shall track supplier purchases,
    payments, and outstanding balances.
-   **SRS-SUP-003 (P0):** The system shall provide supplier transaction
    history and statements.
-   **SRS-CRM-004 (P1):** The system should support customer groups,
    price levels, and loyalty features.
-   **SRS-SUP-004 (P1):** The system should support supplier-specific
    product price history.

### 4.10 Payments, Expenses, and Income

-   **SRS-PAY-001 (P0):** The system shall support configured cash,
    bank, and other payment accounts.
-   **SRS-PAY-002 (P0):** The system shall record receipts, payments,
    deposits, withdrawals, and transfers.
-   **SRS-PAY-003 (P0):** The system shall distinguish transaction
    value, amount paid, and outstanding balance.
-   **SRS-PAY-004 (P0):** The system shall maintain a traceable payment
    history.
-   **SRS-EXP-001 (P0):** Authorized users shall be able to record
    expenses with category, amount, date, and payment account.
-   **SRS-EXP-002 (P1):** The system should support attachments,
    approval, recurring expenses, and branch-level reporting.
-   **SRS-EXP-003 (P1):** The system should support other income records
    and reports.

### 4.11 Returns and Refunds

-   **SRS-RET-001 (P1):** Authorized users shall be able to initiate a
    return against an original sale where available.
-   **SRS-RET-002 (P1):** The system shall support full and partial
    returns.
-   **SRS-RET-003 (P1):** The system shall record a return reason and
    the responsible user.
-   **SRS-RET-004 (P1):** The system shall determine whether returned
    items are restocked, quarantined, or otherwise excluded from sale.
-   **SRS-RET-005 (P1):** Refunds shall update payment, customer
    balance, inventory, and accounting records consistently.
-   **SRS-RET-006 (P1):** Sensitive refunds shall require appropriate
    permission or approval.
-   **SRS-RET-007 (P1):** Completed sales shall not be silently edited
    to simulate returns.

### 4.12 Branches and Warehouses

-   **SRS-BRN-001 (P1):** A tenant shall be able to configure multiple
    branches if permitted by its plan.
-   **SRS-BRN-002 (P1):** The system shall restrict users to their
    authorized branches.
-   **SRS-BRN-003 (P1):** Stock shall be trackable by branch and
    warehouse.
-   **SRS-BRN-004 (P1):** Transfers shall have explicit source,
    destination, quantity, status, and receiving confirmation.
-   **SRS-BRN-005 (P1):** Reports shall support branch-level and
    consolidated views.
-   **SRS-BRN-006 (P1):** Branch limits shall be enforced according to
    subscription entitlements.

### 4.13 Accounting

-   **SRS-ACC-001 (P1):** The system should support a configurable chart
    of accounts.
-   **SRS-ACC-002 (P1):** The system should support journal entries and
    general ledger reporting.
-   **SRS-ACC-003 (P1):** Posted transactions should generate accounting
    entries according to the approved accounting policy.
-   **SRS-ACC-004 (P1):** The system should provide trial balance,
    profit and loss, balance sheet, and cash flow reports.
-   **SRS-ACC-005 (P1):** Financial periods should be lockable to
    prevent unauthorized changes.
-   **SRS-ACC-006 (P1):** Corrections to posted entries shall be
    traceable.
-   **SRS-ACC-007 (P1):** Financial calculations shall be reproducible
    from source transactions and documented rules.

### 4.14 Reports and Exports

-   **SRS-REP-001 (P0):** The system shall provide basic date-filtered
    sales, purchase, and stock reports.
-   **SRS-REP-002 (P0):** Reports shall be tenant-scoped and
    permission-controlled.
-   **SRS-REP-003 (P1):** Reports should support filters relevant to the
    report, including branch, product, category, customer, supplier, and
    payment method.
-   **SRS-REP-004 (P1):** Reports should support CSV and spreadsheet
    export.
-   **SRS-REP-005 (P1):** Printable and PDF output should be provided
    for selected reports.
-   **SRS-REP-006 (P1):** The system shall distinguish revenue, cash
    received, gross profit, and net profit.
-   **SRS-REP-007 (P1):** Export jobs shall enforce tenant and user
    permissions and should be audited.

### 4.15 Audit Logs and Notifications

-   **SRS-AUD-001 (P0):** The system shall record important security and
    business actions.
-   **SRS-AUD-002 (P0):** Audit records shall include actor, tenant,
    action, timestamp, and affected record where applicable.
-   **SRS-AUD-003 (P0):** Ordinary users shall not be able to alter
    audit history.
-   **SRS-NOT-001 (P1):** The system should provide configurable
    low-stock, expiry, due, and subscription notifications.
-   **SRS-NOT-002 (P1):** Notifications should support in-app and email
    channels; SMS/WhatsApp depend on integrations.
-   **SRS-NOT-003 (P1):** Notification delivery failures should be
    logged and retried according to a bounded policy.

### 4.16 Industry Modules

-   **SRS-IND-001 (P2):** Restaurant mode should support table orders,
    takeaway, kitchen tickets, order status, and recipe/ingredient
    costing.
-   **SRS-IND-002 (P3):** Pharmacy mode should support batch and expiry
    tracking and other required medicine fields.
-   **SRS-IND-003 (P3):** HRM should support employee profiles,
    attendance, leave, and payroll workflows.
-   **SRS-IND-004 (P3):** E-commerce integration should support
    authenticated APIs, webhooks, and safe synchronization.

## 5. Business Rules

1.  All business data must belong to one tenant and, where relevant, one
    branch or stock location.
2.  A user may only perform actions permitted by their role, tenant,
    branch, and subscription.
3.  Invoice numbers must be unique within the defined numbering scope.
4.  Currency and decimal precision must be configurable with a
    consistent rounding policy.
5.  Sales and purchases must preserve their item-level quantities,
    prices, discounts, tax values, and totals.
6.  Amount received and outstanding balance must be stored or
    reproducibly derived without ambiguity.
7.  Stock movements must be traceable to their originating transaction
    or adjustment.
8.  A completed sale cannot be silently deleted; use cancellation,
    return, refund, or reversal workflows.
9.  Duplicate API requests and repeated payment callbacks must not
    create duplicate business effects.
10. Reports must state their date boundaries, timezone, filters, and
    calculation basis.
11. Plan limits must be enforced on the server.
12. Subscription suspension or expiry must follow a documented policy
    and must not cause accidental data loss.
13. Tax/VAT behavior must be configurable and validated against
    applicable local requirements.
14. Financial and inventory costing policies must be selected and
    documented before production use.

## 6. External Interface Requirements

### 6.1 User Interface

-   Responsive layout for desktop, tablet, and mobile.
-   POS optimized for rapid keyboard and barcode-scanner use.
-   Clear validation and actionable error messages.
-   Consistent loading, empty, success, and error states.
-   Accessible labels and keyboard navigation for core workflows.
-   Confirmation for destructive or financially sensitive actions.
-   Bengali and English localization should be considered.

### 6.2 Hardware Interfaces

-   Barcode scanners compatible with keyboard-wedge input where
    applicable.
-   Standard browser printing for supported receipt and A4 printers.
-   Thermal receipt and label printer compatibility to be validated on
    target devices.
-   Cash drawer integration is optional and depends on hardware/browser
    support.

### 6.3 Software Interfaces

-   REST API documented with OpenAPI.
-   Payment gateways through supported provider APIs.
-   Email/SMS providers through configured integrations.
-   Optional object storage for attachments and exports.
-   Optional e-commerce systems through authenticated APIs and webhooks.

## 7. Non-Functional Requirements

Targets below are initial engineering goals and must be validated
through load testing.

### 7.1 Performance

-   **SRS-NFR-PERF-001:** Common POS actions should respond within 300
    ms at the 95th percentile under the agreed test load, excluding
    external payment-provider latency.
-   **SRS-NFR-PERF-002:** Standard dashboard/report queries should
    normally complete within 3 seconds at the 95th percentile for the
    agreed dataset and load.
-   **SRS-NFR-PERF-003:** Long-running exports and imports should run
    asynchronously where necessary.
-   **SRS-NFR-PERF-004:** The system shall use pagination and
    appropriate database indexes for large result sets.

### 7.2 Availability and Reliability

-   **SRS-NFR-REL-001:** The production service should target 99.5%
    monthly availability initially, excluding agreed maintenance,
    subject to hosting capability.
-   **SRS-NFR-REL-002:** Failed transactions shall not leave partial
    sale/payment/stock records.
-   **SRS-NFR-REL-003:** Background jobs should be retryable and
    idempotent where appropriate.
-   **SRS-NFR-REL-004:** Health checks and operational alerts shall be
    configured for critical services.

### 7.3 Security and Privacy

-   **SRS-NFR-SEC-001:** All external application traffic shall use
    HTTPS.
-   **SRS-NFR-SEC-002:** Passwords shall be stored using a modern
    password-hashing algorithm; plaintext passwords must never be
    stored.
-   **SRS-NFR-SEC-003:** Authorization shall be enforced server-side for
    every protected operation.
-   **SRS-NFR-SEC-004:** Secrets and payment credentials shall not be
    committed to source control or exposed to clients.
-   **SRS-NFR-SEC-005:** The system shall apply rate limiting and input
    validation to sensitive endpoints.
-   **SRS-NFR-SEC-006:** Tenant-isolation and privilege-escalation tests
    shall be part of release testing.
-   **SRS-NFR-SEC-007:** Personal and business data access, export,
    retention, and deletion shall follow an approved policy and
    applicable law.
-   **SRS-NFR-SEC-008:** Security-relevant events shall be logged
    without unnecessarily recording secrets or sensitive payment data.

### 7.4 Scalability

-   **SRS-NFR-SCALE-001:** Application instances should be stateless
    where practical so they can scale horizontally.
-   **SRS-NFR-SCALE-002:** Database connection pooling and query
    monitoring shall be used.
-   **SRS-NFR-SCALE-003:** Heavy exports, imports, and notification work
    should be offloaded to background workers.
-   **SRS-NFR-SCALE-004:** Tenant-specific usage and resource limits
    should be configurable.

### 7.5 Maintainability

-   **SRS-NFR-MAINT-001:** The backend shall use clear module boundaries
    and documented interfaces.
-   **SRS-NFR-MAINT-002:** Database schema changes shall use versioned
    migrations.
-   **SRS-NFR-MAINT-003:** The API shall be documented and versioning
    policy defined.
-   **SRS-NFR-MAINT-004:** Automated tests shall cover critical business
    rules.
-   **SRS-NFR-MAINT-005:** Production errors shall be logged with
    correlation/request identifiers without leaking sensitive data.

### 7.6 Backup and Recovery

-   **SRS-NFR-BCP-001:** Automated backups shall run on a defined
    schedule.
-   **SRS-NFR-BCP-002:** Backup retention shall be documented and
    access-controlled.
-   **SRS-NFR-BCP-003:** Restoration shall be tested periodically.
-   **SRS-NFR-BCP-004:** The owner shall define target Recovery Point
    Objective (RPO) and Recovery Time Objective (RTO) before production
    launch.
-   **SRS-NFR-BCP-005:** Backups shall be stored separately from the
    primary production database.

### 7.7 Usability and Accessibility

-   **SRS-NFR-UX-001:** Core workflows shall minimize unnecessary steps.
-   **SRS-NFR-UX-002:** Forms shall provide clear validation and error
    recovery.
-   **SRS-NFR-UX-003:** POS shall support keyboard-oriented operation.
-   **SRS-NFR-UX-004:** Common UI components and design tokens shall be
    consistent.
-   **SRS-NFR-UX-005:** Bengali and English labels should be supported
    through localization rather than duplicated screens.

## 8. Data Requirements

### 8.1 Initial Entity List

The detailed ERD and schema are separate deliverables. Initial entities
include:

-   Tenant/Business
-   Subscription Plan
-   Tenant Subscription
-   Feature Entitlement
-   User
-   Role
-   Permission
-   User Role/Scope
-   Branch
-   Warehouse
-   Product
-   Category
-   Brand
-   Unit
-   Product Variant
-   Stock Balance
-   Stock Movement
-   Supplier
-   Customer
-   Purchase
-   Purchase Item
-   Goods Receipt
-   Purchase Payment
-   Sale
-   Sale Item
-   Payment
-   Return
-   Refund
-   Expense
-   Other Income
-   Cash/Bank Account
-   Journal Entry
-   Journal Line
-   Notification
-   Audit Log
-   Import/Export Job

### 8.2 Data Integrity and Auditability

-   Use primary keys and foreign keys.
-   Add unique constraints for identifiers within their correct tenant
    scope.
-   Use decimal/numeric types for monetary values; avoid floating-point
    arithmetic for money.
-   Store timestamps consistently and display them using the configured
    business timezone.
-   Define deletion/archival rules for referenced records.
-   Preserve source transaction references for stock and accounting
    entries.
-   Keep a traceable record of adjustments and reversals.
-   Define retention and privacy policies before launch.

### 8.3 Import Requirements

Imports should support: - Downloadable templates - Field mapping -
Preview and validation - Row-level error reporting - Duplicate handling
policy - Transaction-safe or staged processing - Import history and
downloadable error report

## 9. Core Workflows

### 9.1 First-Time Business Setup

1.  Owner registers and verifies contact information.
2.  System creates tenant workspace.
3.  Owner completes business profile.
4.  Owner creates branch and configures invoice/payment settings.
5.  Owner imports or creates products.
6.  Owner enters opening stock if required.
7.  Owner creates cashier users and assigns permissions.
8.  Owner completes a test sale or begins normal operations.

### 9.2 Sale Completion

1.  Cashier opens POS in an authorized branch.
2.  Cashier scans or searches for products.
3.  System validates product status, price, and available stock
    according to policy.
4.  Cashier selects customer if applicable.
5.  System calculates quantities, discounts, taxes, and total.
6.  Cashier records payment amount and method.
7.  System validates permissions and request idempotency.
8.  System commits sale, sale items, payment, stock movement, and
    accounting effects where enabled in one consistent transaction or
    coordinated reliable workflow.
9.  System generates invoice/receipt.
10. Sale appears in authorized reports.

### 9.3 Purchase Receiving

1.  User selects supplier and creates purchase/order.
2.  User enters products, quantities, prices, discounts, and applicable
    taxes.
3.  If approval is enabled, authorized user approves.
4.  User records goods received in full or in part.
5.  System records stock movement for received quantity.
6.  System records supplier payable and payments.
7.  Purchase and stock reports update.

### 9.4 Return and Refund

1.  Authorized user finds original sale.
2.  User selects items and quantities to return.
3.  System validates quantities already returned.
4.  User records reason and item condition.
5.  Authorized user approves if required.
6.  System calculates refund or store credit.
7.  System records return, refund/payment effects, stock disposition,
    and accounting entries.
8.  System provides a return receipt and audit record.

### 9.5 Subscription Expiry

1.  System identifies subscription nearing expiry.
2.  System sends reminders according to configuration.
3.  If unpaid after the applicable grace period, system applies the
    approved restriction policy.
4.  The policy blocks paid features as configured without cross-tenant
    effects or accidental deletion.
5.  Authorized staff can reactivate the subscription after verified
    payment.
6.  Status changes are audited.

## 10. Acceptance Criteria

The MVP is acceptable when all of the following are demonstrated in a
staging environment:

1.  A business can register and create an isolated workspace.
2.  A tenant user cannot read or modify another tenant's data, including
    through guessed IDs or altered API parameters.
3.  An authorized admin can create products and configure prices.
4.  A cashier can complete a sale and print an invoice.
5.  Completed sales update stock and payment records consistently.
6.  Partial payments create correct outstanding balances.
7.  Purchases and goods receiving update stock and supplier balances
    correctly.
8.  Stock adjustments require permissions and a reason.
9.  Basic sales, purchase, and stock reports return correct results for
    the selected date range.
10. Plan limits are enforced on the server.
11. Audit logs capture critical changes.
12. Duplicate submissions do not create duplicate sales or payments.
13. Backup restoration has been successfully tested.
14. Critical validation, authorization, and tenant-isolation tests pass.
15. The agreed performance targets are tested with representative data
    and load.

## 11. Testing Requirements

### 11.1 Test Types

-   Unit tests for calculations and business rules
-   Integration tests for database transactions
-   API tests for validation and authorization
-   Tenant-isolation and privilege-escalation tests
-   End-to-end tests for onboarding, sales, purchases, and returns
-   Regression tests for stock and accounting calculations
-   Load tests for POS and reporting
-   Backup/restore tests
-   Browser and printer compatibility tests
-   Security review before production launch

### 11.2 Critical Test Scenarios

-   Duplicate sale submission
-   Repeated payment gateway callback
-   Two cashiers selling the last available item concurrently
-   Return quantity greater than sold quantity
-   Unauthorized price override
-   Cross-tenant access through URL/API identifier manipulation
-   Expired subscription with active user sessions
-   Failed database operation during sale completion
-   Stock transfer dispatched but not received
-   Import containing invalid or duplicate product rows
-   Report/export request by a user without permission
-   Restoration from backup to a clean environment

## 12. Deployment and Operations

-   Separate development, staging, and production environments.
-   Store secrets outside source control.
-   Apply database migrations through a controlled release process.
-   Configure HTTPS and secure headers.
-   Configure structured logs, monitoring, and alerts.
-   Monitor database connections, query latency, job failures, and disk
    capacity.
-   Document backup schedules and restoration steps.
-   Define release rollback procedures.
-   Use least-privilege access for production systems.
-   Establish a process for security updates and incident response.

## 13. Risks and Mitigations

  -----------------------------------------------------------------------
  Risk                    Impact                  Mitigation
  ----------------------- ----------------------- -----------------------
  Cross-tenant data       Critical                Tenant scoping, RLS
  leakage                                         where appropriate,
                                                  automated isolation
                                                  tests, security review

  Incorrect stock         High                    Stock ledger,
  balances                                        transactional updates,
                                                  reconciliation reports

  Incorrect financial     High                    Documented accounting
  reports                                         rules, automated tests,
                                                  traceability to source
                                                  transactions

  Duplicate               High                    Idempotency, unique
  sales/payments                                  request references,
                                                  transaction boundaries

  Slow POS performance    High                    Query/index review,
                                                  load testing, minimal
                                                  checkout dependencies

  Subscription            Medium/High             Explicit state machine,
  restrictions applied                            test matrix,
  incorrectly                                     grace-period policy

  Failed or unusable      Critical                Separate storage and
  backups                                         periodic restore drills

  Scope expansion         High                    MVP boundary,
                                                  prioritized backlog,
                                                  change approval

  Third-party provider    Medium                  Timeouts, retries,
  outage                                          failure states,
                                                  reconciliation process

  Tax/VAT                 High                    Configurable rules and
  misconfiguration                                qualified local review
  -----------------------------------------------------------------------

## 14. Out of Scope for MVP

Unless separately approved, the MVP does not require: - Fully offline
POS - Restaurant kitchen display and recipe costing - Pharmacy-specific
regulatory workflows - Full HRM/payroll - White-label custom domains -
Dedicated database per tenant - E-commerce marketplace integrations -
Advanced AI forecasting - Complex multi-country tax support

## 15. Decisions Required Before Development

The product owner should confirm:

1.  Initial target business types and the first launch segment.
2.  Whether the initial version includes multiple branches.
3.  Subscription packages, prices, trials, and grace-period policy.
4.  Shared database versus dedicated database requirements.
5.  Whether sales can proceed when stock is insufficient.
6.  Inventory costing method (for example, FIFO or weighted average).
7.  Tax/VAT calculation and invoice requirements.
8.  Invoice numbering rules by tenant and branch.
9.  Allowed payment methods and payment gateway provider.
10. Return, refund, and credit-sale policies.
11. Whether basic double-entry accounting is in MVP or Phase 2.
12. Required languages and default currency.
13. Expected initial tenant count, concurrent users, and transaction
    volume.
14. Availability target, backup retention, RPO, and RTO.
15. Data export, retention, and account-deletion policy.

## 16. Reference and Further Reading

-   Reference product: [Hisab Nikash](https://hisabnikash.com/)
-   OWASP Multi-Tenant Security Cheat Sheet:
    https://cheatsheetseries.owasp.org/cheatsheets/Multi_Tenant_Security_Cheat_Sheet.html
-   AWS SaaS architecture resources:
    https://aws.amazon.com/isv/resources/building-blocks-of-multi-tenant/

These references inform product and security planning; they do not
replace product-specific architecture review or local legal/accounting
advice.

## 17. Approval

  Role                   Name   Status    Date
  ---------------------- ------ --------- ------
  Product Owner                 Pending   
  Technical Lead                Pending   
  QA Lead                       Pending   
  Business Stakeholder          Pending   
