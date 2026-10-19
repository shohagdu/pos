//! Roles and the permission catalogue (E1.5, Requirement Analysis §10).
//! System roles are read-only templates; custom roles are created from scratch
//! or by cloning one. Changing a role bumps `perms_version` of every member
//! who holds it, so their next request needs a fresh token (AUTH-R07).

use std::collections::BTreeMap;

use pos_audit::AuditEntry;
use pos_auth::{TenantAccess, rbac::PERMISSIONS, token::random_bytes};
use pos_core::{AppError, AppResult, RequestMeta, id::new_id};
use pos_db::{TenantTx, map_db_error};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::{
    grants::{OWNER_ONLY_PERMISSIONS, ensure_grantable},
    provisioning::clean,
};

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct Permission {
    pub code: String,
    pub module: String,
    pub description: String,
    pub is_sensitive: bool,
}

/// The catalogue in the order of §10.1 (grouped by module).
pub async fn catalogue(pool: &MySqlPool) -> AppResult<Vec<Permission>> {
    let mut rows: Vec<Permission> = sqlx::query_as("SELECT code, module, description, is_sensitive FROM permissions")
        .fetch_all(pool)
        .await
        .map_err(map_db_error)?;
    let order = |code: &str| PERMISSIONS.iter().position(|p| *p == code).unwrap_or(usize::MAX);
    rows.sort_by_key(|p| order(&p.code));
    Ok(rows)
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct Role {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub is_system: bool,
    /// Highest discount the POS allows this role, in percent (decimal string).
    #[schema(value_type = Option<String>)]
    pub max_discount_pct: Option<Decimal>,
    pub permissions: Vec<String>,
    /// Active members holding the role.
    pub member_count: i64,
    pub version: i32,
}

#[derive(sqlx::FromRow)]
struct RoleRow {
    id: Uuid,
    code: String,
    name: String,
    description: Option<String>,
    is_system: bool,
    max_discount_pct: Option<Decimal>,
    member_count: i64,
    version: i32,
}

const SELECT_ROLES: &str = "SELECT r.id, r.code, r.name, r.description, r.is_system, r.max_discount_pct, r.version, \
       (SELECT COUNT(DISTINCT mr.membership_id) FROM membership_roles mr \
          JOIN tenant_memberships m ON m.tenant_id = mr.tenant_id AND m.id = mr.membership_id \
         WHERE mr.tenant_id = r.tenant_id AND mr.role_id = r.id AND m.status = 'active') AS member_count \
     FROM roles r WHERE r.tenant_id = ?";

pub async fn list(tx: &mut TenantTx) -> AppResult<Vec<Role>> {
    let tenant_id = tx.tenant_id();
    let rows: Vec<RoleRow> =
        sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT_ROLES} ORDER BY r.is_system DESC, r.name")))
            .bind(tenant_id)
            .fetch_all(&mut **tx)
            .await
            .map_err(map_db_error)?;
    let grants: Vec<(Uuid, String)> =
        sqlx::query_as("SELECT role_id, permission_code FROM role_permissions WHERE tenant_id = ?")
            .bind(tenant_id)
            .fetch_all(&mut **tx)
            .await
            .map_err(map_db_error)?;
    let mut by_role: BTreeMap<Uuid, Vec<String>> = BTreeMap::new();
    for (role_id, code) in grants {
        by_role.entry(role_id).or_default().push(code);
    }
    let order = |code: &String| PERMISSIONS.iter().position(|p| p == code).unwrap_or(usize::MAX);
    Ok(rows
        .into_iter()
        .map(|r| {
            let mut permissions = by_role.remove(&r.id).unwrap_or_default();
            permissions.sort_by_key(order);
            Role {
                id: r.id,
                code: r.code,
                name: r.name,
                description: r.description,
                is_system: r.is_system,
                // DECIMAL(7,4) comes back as 15.5000; show 15.5.
                max_discount_pct: r.max_discount_pct.map(|d| d.normalize()),
                permissions,
                member_count: r.member_count,
                version: r.version,
            }
        })
        .collect())
}

