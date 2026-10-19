//! Manual subscription billing for the business owner (SUB-R05, SUB-R08,
//! E5.4): the plan catalogue, subscription invoices (`SUB-2026-000042`,
//! numbered platform-wide) and payment claims (bKash/Nagad/bank transfer with
//! the transaction id) that platform staff verify (`crate::claims`).

use chrono::{DateTime, Months, Utc};
use pos_audit::AuditEntry;
use pos_core::{AppError, AppResult, RequestMeta, id::new_id, money::amount};
use pos_db::{TenantTx, map_db_error};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{MySqlConnection, MySqlPool};
use uuid::Uuid;

pub const CYCLES: &[&str] = &["monthly", "quarterly", "annual"];
pub const CLAIM_METHODS: &[&str] = &["bkash", "nagad", "rocket", "bank_transfer", "cash", "other"];

/// Months bought by one invoice of a billing cycle.
pub fn cycle_months(cycle: &str) -> u32 {
    match cycle {
        "quarterly" => 3,
        "annual" => 12,
        _ => 1,
    }
}

// ------------------------------------------------------------------ catalogue

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct PlanPrice {
    pub billing_cycle: String,
    #[schema(value_type = String)]
    pub amount: Decimal,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct PlanFeature {
    pub code: String,
    pub name: String,
    pub kind: String,
    pub enabled: bool,
    /// `null` = unlimited (for limits).
    pub limit_value: Option<i64>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct PlanOffer {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub prices: Vec<PlanPrice>,
    pub features: Vec<PlanFeature>,
}

/// Public, active, paid plans with their prices and features.
pub async fn offers(conn: &mut MySqlConnection) -> AppResult<Vec<PlanOffer>> {
    let plans: Vec<(Uuid, String, String, Option<String>)> = sqlx::query_as(
        "SELECT id, code, name, description FROM plans WHERE is_public AND NOT is_trial AND status = 'active' \
         ORDER BY sort_order, name",
    )
    .fetch_all(&mut *conn)
    .await
    .map_err(map_db_error)?;
    let mut out = Vec::with_capacity(plans.len());
    for (id, code, name, description) in plans {
        let prices = sqlx::query_as(
            "SELECT billing_cycle, amount FROM plan_prices WHERE plan_id = ? AND is_active AND currency_code = 'BDT' \
             ORDER BY FIELD(billing_cycle, 'monthly', 'quarterly', 'annual')",
        )
        .bind(id)
        .fetch_all(&mut *conn)
        .await
        .map_err(map_db_error)?;
        let features = sqlx::query_as(
            "SELECT f.code, f.name, f.kind, COALESCE(pf.enabled, FALSE) AS enabled, pf.limit_value FROM features f \
             LEFT JOIN plan_features pf ON pf.feature_code = f.code AND pf.plan_id = ? ORDER BY f.kind DESC, f.code",
        )
        .bind(id)
        .fetch_all(&mut *conn)
        .await
        .map_err(map_db_error)?;
        out.push(PlanOffer { id, code, name, description, prices, features });
    }
    Ok(out)
}

/// What the owner should do to pay (edited by the platform owner).
pub async fn payment_instructions(conn: &mut MySqlConnection) -> AppResult<serde_json::Value> {
    let value: Option<serde_json::Value> =
        sqlx::query_scalar("SELECT value FROM platform_settings WHERE setting_key = 'billing.payment_instructions'")
            .fetch_optional(&mut *conn)
            .await
            .map_err(map_db_error)?;
    Ok(value.unwrap_or_else(|| serde_json::json!({})))
}

// ------------------------------------------------------------------- invoices

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct Invoice {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub invoice_no: String,
    pub plan_code: String,
    pub plan_name: String,
    pub billing_cycle: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    #[schema(value_type = String)]
    pub subtotal: Decimal,
    #[schema(value_type = String)]
    pub discount_amount: Decimal,
    #[schema(value_type = String)]
    pub total: Decimal,
    /// `open`, `paid` or `void`.
    pub status: String,
    pub paid_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct PaymentClaim {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub tenant_name: String,
    pub invoice_id: Uuid,
    pub invoice_no: String,
    pub plan_name: String,
    pub billing_cycle: String,
    #[schema(value_type = String)]
    pub invoice_total: Decimal,
    /// `bkash`, `nagad`, `rocket`, `bank_transfer`, `cash` or `other`.
    pub method: String,
    #[schema(value_type = String)]
    pub amount: Decimal,
    pub transaction_ref: String,
    pub payer_account: Option<String>,
    /// `pending`, `approved` or `rejected`.
    pub status: String,
    pub rejection_reason: Option<String>,
    pub submitted_by_name: Option<String>,
    pub reviewed_by_name: Option<String>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

pub(crate) const INVOICE_SELECT: &str = "SELECT i.id, i.tenant_id, i.invoice_no, p.code AS plan_code, p.name AS plan_name, \
     i.billing_cycle, i.period_start, i.period_end, i.subtotal, i.discount_amount, i.total, i.status, i.paid_at, i.created_at \
     FROM subscription_invoices i JOIN plans p ON p.id = i.plan_id";

pub(crate) const CLAIM_SELECT: &str = "SELECT c.id, c.tenant_id, t.name AS tenant_name, c.invoice_id, i.invoice_no, \
     p.name AS plan_name, i.billing_cycle, i.total AS invoice_total, c.method, c.amount, c.transaction_ref, c.payer_account, \
     c.status, c.rejection_reason, (SELECT u.full_name FROM users u WHERE u.id = c.submitted_by) AS submitted_by_name, \
     (SELECT u.full_name FROM users u WHERE u.id = c.reviewed_by) AS reviewed_by_name, c.reviewed_at, c.created_at \
     FROM subscription_payment_claims c JOIN tenants t ON t.id = c.tenant_id \
     JOIN subscription_invoices i ON i.tenant_id = c.tenant_id AND i.id = c.invoice_id JOIN plans p ON p.id = i.plan_id";

/// A business's invoices, newest first (owner and staff views).
pub async fn invoices_of(pool: &MySqlPool, tenant_id: Uuid) -> AppResult<Vec<Invoice>> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{INVOICE_SELECT} WHERE i.tenant_id = ? ORDER BY i.created_at DESC LIMIT 100"
    )))
    .bind(tenant_id)
    .fetch_all(pool)
    .await
    .map_err(map_db_error)
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct BillingOverview {
    pub plans: Vec<PlanOffer>,
    pub invoices: Vec<Invoice>,
    pub claims: Vec<PaymentClaim>,
    /// Free-form text per method (`bkash`, `nagad`, `bank_transfer`, `note`).
    #[schema(value_type = Object)]
    pub payment_instructions: serde_json::Value,
}

pub async fn overview(tx: &mut TenantTx) -> AppResult<BillingOverview> {
    let tenant_id = tx.tenant_id();
    let plans = offers(tx).await?;
    let invoices = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{INVOICE_SELECT} WHERE i.tenant_id = ? ORDER BY i.created_at DESC LIMIT 100"
    )))
    .bind(tenant_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let claims = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{CLAIM_SELECT} WHERE c.tenant_id = ? ORDER BY c.created_at DESC LIMIT 100"
    )))
    .bind(tenant_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let payment_instructions = payment_instructions(tx).await?;
    Ok(BillingOverview { plans, invoices, claims, payment_instructions })
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct InvoiceRequest {
    pub plan_code: String,
    /// `monthly`, `quarterly` or `annual`.
    pub billing_cycle: String,
}

/// The next `SUB-{YYYY}-{SEQ:6}` number (platform-wide row lock).
async fn next_invoice_no(conn: &mut MySqlConnection, now: DateTime<Utc>) -> AppResult<String> {
    let next: i64 =
        sqlx::query_scalar("SELECT next_value FROM platform_sequences WHERE name = 'subscription_invoice' FOR UPDATE")
            .fetch_one(&mut *conn)
            .await
            .map_err(map_db_error)?;
    sqlx::query("UPDATE platform_sequences SET next_value = next_value + 1 WHERE name = 'subscription_invoice'")
        .execute(&mut *conn)
        .await
        .map_err(map_db_error)?;
    Ok(format!("SUB-{}-{next:06}", now.format("%Y")))
}

/// Where a renewal would start: the end of a paid period still running, else now.
pub(crate) fn renewal_start(
    stored_status: &str,
    current_period_end: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> DateTime<Utc> {
    match current_period_end {
        Some(end) if stored_status == "active" && end > now => end,
        _ => now,
    }
}

/// Opens an invoice for a plan and cycle, or returns the open one for the
/// same choice so a double click cannot bill twice.
pub async fn request_invoice(tx: &mut TenantTx, meta: &RequestMeta, input: &InvoiceRequest) -> AppResult<Invoice> {
    if !CYCLES.contains(&input.billing_cycle.as_str()) {
        return Err(AppError::field("billing_cycle", "INVALID_CYCLE", "use monthly, quarterly or annual"));
    }
    let tenant_id = tx.tenant_id();
    let plan: Option<(Uuid, Decimal)> = sqlx::query_as(
        "SELECT p.id, pp.amount FROM plans p JOIN plan_prices pp ON pp.plan_id = p.id AND pp.billing_cycle = ? \
           AND pp.currency_code = 'BDT' AND pp.is_active \
         WHERE p.code = ? AND p.status = 'active' AND NOT p.is_trial",
    )
    .bind(&input.billing_cycle)
    .bind(&input.plan_code)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let (plan_id, price) =
        plan.ok_or_else(|| AppError::field("plan_code", "NOT_FOUND", "choose a plan that is on sale"))?;
    let open: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM subscription_invoices WHERE tenant_id = ? AND plan_id = ? AND billing_cycle = ? AND status = 'open' \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(tenant_id)
    .bind(plan_id)
    .bind(&input.billing_cycle)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if let Some(id) = open {
        return invoice(tx, id).await;
    }
    let (status, period_end): (String, Option<DateTime<Utc>>) =
        sqlx::query_as("SELECT status, current_period_end FROM subscriptions WHERE tenant_id = ?")
            .bind(tenant_id)
            .fetch_one(&mut **tx)
            .await
            .map_err(map_db_error)?;
    let now = Utc::now();
    let start = renewal_start(&status, period_end, now);
    let end = start
        .checked_add_months(Months::new(cycle_months(&input.billing_cycle)))
        .ok_or_else(|| AppError::internal("invoice period overflow"))?;
    let invoice_no = next_invoice_no(tx, now).await?;
    let id = new_id();
    sqlx::query(
        "INSERT INTO subscription_invoices (tenant_id, id, invoice_no, plan_id, billing_cycle, period_start, period_end, \
         subtotal, total) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(id)
    .bind(&invoice_no)
    .bind(plan_id)
    .bind(&input.billing_cycle)
    .bind(start)
    .bind(end)
    .bind(price)
    .bind(price)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("subscription.invoice_created")
            .entity("subscription_invoice", id)
            .change(None, Some(serde_json::json!({ "invoice_no": invoice_no, "plan": input.plan_code, "cycle": input.billing_cycle, "total": price }))),
    )
    .await?;
    invoice(tx, id).await
}

async fn invoice(tx: &mut TenantTx, id: Uuid) -> AppResult<Invoice> {
    let tenant_id = tx.tenant_id();
    sqlx::query_as(sqlx::AssertSqlSafe(format!("{INVOICE_SELECT} WHERE i.tenant_id = ? AND i.id = ?")))
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_db_error)?
        .ok_or(AppError::NotFound)
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ClaimInput {
    /// `bkash`, `nagad`, `rocket`, `bank_transfer`, `cash` or `other`.
    pub method: String,
    #[schema(value_type = String)]
    pub amount: Decimal,
    /// TrxID / deposit reference.
    pub transaction_ref: String,
    /// The number or account the money came from.
    #[serde(default)]
    pub payer_account: Option<String>,
}

/// Reports a payment for an open invoice (ADM-R05 queue). One pending claim
/// per invoice; a transaction id can only be claimed once.
pub async fn submit_claim(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    invoice_id: Uuid,
    input: &ClaimInput,
) -> AppResult<PaymentClaim> {
    if !CLAIM_METHODS.contains(&input.method.as_str()) {
        return Err(AppError::field("method", "INVALID_METHOD", "choose how you paid"));
    }
    let value = amount(input.amount, "amount", true)?;
    let reference = input.transaction_ref.trim();
    if reference.len() < 4 || reference.chars().count() > 60 {
        return Err(AppError::field(
            "transaction_ref",
            "INVALID_REFERENCE",
            "enter the transaction ID (4–60 characters)",
        ));
    }
    let payer = input
        .payer_account
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|p| p.chars().take(60).collect::<String>());
    let tenant_id = tx.tenant_id();
    let user_id = tx.user_id();
    let status: Option<String> =
        sqlx::query_scalar("SELECT status FROM subscription_invoices WHERE tenant_id = ? AND id = ? FOR UPDATE")
            .bind(tenant_id)
            .bind(invoice_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?;
    match status.as_deref() {
        None => return Err(AppError::NotFound),
        Some("open") => {}
        Some(_) => {
            return Err(AppError::Unprocessable {
                code: "INVOICE_NOT_OPEN",
                message: "this invoice is already settled".into(),
            });
        }
    }
    let pending: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM subscription_payment_claims WHERE tenant_id = ? AND invoice_id = ? AND status = 'pending')",
    )
    .bind(tenant_id)
    .bind(invoice_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if pending {
        return Err(AppError::Conflict {
            code: "CLAIM_PENDING",
            message: "a payment for this invoice is already waiting for verification".into(),
        });
    }
    let id = new_id();
    sqlx::query(
        "INSERT INTO subscription_payment_claims (tenant_id, id, invoice_id, method, amount, transaction_ref, payer_account, \
         submitted_by) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(id)
    .bind(invoice_id)
    .bind(&input.method)
    .bind(value)
    .bind(reference)
    .bind(&payer)
    .bind(user_id)
    .execute(&mut **tx)
    .await
    .map_err(|err| match map_db_error(err) {
        AppError::Conflict { code: "DUPLICATE", .. } => AppError::Conflict {
            code: "DUPLICATE_TRANSACTION",
            message: "this transaction ID was already submitted".into(),
        },
        other => other,
    })?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("subscription.payment_claimed").entity("subscription_payment_claim", id).change(
            None,
            Some(serde_json::json!({ "method": input.method, "amount": value, "transaction_ref": reference })),
        ),
    )
    .await?;
    sqlx::query_as(sqlx::AssertSqlSafe(format!("{CLAIM_SELECT} WHERE c.tenant_id = ? AND c.id = ?")))
        .bind(tenant_id)
        .bind(id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;

    #[test]
    fn renewals_start_after_a_running_paid_period() {
        let now = Utc::now();
        let later = now + Duration::days(10);
        assert_eq!(renewal_start("active", Some(later), now), later);
        assert_eq!(renewal_start("trialing", Some(later), now), now, "trial time is not carried over");
        assert_eq!(renewal_start("active", Some(now - Duration::days(1)), now), now);
        assert_eq!(cycle_months("annual"), 12);
    }
}
