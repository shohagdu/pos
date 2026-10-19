# Development Guide

## Prerequisites

| Tool | Version | Notes |
|---|---|---|
| Rust | 1.90+ (CI uses stable) | `rustup component add rustfmt clippy` |
| Node.js | 22.22+ (24 recommended) | npm 11 |
| Docker + Compose | recent | for MySQL/Redis/MinIO/Mailpit; or bring your own MySQL 8.4 |
| Python 3 | 3.10+ | guard-trigger generator and SQL lint scripts |

## First run

```bash
make deps-up                          # waits until MySQL is healthy
cp backend/.env.example backend/.env  # RUN_MIGRATIONS_ON_START=true
make api                              # http://127.0.0.1:8080/healthz
make worker                           # optional: partitions + clean-up jobs
cd frontend && npm install && cd ..
make web                              # http://localhost:5173 (proxies /api to :8080)
```

Mail sent in development appears in Mailpit: <http://localhost:8025>.
MinIO console: <http://localhost:9001> (`pos_minio` / `pos_minio_dev_only`).

## Layout

```
backend/
  crates/core     config (12-factor env), AppError, ids (UUIDv7), request context
  crates/db       pool + session hooks, TenantTx / PlatformTx, error mapping, schema guards, MIGRATOR
  crates/audit    append-only audit writer and the audit log viewer query
  crates/auth     identity: passwords (Argon2id), JWT + rotating refresh tokens, OTP, lockout, RBAC
  crates/tenancy  provisioning, business settings, branches, onboarding, members, roles, invitations
  crates/billing  plans, entitlements (can / within_limit), subscription state, entitlement cache
  crates/catalog  products, variants, barcodes, categories, brands, units, tax rates
  crates/inventory stock posting (moving average), balances, movement ledger, adjustments, opening stock,
                  branch transfers, stock counts
  crates/parties  customers, suppliers, party ledger, statements
  crates/money    payment accounts/methods, payments + allocations, cash/bank book, transfers, expenses,
                  other income
  crates/accounting chart of accounts, balanced journals, fiscal periods, TB / P&L / balance sheet / GL
  crates/sales    pricing engine, POS sales, cancellation, returns, held carts, customer collections,
                  cash registers and shifts (X/Z)
  crates/purchasing purchase bills (draft → post with GRN), landed cost, supplier payments, purchase returns
  crates/reporting read-only reports: dashboard, sales, purchases, expenses, payments, profit estimate,
                  aging, VAT summary, discounts, product movement
  crates/platform Super Admin (staff, businesses, plans, overrides, payment claims), lifecycle job, data export
  crates/api      HTTP server: problem+json errors, request id, security headers, CORS, OpenAPI
  crates/worker   scheduled jobs: monthly partitions, idempotency-key purge
  migrations/     SQLx migrations (MySQL); guard triggers generated at the end of each file
  scripts/        gen_guards.py, check_tenant_sql.py
frontend/src/
  app/            router, layouts, providers
  api/            fetch client (Problem Details), query hooks
  features/       one folder per module (auth, onboarding, settings, account, dashboard, …)
  components/ui/  design-system primitives
  i18n/           en.json, bn.json
  lib/            formatting (BDT, quantities)
```

## Authentication and onboarding

Flow: `POST /auth/register` → SMS code → `POST /auth/verify-phone` (signs in)
→ `POST /tenants` (creates the business, 14-day trial) → `POST /auth/switch-tenant`
→ wizard (`PATCH /settings`, `PATCH /branches/{id}`, `POST /onboarding/steps/{step}`).
The OpenAPI document at `/api/v1/openapi.json` lists every route.

- **Tokens.** The access token (JWT, 15 min) lives in memory in the SPA and goes in
  `Authorization: Bearer`. The refresh token is an httpOnly, `SameSite=Strict`
  cookie on `/api/v1/auth`, rotated on every refresh. A rotated token used again
  signs out its whole family (reuse detection). The SPA serialises refreshes
  across tabs with a Web Lock for that reason.
- **Tenant from the token only.** `TenantAuth` (crates/api/src/auth.rs) reloads the
  membership, tenant status and permissions on every request, so role changes
  (`perms_version`) and deactivation take effect immediately. Handlers call
  `auth.require("settings.manage")?` and open `auth.begin(&state)` for a `TenantTx`.
- **OTP codes in development.** There is no SMS gateway yet. With
  `AUTH_EXPOSE_OTP=true` (development/test only) responses include `dev_code`
  and the screens show it; codes are also logged. `pos-api serve` refuses to
  start with `APP_ENV=production` until a gateway implements `pos_auth::otp::OtpSender`.
- **Lockout.** 5 failed logins per identifier + IP in 15 minutes return `429`
  with `Retry-After`. Behind Nginx set `HTTP_TRUST_PROXY_HEADERS=true`, otherwise
  the proxy's IP is used for every client.
