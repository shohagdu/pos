//! Entitlement resolution and checks (SUB-R03, DB design §9.5): plan features,
//! overridden by unexpired tenant overrides.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use pos_core::{AppError, AppResult};
use pos_db::map_db_error;
use serde::Serialize;
use sqlx::MySqlConnection;
use uuid::Uuid;

use crate::status::{self, Dates, SubscriptionStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FeatureKind {
    Flag,
    Limit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct Feature {
    pub code: String,
    pub name: String,
    pub kind: FeatureKind,
    pub enabled: bool,
    /// For limits: the maximum; `null` = unlimited.
    pub limit: Option<i64>,
    /// An override (Super Admin) replaces the plan's value.
    pub overridden: bool,
}

/// Plan and state for the app shell and the plan page.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SubscriptionSummary {
    /// Effective state now (see [`status::effective`]).
    pub status: SubscriptionStatus,
    /// State as stored; differs from `status` until the daily job catches up.
    pub stored_status: SubscriptionStatus,
    pub plan_code: String,
    pub plan_name: String,
    pub trial_ends_at: Option<DateTime<Utc>>,
    pub current_period_end: Option<DateTime<Utc>>,
    /// Whole days left in the current phase (trial, period, grace, read-only).
    pub days_left: Option<i64>,
}

/// A tenant's plan, subscription dates and features at resolution time.
#[derive(Debug, Clone)]
pub struct Entitlements {
    pub tenant_id: Uuid,
    pub plan_code: String,
    pub plan_name: String,
    pub stored_status: SubscriptionStatus,
    pub dates: Dates,
    pub features: BTreeMap<String, Feature>,
}

impl Entitlements {
    /// The state that applies now (derived from dates, see [`status::effective`]).
    pub fn status(&self) -> SubscriptionStatus {
        self.status_at(Utc::now())
    }

    pub fn status_at(&self, now: DateTime<Utc>) -> SubscriptionStatus {
        status::effective(self.stored_status, &self.dates, now)
    }

    pub fn days_left(&self) -> Option<i64> {
        let now = Utc::now();
        status::days_left(self.status_at(now), &self.dates, now)
    }

    pub fn summary(&self) -> SubscriptionSummary {
        let now = Utc::now();
        let status = self.status_at(now);
        SubscriptionSummary {
            status,
            stored_status: self.stored_status,
            plan_code: self.plan_code.clone(),
            plan_name: self.plan_name.clone(),
            trial_ends_at: self.dates.trial_ends_at,
            current_period_end: self.dates.current_period_end,
            days_left: status::days_left(status, &self.dates, now),
        }
    }

    /// Flags that are on, for hiding features in the UI.
    pub fn enabled_flags(&self) -> Vec<String> {
        self.features.values().filter(|f| f.kind == FeatureKind::Flag && f.enabled).map(|f| f.code.clone()).collect()
    }

    /// `can(feature)`: a flag that is on. Unknown features are off.
    pub fn can(&self, feature: &str) -> bool {
        self.features.get(feature).is_some_and(|f| f.kind == FeatureKind::Flag && f.enabled)
    }

    pub fn require(&self, feature: &'static str) -> AppResult<()> {
        if self.can(feature) { Ok(()) } else { Err(AppError::FeatureNotAvailable { feature }) }
    }

    /// The maximum for a limit; `None` = unlimited. A disabled or unknown limit is 0.
    pub fn limit(&self, key: &str) -> Option<i64> {
        match self.features.get(key) {
            Some(f) if f.enabled => f.limit,
            _ => Some(0),
        }
    }

    /// `within_limit(key, used + delta)`: fails with `402 PLAN_LIMIT_REACHED`.
    /// Shrinking (`delta <= 0`) is always allowed, so tenants over a limit can
    /// reduce usage (SUB-R04).
    pub fn within_limit(&self, key: &'static str, used: i64, delta: i64) -> AppResult<()> {
        match self.limit(key) {
            Some(max) if delta > 0 && used + delta > max => Err(AppError::PlanLimitReached { limit: key, max }),
            _ => Ok(()),
        }
    }

    /// New transactions and changes need a subscription in good standing (§9.5).
    pub fn ensure_writable(&self) -> AppResult<()> {
        match self.status() {
            s if s.allows_writes() => Ok(()),
            SubscriptionStatus::ReadOnly => Err(AppError::PaymentRequired {
                code: "SUBSCRIPTION_READ_ONLY",
                message: "the subscription has ended; renew to make changes".into(),
            }),
            _ => Err(suspended()),
        }
    }

    pub fn ensure_readable(&self) -> AppResult<()> {
        if self.status().allows_reads() { Ok(()) } else { Err(suspended()) }
    }
}

fn suspended() -> AppError {
    AppError::PaymentRequired {
        code: "SUBSCRIPTION_SUSPENDED",
        message: "this business is suspended; the owner can renew the subscription".into(),
    }
}

#[derive(sqlx::FromRow)]
struct SubscriptionRow {
    status: String,
    plan_code: String,
    plan_name: String,
    trial_ends_at: Option<DateTime<Utc>>,
    current_period_end: Option<DateTime<Utc>>,
    grace_ends_at: Option<DateTime<Utc>>,
    read_only_ends_at: Option<DateTime<Utc>>,
}

#[derive(sqlx::FromRow)]
struct FeatureRow {
    code: String,
    name: String,
    kind: String,
    enabled: Option<bool>,
    limit_value: Option<i64>,
    overridden: bool,
}

/// Reads the current entitlements. Write paths call this inside their
/// transaction for an exact answer; request checks use the cache.
pub async fn resolve(conn: &mut MySqlConnection, tenant_id: Uuid) -> AppResult<Entitlements> {
    let sub: Option<SubscriptionRow> = sqlx::query_as(
        "SELECT s.status, p.code AS plan_code, p.name AS plan_name, s.trial_ends_at, s.current_period_end, \
                s.grace_ends_at, s.read_only_ends_at \
         FROM subscriptions s JOIN plans p ON p.id = s.plan_id WHERE s.tenant_id = ?",
    )
    .bind(tenant_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(map_db_error)?;

    // Every provisioned tenant has a subscription (SUB-R01); without one, nothing is allowed.
    let Some(sub) = sub else {
        tracing::warn!(%tenant_id, "tenant without a subscription");
        return Ok(Entitlements {
            tenant_id,
            plan_code: String::new(),
            plan_name: String::new(),
            stored_status: SubscriptionStatus::Suspended,
            dates: Dates::default(),
            features: BTreeMap::new(),
        });
    };

    // DB design §9.5 "Entitlement resolution". An override's limit replaces the
    // plan's when it sets a limit, or when it leaves `enabled` unset (then NULL
    // means unlimited); an override that only switches `enabled` keeps the plan limit.
    let rows: Vec<FeatureRow> = sqlx::query_as(
        "SELECT f.code, f.name, f.kind, COALESCE(o.enabled, pf.enabled) AS enabled, \
                IF(o.id IS NOT NULL AND (o.limit_value IS NOT NULL OR o.enabled IS NULL), o.limit_value, pf.limit_value) \
                  AS limit_value, \
                o.id IS NOT NULL AS overridden \
         FROM features f \
         JOIN subscriptions s ON s.tenant_id = ? \
         LEFT JOIN plan_features pf ON pf.plan_id = s.plan_id AND pf.feature_code = f.code \
         LEFT JOIN tenant_feature_overrides o \
                ON o.tenant_id = s.tenant_id AND o.feature_code = f.code \
               AND (o.expires_at IS NULL OR o.expires_at > UTC_TIMESTAMP(6)) \
         ORDER BY f.code",
    )
    .bind(tenant_id)
    .fetch_all(&mut *conn)
    .await
    .map_err(map_db_error)?;

    let features = rows
        .into_iter()
        .map(|r| {
            let kind = if r.kind == "limit" { FeatureKind::Limit } else { FeatureKind::Flag };
            let feature = Feature {
                enabled: r.enabled.unwrap_or(false),
                limit: if kind == FeatureKind::Limit { r.limit_value } else { None },
                code: r.code.clone(),
                name: r.name,
                kind,
                overridden: r.overridden,
            };
            (r.code, feature)
        })
        .collect();

    Ok(Entitlements {
        tenant_id,
        stored_status: SubscriptionStatus::parse(&sub.status).unwrap_or(SubscriptionStatus::Suspended),
        plan_code: sub.plan_code,
        plan_name: sub.plan_name,
        dates: Dates {
            trial_ends_at: sub.trial_ends_at,
            current_period_end: sub.current_period_end,
            grace_ends_at: sub.grace_ends_at,
            read_only_ends_at: sub.read_only_ends_at,
        },
        features,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ents(features: &[(&str, FeatureKind, bool, Option<i64>)]) -> Entitlements {
        Entitlements {
            tenant_id: Uuid::nil(),
            plan_code: "starter".into(),
            plan_name: "Starter".into(),
            stored_status: SubscriptionStatus::Active,
            dates: Dates::default(),
            features: features
                .iter()
                .map(|(code, kind, enabled, limit)| {
                    let f = Feature {
                        code: (*code).into(),
                        name: (*code).into(),
                        kind: *kind,
                        enabled: *enabled,
                        limit: *limit,
                        overridden: false,
                    };
                    ((*code).to_owned(), f)
                })
                .collect(),
        }
    }

    #[test]
    fn flags_and_limits() {
        let e = ents(&[
            ("pos.core", FeatureKind::Flag, true, None),
            ("api.access", FeatureKind::Flag, false, None),
            ("users.max", FeatureKind::Limit, true, Some(3)),
            ("products.max", FeatureKind::Limit, true, None),
            ("registers.max", FeatureKind::Limit, false, Some(5)),
        ]);
        assert!(e.can("pos.core") && !e.can("api.access") && !e.can("nope") && !e.can("users.max"));
        assert_eq!(e.require("api.access").map_err(|e| e.code()).err(), Some("FEATURE_NOT_AVAILABLE"));
        assert!(e.within_limit("users.max", 2, 1).is_ok());
        let err = e.within_limit("users.max", 3, 1).err();
        assert_eq!(err.as_ref().and_then(AppError::plan_limit), Some(("users.max", 3)));
        assert!(e.within_limit("users.max", 5, -1).is_ok(), "reducing usage is always allowed");
        assert!(e.within_limit("products.max", 1_000_000, 1).is_ok(), "unlimited");
        assert_eq!(e.limit("registers.max"), Some(0), "disabled limit");
        assert_eq!(e.limit("unknown.max"), Some(0));
    }

    #[test]
    fn subscription_state_gates_writes() {
        let mut e = ents(&[]);
        assert!(e.ensure_writable().is_ok());
        e.stored_status = SubscriptionStatus::ReadOnly;
        assert_eq!(e.ensure_writable().map_err(|e| e.code()).err(), Some("SUBSCRIPTION_READ_ONLY"));
        assert!(e.ensure_readable().is_ok());
        e.stored_status = SubscriptionStatus::Suspended;
        assert_eq!(e.ensure_readable().map_err(|e| e.code()).err(), Some("SUBSCRIPTION_SUSPENDED"));
    }
}
