//! Adding staff directly (no invitation): the owner or a manager registers
//! the person with a name, mobile number and password and gives them roles.
//! The account works at once; whoever added it hands over the password.
//!
//! The adder vouches for the number, so it is marked verified. A number that
//! already has a verified account is NOT added directly: it gets an
//! invitation, sent to that number, which its holder accepts while signed in
//! (`invitations::accept` checks the account owns the number). So adding
//! staff can neither take over somebody's account nor put an account into a
//! business without its holder's consent, and a number someone else
//! registered first cannot be slipped into the business as "the employee".

use pos_audit::AuditEntry;
use pos_auth::{
    TenantAccess,
    identifier::{mask, normalize_email, normalize_phone},
    password,
};
use pos_core::{AppError, AppResult, RequestMeta, id::new_id};
use pos_db::{TenantTx, map_db_error};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{
    entitlements,
    invitations::{self, Invitation, IssuedInvitation, NewInvitation},
    members::{self, Member, RoleAssignment},
    onboarding,
};

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NewMember {
    pub full_name: String,
    /// Mobile number; it is also the sign-in name.
    pub phone: String,
    #[serde(default)]
    pub email: Option<String>,
    /// The first password (password policy applies). Ignored when the number
    /// already has an account, which keeps its own password.
    #[serde(default)]
    pub password: Option<String>,
    pub role_ids: Vec<Uuid>,
    /// Restrict the roles to these branches; empty or absent = all branches.
    #[serde(default)]
    pub branch_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct CreatedMember {
    /// The new member; absent when the number already had an account and an
    /// invitation was sent instead.
    pub member: Option<Member>,
    /// The number already had an account: it was invited, not added.
    pub existing_account: bool,
    /// The invitation sent to that account (it joins once accepted).
    pub invitation: Option<Invitation>,
}

/// What adding staff did: added a new account, or invited an existing one.
pub enum StaffAdded {
    Created(Member),
    /// The plain token is sent by the caller (`deliver`), never returned.
    Invited(IssuedInvitation),
}

pub async fn create(
    tx: &mut TenantTx,
    actor: &TenantAccess,
    meta: &RequestMeta,
    input: NewMember,
) -> AppResult<StaffAdded> {
    let full_name = input.full_name.trim().to_owned();
    if !(2..=150).contains(&full_name.chars().count()) {
        return Err(AppError::field("full_name", "INVALID_NAME", "enter the full name (2–150 characters)"));
    }
    let phone = normalize_phone(&input.phone)
        .ok_or_else(|| AppError::field("phone", "INVALID_PHONE", "enter a valid mobile number"))?;
    let email = match input.email.as_deref().map(str::trim).filter(|e| !e.is_empty()) {
        Some(raw) => Some(
            normalize_email(raw)
                .ok_or_else(|| AppError::field("email", "INVALID_EMAIL", "enter a valid email address"))?,
        ),
        None => None,
    };
    let assignments: Vec<RoleAssignment> = if input.branch_ids.is_empty() {
        input.role_ids.iter().map(|&role_id| RoleAssignment { role_id, branch_id: None }).collect()
    } else {
        input
            .role_ids
            .iter()
            .flat_map(|&role_id| input.branch_ids.iter().map(move |&b| RoleAssignment { role_id, branch_id: Some(b) }))
            .collect()
    };
    let assignments = members::validate_assignments(tx, actor, &assignments).await?;

    let tenant_id = tx.tenant_id();
    let existing: Option<(Uuid, bool, String)> =
        sqlx::query_as("SELECT id, phone_verified_at IS NOT NULL, status FROM users WHERE phone = ? FOR UPDATE")
            .bind(&phone)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?;
    if existing.as_ref().is_some_and(|(_, _, status)| status != "active") {
        return Err(AppError::field("phone", "ACCOUNT_DISABLED", "this number's account is disabled"));
    }
    let existing_account = existing.as_ref().is_some_and(|(_, verified, _)| *verified);
    if existing_account {
        // Their account, their consent: invite rather than add.
        let issued = invitations::create(
            tx,
            actor,
            meta,
            NewInvitation { destination: phone, role_ids: input.role_ids, branch_ids: input.branch_ids },
        )
        .await?;
        return Ok(StaffAdded::Invited(issued));
    }

    let user_id = {
        // A new account, or an unverified sign-up nobody completed (it
        // proves nothing yet, so it is taken over like `register` does).
        let personal = [full_name.as_str(), phone.as_str(), email.as_deref().unwrap_or("")];
        let Some(plain) = input.password.filter(|p| !p.is_empty()) else {
            return Err(AppError::field("password", "REQUIRED", "enter a password for the new account"));
        };
        password::validate_policy(&plain, "password", &personal)?;
        let password_hash = password::hash(plain).await?;
        let user_id = existing.as_ref().map_or_else(new_id, |(id, _, _)| *id);
        if let Some(email) = &email {
            let taken: Option<Uuid> = sqlx::query_scalar("SELECT id FROM users WHERE email = ? AND id <> ?")
                .bind(email)
                .bind(user_id)
                .fetch_optional(&mut **tx)
                .await
                .map_err(map_db_error)?;
            if taken.is_some() {
                return Err(AppError::field("email", "EMAIL_TAKEN", "this email address belongs to another account"));
            }
        }
        if existing.is_some() {
            // Whoever started that sign-up loses any session it had.
            pos_auth::session::revoke_all_for_user(tx, user_id, "account_reclaimed").await?;
            sqlx::query(
                "UPDATE users SET full_name = ?, email = ?, password_hash = ?, phone_verified_at = NOW(6) WHERE id = ?",
            )
            .bind(&full_name)
            .bind(&email)
            .bind(&password_hash)
            .bind(user_id)
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
        } else {
            sqlx::query(
                "INSERT INTO users (id, full_name, phone, email, password_hash, phone_verified_at) \
                 VALUES (?, ?, ?, ?, ?, NOW(6))",
            )
            .bind(user_id)
            .bind(&full_name)
            .bind(&phone)
            .bind(&email)
            .bind(&password_hash)
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
        }
        user_id
    };

    let membership: Option<(Uuid, String)> =
        sqlx::query_as("SELECT id, status FROM tenant_memberships WHERE tenant_id = ? AND user_id = ? FOR UPDATE")
            .bind(tenant_id)
            .bind(user_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?;
    if matches!(&membership, Some((_, s)) if s == "active" || s == "suspended") {
        return Err(AppError::Conflict { code: "ALREADY_MEMBER", message: "this person is already a member".into() });
    }
    entitlements::lock_limits(tx).await?;
    let used = entitlements::seats_used(tx, None).await?;
    entitlements::ensure_within(tx, "users.max", used, 1).await?;

    let membership_id = match membership {
        Some((id, _)) => {
            sqlx::query(
                "UPDATE tenant_memberships SET status = 'active', perms_version = perms_version + 1, joined_at = NOW(6), \
                 display_name = ? WHERE tenant_id = ? AND id = ?",
            )
            .bind(&full_name)
            .bind(tenant_id)
            .bind(id)
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
            // A returning member starts from the roles given now.
            sqlx::query("DELETE FROM membership_roles WHERE tenant_id = ? AND membership_id = ?")
                .bind(tenant_id)
                .bind(id)
                .execute(&mut **tx)
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
            .bind(actor.user_id)
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
            id
        }
    };
    members::insert_assignments(tx, membership_id, &assignments, actor.user_id).await?;
    // An open invitation to the same number is no longer needed.
    sqlx::query(
        "UPDATE invitations SET status = 'revoked' WHERE tenant_id = ? AND destination = ? AND status = 'pending'",
    )
    .bind(tenant_id)
    .bind(&phone)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;

    onboarding::complete_step(tx, meta, "staff").await?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("member.created")
            .entity("membership", membership_id)
            .change(None, Some(json!({ "phone": mask(&phone), "roles": assignments }))),
    )
    .await?;
    let member = members::get(tx, membership_id).await?;
    Ok(StaffAdded::Created(member))
}