- **Config.** `JWT_SECRET` (≥ 32 bytes) is required by every binary, including
  `migrate`. For a rotation, move the old value to `JWT_SECRET_PREVIOUS`.
  `AUTH_COOKIE_SECURE=false` only for plain-HTTP development.
- **Provisioning** creates every default whose table exists. When a later module
  adds a default (tax rates, units, payment methods, walk-in customer, expense
  categories), add it to `crates/tenancy/src/provisioning.rs` in the same transaction.

## Staff, invitations and roles

- **Invitations** (`POST /invitations`) are single-use links valid for 7 days,
  sent to a phone or email. Only the SHA-256 of the token is stored. The link
  is `APP_URL/invite#<token>`: the token sits in the URL fragment, which
  browsers never send to a server, and the API takes it in a POST body
  (`/invitations/preview`, `/invitations/accept`), because request paths are
  logged. Accepting requires a signed-in user whose verified phone (or email)
  matches the destination. In development the response carries `dev_link`.
- **Seats.** An invitation reserves a seat: active members plus open invitations
  are checked against `users.max` (`402 PLAN_LIMIT_REACHED` with `limit` and
  `max`). `crates/tenancy/src/entitlements.rs` is the minimal check until the
  entitlement service (E1.6) replaces it.
- **Rules the API enforces** (`crates/tenancy/src/members.rs`, `grants.rs`):
  nobody changes their own access; the owner cannot be changed or signed out by
  others; only the owner changes, removes or creates admins; the owner role and
  `subscription.manage` are never handed out; and nobody grants a permission
  they do not hold (`PRIVILEGE_ESCALATION`, owners excepted). The UI mirrors
  these rules but does not replace them.
- **Changes apply at once.** Changing a member's roles, or a custom role's
  permissions, bumps `perms_version`, so the next request with an old token gets
  `401 TOKEN_STALE` and the SPA refreshes. Suspending or removing a member also
  ends their sessions in that business.
- **System roles** are read-only templates. Businesses duplicate them into
  custom roles (`POST /roles`), which can be edited (`PUT /roles/{id}` with
  `If-Match`) and deleted while unused.

## Plans, entitlements and subscription state

- **Checks.** `pos_billing::Entitlements` answers `can("reports.export")`,
  `require(..)` (`402 FEATURE_NOT_AVAILABLE`) and
  `within_limit("users.max", used, delta)` (`402 PLAN_LIMIT_REACHED` with
  `limit` and `max`). Plan values are overridden by unexpired
  `tenant_feature_overrides`; an override with a limit (or with `enabled` unset)
  replaces the plan limit, `NULL` meaning unlimited.
- **Where to call them.** Handlers get cached entitlements as
  `auth.entitlements` (`TenantAuth`), which is fine for feature flags. A write
  that must not exceed a limit resolves inside its own transaction with
  `pos_tenancy::entitlements::current(&mut tx)` and counts usage in the same
  transaction. When a new module adds a limited resource (products, registers,
  invoices, storage), count it in `entitlements::usage` and check it on create.
- **Cache.** In-process, 60 s TTL per API replica (`EntitlementCache`). Code that
  changes a plan, subscription or override must call `invalidate(tenant_id)`;
  other replicas catch up within the TTL. Move it to Redis if that delay matters.
- **Subscription state.** The effective state is computed from the dates on every
  request: trial or paid period → grace (7 days, full access) → read-only
  (30 days: reads and `DELETE`s only, `402 SUBSCRIPTION_READ_ONLY`) →
  suspended (`402 SUBSCRIPTION_SUSPENDED` except `/api/v1/subscription`). The
  daily scheduler that stores these transitions, reminders, payment claims and
  plan changes are Phase 5 (E5.3, E5.4). `GET /api/v1/subscription` returns the
  plan, effective state, features and usage meters.

## Audit log

- **Writing.** Call `pos_audit::record(&mut tx, meta, AuditEntry::new("thing.done")…)`
  inside the business transaction, so the entry exists only if the change commits.
  Action codes are `entity.verb` in `[a-z0-9_.]`; pass before/after JSON for
  master-data changes (AUD-R01).
- **Reading.** `GET /api/v1/audit-logs` (`audit.view`: owner and admin) returns
  the business's entries newest first, including platform/system entries about
  it. Filters: `actor_id` (user id), `action` (exact, or a prefix such as
  `member.*`), `entity_type`, `entity_id`, `branch_id`, `from` (inclusive) and
  `to` (exclusive) as RFC 3339 times. The viewer UI is E5.6.

## Tenant isolation rules (read before writing SQL)

MySQL has no row-level security (DB design §3). Every change must keep these
five layers intact:

