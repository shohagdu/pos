//! Onboarding wizard progress and the setup checklist (Requirement Analysis
//! §7.2, ONB-R03), kept in `tenant_settings.extra.onboarding`.

use pos_audit::AuditEntry;
use pos_core::{AppError, AppResult, RequestMeta};
use pos_db::{TenantTx, map_db_error};
use serde::Serialize;
use serde_json::{Value, json};

/// Steps after account creation, in wizard order. `products` and `staff` are
/// checklist items that can be skipped. Payment methods join with the money module.
pub const STEPS: &[&str] = &["business", "regional", "branch", "invoice", "products", "staff"];

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct StepStatus {
    pub key: &'static str,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct OnboardingStatus {
    pub steps: Vec<StepStatus>,
    pub dismissed: bool,
    /// Every step done; the checklist disappears from the dashboard.
    pub complete: bool,
}

/// `extra` for a new tenant: the business profile is step 2 of provisioning.
pub fn initial_extra() -> Value {
    json!({ "onboarding": { "completed": ["business"], "dismissed": false } })
}

async fn load_extra(tx: &mut TenantTx, lock: bool) -> AppResult<Value> {
    let sql = if lock {
        "SELECT extra FROM tenant_settings WHERE tenant_id = ? FOR UPDATE"
    } else {
        "SELECT extra FROM tenant_settings WHERE tenant_id = ?"
    };
    let tenant_id = tx.tenant_id();
    sqlx::query_scalar(sql).bind(tenant_id).fetch_one(&mut **tx).await.map_err(map_db_error)
}

fn status_from(extra: &Value) -> OnboardingStatus {
    let state = &extra["onboarding"];
    let completed: Vec<&str> =
        state["completed"].as_array().map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
    let steps: Vec<StepStatus> = STEPS.iter().map(|key| StepStatus { key, done: completed.contains(key) }).collect();
    let complete = steps.iter().all(|s| s.done);
    OnboardingStatus { steps, dismissed: state["dismissed"].as_bool().unwrap_or(false), complete }
}

pub async fn status(tx: &mut TenantTx) -> AppResult<OnboardingStatus> {
    Ok(status_from(&load_extra(tx, false).await?))
}

async fn save(tx: &mut TenantTx, extra: &Value) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    // Progress is not a settings change: `version` stays, so open forms keep their If-Match.
    sqlx::query("UPDATE tenant_settings SET extra = ? WHERE tenant_id = ?")
        .bind(extra)
        .bind(tenant_id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(())
}

/// Marks a step done (or skipped). Idempotent.
pub async fn complete_step(tx: &mut TenantTx, meta: &RequestMeta, step: &str) -> AppResult<OnboardingStatus> {
    let Some(&key) = STEPS.iter().find(|s| **s == step) else {
        return Err(AppError::NotFound);
    };
    let mut extra = load_extra(tx, true).await?;
    let before = status_from(&extra);
    if before.steps.iter().any(|s| s.key == key && s.done) {
        return Ok(before);
    }
    let completed = &mut extra["onboarding"]["completed"];
    match completed.as_array_mut() {
        Some(list) => list.push(json!(key)),
        None => *completed = json!([key]),
    }
    save(tx, &extra).await?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("onboarding.step_completed").change(None, Some(json!({ "step": key }))),
    )
    .await?;
    Ok(status_from(&extra))
}

/// Hides the checklist from the dashboard (ONB-R03).
pub async fn dismiss(tx: &mut TenantTx) -> AppResult<OnboardingStatus> {
    let mut extra = load_extra(tx, true).await?;
    extra["onboarding"]["dismissed"] = json!(true);
    save(tx, &extra).await?;
    Ok(status_from(&extra))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_tenants_have_the_profile_step_done() {
        let status = status_from(&initial_extra());
        assert!(status.steps.iter().any(|s| s.key == "business" && s.done));
        assert!(!status.complete && !status.dismissed);
    }

    #[test]
    fn tolerates_missing_state() {
        let status = status_from(&json!({}));
        assert_eq!(status.steps.len(), STEPS.len());
        assert!(status.steps.iter().all(|s| !s.done));
    }
}
