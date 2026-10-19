//! Usage counted against plan limits (SUB-R03). The entitlement rules live in
//! `pos-billing`; this module knows which tenant tables each limit counts.

use pos_billing::{Entitlements, FeatureKind};
use pos_core::AppResult;
use pos_db::{TenantTx, map_db_error};
use serde::Serialize;
use uuid::Uuid;

/// Current entitlements, read inside the caller's transaction (exact, uncached).
pub async fn current(tx: &mut TenantTx) -> AppResult<Entitlements> {
    let tenant_id = tx.tenant_id();
    pos_billing::resolve(tx, tenant_id).await
}

/// Fails with `402 PLAN_LIMIT_REACHED` if adding `adding` exceeds the limit.
pub async fn ensure_within(tx: &mut TenantTx, feature: &'static str, used: i64, adding: i64) -> AppResult<()> {
    current(tx).await?.within_limit(feature, used, adding)
}

/// Seats in use for `users.max`: active members plus open invitations.
pub async fn seats_used(tx: &mut TenantTx, exclude_invitation: Option<Uuid>) -> AppResult<i64> {
    let tenant_id = tx.tenant_id();
    let members: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM tenant_memberships WHERE tenant_id = ? AND status = 'active'")
            .bind(tenant_id)
            .fetch_one(&mut **tx)
            .await
            .map_err(map_db_error)?;
    let invites: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM invitations WHERE tenant_id = ? AND status = 'pending' AND expires_at > NOW(6) \
         AND NOT (id <=> ?)",
    )
    .bind(tenant_id)
    .bind(exclude_invitation)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(members + invites)
}

/// Active product variants, counted for `products.max`.
pub async fn products_used(tx: &mut TenantTx) -> AppResult<i64> {
    let tenant_id = tx.tenant_id();
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM product_variants WHERE tenant_id = ? AND status = 'active' AND deleted_at IS NULL",
    )
    .bind(tenant_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)
}

async fn active_branches(tx: &mut TenantTx) -> AppResult<i64> {
    let tenant_id = tx.tenant_id();
    sqlx::query_scalar("SELECT COUNT(*) FROM branches WHERE tenant_id = ? AND status = 'active'")
        .bind(tenant_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct UsageMeter {
    pub feature: String,
    pub name: String,
    /// `null` until the module that owns the data exists (products, registers, …).
    pub used: Option<i64>,
    /// `null` = unlimited.
    pub limit: Option<i64>,
}

/// The usage meter (SUB-R05) for every limit of the plan.
pub async fn usage(tx: &mut TenantTx, ents: &Entitlements) -> AppResult<Vec<UsageMeter>> {
    let mut meters = Vec::new();
    for feature in ents.features.values().filter(|f| f.kind == FeatureKind::Limit) {
        let used = match feature.code.as_str() {
            "users.max" => Some(seats_used(tx, None).await?),
            "branches.max" => Some(active_branches(tx).await?),
            "products.max" => Some(products_used(tx).await?),
            // registers.max, invoices.monthly_max, storage.mb are counted once
            // their modules land (Phases 3–5).
            _ => None,
        };
        meters.push(UsageMeter {
            feature: feature.code.clone(),
            name: feature.name.clone(),
            used,
            limit: ents.limit(&feature.code),
        });
    }
    Ok(meters)
}
