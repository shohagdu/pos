//! Shared rules for handing out access: which roles exist, what they grant,
//! and that nobody can grant more than they hold.

use std::collections::BTreeSet;

use pos_auth::{TenantAccess, rbac::OWNER_ROLE};
use pos_core::{AppError, AppResult};
use pos_db::{TenantTx, map_db_error};
use uuid::Uuid;

/// Granted to the owner only, never through invitations or custom roles (§10.2).
pub const OWNER_ONLY_PERMISSIONS: &[&str] = &["subscription.manage"];
pub const ADMIN_ROLE: &str = "admin";

#[derive(Debug, Clone)]
pub struct RoleRef {
    pub id: Uuid,
    pub code: String,
    pub name: String,
}

fn ids_json(ids: &[Uuid]) -> AppResult<String> {
    serde_json::to_string(&ids.iter().map(Uuid::to_string).collect::<Vec<_>>()).map_err(AppError::internal)
}

/// Loads the tenant's roles with these ids; any unknown id is a 422.
pub async fn roles(tx: &mut TenantTx, ids: &[Uuid]) -> AppResult<Vec<RoleRef>> {
    let unique: BTreeSet<Uuid> = ids.iter().copied().collect();
    let tenant_id = tx.tenant_id();
    let rows: Vec<(Uuid, String, String)> = sqlx::query_as(
        "SELECT r.id, r.code, r.name FROM roles r \
         JOIN JSON_TABLE(?, '$[*]' COLUMNS (id CHAR(36) PATH '$')) AS j ON r.id = UUID_TO_BIN(j.id) \
         WHERE r.tenant_id = ?",
    )
    .bind(ids_json(&unique.iter().copied().collect::<Vec<_>>())?)
    .bind(tenant_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if rows.len() != unique.len() {
        return Err(AppError::field("role_ids", "UNKNOWN_ROLE", "a selected role does not exist"));
    }
    Ok(rows.into_iter().map(|(id, code, name)| RoleRef { id, code, name }).collect())
}

/// Checks that the branches exist and are active in this tenant.
pub async fn ensure_branches(tx: &mut TenantTx, ids: &[Uuid]) -> AppResult<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let unique: BTreeSet<Uuid> = ids.iter().copied().collect();
    let tenant_id = tx.tenant_id();
    let found: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM branches b \
         JOIN JSON_TABLE(?, '$[*]' COLUMNS (id CHAR(36) PATH '$')) AS j ON b.id = UUID_TO_BIN(j.id) \
         WHERE b.tenant_id = ? AND b.status = 'active'",
    )
    .bind(ids_json(&unique.iter().copied().collect::<Vec<_>>())?)
    .bind(tenant_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if usize::try_from(found).unwrap_or(0) != unique.len() {
        return Err(AppError::field("branch_ids", "UNKNOWN_BRANCH", "a selected branch does not exist"));
    }
    Ok(())
}

/// Permission codes the given roles grant together.
pub async fn role_permissions(tx: &mut TenantTx, role_ids: &[Uuid]) -> AppResult<Vec<String>> {
    let tenant_id = tx.tenant_id();
    sqlx::query_scalar(
        "SELECT DISTINCT rp.permission_code FROM role_permissions rp \
         JOIN JSON_TABLE(?, '$[*]' COLUMNS (id CHAR(36) PATH '$')) AS j ON rp.role_id = UUID_TO_BIN(j.id) \
         WHERE rp.tenant_id = ?",
    )
    .bind(ids_json(role_ids)?)
    .bind(tenant_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)
}

/// No privilege escalation: a non-owner can only grant permissions they hold.
pub fn ensure_grantable(actor: &TenantAccess, permissions: &[String]) -> AppResult<()> {
    if actor.is_owner {
        return Ok(());
    }
    match permissions.iter().find(|p| !actor.can(p)) {
        Some(p) => Err(AppError::Denied {
            code: "PRIVILEGE_ESCALATION",
            message: format!("you cannot grant '{p}' because you do not have it"),
        }),
        None => Ok(()),
    }
}

/// The owner role is only held by owners; it is never handed out here.
pub fn reject_owner_role(roles: &[RoleRef]) -> AppResult<()> {
    if roles.iter().any(|r| r.code == OWNER_ROLE) {
        return Err(AppError::field("role_ids", "OWNER_ROLE_RESERVED", "the owner role cannot be assigned"));
    }
    Ok(())
}