1. **Tenant data only through `pos_db::begin_tenant(&pool, &ctx)`.** Bind
   `tx.tenant_id()` into every query: `WHERE tenant_id = ? AND id = ?`. Never
   take a tenant id from a request body or URL.
2. **New tenant tables** use `PRIMARY KEY (tenant_id, id)` and composite foreign
   keys `(tenant_id, parent_id)`.
3. **Guard triggers**: after writing a new migration, run `make guards`. That
   appends the generated triggers to the file. Only run it on migrations that
   have **not** been applied anywhere yet, because editing an applied migration
   changes its checksum.
4. **SQL lint**: `python3 backend/scripts/check_tenant_sql.py` fails if a query on a
   tenant table lacks `tenant_id`. Real exceptions use an inline
   `-- tenant-lint: allow <reason>` comment inside the SQL string.
5. **Tests**: every new endpoint needs a cross-tenant test (tenant B gets 404 for
   tenant A's ids).

Platform-wide code (Super Admin, jobs) uses `begin_platform` and must write an
audit record.

## Database

- Dev URL: `mysql://pos_app:pos_app_dev_only@127.0.0.1:3306/pos`. Use **TCP**:
  sqlx 0.9 reports a broken pipe when closing MySQL Unix-socket connections,
  which breaks `#[sqlx::test]` setup.
- `cargo run -p pos-api -- migrate` applies migrations. `-- check-schema` runs
  the schema guards (tenant-first primary keys, guard triggers present, no
  FLOAT/DOUBLE).
- MySQL DDL is not transactional. Keep **one logical change per migration**, and
  rehearse risky migrations on a staging copy.
- Audit-log partitions are added by the worker (3 months ahead).

## Tests

```bash
# backend: needs a MySQL user allowed to CREATE/DROP databases
cd backend
DATABASE_URL=mysql://pos_test:pos_test_dev_only@127.0.0.1:3306/pos_test cargo test --workspace

# frontend
cd frontend && npm test
```

`#[sqlx::test]` creates a fresh database per test, applies all migrations and
drops it afterwards. The database named in `DATABASE_URL` (`pos_test`) must
exist; Compose creates it. `make check` runs everything CI runs.

## Conventions

- Errors: return `AppError` and let the API render RFC 9457 Problem Details.
  Clients switch on `code`, not on `detail`.
- Money: `rust_decimal` / `DECIMAL`, JSON strings, never floats. Use
  `formatMoney` in the UI.
- Lists: cursor pagination (API-03) with `pos_core::page`: `?limit=` (default
  50, max 200) and `?cursor=`, response `{ "data": [...], "next_cursor": … }`.
  Order by `(time DESC, id DESC)` and encode the last row as a `Keyset`.
- IDs: `pos_core::id::new_id()` (UUIDv7, stored as `BINARY(16)`).
- Dynamic SQL must be wrapped in `sqlx::AssertSqlSafe(..)`. sqlx 0.9 refuses
  non-literal strings otherwise, so every dynamic query is visible in review.
- UI strings live in `i18n/en.json` and `i18n/bn.json`. Never hard-code them.
- Formatting: `cargo fmt` (120 columns), `npm run lint` (oxlint).

## Trading, money and accounting (Phases 2–6)

- **One posting transaction per business event.** A sale, return, purchase
  posting, payment, expense, transfer or stock adjustment writes its document,
  stock movements, payments and allocations, cash/bank book lines, party ledger
  entries, its journal and the audit record in one `TenantTx`. Lock order:
  idempotency key → documents being changed → stock balances (sorted) →
  document numbers → payment accounts → party row → journal number.
- **Only the posting services change balances**: `pos_inventory::post_movements`
  (stock, moving weighted average §8.4), `pos_money::accounts::post_transaction`
  (account balances), `pos_parties::ledger::post` (customer/supplier balances).
  The cached `current_balance` columns always equal their ledgers.
- **Idempotency.** Posting endpoints accept `Idempotency-Key` (a UUID);
  `POST /sales` and `POST /sale-returns` require it. The key is claimed inside the
  transaction (`pos_audit::idempotency`), a repeat returns the stored response,
  and a deadlock retries the whole transaction up to three times
  (`crates/api/src/posting.rs`).
- **Pricing** (§8.2) lives in `crates/sales/src/pricing.rs` and its twin
  `frontend/src/lib/pricing.ts`; both run the vectors in
  `frontend/src/lib/pricing.vectors.json`. Change all three together.
- **Accounting is always on.** Every event is journalled by
  `pos_accounting::journal::post`, which refuses unbalanced journals and
  re-reads the line sums before commit. The chart of accounts is seeded at
  provisioning (`CHART_OF_ACCOUNTS`) and by migration 0105 for older
  businesses. Cancellations and voids post reversing journals; posted journals
  are never edited. Locked fiscal months refuse new journals (`423 PERIOD_LOCKED`).
- **Reports** use business dates in the tenant timezone. Net sales exclude VAT,
  charges and rounding and are after returns; gross profit = net sales − COGS;
  the profit estimate (`/reports/profit`) and the P&L account must agree.
- **New defaults** (payment accounts and methods, walk-in customer, expense
  categories, chart of accounts, journal and purchase-return numbers) are created
  by provisioning and back-filled for existing businesses by migrations 0012,
  0013, 0017, 0103 and 0105 (the reserved `platform` tenant is skipped).
- **End-to-end check:** `crates/api/tests/flows.rs` runs the whole
  buy → stock → sell → collect → return → expense cycle and checks that the
  trial balance and balance sheet balance and that ledger inventory equals the
  stock valuation.

## Super Admin, subscription billing and data export (Phase 5a)

- **Platform staff** are ordinary users with a row in `platform_staff`
  (`platform_owner` | `billing` | `support`, ADM-R11). Create the first one with
  `cargo run -p pos-api -- platform-staff <phone|email> platform_owner`; they sign
  in normally and open `/admin`. `PlatformAuth` (crates/api/src/auth.rs) re-reads
  the staff record per request; non-staff get `403 PLATFORM_STAFF_ONLY`. Staff act
  in a `PlatformTx` and every change is audited with `record_platform`.
  `support` is read-only, `billing` also verifies payments and sets overrides,
  `platform_owner` also suspends businesses and edits plans and settings.
  MFA for staff (`mfa_required`) is not enforced yet.
- **Manual billing (SUB-R05, ADM-R05).** The owner opens an invoice for a plan and
  cycle (`SUB-YYYY-NNNNNN`, platform-wide sequence), pays by bKash/Nagad/bank and
  submits the transaction id as a claim. Staff approve it in `/admin/claims`: the
  invoice is paid and the subscription becomes `active` on that plan for the paid
  months, starting at the end of a still-running paid period (no lost days).
  Payment instructions are edited in `/admin/settings`. Screenshots of payments
  wait for file storage (migration 0011).
- **Lifecycle (SUB-R02, SUB-R06).** The worker runs `pos_platform::lifecycle` every
  tick: it stores the state the dates imply (trial/period end → grace 7 days →
  read-only 30 days → suspended) with a `status_changed` event, and records the
  T-7/T-3/T-1 reminders (shown in-app by the trial/renewal banner).
- **Data export (NFR-PRIV-01).** `GET /api/v1/subscription/export` (owner) returns
  a ZIP with one CSV per business table (`pos_platform::export::TABLES`). It lives
  under `/subscription` so it also works while read-only or suspended.
- **Audit viewer (E5.6)** is at `/audit` in the app.

## Branches, transfers, counts, shifts and other income (Phase 6)

- **Working branch (BRN-R02).** Requests may carry `X-Branch-Id`. The branch must
  be active and inside the member's branch scope, else `403 BRANCH_NOT_ALLOWED`.
  Without it, postings go to the default branch and lists cover every branch the
  member may see (members limited to some branches get the first of them).
  `GET /auth/me` returns `tenant.branch_ids` (`null` = all). The web app keeps the
  choice per business in `localStorage` (`pos.branch.<tenant>`), sends it from
  `apiFetch` (never on `/auth/*`), resets cached queries on a switch and forgets
  a branch the API refuses.
- **Stock transfers** (`/stock-transfers`, plan feature `inventory.transfers`):
  draft → dispatched (`transfer_out` at the source average) → received
  (`transfer_in` at the dispatch cost; a shortfall is booked as transit loss) or
  cancelled. Receiving needs `inventory.transfer_receive`.
- **Stock counts** (`/stock-counts`): starting a count snapshots system
  quantities for a location (optionally one category); entries are saved with
  `PUT /items`; posting (`inventory.count` + `inventory.adjust`) writes the
  differences as `count_adjustment` movements and a journal.
- **Registers and shifts** (`/registers`, `/shifts`, plan limit `registers.max`):
  a cashier opens a shift on a register with an opening float; sales made while
  it is open are linked to it. `GET /shifts/{id}/report` is the X report while
  open and the Z report after `POST /shifts/{id}/close` stores expected, counted
  and the over/short difference.
- **Other income** (`/incomes`, `/income-categories`) posts money in to a
  payment account with its journal; void posts the reversal.
- **Reports:** `/reports/aging` (customers or suppliers, 0–30/31–60/61–90/90+),
  `/reports/vat-summary` (output vs input VAT by rate), `/reports/discounts`,
  `/reports/product-movement` (fast / slow / non-moving).
- **End-to-end check:** `crates/api/tests/branches.rs`.
