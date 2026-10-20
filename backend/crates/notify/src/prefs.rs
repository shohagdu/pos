//! Per-user channel choices per notification type (NOT-R01).

use pos_core::{AppError, AppResult};
use pos_db::{TenantTx, map_db_error};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::catalog::{self, Audience, CHANNELS, SMS, TYPES};

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct Preference {
    pub notif_type: String,
    /// Chosen channels (`in_app`, `email`, `sms`); empty = off.
    pub channels: Vec<String>,
    pub default_channels: Vec<String>,
    /// Channels this business and user can use (SMS needs the plan feature).
    pub available_channels: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PreferenceInput {
    pub notif_type: String,
    pub channels: Vec<String>,
}

/// The user's channels for a type, or the type's defaults.
pub async fn channels_for(tx: &mut TenantTx, user_id: Uuid, notif_type: &str, defaults: &[&str]) -> AppResult<Vec<String>> {
    let tenant_id = tx.tenant_id();
    let saved: Option<sqlx::types::Json<Vec<String>>> = sqlx::query_scalar(
        "SELECT channels FROM notification_preferences WHERE tenant_id = ? AND user_id = ? AND notif_type = ?",
    )
    .bind(tenant_id)
    .bind(user_id)
    .bind(notif_type)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(saved.map_or_else(|| defaults.iter().map(|c| (*c).to_owned()).collect(), |j| j.0))
}

/// Whether a member with these permissions can receive the type at all.
fn receives(audience: Audience, is_owner: bool, can: &dyn Fn(&str) -> bool) -> bool {
    match audience {
        Audience::Owners => is_owner,
        Audience::Subject => true,
        Audience::Permission(p) if p == "purchases.approve" => is_owner || can(p) || can("expenses.approve"),
        Audience::Permission(p) => is_owner || can(p),
    }
}

fn available(sms_allowed: bool) -> Vec<String> {
    CHANNELS.iter().filter(|c| sms_allowed || **c != SMS).map(|c| (*c).to_owned()).collect()
}

/// The signed-in member's preferences for the types they can receive.
pub async fn list(tx: &mut TenantTx, is_owner: bool, can: &dyn Fn(&str) -> bool, sms_allowed: bool) -> AppResult<Vec<Preference>> {
    let user_id = tx.user_id();
    let mut out = Vec::new();
    for t in TYPES.iter().filter(|t| receives(t.audience, is_owner, can)) {
        out.push(Preference {
            notif_type: t.code.to_owned(),
            channels: channels_for(tx, user_id, t.code, t.default_channels).await?,
            default_channels: t.default_channels.iter().map(|c| (*c).to_owned()).collect(),
            available_channels: available(sms_allowed),
        });
    }
    Ok(out)
}

/// Saves the signed-in member's channel choices.
pub async fn set(
    tx: &mut TenantTx,
    inputs: &[PreferenceInput],
    is_owner: bool,
    can: &dyn Fn(&str) -> bool,
    sms_allowed: bool,
) -> AppResult<Vec<Preference>> {
    let tenant_id = tx.tenant_id();
    let user_id = tx.user_id();
    let allowed = available(sms_allowed);
    for (i, input) in inputs.iter().enumerate() {
        let known = catalog::find(&input.notif_type).filter(|t| receives(t.audience, is_owner, can));
        if known.is_none() {
            return Err(AppError::field(&format!("[{i}].notif_type"), "UNKNOWN_TYPE", "unknown notification type"));
        }
        let mut channels: Vec<String> = Vec::new();
        for c in &input.channels {
            if !allowed.contains(c) {
                return Err(AppError::field(&format!("[{i}].channels"), "INVALID_CHANNEL", "this channel is not available"));
            }
            if !channels.contains(c) {
                channels.push(c.clone());
            }
        }
        sqlx::query(
            "INSERT INTO notification_preferences (tenant_id, user_id, notif_type, channels) VALUES (?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE channels = VALUES(channels)",
        )
        .bind(tenant_id)
        .bind(user_id)
        .bind(&input.notif_type)
        .bind(sqlx::types::Json(&channels))
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }
    list(tx, is_owner, can, sms_allowed).await
}
