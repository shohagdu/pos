//! Held carts (POS-R07): saved per branch so a manager can resume them, at
//! most 20 per branch, expiring after 24 hours. They do not reserve stock;
//! prices are re-read when the cart is resumed.

use chrono::{DateTime, Duration, Utc};
use pos_core::{AppError, AppResult, id::new_id};
use pos_db::{TenantTx, map_db_error};
use pos_tenancy::context;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

pub const MAX_PER_BRANCH: i64 = 20;
const MAX_CART_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct HoldInput {
    /// "Table 3", a customer name …
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub customer_id: Option<Uuid>,
    /// The cart as the POS keeps it (lines, discounts); opaque to the server.
    #[schema(value_type = Object)]
    pub cart: Value,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct HeldCart {
    pub id: Uuid,
    pub label: Option<String>,
    pub customer_id: Option<Uuid>,
    pub customer_name: Option<String>,
    #[schema(value_type = Object)]
    pub cart: Value,
    pub held_by: Uuid,
    pub held_by_name: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

const SELECT: &str = "SELECT h.id, h.label, h.customer_id, c.name AS customer_name, h.cart, h.held_by, \
     (SELECT u.full_name FROM users u WHERE u.id = h.held_by) AS held_by_name, h.expires_at, h.created_at \
     FROM held_carts h LEFT JOIN customers c ON c.tenant_id = h.tenant_id AND c.id = h.customer_id \
     WHERE h.tenant_id = ?";

async fn purge_expired(tx: &mut TenantTx) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    sqlx::query("DELETE FROM held_carts WHERE tenant_id = ? AND expires_at < UTC_TIMESTAMP(6)")
        .bind(tenant_id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(())
}

pub async fn list(tx: &mut TenantTx) -> AppResult<Vec<HeldCart>> {
    let ctx = context::load(tx).await?;
    let tenant_id = tx.tenant_id();
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{SELECT} AND h.branch_id = ? AND h.expires_at >= UTC_TIMESTAMP(6) ORDER BY h.created_at"
    )))
    .bind(tenant_id)
    .bind(ctx.branch_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)
}

pub async fn hold(tx: &mut TenantTx, input: HoldInput) -> AppResult<HeldCart> {
    if !input.cart.is_object() || input.cart.to_string().len() > MAX_CART_BYTES {
        return Err(AppError::field("cart", "INVALID_CART", "the cart must be an object of at most 64 KB"));
    }
    let label = input
        .label
        .as_deref()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| l.chars().take(100).collect::<String>());
    purge_expired(tx).await?;
    let ctx = context::load(tx).await?;
    let tenant_id = tx.tenant_id();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM held_carts WHERE tenant_id = ? AND branch_id = ?")
        .bind(tenant_id)
        .bind(ctx.branch_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
    if count >= MAX_PER_BRANCH {
        return Err(AppError::Unprocessable {
            code: "TOO_MANY_HELD_CARTS",
            message: format!("at most {MAX_PER_BRANCH} carts can be on hold; resume or discard one"),
        });
    }
    if let Some(customer_id) = input.customer_id {
        pos_parties::customers::get(tx, customer_id)
            .await
            .map_err(|_| AppError::field("customer_id", "NOT_FOUND", "choose a customer"))?;
    }
    let id = new_id();
    let user_id = tx.user_id();
    sqlx::query(
        "INSERT INTO held_carts (tenant_id, id, branch_id, label, customer_id, cart, held_by, expires_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(id)
    .bind(ctx.branch_id)
    .bind(&label)
    .bind(input.customer_id)
    .bind(&input.cart)
    .bind(user_id)
    .bind(ctx.now + Duration::hours(24))
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} AND h.id = ?")))
        .bind(tenant_id)
        .bind(id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)
}

/// Resuming and discarding both remove the cart.
pub async fn remove(tx: &mut TenantTx, id: Uuid) -> AppResult<HeldCart> {
    let tenant_id = tx.tenant_id();
    let cart: HeldCart = sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} AND h.id = ? FOR UPDATE")))
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_db_error)?
        .ok_or(AppError::NotFound)?;
    sqlx::query("DELETE FROM held_carts WHERE tenant_id = ? AND id = ?")
        .bind(tenant_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(cart)
}
