//! Payment methods (how a customer pays: Cash, Card, bKash …). Each method
//! names the account its money lands in by default.

use pos_audit::AuditEntry;
use pos_core::{AppError, AppResult, RequestMeta};
use pos_db::{TenantTx, map_db_error};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct PaymentMethod {
    pub id: Uuid,
    pub name: String,
    /// `cash`, `card`, `mobile_wallet`, `bank_transfer`, `cheque`, `customer_credit` or `other`.
    pub method_type: String,
    pub default_account_id: Option<Uuid>,
    pub default_account_name: Option<String>,
    /// A transaction id / reference must be entered (MFS, bank transfer).
    pub requires_reference: bool,
    pub sort_order: i32,
    pub is_active: bool,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PaymentMethodPatch {
    #[serde(default)]
    pub default_account_id: Option<Uuid>,
    #[serde(default)]
    pub requires_reference: Option<bool>,
    #[serde(default)]
    pub is_active: Option<bool>,
    #[serde(default)]
    pub sort_order: Option<i32>,
}

const SELECT: &str = "SELECT m.id, m.name, m.method_type, m.default_account_id, a.name AS default_account_name, \
     m.requires_reference, m.sort_order, m.is_active FROM payment_methods m \
     LEFT JOIN payment_accounts a ON a.tenant_id = m.tenant_id AND a.id = m.default_account_id \
     WHERE m.tenant_id = ?";

pub async fn list(tx: &mut TenantTx, include_inactive: bool) -> AppResult<Vec<PaymentMethod>> {
    let tenant_id = tx.tenant_id();
    sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} AND (? OR m.is_active) ORDER BY m.sort_order, m.name")))
        .bind(tenant_id)
        .bind(include_inactive)
        .fetch_all(&mut **tx)
        .await
        .map_err(map_db_error)
}

pub async fn get(tx: &mut TenantTx, id: Uuid) -> AppResult<PaymentMethod> {
    let tenant_id = tx.tenant_id();
    sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} AND m.id = ?")))
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_db_error)?
        .ok_or(AppError::NotFound)
}

/// An active method, for `field` in errors.
pub async fn require_active(tx: &mut TenantTx, id: Uuid, field: &str) -> AppResult<PaymentMethod> {
    match get(tx, id).await {
        Ok(m) if m.is_active => Ok(m),
        Ok(_) | Err(AppError::NotFound) => Err(AppError::field(field, "NOT_FOUND", "choose an active payment method")),
        Err(e) => Err(e),
    }
}

/// The method to record a payment from/to `account_id` with when the user
/// only picked an account (expenses, transfers): the first active method
/// pointing at it, else Cash, else any active method.
pub async fn for_account(tx: &mut TenantTx, account_id: Uuid) -> AppResult<PaymentMethod> {
    let tenant_id = tx.tenant_id();
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{SELECT} AND m.is_active ORDER BY m.default_account_id <=> ? DESC, m.method_type = 'cash' DESC, m.sort_order LIMIT 1"
    )))
    .bind(tenant_id)
    .bind(account_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or_else(|| AppError::Unprocessable { code: "NO_PAYMENT_METHOD", message: "activate a payment method first".into() })
}

pub async fn update(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    id: Uuid,
    patch: PaymentMethodPatch,
) -> AppResult<PaymentMethod> {
    let before = get(tx, id).await?;
    if let Some(account_id) = patch.default_account_id {
        crate::accounts::require_active(tx, account_id, "default_account_id").await?;
    }
    let tenant_id = tx.tenant_id();
    sqlx::query(
        "UPDATE payment_methods SET default_account_id = COALESCE(?, default_account_id), \
         requires_reference = COALESCE(?, requires_reference), is_active = COALESCE(?, is_active), \
         sort_order = COALESCE(?, sort_order) WHERE tenant_id = ? AND id = ?",
    )
    .bind(patch.default_account_id)
    .bind(patch.requires_reference)
    .bind(patch.is_active)
    .bind(patch.sort_order)
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let after = get(tx, id).await?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("payment_method.updated").entity("payment_method", id).change(
            Some(serde_json::to_value(&before).map_err(AppError::internal)?),
            Some(serde_json::to_value(&after).map_err(AppError::internal)?),
        ),
    )
    .await?;
    Ok(after)
}
