//! Tenant administration (ADM-R01…R03): list with search and filters, counts
//! by subscription state, detail with subscription history, usage and owner,
//! and suspend / reactivate with a mandatory reason (audited).

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use pos_audit::AuditEntry;
use pos_billing::status::{self, Dates, SubscriptionStatus};
use pos_core::{
    AppError, AppResult, RequestMeta,
    page::{self, OffsetCursor},
};
use pos_db::{PlatformTx, map_db_error};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, MySqlPool, QueryBuilder};
use uuid::Uuid;

use crate::billing::{Invoice, PaymentClaim};

/// The reserved tenant that owns platform files (DB design §13.1).
pub const PLATFORM_SLUG: &str = "platform";

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct TenantFilter {
    /// Name, slug, phone, email or owner phone containing this text.
    pub q: Option<String>,
    /// Effective subscription state: `trialing`, `active`, `grace`, `read_only`, `suspended`, `cancelled`.
    pub subscription_status: Option<String>,
    /// Business status: `active` or `suspended`.
    pub status: Option<String>,
    pub plan_code: Option<String>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct TenantRow {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub business_type: String,
    /// `active`, `suspended` …
    pub status: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub district: Option<String>,
    pub plan_code: Option<String>,
    pub plan_name: Option<String>,
    pub stored_subscription_status: Option<String>,
    pub owner_name: Option<String>,
    pub owner_phone: Option<String>,
    pub trial_ends_at: Option<DateTime<Utc>>,
    pub current_period_end: Option<DateTime<Utc>>,
    #[sqlx(skip)]
    pub subscription_status: Option<SubscriptionStatus>,
    pub created_at: DateTime<Utc>,
    #[sqlx(default)]
    pub grace_ends_at: Option<DateTime<Utc>>,
    #[sqlx(default)]
    pub read_only_ends_at: Option<DateTime<Utc>>,
}

impl TenantRow {
    fn compute_status(&mut self, now: DateTime<Utc>) {
        let stored = self.stored_subscription_status.as_deref().and_then(SubscriptionStatus::parse);
        self.subscription_status = stored.map(|s| {
            let dates = Dates {
                trial_ends_at: self.trial_ends_at,
                current_period_end: self.current_period_end,
                grace_ends_at: self.grace_ends_at,
                read_only_ends_at: self.read_only_ends_at,
            };
            status::effective(s, &dates, now)
        });
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct TenantPage {
    pub data: Vec<TenantRow>,
    pub next_cursor: Option<String>,
    /// Businesses per effective subscription state (all businesses, unfiltered).
    pub counts: BTreeMap<String, i64>,
    pub total: i64,
}

const SELECT: &str = "SELECT t.id, t.name, t.slug, t.business_type, t.status, t.phone, t.email, t.district, \
     p.code AS plan_code, p.name AS plan_name, s.status AS stored_subscription_status, \
     (SELECT u.full_name FROM tenant_memberships m JOIN users u ON u.id = m.user_id \
        WHERE m.tenant_id = t.id AND m.is_owner LIMIT 1) AS owner_name, \
     (SELECT u.phone FROM tenant_memberships m JOIN users u ON u.id = m.user_id \
        WHERE m.tenant_id = t.id AND m.is_owner LIMIT 1) AS owner_phone, \
     s.trial_ends_at, s.current_period_end, t.created_at, s.grace_ends_at, s.read_only_ends_at \
     FROM tenants t LEFT JOIN subscriptions s ON s.tenant_id = t.id LEFT JOIN plans p ON p.id = s.plan_id \
     WHERE t.slug <> 'platform' /* tenant-lint: allow platform staff list every business (ADM-R01) */";

/// Every business with its effective subscription state (the state filter is
/// applied in Rust because the state is derived from dates).
pub async fn list(pool: &MySqlPool, filter: &TenantFilter) -> AppResult<TenantPage> {
    let limit = page::limit(filter.limit)?;
    let start = OffsetCursor::start(filter.cursor.as_deref())?;
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(SELECT);
    if let Some(text) = filter.q.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        let escaped = text.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
        let pattern = format!("%{escaped}%");
        q.push(" AND (t.name LIKE ")
            .push_bind(pattern.clone())
            .push(" OR t.slug LIKE ")
            .push_bind(pattern.clone())
            .push(" OR t.phone LIKE ")
            .push_bind(pattern.clone())
            .push(" OR t.email LIKE ")
            .push_bind(pattern.clone())
            .push(
                " OR EXISTS (SELECT 1 FROM tenant_memberships m JOIN users u ON u.id = m.user_id \
                   WHERE m.tenant_id = t.id AND m.is_owner AND u.phone LIKE ",
            )
            .push_bind(pattern)
            .push("))");
    }
    if let Some(status) = filter.status.as_deref().filter(|s| !s.is_empty()) {
        q.push(" AND t.status = ").push_bind(status.to_owned());
    }
    if let Some(plan) = filter.plan_code.as_deref().filter(|s| !s.is_empty()) {
        q.push(" AND p.code = ").push_bind(plan.to_owned());
    }
    q.push(" ORDER BY t.created_at DESC, t.id DESC");
    let mut rows: Vec<TenantRow> = q.build_query_as().fetch_all(pool).await.map_err(map_db_error)?;
    let now = Utc::now();
    for row in &mut rows {
        row.compute_status(now);
    }

    let all: Vec<TenantRow> = {
        let mut q: QueryBuilder<MySql> = QueryBuilder::new(SELECT);
        let mut all: Vec<TenantRow> = q.build_query_as().fetch_all(pool).await.map_err(map_db_error)?;
        for row in &mut all {
            row.compute_status(now);
        }
        all
    };
    let mut counts: BTreeMap<String, i64> = BTreeMap::new();
    for row in &all {
        let key = row.subscription_status.map_or("none".to_owned(), |s| status_key(s).to_owned());
        *counts.entry(key).or_default() += 1;
        if row.status == "suspended" {
            *counts.entry("business_suspended".to_owned()).or_default() += 1;
        }
    }

    if let Some(wanted) = filter.subscription_status.as_deref().filter(|s| !s.is_empty()) {
        rows.retain(|r| r.subscription_status.is_some_and(|s| status_key(s) == wanted));
    }
    let total = i64::try_from(rows.len()).unwrap_or(i64::MAX);
    let end = start as usize + limit as usize;
    let next_cursor = (rows.len() > end).then(|| OffsetCursor(start + limit).encode());
    let data: Vec<TenantRow> = rows.into_iter().skip(start as usize).take(limit as usize).collect();
    Ok(TenantPage { data, next_cursor, counts, total })
}

pub fn status_key(s: SubscriptionStatus) -> &'static str {
    match s {
        SubscriptionStatus::Trialing => "trialing",
        SubscriptionStatus::Active => "active",
        SubscriptionStatus::Grace => "grace",
        SubscriptionStatus::ReadOnly => "read_only",
        SubscriptionStatus::Suspended => "suspended",
        SubscriptionStatus::Cancelled => "cancelled",
        SubscriptionStatus::Terminated => "terminated",
    }
}

// --------------------------------------------------------------------- detail

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct SubscriptionEvent {
    pub id: Uuid,
    pub event_type: String,
    pub from_status: Option<String>,
    pub to_status: Option<String>,
    pub from_plan: Option<String>,
    pub to_plan: Option<String>,
    pub actor_type: String,
    pub actor_name: Option<String>,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct Usage {
    pub code: String,
    pub used: i64,
    /// `null` = unlimited.
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct Member {
    pub user_id: Uuid,
    pub full_name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub is_owner: bool,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct TenantDetail {
    #[serde(flatten)]
    pub tenant: TenantRow,
    pub suspended_reason: Option<String>,
    pub billing_cycle: Option<String>,
    #[schema(value_type = Option<String>)]
    pub price_amount: Option<Decimal>,
    pub days_left: Option<i64>,
    pub last_activity_at: Option<DateTime<Utc>>,
    pub members: Vec<Member>,
    pub usage: Vec<Usage>,
    pub events: Vec<SubscriptionEvent>,
    pub invoices: Vec<Invoice>,
    pub claims: Vec<PaymentClaim>,
    pub overrides: Vec<crate::overrides::Override>,
    pub entitlements: Vec<pos_billing::Feature>,
}

pub async fn get(pool: &MySqlPool, id: Uuid) -> AppResult<TenantDetail> {
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(SELECT);
    q.push(" AND t.id = ").push_bind(id);
    let mut tenant: TenantRow =
        q.build_query_as().fetch_optional(pool).await.map_err(map_db_error)?.ok_or(AppError::NotFound)?;
    let now = Utc::now();
    tenant.compute_status(now);
    let (suspended_reason, billing_cycle, price_amount): (Option<String>, Option<String>, Option<Decimal>) =
        sqlx::query_as(
            "SELECT t.suspended_reason, s.billing_cycle, s.price_amount FROM tenants t \
         LEFT JOIN subscriptions s ON s.tenant_id = t.id WHERE t.id = ?",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_db_error)?;
    let days_left = tenant.subscription_status.and_then(|s| {
        status::days_left(
            s,
            &Dates {
                trial_ends_at: tenant.trial_ends_at,
                current_period_end: tenant.current_period_end,
                grace_ends_at: tenant.grace_ends_at,
                read_only_ends_at: tenant.read_only_ends_at,
            },
            now,
        )
    });
    let last_activity_at: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT MAX(created_at) FROM audit_logs WHERE tenant_id = ?")
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(map_db_error)?;
    let members = sqlx::query_as(
        "SELECT u.id AS user_id, COALESCE(m.display_name, u.full_name) AS full_name, u.phone, u.email, m.is_owner, m.status \
         FROM tenant_memberships m JOIN users u ON u.id = m.user_id WHERE m.tenant_id = ? \
         ORDER BY m.is_owner DESC, full_name",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(map_db_error)?;
    let events = sqlx::query_as(
        "SELECT e.id, e.event_type, e.from_status, e.to_status, fp.code AS from_plan, tp.code AS to_plan, e.actor_type, \
         (SELECT u.full_name FROM users u WHERE u.id = e.actor_id) AS actor_name, e.reason, e.created_at \
         FROM subscription_events e LEFT JOIN plans fp ON fp.id = e.from_plan_id LEFT JOIN plans tp ON tp.id = e.to_plan_id \
         WHERE e.tenant_id = ? ORDER BY e.created_at DESC LIMIT 100",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(map_db_error)?;

    let mut conn = pool.acquire().await.map_err(map_db_error)?;
    let ents = pos_billing::resolve(&mut conn, id).await?;
    drop(conn);
    let usage = usage(pool, id, &ents).await?;
    let invoices = crate::billing::invoices_of(pool, id).await?;
    let claims = crate::claims::of_tenant(pool, id).await?;
    let overrides = crate::overrides::list(pool, id).await?;
    Ok(TenantDetail {
        tenant,
        suspended_reason,
        billing_cycle,
        price_amount,
        days_left,
        last_activity_at,
        members,
        usage,
        events,
        invoices,
        claims,
        overrides,
        entitlements: ents.features.values().cloned().collect(),
    })
}

async fn usage(pool: &MySqlPool, id: Uuid, ents: &pos_billing::Entitlements) -> AppResult<Vec<Usage>> {
    let counts: (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT \
           (SELECT COUNT(*) FROM tenant_memberships WHERE tenant_id = ? AND status = 'active'), \
           (SELECT COUNT(*) FROM branches WHERE tenant_id = ? AND status = 'active'), \
           (SELECT COUNT(*) FROM product_variants v JOIN products p ON p.tenant_id = v.tenant_id AND p.id = v.product_id \
              WHERE v.tenant_id = ? AND p.status = 'active' AND p.deleted_at IS NULL AND v.deleted_at IS NULL), \
           (SELECT COUNT(*) FROM sales WHERE tenant_id = ? AND sale_date >= DATE_FORMAT(UTC_TIMESTAMP(), '%Y-%m-01'))",
    )
    .bind(id)
    .bind(id)
    .bind(id)
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(map_db_error)?;
    Ok([
        ("users.max", counts.0),
        ("branches.max", counts.1),
        ("products.max", counts.2),
        ("invoices.monthly_max", counts.3),
    ]
    .into_iter()
    .map(|(code, used)| Usage {
        code: code.to_owned(),
        used,
        limit: ents.features.get(code).and_then(|f| if f.enabled { f.limit } else { Some(0) }),
    })
    .collect())
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct StatusChange {
    /// Mandatory (ADM-R03).
    pub reason: String,
}

/// Suspends (`suspend = true`) or reactivates a business. A suspended
/// business cannot be opened by its members (`403 TENANT_SUSPENDED`).
pub async fn set_suspended(
    tx: &mut PlatformTx,
    staff_id: Uuid,
    meta: &RequestMeta,
    id: Uuid,
    suspend: bool,
    reason: &str,
) -> AppResult<()> {
    let reason = reason.trim();
    if reason.is_empty() || reason.chars().count() > 500 {
        return Err(AppError::field("reason", "REQUIRED", "enter a reason of up to 500 characters"));
    }
    let current: Option<(String, String)> = sqlx::query_as("SELECT status, slug FROM tenants WHERE id = ? FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_db_error)?;
    let (status, slug) = current.ok_or(AppError::NotFound)?;
    if slug == PLATFORM_SLUG {
        return Err(AppError::NotFound);
    }
    let (from, to) = if suspend { ("active", "suspended") } else { ("suspended", "active") };
    if status != from {
        return Err(AppError::Unprocessable {
            code: "INVALID_STATUS",
            message: format!("the business is {status}, not {from}"),
        });
    }
    sqlx::query("UPDATE tenants SET status = ?, suspended_reason = ? WHERE id = ?")
        .bind(to)
        .bind(suspend.then_some(reason))
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    let action = if suspend { "tenant.suspended" } else { "tenant.reactivated" };
    pos_audit::record_platform(
        tx,
        Some(id),
        Some(staff_id),
        meta,
        AuditEntry::new(action)
            .entity("tenant", id)
            .reason(reason)
            .change(Some(serde_json::json!({ "status": status })), Some(serde_json::json!({ "status": to }))),
    )
    .await?;
    Ok(())
}
