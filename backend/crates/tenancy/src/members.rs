//! Members of a business and their roles (US-AUTH-04/05, Requirement
//! Analysis §9.7, §10.2 rules).
//!
//! Protections: nobody changes their own access; the owner cannot be changed
//! here (ownership transfer is a separate flow); only the owner may change or
//! remove an admin. Every change bumps `perms_version`, and suspending or
//! removing a member signs out their sessions in this business (AUTH-R08).

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use pos_audit::AuditEntry;
use pos_auth::{TenantAccess, session};
use pos_core::{AppError, AppResult, RequestMeta, id::new_id};
use pos_db::{TenantTx, map_db_error};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{
    entitlements,
    grants::{self, ADMIN_ROLE},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RoleAssignment {
    pub role_id: Uuid,
    /// `null` = all branches (R-TEN-04).
    #[serde(default)]
    pub branch_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct AssignedRole {
    pub role_id: Uuid,
    pub code: String,
    pub name: String,
    pub branch_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct Member {
    pub id: Uuid,
    pub user_id: Uuid,
    pub full_name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub status: String,
    pub is_owner: bool,
    pub roles: Vec<AssignedRole>,
    pub joined_at: Option<DateTime<Utc>>,
    pub last_login_at: Option<DateTime<Utc>>,
}

#[derive(sqlx::FromRow)]
struct MemberRow {
    id: Uuid,
    user_id: Uuid,
    full_name: String,
    phone: Option<String>,
    email: Option<String>,
    status: String,
    is_owner: bool,
    joined_at: Option<DateTime<Utc>>,
    last_login_at: Option<DateTime<Utc>>,
}

async fn load(tx: &mut TenantTx, only: Option<Uuid>, lock: bool) -> AppResult<Vec<Member>> {
    let tenant_id = tx.tenant_id();
    let sql = if lock {
        "SELECT m.id, m.user_id, COALESCE(m.display_name, u.full_name) AS full_name, u.phone, u.email, m.status, \
         m.is_owner, m.joined_at, u.last_login_at FROM tenant_memberships m JOIN users u ON u.id = m.user_id \
         WHERE m.tenant_id = ? AND m.status <> 'removed' AND (? IS NULL OR m.id = ?) FOR UPDATE OF m"
    } else {
        "SELECT m.id, m.user_id, COALESCE(m.display_name, u.full_name) AS full_name, u.phone, u.email, m.status, \
         m.is_owner, m.joined_at, u.last_login_at FROM tenant_memberships m JOIN users u ON u.id = m.user_id \
         WHERE m.tenant_id = ? AND m.status <> 'removed' AND (? IS NULL OR m.id = ?) \
         ORDER BY m.is_owner DESC, full_name"
    };
    let rows: Vec<MemberRow> =
        sqlx::query_as(sql).bind(tenant_id).bind(only).bind(only).fetch_all(&mut **tx).await.map_err(map_db_error)?;

    let assigned: Vec<(Uuid, Uuid, String, String, Option<Uuid>)> = sqlx::query_as(
        "SELECT mr.membership_id, r.id, r.code, r.name, mr.branch_id FROM membership_roles mr \
         JOIN roles r ON r.tenant_id = mr.tenant_id AND r.id = mr.role_id \
         WHERE mr.tenant_id = ? AND (? IS NULL OR mr.membership_id = ?) ORDER BY r.is_system DESC, r.name",
    )
    .bind(tenant_id)
    .bind(only)
    .bind(only)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let mut by_member: BTreeMap<Uuid, Vec<AssignedRole>> = BTreeMap::new();
    for (membership_id, role_id, code, name, branch_id) in assigned {
        by_member.entry(membership_id).or_default().push(AssignedRole { role_id, code, name, branch_id });
    }

    Ok(rows
        .into_iter()
        .map(|r| Member {
            roles: by_member.remove(&r.id).unwrap_or_default(),
            id: r.id,
            user_id: r.user_id,
            full_name: r.full_name,
            phone: r.phone,
            email: r.email,
            status: r.status,
            is_owner: r.is_owner,
            joined_at: r.joined_at,
            last_login_at: r.last_login_at,
        })
        .collect())
}

pub async fn list(tx: &mut TenantTx) -> AppResult<Vec<Member>> {
    load(tx, None, false).await
}

pub async fn get(tx: &mut TenantTx, membership_id: Uuid) -> AppResult<Member> {
    load(tx, Some(membership_id), false).await?.pop().ok_or(AppError::NotFound)
}

/// Loads and locks the member an actor wants to change, applying the protections.
async fn target(tx: &mut TenantTx, actor: &TenantAccess, membership_id: Uuid) -> AppResult<Member> {
    let member = load(tx, Some(membership_id), true).await?.pop().ok_or(AppError::NotFound)?;
    if member.id == actor.membership_id {
        return Err(AppError::Denied {
            code: "CANNOT_CHANGE_SELF",
            message: "you cannot change your own access".into(),
        });
    }
    if member.is_owner {
        return Err(AppError::Denied {
            code: "OWNER_PROTECTED",
            message: "the owner's access cannot be changed".into(),
        });
    }
    if !actor.is_owner && member.roles.iter().any(|r| r.code == ADMIN_ROLE) {
        return Err(AppError::Denied { code: "ADMIN_PROTECTED", message: "only the owner can change an admin".into() });
    }
    Ok(member)
}

async fn bump(tx: &mut TenantTx, membership_id: Uuid, status: Option<&str>) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    sqlx::query(
        "UPDATE tenant_memberships SET perms_version = perms_version + 1, status = COALESCE(?, status) \
         WHERE tenant_id = ? AND id = ?",
    )
    .bind(status)
    .bind(tenant_id)
    .bind(membership_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}

pub(crate) async fn insert_assignments(
    tx: &mut TenantTx,
    membership_id: Uuid,
    assignments: &[RoleAssignment],
    created_by: Uuid,
) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    for a in assignments {
        sqlx::query(
            "INSERT INTO membership_roles (tenant_id, id, membership_id, role_id, branch_id, created_by) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(tenant_id)
        .bind(new_id())
        .bind(membership_id)
        .bind(a.role_id)
        .bind(a.branch_id)
        .bind(created_by)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }
    Ok(())
}

/// Validates assignments: roles and branches exist, the owner role is not
/// handed out, and the actor holds every permission being granted.
pub(crate) async fn validate_assignments(
    tx: &mut TenantTx,
    actor: &TenantAccess,
    assignments: &[RoleAssignment],
) -> AppResult<Vec<RoleAssignment>> {
    let mut unique = assignments.to_vec();
    unique.sort_by_key(|a| (a.role_id, a.branch_id));
    unique.dedup();
    if unique.is_empty() {
        return Err(AppError::field("role_ids", "NO_ROLES", "choose at least one role"));
    }
    if unique.len() > 20 {
        return Err(AppError::field("role_ids", "TOO_MANY_ROLES", "choose at most 20 role assignments"));
    }
    let role_ids: Vec<Uuid> = unique.iter().map(|a| a.role_id).collect();
    let roles = grants::roles(tx, &role_ids).await?;
    grants::reject_owner_role(&roles)?;
    let branch_ids: Vec<Uuid> = unique.iter().filter_map(|a| a.branch_id).collect();
    grants::ensure_branches(tx, &branch_ids).await?;
    let permissions = grants::role_permissions(tx, &role_ids).await?;
    grants::ensure_grantable(actor, &permissions)?;
    if !actor.is_owner && roles.iter().any(|r| r.code == ADMIN_ROLE) {
        return Err(AppError::Denied {
            code: "ADMIN_PROTECTED",
            message: "only the owner can make someone an admin".into(),
        });
    }
    Ok(unique)
}

/// Replaces a member's roles.
pub async fn set_roles(
    tx: &mut TenantTx,
    actor: &TenantAccess,
    meta: &RequestMeta,
    membership_id: Uuid,
    assignments: Vec<RoleAssignment>,
) -> AppResult<Member> {
    let member = target(tx, actor, membership_id).await?;
    let assignments = validate_assignments(tx, actor, &assignments).await?;
    let tenant_id = tx.tenant_id();
    sqlx::query("DELETE FROM membership_roles WHERE tenant_id = ? AND membership_id = ?")
        .bind(tenant_id)
        .bind(membership_id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    insert_assignments(tx, membership_id, &assignments, actor.user_id).await?;
    bump(tx, membership_id, None).await?;

    let before: Vec<RoleAssignment> =
        member.roles.iter().map(|r| RoleAssignment { role_id: r.role_id, branch_id: r.branch_id }).collect();
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("member.roles_changed")
            .entity("membership", membership_id)
            .change(Some(json!({ "roles": before })), Some(json!({ "roles": assignments }))),
    )
    .await?;
    get(tx, membership_id).await
}

/// `active → suspended`: access ends immediately, sessions in this business end.
pub async fn suspend(
    tx: &mut TenantTx,
    actor: &TenantAccess,
    meta: &RequestMeta,
    membership_id: Uuid,
) -> AppResult<Member> {
    let member = target(tx, actor, membership_id).await?;
    if member.status != "active" {
        return Err(AppError::Conflict {
            code: "INVALID_STATUS",
            message: "only active members can be suspended".into(),
        });
    }
    bump(tx, membership_id, Some("suspended")).await?;
    let tenant_id = tx.tenant_id();
    let revoked = session::revoke_for_membership(tx, tenant_id, member.user_id, "membership_suspended").await?;
    audit_status(tx, meta, membership_id, "member.suspended", "active", "suspended", revoked).await?;
    get(tx, membership_id).await
}

/// `suspended → active`, within the plan's user limit.
pub async fn reactivate(
    tx: &mut TenantTx,
    actor: &TenantAccess,
    meta: &RequestMeta,
    membership_id: Uuid,
) -> AppResult<Member> {
    let member = target(tx, actor, membership_id).await?;
    if member.status != "suspended" {
        return Err(AppError::Conflict {
            code: "INVALID_STATUS",
            message: "only suspended members can be reactivated".into(),
        });
    }
    let used = entitlements::seats_used(tx, None).await?;
    entitlements::ensure_within(tx, "users.max", used, 1).await?;
    bump(tx, membership_id, Some("active")).await?;
    audit_status(tx, meta, membership_id, "member.reactivated", "suspended", "active", 0).await?;
    get(tx, membership_id).await
}

/// `→ removed`: roles are dropped and sessions in this business end. The
/// person can be invited again later.
pub async fn remove(tx: &mut TenantTx, actor: &TenantAccess, meta: &RequestMeta, membership_id: Uuid) -> AppResult<()> {
    let member = target(tx, actor, membership_id).await?;
    let tenant_id = tx.tenant_id();
    sqlx::query("DELETE FROM membership_roles WHERE tenant_id = ? AND membership_id = ?")
        .bind(tenant_id)
        .bind(membership_id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    bump(tx, membership_id, Some("removed")).await?;
    let revoked = session::revoke_for_membership(tx, tenant_id, member.user_id, "membership_removed").await?;
    audit_status(tx, meta, membership_id, "member.removed", &member.status, "removed", revoked).await
}

async fn audit_status(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    membership_id: Uuid,
    action: &'static str,
    from: &str,
    to: &str,
    revoked_sessions: u64,
) -> AppResult<()> {
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new(action).entity("membership", membership_id).change(
            Some(json!({ "status": from })),
            Some(json!({ "status": to, "revoked_sessions": revoked_sessions })),
        ),
    )
    .await
    .map(|_| ())
}

/// The member's devices signed in to this business.
pub async fn sessions(tx: &mut TenantTx, membership_id: Uuid) -> AppResult<Vec<session::SessionInfo>> {
    let member = get(tx, membership_id).await?;
    let tenant_id = tx.tenant_id();
    session::list_for_member(tx, tenant_id, member.user_id).await
}

/// Signs the member out of this business on every device (US-AUTH-05).
/// Allowed for the owner's sessions only when the owner asks.
pub async fn revoke_sessions(
    tx: &mut TenantTx,
    actor: &TenantAccess,
    meta: &RequestMeta,
    membership_id: Uuid,
) -> AppResult<u64> {
    let member = get(tx, membership_id).await?;
    if member.is_owner && !actor.is_owner {
        return Err(AppError::Denied {
            code: "OWNER_PROTECTED",
            message: "the owner's sessions cannot be ended".into(),
        });
    }
    let tenant_id = tx.tenant_id();
    let revoked = session::revoke_for_membership(tx, tenant_id, member.user_id, "admin_revoked").await?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("member.sessions_revoked")
            .entity("membership", membership_id)
            .change(None, Some(json!({ "revoked_sessions": revoked }))),
    )
    .await?;
    Ok(revoked)
}