pub async fn get(tx: &mut TenantTx, role_id: Uuid) -> AppResult<Role> {
    list(tx).await?.into_iter().find(|r| r.id == role_id).ok_or(AppError::NotFound)
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RoleInput {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub permissions: Vec<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub max_discount_pct: Option<Decimal>,
}

struct ValidRole {
    name: String,
    description: Option<String>,
    permissions: Vec<String>,
    max_discount_pct: Option<Decimal>,
}

async fn validate(
    tx: &mut TenantTx,
    actor: &TenantAccess,
    input: RoleInput,
    except: Option<Uuid>,
) -> AppResult<ValidRole> {
    let name = crate::provisioning::validate_name(&input.name, "name", 100)?;
    let description = clean(input.description, "description", 255)?;
    let mut permissions = input.permissions;
    permissions.sort();
    permissions.dedup();
    if permissions.is_empty() {
        return Err(AppError::field("permissions", "NO_PERMISSIONS", "choose at least one permission"));
    }
    if let Some(p) = permissions.iter().find(|p| !PERMISSIONS.contains(&p.as_str())) {
        return Err(AppError::field("permissions", "UNKNOWN_PERMISSION", format!("unknown permission '{p}'")));
    }
    if let Some(p) = permissions.iter().find(|p| OWNER_ONLY_PERMISSIONS.contains(&p.as_str())) {
        return Err(AppError::field(
            "permissions",
            "OWNER_ONLY_PERMISSION",
            format!("'{p}' is reserved for the owner"),
        ));
    }
    ensure_grantable(actor, &permissions)?;
    if let Some(pct) = input.max_discount_pct
        && (pct < Decimal::ZERO || pct > Decimal::ONE_HUNDRED)
    {
        return Err(AppError::field("max_discount_pct", "INVALID_PERCENT", "use a value from 0 to 100"));
    }

    let tenant_id = tx.tenant_id();
    let taken: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM roles WHERE tenant_id = ? AND name = ? AND NOT (id <=> ?))")
            .bind(tenant_id)
            .bind(&name)
            .bind(except)
            .fetch_one(&mut **tx)
            .await
            .map_err(map_db_error)?;
    if taken {
        return Err(AppError::field("name", "ROLE_NAME_TAKEN", "a role with this name already exists"));
    }
    Ok(ValidRole { name, description, permissions, max_discount_pct: input.max_discount_pct })
}

async fn write_permissions(tx: &mut TenantTx, role_id: Uuid, permissions: &[String]) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    sqlx::query("DELETE FROM role_permissions WHERE tenant_id = ? AND role_id = ?")
        .bind(tenant_id)
        .bind(role_id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    sqlx::query(
        "INSERT INTO role_permissions (tenant_id, role_id, permission_code) \
         SELECT ?, ?, j.code FROM JSON_TABLE(?, '$[*]' COLUMNS (code VARCHAR(60) PATH '$')) AS j",
    )
    .bind(tenant_id)
    .bind(role_id)
    .bind(serde_json::to_string(permissions).map_err(AppError::internal)?)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}

/// Custom role codes are opaque (`c_` + random); names are what people see.
fn custom_code() -> AppResult<String> {
    const ALPHABET: &[u8] = b"abcdefghijkmnpqrstuvwxyz23456789";
    let tail: String =
        random_bytes::<10>()?.iter().map(|b| ALPHABET[usize::from(*b) % ALPHABET.len()] as char).collect();
    Ok(format!("c_{tail}"))
}

pub async fn create(tx: &mut TenantTx, actor: &TenantAccess, meta: &RequestMeta, input: RoleInput) -> AppResult<Role> {
    let role = validate(tx, actor, input, None).await?;
    let tenant_id = tx.tenant_id();
    let role_id = new_id();
    sqlx::query(
        "INSERT INTO roles (tenant_id, id, code, name, description, is_system, max_discount_pct) \
         VALUES (?, ?, ?, ?, ?, FALSE, ?)",
    )
    .bind(tenant_id)
    .bind(role_id)
    .bind(custom_code()?)
    .bind(&role.name)
    .bind(&role.description)
    .bind(role.max_discount_pct)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    write_permissions(tx, role_id, &role.permissions).await?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("role.created")
            .entity("role", role_id)
            .change(None, Some(json!({ "name": role.name, "permissions": role.permissions }))),
    )
    .await?;
    get(tx, role_id).await
}

