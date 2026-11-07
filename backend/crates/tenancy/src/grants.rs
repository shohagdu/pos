//! Shared rules for handing out access: which roles exist, what they grant,
//! and that nobody can grant more than they hold.

use std::collections::BTreeSet;

use pos_auth::{TenantAccess, rbac::OWNER_ROLE};
use pos_core::{AppError, AppResult};
use pos_db::{MySql, TenantTx, map_db_error};
use sqlx::QueryBuilder;
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

/// Appends `(?, ?, …)` for an `IN` list. Lists are bound value by value:
/// `JSON_TABLE` is missing on MariaDB 10.5. Callers handle the empty list.
fn push_in_list(q: &mut QueryBuilder<MySql>, ids: &BTreeSet<Uuid>) {
    q.push(" IN (");
    let mut list = q.separated(", ");
    for id in ids {
        list.push_bind(*id);
    }
    list.push_unseparated(")");
}

/// Adds the permission codes to a role in one multi-row insert.
pub async fn insert_role_permissions<S: AsRef<str>>(tx: &mut TenantTx, role_id: Uuid, codes: &[S]) -> AppResult<()> {
    if codes.is_empty() {
        return Ok(());
    }
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> =
        QueryBuilder::new("INSERT INTO role_permissions (tenant_id, role_id, permission_code) ");
    q.push_values(codes, |mut row, code| {
        row.push_bind(tenant_id).push_bind(role_id).push_bind(code.as_ref().to_owned());
    });
    q.build().execute(&mut **tx).await.map_err(map_db_error)?;
    Ok(())
}

/// Loads the tenant's roles with these ids; any unknown id is a 422.
pub async fn roles(tx: &mut TenantTx, ids: &[Uuid]) -> AppResult<Vec<RoleRef>> {
    let unique: BTreeSet<Uuid> = ids.iter().copied().collect();
    if unique.is_empty() {
        return Ok(Vec::new());
    }
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new("SELECT r.id, r.code, r.name FROM roles r WHERE r.tenant_id = ");
    q.push_bind(tenant_id).push(" AND r.id");
    push_in_list(&mut q, &unique);
    let rows: Vec<(Uuid, String, String)> = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
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
    let mut q: QueryBuilder<MySql> =
        QueryBuilder::new("SELECT COUNT(*) FROM branches b WHERE b.status = 'active' AND b.tenant_id = ");
    q.push_bind(tenant_id).push(" AND b.id");
    push_in_list(&mut q, &unique);
    let found: i64 = q.build_query_scalar().fetch_one(&mut **tx).await.map_err(map_db_error)?;
    if usize::try_from(found).unwrap_or(0) != unique.len() {
        return Err(AppError::field("branch_ids", "UNKNOWN_BRANCH", "a selected branch does not exist"));
    }
    Ok(())
}

/// Permission codes the given roles grant together.
pub async fn role_permissions(tx: &mut TenantTx, role_ids: &[Uuid]) -> AppResult<Vec<String>> {
    let unique: BTreeSet<Uuid> = role_ids.iter().copied().collect();
    if unique.is_empty() {
        return Ok(Vec::new());
    }
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> =
        QueryBuilder::new("SELECT DISTINCT rp.permission_code FROM role_permissions rp WHERE rp.tenant_id = ");
    q.push_bind(tenant_id).push(" AND rp.role_id");
    push_in_list(&mut q, &unique);
    q.build_query_scalar().fetch_all(&mut **tx).await.map_err(map_db_error)
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
