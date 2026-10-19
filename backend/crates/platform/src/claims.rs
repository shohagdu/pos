//! Payment claim verification (ADM-R05). Approving marks the invoice paid and
//! extends the subscription on the invoiced plan from the end of a running
//! paid period (or from now); rejecting needs a reason. Both are audited and
//! written to the subscription history.

use chrono::{Months, Utc};
use pos_audit::AuditEntry;
use pos_core::{
    AppError, AppResult, RequestMeta,
    id::new_id,
    page::{self, OffsetCursor},
};
use pos_db::{PlatformTx, map_db_error};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, MySqlPool, QueryBuilder};
use uuid::Uuid;

use crate::billing::{CLAIM_SELECT, PaymentClaim, cycle_months, renewal_start};

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct ClaimFilter {
    /// `pending` (default), `approved`, `rejected` or `all`.
    pub status: Option<String>,
    pub tenant_id: Option<Uuid>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ClaimPage {
    pub data: Vec<PaymentClaim>,
    pub next_cursor: Option<String>,
    pub pending_count: i64,
}

pub async fn list(pool: &MySqlPool, filter: &ClaimFilter) -> AppResult<ClaimPage> {
    let limit = page::limit(filter.limit)?;
    let start = OffsetCursor::start(filter.cursor.as_deref())?;
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(format!(
        "{CLAIM_SELECT} WHERE TRUE /* tenant-lint: allow platform staff review every business's claims */"
    ));
    match filter.status.as_deref() {
        None | Some("") | Some("pending") => {
            q.push(" AND c.status = 'pending'");
        }
        Some(s @ ("approved" | "rejected")) => {
            q.push(" AND c.status = ").push_bind(s.to_owned());
        }
        Some("all") => {}
        Some(_) => return Err(AppError::field("status", "INVALID_STATUS", "use pending, approved, rejected or all")),
    }
    if let Some(tenant_id) = filter.tenant_id {
        q.push(" AND c.tenant_id = ").push_bind(tenant_id);
    }
    // Oldest pending first: first come, first verified.
    q.push(
        " ORDER BY c.status = 'pending' DESC, IF(c.status = 'pending', c.created_at, NULL), c.created_at DESC LIMIT ",
    )
    .push_bind(limit + 1)
    .push(" OFFSET ")
    .push_bind(start);
    let mut data: Vec<PaymentClaim> = q.build_query_as().fetch_all(pool).await.map_err(map_db_error)?;
    let next_cursor = OffsetCursor::next(start, limit, data.len());
    data.truncate(limit as usize);
    let pending_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM subscription_payment_claims WHERE status = 'pending' /* tenant-lint: allow platform-wide count */",
    )
    .fetch_one(pool)
    .await
    .map_err(map_db_error)?;
    Ok(ClaimPage { data, next_cursor, pending_count })
}

pub async fn of_tenant(pool: &MySqlPool, tenant_id: Uuid) -> AppResult<Vec<PaymentClaim>> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{CLAIM_SELECT} WHERE c.tenant_id = ? ORDER BY c.created_at DESC LIMIT 100"
    )))
    .bind(tenant_id)
    .fetch_all(pool)
    .await
    .map_err(map_db_error)
}

async fn get(tx: &mut PlatformTx, id: Uuid) -> AppResult<PaymentClaim> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{CLAIM_SELECT} WHERE c.id = ? /* tenant-lint: allow platform staff act on any business's claim */"
    )))
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(AppError::NotFound)
}

