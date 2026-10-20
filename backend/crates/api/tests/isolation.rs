//! Cross-tenant isolation harness (E1.11, SRS-TEN-005, AT-02/AT-03; DB design
//! §3.1 layer L4).
//!
//! Every operation in the OpenAPI document must be classified in `access()`;
//! an unclassified route fails the suite, so new endpoints cannot skip it.
//! Business A is populated, then business B's owner (who holds every
//! permission) calls every operation with A's ids:
//!   * routes on one of A's resources must answer `404` (never `403`, which
//!     would reveal that the id exists);
//!   * list/read routes must not mention any of A's ids or names;
//!   * B's own writes must succeed or fail cleanly (no 5xx);
//!   * afterwards A's data, read as A, is unchanged.
//!
//! `every_route_is_documented` checks that every `#[get(..)]`-style route
//! macro appears in the OpenAPI document, so "registered" means "documented".

mod common;

use std::collections::BTreeSet;

use actix_web::{http::StatusCode, test as actix_test};
use common::{Req, Session, call, create_business, invite, join, role_id, sign_up};
use pos_api::{AppState, app, openapi::ApiDoc};
use serde_json::{Value, json};
use sqlx::MySqlPool;
use utoipa::OpenApi;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Access {
    /// No sign-in, or acts only on the caller's own account/token.
    Public,
    /// Signed in; reads the caller's own account. Must not show other businesses' data.
    UserRead,
    /// Reads the caller's business. Must not show other businesses' data.
    TenantRead,
    /// Writes the caller's own business (no foreign id in the path).
    TenantWrite,
    /// Acts on one resource by id: another business's id must be `404`.
    Resource,
    /// Reads by another business's id given in this query parameter: `404`.
    QueryResource(&'static str),
    /// Reads with another business's ids in this query parameter: `200`, nothing leaked.
    QueryRead(&'static str),
    /// Super Admin console: a business owner who is not platform staff gets `403`.
    Platform,
}

/// The classification of every API operation. Add new routes here.
fn access(method: &str, path: &str) -> Option<Access> {
    use Access::*;
    Some(match (method, path) {
        ("get", "/healthz" | "/readyz" | "/api/v1/meta") => Public,
        (
            "post",
            "/api/v1/auth/register"
            | "/api/v1/auth/verify-phone"
            | "/api/v1/auth/verify-phone/resend"
            | "/api/v1/auth/login"
            | "/api/v1/auth/refresh"
            | "/api/v1/auth/logout"
            | "/api/v1/auth/password/forgot"
            | "/api/v1/auth/password/reset"
            | "/api/v1/invitations/preview"
            | "/api/v1/invitations/accept"
            | "/api/v1/tenants",
        ) => Public,
        // Checked separately below: switching into a business you are not a member of.
        ("post", "/api/v1/auth/switch-tenant") => Public,
        ("get", "/api/v1/auth/me" | "/api/v1/auth/sessions") => UserRead,
        ("delete", "/api/v1/auth/sessions/{id}") => Resource,
        (
            "get",
            "/api/v1/settings"
            | "/api/v1/branches"
            | "/api/v1/onboarding"
            | "/api/v1/subscription"
            | "/api/v1/invitations"
            | "/api/v1/members"
            | "/api/v1/permissions"
            | "/api/v1/roles"
            | "/api/v1/audit-logs"
            | "/api/v1/tax-rates"
            | "/api/v1/units"
            | "/api/v1/categories"
            | "/api/v1/brands"
            | "/api/v1/products"
            | "/api/v1/stock-locations"
            | "/api/v1/stock"
            | "/api/v1/stock/valuation"
            | "/api/v1/stock/movements"
            | "/api/v1/stock-adjustments"
            | "/api/v1/customers"
            | "/api/v1/customers/walk-in"
            | "/api/v1/suppliers"
            | "/api/v1/payment-accounts"
            | "/api/v1/payment-methods"
            | "/api/v1/payments"
            | "/api/v1/account-transfers"
            | "/api/v1/expense-categories"
            | "/api/v1/expenses"
            | "/api/v1/pos/context"
            | "/api/v1/pos/products"
            | "/api/v1/sales"
            | "/api/v1/sale-returns"
            | "/api/v1/held-carts"
            | "/api/v1/purchases"
            | "/api/v1/purchase-returns"
            | "/api/v1/dashboard"
            | "/api/v1/reports/sales-summary"
            | "/api/v1/reports/sales-by-product"
            | "/api/v1/reports/sales-by-category"
            | "/api/v1/reports/sales-by-cashier"
            | "/api/v1/reports/payments"
            | "/api/v1/reports/purchases"
            | "/api/v1/reports/expenses"
            | "/api/v1/reports/profit"
            | "/api/v1/reports/trial-balance"
            | "/api/v1/reports/profit-and-loss"
            | "/api/v1/reports/balance-sheet"
            | "/api/v1/ledger-accounts"
            | "/api/v1/journals"
            | "/api/v1/fiscal-periods"
            | "/api/v1/stock-transfers"
            | "/api/v1/stock-counts"
            | "/api/v1/registers"
            | "/api/v1/shifts"
            | "/api/v1/shifts/current"
            | "/api/v1/income-categories"
            | "/api/v1/incomes"
            | "/api/v1/bank-reconciliations"
            | "/api/v1/reports/aging"
            | "/api/v1/reports/vat-summary"
            | "/api/v1/reports/discounts"
            | "/api/v1/reports/product-movement",
        ) => TenantRead,
        ("get", "/api/v1/purchases/last-prices") => QueryRead("variant_ids"),
        ("get", "/api/v1/subscription/billing" | "/api/v1/subscription/export") => TenantRead,
        ("post", "/api/v1/subscription/invoices") => TenantWrite,
        ("post", "/api/v1/subscription/invoices/{id}/claims") => Resource,
        (_, p) if p.starts_with("/api/v1/admin/") => Platform,
        ("get", "/api/v1/reports/general-ledger") => QueryResource("account_id"),
        ("patch", "/api/v1/settings")
        | ("post", "/api/v1/onboarding/steps/{step}" | "/api/v1/onboarding/dismiss")
        | ("post", "/api/v1/invitations" | "/api/v1/roles")
        | (
            "post",
            "/api/v1/tax-rates" | "/api/v1/units" | "/api/v1/categories" | "/api/v1/brands" | "/api/v1/products",
        )
        | (
            "post",
            "/api/v1/branches"
            | "/api/v1/stock-transfers"
            | "/api/v1/stock-counts"
            | "/api/v1/registers"
            | "/api/v1/shifts"
            | "/api/v1/income-categories"
            | "/api/v1/incomes"
            | "/api/v1/bank-reconciliations"
            | "/api/v1/stock-adjustments"
            | "/api/v1/customers"
            | "/api/v1/suppliers"
            | "/api/v1/payment-accounts"
            | "/api/v1/account-transfers"
            | "/api/v1/expense-categories"
            | "/api/v1/expenses"
            | "/api/v1/sales"
            | "/api/v1/sale-returns"
            | "/api/v1/held-carts"
            | "/api/v1/collections"
            | "/api/v1/purchases"
            | "/api/v1/supplier-payments"
            | "/api/v1/purchase-returns"
            | "/api/v1/journals"
            | "/api/v1/fiscal-periods/lock"
            | "/api/v1/fiscal-periods/reopen",
        ) => TenantWrite,
        ("patch", "/api/v1/branches/{id}")
        | ("post", "/api/v1/branches/{id}/activate" | "/api/v1/branches/{id}/deactivate")
        | ("post", "/api/v1/invitations/{id}/resend")
        | ("delete", "/api/v1/invitations/{id}")
        | ("get", "/api/v1/members/{id}" | "/api/v1/members/{id}/sessions")
        | ("put", "/api/v1/members/{id}/roles")
        | ("post", "/api/v1/members/{id}/suspend" | "/api/v1/members/{id}/reactivate")
        | ("delete", "/api/v1/members/{id}" | "/api/v1/members/{id}/sessions")
        | ("get", "/api/v1/roles/{id}")
        | ("put", "/api/v1/roles/{id}")
        | ("delete", "/api/v1/roles/{id}")
        | ("put", "/api/v1/tax-rates/{id}" | "/api/v1/units/{id}")
        | ("put" | "delete", "/api/v1/categories/{id}" | "/api/v1/brands/{id}")
        | ("get" | "put" | "delete", "/api/v1/products/{id}")
        | ("post", "/api/v1/products/{id}/archive" | "/api/v1/products/{id}/restore")
        | ("get", "/api/v1/products/{id}/price-history")
        | ("get" | "put" | "delete", "/api/v1/customers/{id}" | "/api/v1/suppliers/{id}")
        | ("get", "/api/v1/customers/{id}/statement" | "/api/v1/suppliers/{id}/statement")
        | ("get" | "put", "/api/v1/payment-accounts/{id}")
        | ("get", "/api/v1/payment-accounts/{id}/book")
        | ("patch", "/api/v1/payment-methods/{id}")
        | ("get", "/api/v1/payments/{id}")
        | ("post", "/api/v1/payments/{id}/void")
        | ("put", "/api/v1/expense-categories/{id}")
        | ("get", "/api/v1/expenses/{id}")
        | ("post", "/api/v1/expenses/{id}/void")
        | ("get", "/api/v1/sales/{id}")
        | ("post", "/api/v1/sales/{id}/reprint" | "/api/v1/sales/{id}/cancel")
        | ("get", "/api/v1/sale-returns/{id}")
        | ("delete", "/api/v1/held-carts/{id}")
        | ("get" | "put", "/api/v1/purchases/{id}")
        | (
            "post",
            "/api/v1/purchases/{id}/post"
            | "/api/v1/purchases/{id}/cancel"
            | "/api/v1/purchases/{id}/order"
            | "/api/v1/purchases/{id}/approve"
            | "/api/v1/purchases/{id}/receive"
            | "/api/v1/purchases/{id}/close",
        )
        | ("get", "/api/v1/purchase-returns/{id}")
        | ("get", "/api/v1/stock-adjustments/{id}")
        | ("get", "/api/v1/journals/{id}")
        | ("post", "/api/v1/journals/{id}/reverse")
        | ("get", "/api/v1/bank-reconciliations/{id}")
        | ("put", "/api/v1/bank-reconciliations/{id}/lines")
        | (
            "post",
            "/api/v1/bank-reconciliations/{id}/complete"
            | "/api/v1/bank-reconciliations/{id}/cancel"
            | "/api/v1/bank-reconciliations/{id}/reopen",
        )
        | ("get", "/api/v1/stock-transfers/{id}")
        | (
            "post",
            "/api/v1/stock-transfers/{id}/dispatch"
            | "/api/v1/stock-transfers/{id}/receive"
            | "/api/v1/stock-transfers/{id}/cancel",
        )
        | ("get", "/api/v1/stock-counts/{id}")
        | ("put", "/api/v1/stock-counts/{id}/items")
        | ("post", "/api/v1/stock-counts/{id}/post" | "/api/v1/stock-counts/{id}/cancel")
        | ("put", "/api/v1/registers/{id}")
        | ("get", "/api/v1/shifts/{id}/report")
        | ("post", "/api/v1/shifts/{id}/close")
        | ("put", "/api/v1/income-categories/{id}")
        | ("get", "/api/v1/incomes/{id}")
        | ("post", "/api/v1/incomes/{id}/void") => Resource,
        _ => return None,
    })
}

/// Every (method, path) in the OpenAPI document.
fn operations() -> Vec<(String, String)> {
    let doc = serde_json::to_value(ApiDoc::openapi()).expect("openapi serialises");
    let mut ops = Vec::new();
    for (path, item) in doc["paths"].as_object().into_iter().flatten() {
        for method in ["get", "post", "put", "patch", "delete"] {
            if item.get(method).is_some() {
                ops.push((method.to_owned(), path.clone()));
            }
        }
    }
    ops
}

#[test]
fn every_operation_is_classified() {
    let missing: Vec<String> = operations()
        .into_iter()
        .filter(|(m, p)| access(m, p).is_none())
        .map(|(m, p)| format!("{} {p}", m.to_uppercase()))
        .collect();
    assert!(missing.is_empty(), "classify these routes in tests/isolation.rs access(): {missing:#?}");
}

/// Route macros in `src/routes` (path relative to its scope) must each be in
/// the OpenAPI document, so an undocumented route cannot dodge the harness.
#[test]
fn every_route_is_documented() {
    let documented: Vec<(String, String)> = operations();
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/routes");
    let mut undocumented = Vec::new();
    for entry in std::fs::read_dir(dir).expect("routes dir") {
        let path = entry.expect("entry").path();
        let source = std::fs::read_to_string(&path).expect("route source");
        for line in source.lines().map(str::trim) {
            for method in ["get", "post", "put", "patch", "delete"] {
                let Some(rest) = line.strip_prefix(&format!("#[{method}(\"")) else { continue };
                let Some(route) = rest.split('"').next() else { continue };
                // Served by the document itself.
                if route == "/openapi.json" {
                    continue;
                }
                if !documented.iter().any(|(m, p)| m == method && p.ends_with(route)) {
                    undocumented.push(format!("{} {route} ({})", method.to_uppercase(), path.display()));
                }
            }
        }
    }
    assert!(undocumented.is_empty(), "add these routes to openapi.rs: {undocumented:#?}");
}

/// Business A's data that B must never see or touch.
struct Victim {
    owner: Session,
    tenant_id: String,
    member_id: String,
    invitation_id: String,
    role_id: String,
    branch_id: String,
    session_id: String,
    tax_rate_id: String,
    unit_id: String,
    category_id: String,
    brand_id: String,
    product_id: String,
    trade: Trade,
}

/// Alice's trading documents (Phases 2–6).
#[derive(Default)]
struct Trade {
    variant_id: String,
    customer_id: String,
    supplier_id: String,
    account_id: String,
    method_id: String,
    ledger_account_id: String,
    payment_id: String,
    expense_category_id: String,
    expense_id: String,
    sale_id: String,
    sale_return_id: String,
    held_cart_id: String,
    purchase_id: String,
    purchase_return_id: String,
    adjustment_id: String,
    journal_id: String,
    invoice_id: String,
    location_id: String,
    transfer_id: String,
    count_id: String,
    count_item_id: String,
    register_id: String,
    shift_id: String,
    income_category_id: String,
    income_id: String,
    reconciliation_id: String,
}

impl Victim {
    fn id_for(&self, path: &str) -> &str {
        if path.starts_with("/api/v1/members/") {
            &self.member_id
        } else if path.starts_with("/api/v1/invitations/") {
            &self.invitation_id
        } else if path.starts_with("/api/v1/roles/") {
            &self.role_id
        } else if path.starts_with("/api/v1/branches/") {
            &self.branch_id
        } else if path.starts_with("/api/v1/auth/sessions/") {
            &self.session_id
        } else if path.starts_with("/api/v1/tax-rates/") {
            &self.tax_rate_id
        } else if path.starts_with("/api/v1/units/") {
            &self.unit_id
        } else if path.starts_with("/api/v1/categories/") {
            &self.category_id
        } else if path.starts_with("/api/v1/brands/") {
            &self.brand_id
        } else if path.starts_with("/api/v1/products/") {
            &self.product_id
        } else if path.starts_with("/api/v1/customers/") {
            &self.trade.customer_id
        } else if path.starts_with("/api/v1/suppliers/") {
            &self.trade.supplier_id
        } else if path.starts_with("/api/v1/payment-accounts/") {
            &self.trade.account_id
        } else if path.starts_with("/api/v1/payment-methods/") {
            &self.trade.method_id
        } else if path.starts_with("/api/v1/payments/") {
            &self.trade.payment_id
        } else if path.starts_with("/api/v1/expense-categories/") {
            &self.trade.expense_category_id
        } else if path.starts_with("/api/v1/expenses/") {
            &self.trade.expense_id
        } else if path.starts_with("/api/v1/sales/") {
            &self.trade.sale_id
        } else if path.starts_with("/api/v1/sale-returns/") {
            &self.trade.sale_return_id
        } else if path.starts_with("/api/v1/held-carts/") {
            &self.trade.held_cart_id
        } else if path.starts_with("/api/v1/purchases/last-prices") {
            &self.trade.variant_id
        } else if path.starts_with("/api/v1/purchases/") {
            &self.trade.purchase_id
        } else if path.starts_with("/api/v1/purchase-returns/") {
            &self.trade.purchase_return_id
        } else if path.starts_with("/api/v1/stock-adjustments/") {
            &self.trade.adjustment_id
        } else if path.starts_with("/api/v1/journals/") {
            &self.trade.journal_id
        } else if path.starts_with("/api/v1/stock-transfers/") {
            &self.trade.transfer_id
        } else if path.starts_with("/api/v1/stock-counts/") {
            &self.trade.count_id
        } else if path.starts_with("/api/v1/registers/") {
            &self.trade.register_id
        } else if path.starts_with("/api/v1/shifts/") {
            &self.trade.shift_id
        } else if path.starts_with("/api/v1/income-categories/") {
            &self.trade.income_category_id
        } else if path.starts_with("/api/v1/incomes/") {
            &self.trade.income_id
        } else if path.starts_with("/api/v1/bank-reconciliations/") {
            &self.trade.reconciliation_id
        } else if path.starts_with("/api/v1/subscription/invoices/") {
            &self.trade.invoice_id
        } else if path.starts_with("/api/v1/reports/general-ledger") {
            &self.trade.ledger_account_id
        } else {
            panic!("no fixture id for {path}; add one to Victim")
        }
    }

    /// Strings that must not appear in anything B can read.
    fn markers(&self) -> Vec<&str> {
        let t = &self.trade;
        vec![
            &t.variant_id,
            &t.customer_id,
            &t.supplier_id,
            &t.account_id,
            &t.method_id,
            &t.ledger_account_id,
            &t.payment_id,
            &t.expense_category_id,
            &t.expense_id,
            &t.sale_id,
            &t.sale_return_id,
            &t.held_cart_id,
            &t.purchase_id,
            &t.purchase_return_id,
            &t.adjustment_id,
            &t.journal_id,
            &t.invoice_id,
            &t.location_id,
            &t.transfer_id,
            &t.count_id,
            &t.count_item_id,
            &t.register_id,
            &t.shift_id,
            &t.income_category_id,
            &t.income_id,
            &t.reconciliation_id,
            "Alice Till",
            "Alice Bank Interest",
            "Alice Customer",
            "Alice Supplier",
            "Alice Landlord",
            "Alice Cart",
            "1711000404",
            &self.tenant_id,
            &self.member_id,
            &self.invitation_id,
            &self.role_id,
            &self.branch_id,
            &self.session_id,
            &self.tax_rate_id,
            &self.unit_id,
            &self.category_id,
            &self.brand_id,
            &self.product_id,
            "Alice Store",
            "Alice Product",
            "Alice Category",
            "Alice Brand",
            "Alice VAT",
            "ALICE-SKU-1",
            "Alice Role",
            "Alice Staff",
            "1711000401",
            "1711000402",
            "1711000403",
        ]
    }
}

fn first_id(body: &Value) -> String {
    body.as_array().and_then(|a| a.first()).and_then(|v| v["id"].as_str()).expect("an id").to_owned()
}

fn find_id(body: &Value, key: &str, value: &str) -> String {
    body.as_array()
        .into_iter()
        .flatten()
        .find(|v| v[key] == value)
        .and_then(|v| v["id"].as_str())
        .expect(value)
        .to_owned()
}

/// Request body for an operation (valid in shape, so handlers get as far as
/// the lookup). Writes in B's own business that name A's documents must fail.
fn body_for(method: &str, path: &str, b_role: &str, b_unit: &str, a: &Trade) -> Option<Value> {
    Some(match (method, path) {
        (
            "put",
            "/api/v1/customers/{id}"
            | "/api/v1/suppliers/{id}"
            | "/api/v1/payment-accounts/{id}"
            | "/api/v1/expense-categories/{id}",
        ) => {
            json!({ "name": "Hijacked" })
        }
        (
            "post",
            "/api/v1/payments/{id}/void"
            | "/api/v1/expenses/{id}/void"
            | "/api/v1/incomes/{id}/void"
            | "/api/v1/sales/{id}/cancel"
            | "/api/v1/purchases/{id}/cancel"
            | "/api/v1/purchases/{id}/close"
            | "/api/v1/bank-reconciliations/{id}/reopen",
        ) => {
            json!({ "reason": "Hijack" })
        }
        ("put", "/api/v1/purchases/{id}") => json!({ "supplier_id": a.supplier_id, "lines": [] }),
        ("put", "/api/v1/stock-counts/{id}/items") => {
            json!({ "lines": [{ "item_id": a.count_item_id, "counted_qty": "1" }] })
        }
        ("post", "/api/v1/shifts/{id}/close") => json!({ "counted_cash": "0" }),
        ("put", "/api/v1/registers/{id}") => json!({ "name": "Hijacked" }),
        ("put", "/api/v1/income-categories/{id}") => json!({ "name": "Hijacked" }),
        ("post", "/api/v1/stock-transfers") => json!({
            "to_location_id": a.location_id, "lines": [{ "variant_id": a.variant_id, "quantity": "1" }],
        }),
        ("post", "/api/v1/stock-counts") => json!({ "location_id": a.location_id }),
        ("post", "/api/v1/registers") => json!({ "name": "Bob Till" }),
        ("post", "/api/v1/shifts") => json!({ "register_id": a.register_id, "opening_cash": "0" }),
        ("post", "/api/v1/income-categories") => json!({ "name": "Bob Income" }),
        ("post", "/api/v1/incomes") => json!({ "category_id": a.income_category_id, "amount": "1" }),
        ("post", "/api/v1/bank-reconciliations") => {
            json!({ "account_id": a.account_id, "statement_date": "2026-01-31", "statement_balance": "0" })
        }
        ("put", "/api/v1/bank-reconciliations/{id}/lines") => json!({ "clear": [a.payment_id] }),
        ("post", "/api/v1/subscription/invoices/{id}/claims") => {
            json!({ "method": "bkash", "amount": "990", "transaction_ref": "TRXHIJACK1" })
        }
        ("post", "/api/v1/sales") => json!({
            "customer_id": a.customer_id, "lines": [{ "variant_id": a.variant_id, "quantity": "1" }],
            "payments": [{ "method_id": a.method_id, "amount": "120" }],
        }),
        ("post", "/api/v1/sale-returns") => json!({
            "sale_id": a.sale_id, "reason": "Hijack", "refund_mode": "cash", "lines": [],
        }),
        ("post", "/api/v1/collections") => {
            json!({ "customer_id": a.customer_id, "amount": "1", "method_id": a.method_id })
        }
        ("post", "/api/v1/supplier-payments") => {
            json!({ "supplier_id": a.supplier_id, "amount": "1", "method_id": a.method_id })
        }
        ("post", "/api/v1/purchases") => json!({
            "supplier_id": a.supplier_id, "lines": [{ "variant_id": a.variant_id, "quantity": "1", "unit_cost": "1" }],
        }),
        ("post", "/api/v1/purchase-returns") => {
            json!({ "purchase_id": a.purchase_id, "reason": "Hijack", "lines": [] })
        }
        ("post", "/api/v1/stock-adjustments") => json!({
            "reason": "correction", "lines": [{ "variant_id": a.variant_id, "quantity": "-1" }],
        }),
        ("post", "/api/v1/expenses") => json!({ "category_id": a.expense_category_id, "amount": "1" }),
        ("post", "/api/v1/account-transfers") => json!({
            "from_account_id": a.account_id, "to_account_id": a.account_id, "amount": "1",
        }),
        ("post", "/api/v1/held-carts") => json!({ "customer_id": a.customer_id, "cart": {} }),
        ("post", "/api/v1/journals") => json!({
            "memo": "Hijack", "lines": [{ "account_id": a.ledger_account_id, "debit": "1" }, { "account_id": a.ledger_account_id, "credit": "1" }],
        }),
        ("patch", "/api/v1/settings") => json!({ "district": "Dhaka" }),
        ("post", "/api/v1/invitations") => json!({ "destination": "01711000499", "role_ids": [b_role] }),
        ("post", "/api/v1/roles") => json!({ "name": "Bob Role", "permissions": ["pos.sell"] }),
        ("patch", "/api/v1/branches/{id}") => json!({ "name": "Hijacked" }),
        ("post", "/api/v1/branches") => json!({ "code": "BOB2", "name": "Bob Second" }),
        ("put", "/api/v1/members/{id}/roles") => json!({ "assignments": [{ "role_id": b_role }] }),
        ("put", "/api/v1/roles/{id}") => json!({ "name": "Hijacked", "permissions": ["pos.sell"] }),
        ("post", "/api/v1/tax-rates") => json!({ "name": "Bob VAT", "rate_pct": "2" }),
        ("put", "/api/v1/tax-rates/{id}") => json!({ "name": "Hijacked", "rate_pct": "1" }),
        ("post", "/api/v1/units") => json!({ "name": "Bob Unit", "short_name": "bu" }),
        ("put", "/api/v1/units/{id}") => json!({ "name": "Hijacked", "short_name": "hj" }),
        (
            "post" | "put",
            "/api/v1/categories" | "/api/v1/categories/{id}" | "/api/v1/brands" | "/api/v1/brands/{id}",
        ) => {
            json!({ "name": "Bob Thing" })
        }
        // Bob's own product with Alice's barcode: allowed across businesses (AC-PROD-01).
        ("post", "/api/v1/products") => {
            json!({ "name": "Bob Product", "unit_id": b_unit, "selling_price": "10", "barcodes": ["4006381333931"] })
        }
        ("put", "/api/v1/products/{id}") => json!({ "name": "Hijacked", "unit_id": b_unit, "selling_price": "1" }),
        ("post", _) | ("patch", _) | ("put", _) => json!({}),
        _ => return None,
    })
}

fn request(method: &str, uri: &str, body: Option<Value>) -> Req {
    match (method, body) {
        ("get", _) => Req::get(uri),
        ("delete", _) => Req::delete(uri),
        ("post", body) => Req::post(uri, body.unwrap_or_else(|| json!({}))),
        ("patch", body) => Req::patch(uri, body.unwrap_or_else(|| json!({}))),
        ("put", body) => Req::put(uri, body.unwrap_or_else(|| json!({}))),
        _ => unreachable!("{method}"),
    }
}

/// What A sees of its own business; compared before and after B's attempts.
async fn snapshot<S, B>(srv: &S, a: &Session) -> Vec<Value>
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let mut out = Vec::new();
    for uri in [
        "/api/v1/settings",
        "/api/v1/branches",
        "/api/v1/members",
        "/api/v1/invitations",
        "/api/v1/roles",
        "/api/v1/tax-rates",
        "/api/v1/units",
        "/api/v1/categories",
        "/api/v1/brands",
        "/api/v1/products?status=all",
        "/api/v1/stock",
        "/api/v1/customers?status=all",
        "/api/v1/suppliers?status=all",
        "/api/v1/payment-accounts?include_inactive=true",
        "/api/v1/payment-methods?include_inactive=true",
        "/api/v1/payments?status=all",
        "/api/v1/expense-categories",
        "/api/v1/expenses?status=all",
        "/api/v1/sales",
        "/api/v1/sale-returns",
        "/api/v1/held-carts",
        "/api/v1/purchases",
        "/api/v1/purchase-returns",
        "/api/v1/stock-adjustments",
        "/api/v1/journals",
        "/api/v1/reports/trial-balance",
    ] {
        let res = call(srv, Req::get(uri).bearer(&a.token).build()).await;
        assert_eq!(res.status, StatusCode::OK, "{uri}: {}", res.body);
        out.push(res.body);
    }
    let sessions = call(srv, Req::get("/api/v1/auth/sessions").bearer(&a.token).build()).await;
    let mut ids: Vec<String> = sessions
        .body
        .as_array()
        .into_iter()
        .flatten()
        .filter(|s| s["revoked_at"].is_null())
        .filter_map(|s| s["id"].as_str().map(str::to_owned))
        .collect();
    ids.sort();
    out.push(json!(ids));
    out
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn other_businesses_cannot_see_or_touch_data(pool: MySqlPool) {
    // Transfers are a paid feature; switch them on for the trial plan here.
    sqlx::query(
        "UPDATE plan_features pf JOIN plans p ON p.id = pf.plan_id SET pf.enabled = TRUE \
         WHERE p.code = 'trial' AND pf.feature_code = 'inventory.transfers'",
    )
    .execute(&pool)
    .await
    .expect("enable transfers");
    let srv = actix_test::init_service(app(AppState::new(pool.clone(), common::config()))).await;

    let mut alice = sign_up(&srv, "Alice", "01711000401").await;
    let tenant_id = create_business(&srv, &mut alice, "Alice Store").await;
    join(&srv, &alice, &tenant_id, "Alice Staff", "01711000402", &["cashier"]).await;
    let invited = invite(&srv, &alice, "01711000403", &["cashier"]).await;
    let invitation_id = invited.body["invitation"]["id"].as_str().expect("invitation").to_owned();
    let role = call(
        &srv,
        Req::post("/api/v1/roles", json!({ "name": "Alice Role", "permissions": ["pos.sell"] }))
            .bearer(&alice.token)
            .build(),
    )
    .await;
    assert_eq!(role.status, StatusCode::CREATED, "{}", role.body);
    let post = |uri: &str, body: Value| Req::post(uri, body).bearer(&alice.token).build();
    let tax = call(&srv, post("/api/v1/tax-rates", json!({ "name": "Alice VAT", "rate_pct": "3" }))).await;
    let unit = call(&srv, post("/api/v1/units", json!({ "name": "Alice Unit", "short_name": "au" }))).await;
    let category = call(&srv, post("/api/v1/categories", json!({ "name": "Alice Category" }))).await;
    let brand = call(&srv, post("/api/v1/brands", json!({ "name": "Alice Brand" }))).await;
    let id_of =
        |res: &common::Res| res.body["id"].as_str().unwrap_or_else(|| panic!("created: {}", res.body)).to_owned();
    let product = call(
        &srv,
        post(
            "/api/v1/products",
            json!({ "name": "Alice Product", "sku": "ALICE-SKU-1", "barcodes": ["4006381333931"],
                    "unit_id": id_of(&unit), "category_id": id_of(&category), "brand_id": id_of(&brand),
                    "tax_rate_id": id_of(&tax), "selling_price": "120", "purchase_price": "90" }),
        ),
    )
    .await;
    let trade = alice_trades(&srv, &alice, &id_of(&product)).await;
    let members = call(&srv, Req::get("/api/v1/members").bearer(&alice.token).build()).await;
    let branches = call(&srv, Req::get("/api/v1/branches").bearer(&alice.token).build()).await;
    let sessions = call(&srv, Req::get("/api/v1/auth/sessions").bearer(&alice.token).build()).await;
    let victim = Victim {
        member_id: find_id(&members.body, "full_name", "Alice Staff"),
        role_id: role.body["id"].as_str().expect("role id").to_owned(),
        branch_id: first_id(&branches.body),
        session_id: first_id(&sessions.body),
        tax_rate_id: id_of(&tax),
        unit_id: id_of(&unit),
        category_id: id_of(&category),
        brand_id: id_of(&brand),
        product_id: id_of(&product),
        trade,
        invitation_id,
        tenant_id,
        owner: alice,
    };
    let before = snapshot(&srv, &victim.owner).await;

    let mut bob = sign_up(&srv, "Bob", "01711000411").await;
    create_business(&srv, &mut bob, "Bob Store").await;
    let b_role = role_id(&srv, &bob, "cashier").await;
    let b_units = call(&srv, Req::get("/api/v1/units").bearer(&bob.token).build()).await;
    let b_unit = first_id(&b_units.body);

    let mut failures = Vec::new();
    let mut checked = BTreeSet::new();
    for (method, path) in operations() {
        let Some(kind) = access(&method, &path) else { continue };
        let label = format!("{} {path}", method.to_uppercase());
        match kind {
            Access::Public => continue,
            Access::UserRead | Access::TenantRead => {
                let res = call(&srv, request(&method, &path, None).bearer(&bob.token).build()).await;
                let text = res.body.to_string();
                if res.status != StatusCode::OK {
                    failures.push(format!("{label}: expected 200, got {} {}", res.status, res.body));
                }
                for marker in victim.markers() {
                    if text.contains(marker) {
                        failures.push(format!("{label}: response leaks `{marker}`"));
                    }
                }
            }
            Access::QueryRead(param) => {
                let uri = format!("{path}?{param}={}", victim.id_for(&path));
                let res = call(&srv, Req::get(&uri).bearer(&bob.token).build()).await;
                if res.status != StatusCode::OK {
                    failures.push(format!("{label}: expected 200, got {} {}", res.status, res.body));
                }
                for marker in victim.markers() {
                    if res.body.to_string().contains(marker) {
                        failures.push(format!("{label}: response leaks `{marker}`"));
                    }
                }
            }
            Access::Platform => {
                let uri = path.replace("{id}", &victim.tenant_id).replace("{override_id}", &victim.tenant_id);
                // Valid bodies, so a 403 comes from the staff check, not from parsing.
                let body = match path.as_str() {
                    p if p.ends_with("/suspend") || p.ends_with("/reactivate") || p.ends_with("/reject") => {
                        json!({ "reason": "Hijack" })
                    }
                    p if p.ends_with("/overrides") => {
                        json!({ "feature_code": "users.max", "limit_value": 99, "reason": "Hijack" })
                    }
                    p if p.contains("/admin/plans") => json!({ "name": "Hijack", "is_public": true }),
                    _ => json!({}),
                };
                let req = request(&method, &uri, Some(body)).bearer(&bob.token);
                let res = call(&srv, req.build()).await;
                if res.status != StatusCode::FORBIDDEN || res.code() != "PLATFORM_STAFF_ONLY" {
                    failures
                        .push(format!("{label}: expected 403 PLATFORM_STAFF_ONLY, got {} {}", res.status, res.body));
                }
            }
            Access::QueryResource(param) => {
                let uri = format!("{path}?{param}={}", victim.id_for(&path));
                let res = call(&srv, Req::get(&uri).bearer(&bob.token).build()).await;
                if res.status != StatusCode::NOT_FOUND {
                    failures.push(format!("{label}: expected 404, got {} {}", res.status, res.body));
                }
            }
            Access::TenantWrite => {
                let uri = path.replace("{step}", "business");
                let body = body_for(&method, &path, &b_role, &b_unit, &victim.trade);
                let names_a = body.as_ref().is_some_and(|b| victim.markers().iter().any(|m| b.to_string().contains(m)));
                let req = request(&method, &uri, body)
                    .bearer(&bob.token)
                    .header("idempotency-key", &uuid::Uuid::now_v7().to_string());
                let res = call(&srv, req.build()).await;
                if res.status.is_server_error() {
                    failures.push(format!("{label}: {} {}", res.status, res.body));
                }
                // Naming A's documents from B's business must never succeed.
                if names_a && res.status.is_success() {
                    failures.push(format!("{label}: B wrote against A's records: {} {}", res.status, res.body));
                }
                // A barcode is unique per business, not globally (AC-PROD-01).
                if path == "/api/v1/products" && res.status != StatusCode::CREATED {
                    failures.push(format!("{label}: Bob could not reuse Alice's barcode: {} {}", res.status, res.body));
                }
            }
            Access::Resource => {
                let uri = path.replace("{id}", victim.id_for(&path));
                let req = request(&method, &uri, body_for(&method, &path, &b_role, &b_unit, &victim.trade))
                    .bearer(&bob.token)
                    .if_match(1)
                    .header("idempotency-key", &uuid::Uuid::now_v7().to_string());
                let res = call(&srv, req.build()).await;
                if res.status != StatusCode::NOT_FOUND {
                    failures.push(format!("{label}: expected 404, got {} {}", res.status, res.body));
                }
            }
        }
        checked.insert(label);
    }

    // Signing in to a business you are not a member of.
    let switch = call(
        &srv,
        Req::post("/api/v1/auth/switch-tenant", json!({ "tenant_id": victim.tenant_id }))
            .bearer(&bob.token)
            .cookie(&bob.cookie)
            .build(),
    )
    .await;
    if !matches!(switch.status, StatusCode::NOT_FOUND | StatusCode::FORBIDDEN) {
        failures.push(format!("switch-tenant into A: {} {}", switch.status, switch.body));
    }

    // AT-03: a tenant_id in a body is never honoured (rejected as an unknown field, or ignored).
    let smuggled = call(
        &srv,
        Req::post(
            "/api/v1/roles",
            json!({ "name": "Smuggled", "permissions": ["pos.sell"], "tenant_id": victim.tenant_id }),
        )
        .bearer(&bob.token)
        .build(),
    )
    .await;
    if smuggled.status.is_server_error() {
        failures.push(format!("POST /roles with tenant_id: {} {}", smuggled.status, smuggled.body));
    }
    let in_a: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM roles WHERE tenant_id = UUID_TO_BIN(?) AND name = 'Smuggled'")
            .bind(&victim.tenant_id)
            .fetch_one(&pool)
            .await
            .expect("count");
    assert_eq!(in_a, 0, "role created in A");

    assert!(failures.is_empty(), "cross-tenant failures:\n{}", failures.join("\n"));
    assert!(checked.len() > 25, "harness checked only {checked:?}");
    assert_eq!(snapshot(&srv, &victim.owner).await, before, "A's data changed");
}

/// Alice buys, sells, returns, spends and journals, so every trading route
/// has one of her records to aim at.
async fn alice_trades<S, B>(srv: &S, alice: &Session, product_id: &str) -> Trade
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let get = |uri: &str| Req::get(uri).bearer(&alice.token).build();
    let post = |uri: &str, body: Value| {
        Req::post(uri, body).bearer(&alice.token).header("idempotency-key", &uuid::Uuid::now_v7().to_string()).build()
    };
    let ok = |res: &common::Res, what: &str| {
        assert!(res.status.is_success(), "{what}: {} {}", res.status, res.body);
        res.body["id"].as_str().unwrap_or_else(|| panic!("{what} id: {}", res.body)).to_owned()
    };
    let product = call(srv, get(&format!("/api/v1/products/{product_id}"))).await;
    let variant_id = product.body["variant_id"].as_str().expect("variant").to_owned();
    let methods = call(srv, get("/api/v1/payment-methods")).await;
    let method_id = find_id(&methods.body, "name", "Cash");
    let accounts = call(srv, get("/api/v1/payment-accounts")).await;
    let account_id = find_id(&accounts.body, "name", "Cash");
    let ledger = call(srv, get("/api/v1/ledger-accounts")).await;
    let ledger_account_id = find_id(&ledger.body, "code", "1010");
    let capital = find_id(&ledger.body, "code", "3100");
    let categories = call(srv, get("/api/v1/expense-categories")).await;
    let expense_category_id = find_id(&categories.body, "name", "Rent");

    let opening = call(
        srv,
        post("/api/v1/stock-adjustments", json!({ "reason": "opening", "lines": [{ "variant_id": variant_id, "quantity": "20", "unit_cost": "90" }] })),
    )
    .await;
    let adjustment_id = ok(&opening, "opening stock");
    let customer =
        call(srv, post("/api/v1/customers", json!({ "name": "Alice Customer", "phone": "01711000404" }))).await;
    let customer_id = ok(&customer, "customer");
    let supplier = call(srv, post("/api/v1/suppliers", json!({ "name": "Alice Supplier" }))).await;
    let supplier_id = ok(&supplier, "supplier");
    let draft = call(
        srv,
        post("/api/v1/purchases", json!({ "supplier_id": supplier_id, "lines": [{ "variant_id": variant_id, "quantity": "5", "unit_cost": "95" }] })),
    )
    .await;
    let purchase_id = ok(&draft, "purchase draft");
    let posted = call(
        srv,
        post(
            &format!("/api/v1/purchases/{purchase_id}/post"),
            json!({ "payment": { "method_id": method_id, "amount": "100" } }),
        ),
    )
    .await;
    assert!(posted.status.is_success(), "post purchase: {}", posted.body);
    let payment_id = posted.body["payments"][0]["payment_id"].as_str().expect("payment").to_owned();
    let purchase_item = posted.body["items"][0]["id"].as_str().expect("item").to_owned();
    let purchase_return = call(
        srv,
        post("/api/v1/purchase-returns", json!({ "purchase_id": purchase_id, "reason": "Torn", "lines": [{ "purchase_item_id": purchase_item, "quantity": "1" }] })),
    )
    .await;
    let purchase_return_id = ok(&purchase_return, "purchase return");
    let sale = call(
        srv,
        post(
            "/api/v1/sales",
            json!({ "customer_id": customer_id, "lines": [{ "variant_id": variant_id, "quantity": "2" }],
                                      "payments": [{ "method_id": method_id, "amount": "100" }] }),
        ),
    )
    .await;
    let sale_id = ok(&sale, "sale");
    let sale_item = sale.body["items"][0]["id"].as_str().expect("sale item").to_owned();
    let sale_return = call(
        srv,
        post(
            "/api/v1/sale-returns",
            json!({ "sale_id": sale_id, "reason": "Broken", "refund_mode": "customer_credit",
                                             "lines": [{ "sale_item_id": sale_item, "quantity": "1" }] }),
        ),
    )
    .await;
    let sale_return_id = ok(&sale_return, "sale return");
    let expense = call(
        srv,
        post(
            "/api/v1/expenses",
            json!({ "category_id": expense_category_id, "amount": "50", "payee": "Alice Landlord" }),
        ),
    )
    .await;
    let expense_id = ok(&expense, "expense");
    let cart = call(srv, post("/api/v1/held-carts", json!({ "label": "Alice Cart", "cart": { "lines": [] } }))).await;
    let held_cart_id = ok(&cart, "held cart");
    let journal = call(
        srv,
        post(
            "/api/v1/journals",
            json!({ "memo": "Owner capital", "lines": [
            { "account_id": ledger_account_id, "debit": "500" }, { "account_id": capital, "credit": "500" }] }),
        ),
    )
    .await;
    let journal_id = ok(&journal, "journal");
    let invoice =
        call(srv, post("/api/v1/subscription/invoices", json!({ "plan_code": "starter", "billing_cycle": "monthly" })))
            .await;
    let invoice_id = ok(&invoice, "subscription invoice");

    // Phase 6: a transfer to the damaged location, a count, a register shift, other income.
    let locations = call(srv, get("/api/v1/stock-locations")).await;
    let location_id = locations
        .body
        .as_array()
        .into_iter()
        .flatten()
        .find(|l| l["location_type"] == "damaged")
        .and_then(|l| l["id"].as_str())
        .expect("damaged location")
        .to_owned();
    let transfer = call(
        srv,
        post(
            "/api/v1/stock-transfers",
            json!({ "to_location_id": location_id, "dispatch": true,
            "lines": [{ "variant_id": variant_id, "quantity": "1" }] }),
        ),
    )
    .await;
    let transfer_id = ok(&transfer, "stock transfer");
    let count = call(srv, post("/api/v1/stock-counts", json!({ "note": "Alice count" }))).await;
    let count_id = ok(&count, "stock count");
    let count_item_id = count.body["items"][0]["id"].as_str().expect("count item").to_owned();
    let register = call(srv, post("/api/v1/registers", json!({ "name": "Alice Till" }))).await;
    let register_id = ok(&register, "register");
    let shift = call(srv, post("/api/v1/shifts", json!({ "register_id": register_id, "opening_cash": "500" }))).await;
    let shift_id = ok(&shift, "shift");
    let income_categories = call(srv, get("/api/v1/income-categories")).await;
    let income_category_id = find_id(&income_categories.body, "name", "Interest");
    let income = call(
        srv,
        post(
            "/api/v1/incomes",
            json!({ "category_id": income_category_id, "amount": "25", "source": "Alice Bank Interest" }),
        ),
    )
    .await;
    let income_id = ok(&income, "other income");
    let bank = find_id(&accounts.body, "name", "Bank Account");
    let reconciliation = call(
        srv,
        post(
            "/api/v1/bank-reconciliations",
            json!({ "account_id": bank, "statement_date": "2026-01-31", "statement_balance": "0" }),
        ),
    )
    .await;
    let reconciliation_id = ok(&reconciliation, "bank reconciliation");
    Trade {
        variant_id,
        customer_id,
        supplier_id,
        account_id,
        method_id,
        ledger_account_id,
        payment_id,
        expense_category_id,
        expense_id,
        sale_id,
        sale_return_id,
        held_cart_id,
        purchase_id,
        purchase_return_id,
        adjustment_id,
        journal_id,
        invoice_id,
        location_id,
        transfer_id,
        count_id,
        count_item_id,
        register_id,
        shift_id,
        income_category_id,
        income_id,
        reconciliation_id,
    }
}
