//! Staff invitations (US-AUTH-04, AUTH-R09): a single-use link, valid for 7
//! days, sent to a phone number or email address. Only a SHA-256 hash of the
//! token is stored. Accepting needs a signed-in user whose verified phone (or
//! email) matches the destination.

use chrono::{DateTime, Utc};
use pos_audit::AuditEntry;
use pos_auth::{
    TenantAccess,
    identifier::{Identifier, mask},
    token::{hash_opaque_token as hash_token, new_opaque_token as new_token},
};
use pos_core::{AppError, AppResult, RequestMeta, TenantCtx, id::new_id};
use pos_db::{TenantTx, map_db_error};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::{
    entitlements,
    members::{self, RoleAssignment},
    onboarding,
};

pub const VALID_DAYS: i64 = 7;

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NewInvitation {
    /// Mobile number or email address.
    pub destination: String,
    pub role_ids: Vec<Uuid>,
    /// Restrict the roles to these branches; empty or absent = all branches.
    #[serde(default)]
    pub branch_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct InvitationRole {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct Invitation {
    pub id: Uuid,
    pub destination: String,
    pub roles: Vec<InvitationRole>,
    pub branch_ids: Vec<Uuid>,
    /// `pending`, `accepted`, `revoked` or `expired` (pending past its expiry).
    pub status: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub invited_by: String,
}

/// A stored invitation plus the plain token, which only exists until it is sent.
pub struct IssuedInvitation {
    pub invitation: Invitation,
    pub token: String,
    pub tenant_name: String,
}

#[derive(sqlx::FromRow)]
struct InvitationRow {
    id: Uuid,
    destination: String,
    role_ids: serde_json::Value,
    branch_ids: Option<serde_json::Value>,
    status: String,
    expired: bool,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    invited_by: String,
}

fn uuids(value: Option<&serde_json::Value>) -> Vec<Uuid> {
    value
        .and_then(serde_json::Value::as_array)
        .map(|a| a.iter().filter_map(|v| v.as_str().and_then(|s| Uuid::parse_str(s).ok())).collect())
        .unwrap_or_default()
}

fn json_ids(ids: &[Uuid]) -> serde_json::Value {
    json!(ids.iter().map(Uuid::to_string).collect::<Vec<_>>())
}

async fn load(tx: &mut TenantTx, only: Option<Uuid>) -> AppResult<Vec<Invitation>> {
    let tenant_id = tx.tenant_id();
    let rows: Vec<InvitationRow> = sqlx::query_as(
        "SELECT i.id, i.destination, i.role_ids, i.branch_ids, i.status, \
                (i.status = 'pending' AND i.expires_at <= NOW(6)) AS expired, i.expires_at, i.created_at, \
                u.full_name AS invited_by \
         FROM invitations i JOIN users u ON u.id = i.created_by \
         WHERE i.tenant_id = ? AND (? IS NULL OR i.id = ?) ORDER BY i.created_at DESC",
    )
    .bind(tenant_id)
    .bind(only)
    .bind(only)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let names: Vec<(Uuid, String)> = sqlx::query_as("SELECT id, name FROM roles WHERE tenant_id = ?")
        .bind(tenant_id)
        .fetch_all(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(rows
        .into_iter()
        .map(|r| Invitation {
            roles: uuids(Some(&r.role_ids))
                .into_iter()
                .filter_map(|id| {
                    names.iter().find(|(rid, _)| *rid == id).map(|(_, n)| InvitationRole { id, name: n.clone() })
                })
                .collect(),
            branch_ids: uuids(r.branch_ids.as_ref()),
            status: if r.expired { "expired".to_owned() } else { r.status },
            id: r.id,
            destination: r.destination,
            expires_at: r.expires_at,
            created_at: r.created_at,
            invited_by: r.invited_by,
        })
        .collect())
}

/// Open and recent invitations (accepted/revoked ones from the last 30 days).
pub async fn list(tx: &mut TenantTx) -> AppResult<Vec<Invitation>> {
    let cutoff = Utc::now() - chrono::Duration::days(30);
    Ok(load(tx, None).await?.into_iter().filter(|i| i.status == "pending" || i.created_at > cutoff).collect())
}

async fn tenant_name(tx: &mut TenantTx) -> AppResult<String> {
    let tenant_id = tx.tenant_id();
    sqlx::query_scalar("SELECT name FROM tenants WHERE id = ?")
        .bind(tenant_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)
}

pub async fn create(
    tx: &mut TenantTx,
    actor: &TenantAccess,
    meta: &RequestMeta,
    input: NewInvitation,
) -> AppResult<IssuedInvitation> {
    let destination = Identifier::parse(&input.destination, "destination")?;
    let assignments: Vec<RoleAssignment> = if input.branch_ids.is_empty() {
        input.role_ids.iter().map(|&role_id| RoleAssignment { role_id, branch_id: None }).collect()
    } else {
        input
            .role_ids
            .iter()
            .flat_map(|&role_id| input.branch_ids.iter().map(move |&b| RoleAssignment { role_id, branch_id: Some(b) }))
            .collect()
    };
    members::validate_assignments(tx, actor, &assignments).await?;

    let tenant_id = tx.tenant_id();
    let already: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM tenant_memberships m JOIN users u ON u.id = m.user_id \
         WHERE m.tenant_id = ? AND m.status IN ('active', 'suspended') AND (u.phone = ? OR u.email = ?))",
    )
    .bind(tenant_id)
    .bind(destination.as_str())
    .bind(destination.as_str())
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if already {
        return Err(AppError::Conflict { code: "ALREADY_MEMBER", message: "this person is already a member".into() });
    }

    // A new invitation replaces an open one to the same person.
    sqlx::query(
        "UPDATE invitations SET status = 'revoked' WHERE tenant_id = ? AND destination = ? AND status = 'pending'",
    )
    .bind(tenant_id)
    .bind(destination.as_str())
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let used = entitlements::seats_used(tx, None).await?;
    entitlements::ensure_within(tx, "users.max", used, 1).await?;

    let id = new_id();
    let (token, hash) = new_token()?;
    let mut role_ids = input.role_ids.clone();
    role_ids.sort();
    role_ids.dedup();
    let mut branch_ids = input.branch_ids.clone();
    branch_ids.sort();
    branch_ids.dedup();
    sqlx::query(
        "INSERT INTO invitations (tenant_id, id, destination, token_hash, role_ids, branch_ids, expires_at, created_by) \
         VALUES (?, ?, ?, ?, ?, ?, NOW(6) + INTERVAL ? DAY, ?)",
    )
    .bind(tenant_id)
    .bind(id)
    .bind(destination.as_str())
    .bind(hash.as_slice())
    .bind(json_ids(&role_ids))
    .bind((!branch_ids.is_empty()).then(|| json_ids(&branch_ids)))
    .bind(VALID_DAYS)
    .bind(actor.user_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;

    // Inviting the first person ticks off the setup checklist item.
    onboarding::complete_step(tx, meta, "staff").await?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("invitation.created")
            .entity("invitation", id)
            .change(None, Some(json!({ "destination": mask(destination.as_str()), "role_ids": role_ids }))),
    )
    .await?;
    let invitation = load(tx, Some(id)).await?.pop().ok_or(AppError::NotFound)?;
    Ok(IssuedInvitation { invitation, token, tenant_name: tenant_name(tx).await? })
}

async fn lock(tx: &mut TenantTx, id: Uuid) -> AppResult<(String, bool)> {
    let tenant_id = tx.tenant_id();
    let row: Option<(String, bool)> = sqlx::query_as(
        "SELECT status, expires_at <= NOW(6) AS expired FROM invitations WHERE tenant_id = ? AND id = ? FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    row.ok_or(AppError::NotFound)
}

/// Issues a fresh link (new token, new 7 days) for an open or expired invitation.
pub async fn resend(tx: &mut TenantTx, meta: &RequestMeta, id: Uuid) -> AppResult<IssuedInvitation> {
    let (status, expired) = lock(tx, id).await?;
    if status != "pending" {
        return Err(AppError::Conflict {
            code: "INVITATION_CLOSED",
            message: "this invitation was already used or revoked".into(),
        });
    }
    if expired {
        // An expired invitation stopped holding a seat; it needs one again.
        let used = entitlements::seats_used(tx, Some(id)).await?;
        entitlements::ensure_within(tx, "users.max", used, 1).await?;
    }
    let tenant_id = tx.tenant_id();
    let (token, hash) = new_token()?;
    sqlx::query(
        "UPDATE invitations SET token_hash = ?, expires_at = NOW(6) + INTERVAL ? DAY WHERE tenant_id = ? AND id = ?",
    )
    .bind(hash.as_slice())
    .bind(VALID_DAYS)
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    pos_audit::record(tx, meta, AuditEntry::new("invitation.resent").entity("invitation", id)).await?;
    let invitation = load(tx, Some(id)).await?.pop().ok_or(AppError::NotFound)?;
    Ok(IssuedInvitation { invitation, token, tenant_name: tenant_name(tx).await? })
}

pub async fn revoke(tx: &mut TenantTx, meta: &RequestMeta, id: Uuid) -> AppResult<()> {
    let (status, _) = lock(tx, id).await?;
    if status != "pending" {
        return Err(AppError::Conflict {
            code: "INVITATION_CLOSED",
            message: "this invitation was already used or revoked".into(),
        });
    }
    let tenant_id = tx.tenant_id();
    sqlx::query("UPDATE invitations SET status = 'revoked' WHERE tenant_id = ? AND id = ?")
        .bind(tenant_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    pos_audit::record(tx, meta, AuditEntry::new("invitation.revoked").entity("invitation", id)).await?;
    Ok(())
}

// ------------------------------------------------------------------ public

/// What the invitation page shows before signing in. The destination is masked.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct InvitationPreview {
    pub business_name: String,
    pub invited_by: String,
    pub destination: String,
    /// `phone` or `email`.
    pub destination_kind: &'static str,
    pub roles: Vec<String>,
    pub status: String,
    pub expires_at: DateTime<Utc>,
}

struct Located {
    tenant_id: Uuid,
    id: Uuid,
}

/// Token → invitation. Tokens are global and unique, so this is the one
/// lookup that does not start from a tenant.
async fn locate(pool: &MySqlPool, token: &str) -> AppResult<Located> {
    let row: Option<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT tenant_id, id FROM invitations WHERE token_hash = ? \
         /* tenant-lint: allow lookup by a secret single-use token; the tenant comes from the row */",
    )
    .bind(hash_token(token).as_slice())
    .fetch_optional(pool)
    .await
    .map_err(map_db_error)?;
    row.map(|(tenant_id, id)| Located { tenant_id, id }).ok_or(AppError::NotFound)
}

pub async fn preview(pool: &MySqlPool, token: &str) -> AppResult<InvitationPreview> {
    let found = locate(pool, token).await?;
    let row: (String, String, String, String, bool, DateTime<Utc>, serde_json::Value) = sqlx::query_as(
        "SELECT t.name, u.full_name, i.destination, i.status, (i.status = 'pending' AND i.expires_at <= NOW(6)), \
                i.expires_at, i.role_ids \
         FROM invitations i JOIN tenants t ON t.id = i.tenant_id JOIN users u ON u.id = i.created_by \
         WHERE i.tenant_id = ? AND i.id = ?",
    )
    .bind(found.tenant_id)
    .bind(found.id)
    .fetch_one(pool)
    .await
    .map_err(map_db_error)?;
    let (business_name, invited_by, destination, status, expired, expires_at, role_ids) = row;
    let names: Vec<(Uuid, String)> = sqlx::query_as("SELECT id, name FROM roles WHERE tenant_id = ?")
        .bind(found.tenant_id)
        .fetch_all(pool)
        .await
        .map_err(map_db_error)?;
    let roles = uuids(Some(&role_ids))
        .into_iter()
        .filter_map(|id| names.iter().find(|(rid, _)| *rid == id).map(|(_, n)| n.clone()))
        .collect();
    Ok(InvitationPreview {
        business_name,
        invited_by,
        destination_kind: if destination.contains('@') { "email" } else { "phone" },
        destination: mask(&destination),
        roles,
        status: if expired { "expired".to_owned() } else { status },
        expires_at,
    })
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct AcceptedInvitation {
    pub tenant_id: Uuid,
    pub membership_id: Uuid,
}

/// Joins the business as the signed-in user. The user's verified phone, or
/// email, must match where the invitation was sent.
pub async fn accept(pool: &MySqlPool, user_id: Uuid, token: &str, meta: &RequestMeta) -> AppResult<AcceptedInvitation> {
    let found = locate(pool, token).await?;
    let user: Option<(Option<String>, bool, Option<String>, String)> = sqlx::query_as(
        "SELECT phone, phone_verified_at IS NOT NULL, email, full_name FROM users WHERE id = ? AND status = 'active'",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(map_db_error)?;
    let (phone, phone_verified, email, full_name) = user.ok_or(AppError::Unauthorized)?;

    let ctx = TenantCtx { tenant_id: found.tenant_id, user_id, meta: meta.clone() };
    let mut tx = pos_db::begin_tenant(pool, &ctx).await?;
    let tenant_id = tx.tenant_id();
    let row: (String, bool, String, serde_json::Value, Option<serde_json::Value>, Uuid, String) = sqlx::query_as(
        "SELECT i.status, i.expires_at <= NOW(6), i.destination, i.role_ids, i.branch_ids, i.created_by, t.status \
         FROM invitations i JOIN tenants t ON t.id = i.tenant_id WHERE i.tenant_id = ? AND i.id = ? FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(found.id)
    .fetch_one(&mut *tx)
    .await
    .map_err(map_db_error)?;
    let (status, expired, destination, role_ids, branch_ids, invited_by, tenant_status) = row;
    pos_auth::rbac::check_tenant_status(&tenant_status)?;
    match (status.as_str(), expired) {
        ("pending", false) => {}
        ("pending", true) => {
            return Err(AppError::Unprocessable {
                code: "INVITATION_EXPIRED",
                message: "this invitation has expired".into(),
            });
        }
        _ => {
            return Err(AppError::Unprocessable {
                code: "INVITATION_CLOSED",
                message: "this invitation was already used or revoked".into(),
            });
        }
    }

    let matches = if destination.contains('@') {
        email.as_deref() == Some(destination.as_str())
    } else {
        phone_verified && phone.as_deref() == Some(destination.as_str())
    };
    if !matches {
        return Err(AppError::Denied {
            code: "INVITATION_MISMATCH",
            message: format!("this invitation was sent to {}; sign in with that account", mask(&destination)),
        });
    }

    let existing: Option<(Uuid, String)> =
        sqlx::query_as("SELECT id, status FROM tenant_memberships WHERE tenant_id = ? AND user_id = ? FOR UPDATE")
            .bind(tenant_id)
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_db_error)?;
    let membership_id = match existing {
        Some((_, s)) if s == "active" || s == "suspended" => {
            return Err(AppError::Conflict {
                code: "ALREADY_MEMBER",
                message: "you are already a member of this business".into(),
            });
        }
        Some((id, _)) => {
            sqlx::query(
                "UPDATE tenant_memberships SET status = 'active', perms_version = perms_version + 1, joined_at = NOW(6), \
                 display_name = ? WHERE tenant_id = ? AND id = ?",
            )
            .bind(&full_name)
            .bind(tenant_id)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
            id
        }
        None => {
            let id = new_id();
            sqlx::query(
                "INSERT INTO tenant_memberships (tenant_id, id, user_id, status, display_name, joined_at, created_by) \
                 VALUES (?, ?, ?, 'active', ?, NOW(6), ?)",
            )
            .bind(tenant_id)
            .bind(id)
            .bind(user_id)
            .bind(&full_name)
            .bind(invited_by)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
            id
        }
    };

    // Roles deleted since the invitation was sent are skipped; at least one must remain.
    let wanted = uuids(Some(&role_ids));
    let still: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM roles WHERE tenant_id = ? AND code <> 'owner'")
        .bind(tenant_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_db_error)?;
    let role_ids: Vec<Uuid> = wanted.into_iter().filter(|id| still.contains(id)).collect();
    if role_ids.is_empty() {
        return Err(AppError::Unprocessable {
            code: "INVITATION_ROLES_GONE",
            message: "the roles in this invitation no longer exist; ask for a new invitation".into(),
        });
    }
    let branches = uuids(branch_ids.as_ref());
    let assignments: Vec<RoleAssignment> = if branches.is_empty() {
        role_ids.iter().map(|&role_id| RoleAssignment { role_id, branch_id: None }).collect()
    } else {
        role_ids
            .iter()
            .flat_map(|&r| branches.iter().map(move |&b| RoleAssignment { role_id: r, branch_id: Some(b) }))
            .collect()
    };
    members::insert_assignments(&mut tx, membership_id, &assignments, invited_by).await?;

    sqlx::query("UPDATE invitations SET status = 'accepted', accepted_by = ? WHERE tenant_id = ? AND id = ?")
        .bind(user_id)
        .bind(tenant_id)
        .bind(found.id)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
    if destination.contains('@') {
        // The link reached that mailbox, so the address is proven.
        sqlx::query("UPDATE users SET email_verified_at = COALESCE(email_verified_at, NOW(6)) WHERE id = ?")
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
    }
    pos_audit::record(
        &mut tx,
        meta,
        AuditEntry::new("member.joined")
            .entity("membership", membership_id)
            .change(None, Some(json!({ "invitation_id": found.id, "roles": assignments }))),
    )
    .await?;
    tx.commit().await?;
    Ok(AcceptedInvitation { tenant_id, membership_id })
}