/// Locks a pending claim; returns its tenant and invoice.
async fn lock_pending(tx: &mut PlatformTx, id: Uuid) -> AppResult<(Uuid, Uuid)> {
    let row: Option<(Uuid, Uuid, String)> = sqlx::query_as(
        "SELECT tenant_id, invoice_id, status FROM subscription_payment_claims WHERE id = ? FOR UPDATE \
         /* tenant-lint: allow platform staff act on any business's claim */",
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let (tenant_id, invoice_id, status) = row.ok_or(AppError::NotFound)?;
    if status != "pending" {
        return Err(AppError::Unprocessable {
            code: "CLAIM_NOT_PENDING",
            message: format!("the claim is already {status}"),
        });
    }
    Ok((tenant_id, invoice_id))
}

/// Approves a claim; returns the tenant whose entitlements changed.
pub async fn approve(
    tx: &mut PlatformTx,
    staff_id: Uuid,
    meta: &RequestMeta,
    id: Uuid,
) -> AppResult<(Uuid, PaymentClaim)> {
    let (tenant_id, invoice_id) = lock_pending(tx, id).await?;
    let invoice: (Uuid, String, Decimal, String, String) = sqlx::query_as(
        "SELECT plan_id, billing_cycle, total, status, invoice_no FROM subscription_invoices WHERE tenant_id = ? AND id = ? FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(invoice_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let (plan_id, cycle, total, invoice_status, invoice_no) = invoice;
    if invoice_status != "open" {
        return Err(AppError::Unprocessable {
            code: "INVOICE_NOT_OPEN",
            message: "the invoice is already settled".into(),
        });
    }
    let sub: (Uuid, String, Uuid, Option<chrono::DateTime<Utc>>) = sqlx::query_as(
        "SELECT id, status, plan_id, current_period_end FROM subscriptions WHERE tenant_id = ? FOR UPDATE",
    )
    .bind(tenant_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let (subscription_id, from_status, from_plan, period_end) = sub;
    let now = Utc::now();
    let start = renewal_start(&from_status, period_end, now);
    let end = start
        .checked_add_months(Months::new(cycle_months(&cycle)))
        .ok_or_else(|| AppError::internal("subscription period overflow"))?;

    sqlx::query(
        "UPDATE subscriptions SET plan_id = ?, status = 'active', billing_cycle = ?, price_amount = ?, \
         current_period_start = ?, current_period_end = ?, grace_ends_at = NULL, read_only_ends_at = NULL, \
         cancel_at_period_end = FALSE, cancelled_at = NULL, pending_plan_id = NULL, version = version + 1 \
         WHERE tenant_id = ? AND id = ?",
    )
    .bind(plan_id)
    .bind(&cycle)
    .bind(total)
    .bind(start)
    .bind(end)
    .bind(tenant_id)
    .bind(subscription_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    sqlx::query(
        "UPDATE subscription_invoices SET status = 'paid', paid_at = ?, period_start = ?, period_end = ? \
         WHERE tenant_id = ? AND id = ?",
    )
    .bind(now)
    .bind(start)
    .bind(end)
    .bind(tenant_id)
    .bind(invoice_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    sqlx::query(
        "UPDATE subscription_payment_claims SET status = 'approved', reviewed_by = ?, reviewed_at = ? WHERE tenant_id = ? AND id = ?",
    )
    .bind(staff_id)
    .bind(now)
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    sqlx::query(
        "INSERT INTO subscription_events (tenant_id, id, subscription_id, event_type, from_status, to_status, from_plan_id, \
         to_plan_id, actor_type, actor_id, metadata) VALUES (?, ?, ?, 'payment_verified', ?, 'active', ?, ?, 'platform_staff', ?, ?)",
    )
    .bind(tenant_id)
    .bind(new_id())
    .bind(subscription_id)
    .bind(&from_status)
    .bind(from_plan)
    .bind(plan_id)
    .bind(staff_id)
    .bind(serde_json::json!({ "invoice_no": invoice_no, "period_end": end }))
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    pos_audit::record_platform(
        tx,
        Some(tenant_id),
        Some(staff_id),
        meta,
        AuditEntry::new("subscription.payment_approved").entity("subscription_payment_claim", id).change(
            Some(serde_json::json!({ "status": from_status })),
            Some(serde_json::json!({ "status": "active", "invoice_no": invoice_no, "period_end": end })),
        ),
    )
    .await?;
    Ok((tenant_id, get(tx, id).await?))
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RejectInput {
    pub reason: String,
}

pub async fn reject(
    tx: &mut PlatformTx,
    staff_id: Uuid,
    meta: &RequestMeta,
    id: Uuid,
    reason: &str,
) -> AppResult<PaymentClaim> {
    let reason = reason.trim();
    if reason.is_empty() || reason.chars().count() > 500 {
        return Err(AppError::field("reason", "REQUIRED", "tell the business why (up to 500 characters)"));
    }
    let (tenant_id, _) = lock_pending(tx, id).await?;
    sqlx::query(
        "UPDATE subscription_payment_claims SET status = 'rejected', rejection_reason = ?, reviewed_by = ?, \
         reviewed_at = UTC_TIMESTAMP(6) WHERE tenant_id = ? AND id = ?",
    )
    .bind(reason)
    .bind(staff_id)
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    pos_audit::record_platform(
        tx,
        Some(tenant_id),
        Some(staff_id),
        meta,
        AuditEntry::new("subscription.payment_rejected").entity("subscription_payment_claim", id).reason(reason),
    )
    .await?;
    get(tx, id).await
}