async fn editable(tx: &mut TenantTx, role_id: Uuid, expected_version: Option<i32>) -> AppResult<Role> {
    let tenant_id = tx.tenant_id();
    // Lock the row so concurrent edits serialise on the version check.
    let locked: Option<i32> = sqlx::query_scalar("SELECT version FROM roles WHERE tenant_id = ? AND id = ? FOR UPDATE")
        .bind(tenant_id)
        .bind(role_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_db_error)?;
    let version = locked.ok_or(AppError::NotFound)?;
    let current = get(tx, role_id).await?;
    if current.is_system {
        return Err(AppError::Denied {
            code: "SYSTEM_ROLE_READ_ONLY",
            message: "system roles cannot be changed; duplicate it to make a custom role".into(),
        });
    }
    if expected_version.is_some_and(|v| v != version) {
        return Err(AppError::PreconditionFailed {
            code: "VERSION_MISMATCH",
            message: "the role was changed by someone else; reload and try again".into(),
        });
    }
    Ok(current)
}

pub async fn update(
    tx: &mut TenantTx,
    actor: &TenantAccess,
    meta: &RequestMeta,
    role_id: Uuid,
    expected_version: i32,
    input: RoleInput,
) -> AppResult<Role> {
    let current = editable(tx, role_id, Some(expected_version)).await?;
    // Editing a role you could not have created is escalation too (removing
    // permissions you lack from it would be harmless, but keep one rule).
    ensure_grantable(actor, &current.permissions)?;
    let role = validate(tx, actor, input, Some(role_id)).await?;
    let tenant_id = tx.tenant_id();
    sqlx::query(
        "UPDATE roles SET name = ?, description = ?, max_discount_pct = ?, version = version + 1 \
         WHERE tenant_id = ? AND id = ?",
    )
    .bind(&role.name)
    .bind(&role.description)
    .bind(role.max_discount_pct)
    .bind(tenant_id)
    .bind(role_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    write_permissions(tx, role_id, &role.permissions).await?;
    bump_holders(tx, role_id).await?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("role.updated").entity("role", role_id).change(
            Some(json!({ "name": current.name, "permissions": current.permissions })),
            Some(json!({ "name": role.name, "permissions": role.permissions })),
        ),
    )
    .await?;
    get(tx, role_id).await
}

/// Members holding the role must re-read their permissions (AUTH-R07).
async fn bump_holders(tx: &mut TenantTx, role_id: Uuid) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    sqlx::query(
        "UPDATE tenant_memberships SET perms_version = perms_version + 1 \
         WHERE tenant_id = ? AND id IN (SELECT membership_id FROM membership_roles WHERE tenant_id = ? AND role_id = ?)",
    )
    .bind(tenant_id)
    .bind(tenant_id)
    .bind(role_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}

/// Deletes an unused custom role.
pub async fn delete(tx: &mut TenantTx, meta: &RequestMeta, role_id: Uuid) -> AppResult<()> {
    let current = editable(tx, role_id, None).await?;
    let tenant_id = tx.tenant_id();
    let assigned: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM membership_roles WHERE tenant_id = ? AND role_id = ?) \
             OR EXISTS (SELECT 1 FROM invitations WHERE tenant_id = ? AND status = 'pending' \
                        AND JSON_CONTAINS(role_ids, JSON_QUOTE(?)))",
    )
    .bind(tenant_id)
    .bind(role_id)
    .bind(tenant_id)
    .bind(role_id.to_string())
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if assigned {
        return Err(AppError::Conflict {
            code: "ROLE_IN_USE",
            message: "the role is assigned to members or open invitations".into(),
        });
    }
    sqlx::query("DELETE FROM roles WHERE tenant_id = ? AND id = ?")
        .bind(tenant_id)
        .bind(role_id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("role.deleted")
            .entity("role", role_id)
            .change(Some(json!({ "name": current.name, "permissions": current.permissions })), None),
    )
    .await?;
    Ok(())
}
