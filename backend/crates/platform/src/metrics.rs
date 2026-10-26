//! Platform dashboard (ADM-R08): businesses by subscription state, MRR/ARR,
//! sign-ups, paid revenue and the payment-claim queue.
//!
//! MRR = Σ monthly-normalised price of subscriptions that are `active` or in
//! `grace` (quarterly ÷ 3, annual ÷ 12), excluding trials. ARR = MRR × 12.

use std::collections::BTreeMap;

use pos_core::{AppResult, money::round2};
use pos_db::map_db_error;
use rust_decimal::Decimal;
use serde::Serialize;
use sqlx::MySqlPool;

use crate::tenants::{self, TenantFilter};

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct PlatformMetrics {
    pub businesses: i64,
    /// By effective subscription state.
    pub by_status: BTreeMap<String, i64>,
    #[schema(value_type = String)]
    pub mrr: Decimal,
    #[schema(value_type = String)]
    pub arr: Decimal,
    pub signups_30d: i64,
    pub signups_7d: i64,
    #[schema(value_type = String)]
    pub revenue_30d: Decimal,
    pub pending_claims: i64,
    /// Businesses per plan code.
    pub by_plan: BTreeMap<String, i64>,
}

/// The monthly value of a subscription price (a one-time licence recurs never).
pub fn monthly(price: Decimal, cycle: &str) -> Decimal {
    match cycle {
        "lifetime" => Decimal::ZERO,
        "quarterly" => price / Decimal::from(3),
        "annual" => price / Decimal::from(12),
        _ => price,
    }
}

pub async fn platform(pool: &MySqlPool) -> AppResult<PlatformMetrics> {
    let page = tenants::list(pool, &TenantFilter { limit: Some(1), ..TenantFilter::default() }).await?;
    let active_ids: Vec<(String, String, Decimal, String)> = sqlx::query_as(
        "SELECT s.status, s.billing_cycle, s.price_amount, p.code FROM subscriptions s JOIN plans p ON p.id = s.plan_id \
         JOIN tenants t ON t.id = s.tenant_id WHERE t.slug <> 'platform' /* tenant-lint: allow platform metrics */",
    )
    .fetch_all(pool)
    .await
    .map_err(map_db_error)?;
    let mut by_plan: BTreeMap<String, i64> = BTreeMap::new();
    let mut mrr = Decimal::ZERO;
    for (status, cycle, price, plan) in &active_ids {
        *by_plan.entry(plan.clone()).or_default() += 1;
        if matches!(status.as_str(), "active" | "grace") {
            mrr += monthly(*price, cycle);
        }
    }
    let mrr = round2(mrr);
    let (signups_30d, signups_7d): (i64, i64) = sqlx::query_as(
        "SELECT CAST(COALESCE(SUM(created_at >= UTC_TIMESTAMP(6) - INTERVAL 30 DAY), 0) AS SIGNED), \
                CAST(COALESCE(SUM(created_at >= UTC_TIMESTAMP(6) - INTERVAL 7 DAY), 0) AS SIGNED) \
         FROM tenants WHERE slug <> 'platform'",
    )
    .fetch_one(pool)
    .await
    .map_err(map_db_error)?;
    let revenue_30d: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(total), 0) FROM subscription_invoices WHERE status = 'paid' \
         AND paid_at >= UTC_TIMESTAMP(6) - INTERVAL 30 DAY /* tenant-lint: allow platform metrics */",
    )
    .fetch_one(pool)
    .await
    .map_err(map_db_error)?;
    let pending_claims: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM subscription_payment_claims WHERE status = 'pending' /* tenant-lint: allow platform metrics */",
    )
    .fetch_one(pool)
    .await
    .map_err(map_db_error)?;
    Ok(PlatformMetrics {
        businesses: page.counts.iter().filter(|(k, _)| k.as_str() != "business_suspended").map(|(_, v)| v).sum(),
        by_status: page.counts,
        mrr,
        arr: mrr * Decimal::from(12),
        signups_30d,
        signups_7d,
        revenue_30d,
        pending_claims,
        by_plan,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monthly_normalisation() {
        let d = |s: &str| s.parse::<Decimal>().expect("decimal");
        assert_eq!(monthly(d("990"), "monthly"), d("990"));
        assert_eq!(round2(monthly(d("2821.50"), "quarterly")), d("940.50"));
        assert_eq!(round2(monthly(d("10098"), "annual")), d("841.50"));
    }
}
