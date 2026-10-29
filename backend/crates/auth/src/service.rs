//! Account flows: registration, phone verification, login with lockout,
//! token refresh and tenant switching, logout, password reset, sessions.

use std::sync::Arc;

use pos_core::{AppError, AppResult, RequestMeta, config::AuthConfig, id::new_id};
use pos_db::map_db_error;
use serde::Serialize;
use serde_json::json;
use sqlx::{MySqlConnection, MySqlPool};
use uuid::Uuid;

use crate::{
    events::{self, EventSubject},
    identifier::{Identifier, normalize_email, normalize_phone},
    notify::Notifier,
    otp::{self, OtpPurpose},
    password,
    rbac::check_tenant_status,
    session::{self, SessionInfo},
    token::{AccessClaims, TenantClaim, TokenKeys, sha256},
};

/// AUTH-R03: 5 failures per identifier + IP within 15 minutes lock the pair for 15 minutes.
pub const MAX_LOGIN_FAILURES: i64 = 5;
pub const LOCKOUT_MINUTES: i64 = 15;

/// Tokens for a signed-in session. The refresh token goes into an httpOnly
/// cookie and never into a response body.
#[derive(Debug)]
pub struct Tokens {
    pub access_token: String,
    pub expires_in: u64,
    pub refresh_token: String,
    pub refresh_max_age_secs: u64,
    pub tenant_id: Option<Uuid>,
}

/// Where a code was sent. `dev_code` is only filled when `AUTH_EXPOSE_OTP` is on.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct OtpChallenge {
    pub destination: String,
    pub expires_in: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dev_code: Option<String>,
}

pub struct RegisterInput {
    pub full_name: String,
    pub phone: String,
    pub email: Option<String>,
    pub password: String,
    pub locale: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct UserProfile {
    pub id: Uuid,
    pub full_name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub phone_verified: bool,
    pub locale: String,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct MembershipSummary {
    pub tenant_id: Uuid,
    pub membership_id: Uuid,
    pub tenant_name: String,
    pub tenant_slug: String,
    pub tenant_status: String,
    pub is_owner: bool,
}

/// Which tenant a rotated session should be signed in to.
enum TenantChoice {
    Keep,
    Switch(Option<Uuid>),
}

pub struct Auth {
    keys: TokenKeys,
    otp_pepper: [u8; 32],
    refresh_ttl_days: u64,
    expose_otp: bool,
    notifier: Arc<dyn Notifier>,
}

impl Auth {
    pub fn new(cfg: &AuthConfig, notifier: Arc<dyn Notifier>) -> Self {
        Self {
            keys: TokenKeys::from_config(cfg),
            // Separate key for OTP hashes, derived so there is one secret to manage.
            otp_pepper: sha256(&[b"pos-otp-pepper:", cfg.jwt_secret.expose().as_bytes()]),
            refresh_ttl_days: cfg.refresh_token_ttl_days,
            expose_otp: cfg.expose_otp,
            notifier,
        }
    }

    /// Whether responses may carry one-time secrets (OTP codes, invitation links) for development.
    pub fn expose_dev_secrets(&self) -> bool {
        self.expose_otp
    }

    pub fn notifier(&self) -> &dyn Notifier {
        self.notifier.as_ref()
    }

    pub fn verify_access(&self, token: &str) -> AppResult<AccessClaims> {
        self.keys.verify(token)
    }

    fn challenge(&self, destination: &str, issued: Option<otp::IssuedOtp>) -> OtpChallenge {
        OtpChallenge {
            destination: crate::identifier::mask(destination),
            expires_in: issued.as_ref().map_or(otp::CODE_TTL_SECS, |i| i.expires_in),
            dev_code: issued.filter(|_| self.expose_otp).map(|i| i.code),
        }
    }

    fn deliver(&self, destination: &str, purpose: OtpPurpose, issued: &otp::IssuedOtp) {
        self.notifier.send_otp(destination, purpose, &issued.code);
    }

    // ---------------------------------------------------------------- register

    /// US-AUTH-01. Creates the user (or refreshes an unverified one) and sends
    /// a verification code. The account is usable after `verify_phone`.
    pub async fn register(
        &self,
        pool: &MySqlPool,
        input: RegisterInput,
        meta: &RequestMeta,
    ) -> AppResult<OtpChallenge> {
        let full_name = input.full_name.trim().to_owned();
        let name_len = full_name.chars().count();
        if !(2..=150).contains(&name_len) {
            return Err(AppError::field("full_name", "INVALID_NAME", "enter your full name (2–150 characters)"));
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
        let locale = match input.locale.as_deref() {
            None | Some("en") => "en",
            Some("bn") => "bn",
            Some("it") => "it",
            Some(_) => return Err(AppError::field("locale", "INVALID_LOCALE", "language must be en, bn or it")),
        };
        let personal = [full_name.as_str(), phone.as_str(), email.as_deref().unwrap_or("")];
        password::validate_policy(&input.password, "password", &personal)?;
        let password_hash = password::hash(input.password).await?;

        let mut tx = pool.begin().await.map_err(map_db_error)?;
        let existing: Option<(Uuid, bool)> =
            sqlx::query_as("SELECT id, phone_verified_at IS NOT NULL FROM users WHERE phone = ? FOR UPDATE")
                .bind(&phone)
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_db_error)?;
        if matches!(existing, Some((_, true))) {
            return Err(AppError::Conflict {
                code: "PHONE_TAKEN",
                message: "this mobile number is already registered; sign in or reset your password".into(),
            });
        }
        let user_id = existing.map_or_else(new_id, |(id, _)| id);
        if let Some(email) = &email {
            let taken: Option<Uuid> = sqlx::query_scalar("SELECT id FROM users WHERE email = ? AND id <> ?")
                .bind(email)
                .bind(user_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_db_error)?;
            if taken.is_some() {
                return Err(AppError::Conflict {
                    code: "EMAIL_TAKEN",
                    message: "this email address is already registered".into(),
                });
            }
        }

        if existing.is_some() {
            // An unverified account proves nothing yet; whoever verifies the phone owns it.
            sqlx::query("UPDATE users SET full_name = ?, email = ?, password_hash = ?, locale = ? WHERE id = ?")
                .bind(&full_name)
                .bind(&email)
                .bind(&password_hash)
                .bind(locale)
                .bind(user_id)
                .execute(&mut *tx)
                .await
                .map_err(map_db_error)?;
        } else {
            sqlx::query(
                "INSERT INTO users (id, full_name, phone, email, password_hash, locale) VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(user_id)
            .bind(&full_name)
            .bind(&phone)
            .bind(&email)
            .bind(&password_hash)
            .bind(locale)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }

        let issued = otp::issue(&mut tx, &self.otp_pepper, Some(user_id), &phone, OtpPurpose::VerifyPhone).await?;
        events::record(&mut tx, events::REGISTERED, EventSubject::user(user_id), meta, json!({})).await?;
        tx.commit().await.map_err(map_db_error)?;

        self.deliver(&phone, OtpPurpose::VerifyPhone, &issued);
        Ok(self.challenge(&phone, Some(issued)))
    }

    /// Sends a new verification code. Answers the same way for unknown or
    /// already verified numbers, so it cannot be used to probe accounts.
    pub async fn resend_verification(&self, pool: &MySqlPool, raw_phone: &str) -> AppResult<OtpChallenge> {
        let phone = normalize_phone(raw_phone)
            .ok_or_else(|| AppError::field("phone", "INVALID_PHONE", "enter a valid mobile number"))?;
        let mut tx = pool.begin().await.map_err(map_db_error)?;
        let user: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM users WHERE phone = ? AND phone_verified_at IS NULL")
                .bind(&phone)
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_db_error)?;
        let Some(user_id) = user else {
            return Ok(self.challenge(&phone, None));
        };
        let issued = otp::issue(&mut tx, &self.otp_pepper, Some(user_id), &phone, OtpPurpose::VerifyPhone).await?;
        tx.commit().await.map_err(map_db_error)?;
        self.deliver(&phone, OtpPurpose::VerifyPhone, &issued);
        Ok(self.challenge(&phone, Some(issued)))
    }

    /// Confirms the phone with the code from `register` and signs the user in.
    pub async fn verify_phone(
        &self,
        pool: &MySqlPool,
        raw_phone: &str,
        code: &str,
        meta: &RequestMeta,
    ) -> AppResult<Tokens> {
        let phone = normalize_phone(raw_phone)
            .ok_or_else(|| AppError::field("phone", "INVALID_PHONE", "enter a valid mobile number"))?;
        let matched = otp::check(pool, &self.otp_pepper, &phone, OtpPurpose::VerifyPhone, code).await?;
        let user_id = matched.user_id.ok_or_else(|| AppError::internal("verification code without a user"))?;

        let mut tx = pool.begin().await.map_err(map_db_error)?;
        otp::consume(&mut tx, matched).await?;
        sqlx::query("UPDATE users SET phone_verified_at = NOW(6), last_login_at = NOW(6) WHERE id = ? AND phone = ?")
            .bind(user_id)
            .bind(&phone)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        events::record(&mut tx, events::PHONE_VERIFIED, EventSubject::user(user_id), meta, json!({})).await?;
        let tenant = default_tenant(&mut tx, user_id).await?;
        let tokens = self.start_session(&mut tx, user_id, tenant, meta).await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(tokens)
    }

    // ------------------------------------------------------------------- login

    /// US-AUTH-02 with the AUTH-R03 lockout. Unknown users and wrong passwords
    /// get the same answer in about the same time.
    pub async fn login(
        &self,
        pool: &MySqlPool,
        raw_identifier: &str,
        password: String,
        meta: &RequestMeta,
    ) -> AppResult<Tokens> {
        let identifier = Identifier::parse(raw_identifier, "identifier")?;
        let ip = meta.ip_bytes();

        let (failures, retry_after): (i64, Option<i64>) = sqlx::query_as(
            "SELECT COUNT(*), TIMESTAMPDIFF(SECOND, NOW(6), MAX(created_at) + INTERVAL ? MINUTE) FROM auth_events \
             WHERE identifier = ? AND ip_address <=> ? AND event = ? AND created_at > NOW(6) - INTERVAL ? MINUTE \
             /* tenant-lint: allow security log, not tenant data */",
        )
        .bind(LOCKOUT_MINUTES)
        .bind(identifier.as_str())
        .bind(&ip)
        .bind(events::LOGIN_FAILURE)
        .bind(LOCKOUT_MINUTES)
        .fetch_one(pool)
        .await
        .map_err(map_db_error)?;
        if failures >= MAX_LOGIN_FAILURES {
            return Err(locked_out(retry_after));
        }

        let user = find_user(pool, &identifier).await?;
        let valid = password::verify(password, user.as_ref().map(|u| u.password_hash.clone())).await?;
        let mut tx = pool.begin().await.map_err(map_db_error)?;
        let Some(user) = user.filter(|_| valid) else {
            let subject = EventSubject { identifier: Some(identifier.as_str()), ..EventSubject::default() };
            events::record(&mut tx, events::LOGIN_FAILURE, subject, meta, json!({})).await?;
            let locked = failures + 1 >= MAX_LOGIN_FAILURES;
            if locked {
                events::record(&mut tx, events::LOCKOUT, subject, meta, json!({ "minutes": LOCKOUT_MINUTES })).await?;
            }
            tx.commit().await.map_err(map_db_error)?;
            return Err(if locked {
                locked_out(Some(LOCKOUT_MINUTES * 60))
            } else {
                AppError::AuthFailed {
                    code: "INVALID_CREDENTIALS",
                    message: "the mobile number, email or password is wrong".into(),
                }
            });
        };
        if user.status != "active" {
            return Err(AppError::Denied { code: "ACCOUNT_DISABLED", message: "this account is disabled".into() });
        }

        sqlx::query("UPDATE users SET last_login_at = NOW(6) WHERE id = ?")
            .bind(user.id)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        events::record(&mut tx, events::LOGIN_SUCCESS, EventSubject::user(user.id), meta, json!({})).await?;
        let tenant = default_tenant(&mut tx, user.id).await?;
        let tokens = self.start_session(&mut tx, user.id, tenant, meta).await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(tokens)
    }

    async fn start_session(
        &self,
        conn: &mut MySqlConnection,
        user_id: Uuid,
        tenant: Option<TenantClaim>,
        meta: &RequestMeta,
    ) -> AppResult<Tokens> {
        let issued =
            session::create(conn, user_id, tenant.map(|t| t.tenant_id), new_id(), meta, self.refresh_ttl_days).await?;
        self.tokens(user_id, issued, tenant)
    }

    fn tokens(&self, user_id: Uuid, issued: session::IssuedSession, tenant: Option<TenantClaim>) -> AppResult<Tokens> {
        Ok(Tokens {
            access_token: self.keys.issue(user_id, issued.session_id, tenant)?,
            expires_in: self.keys.ttl_secs(),
            refresh_token: issued.refresh_token,
            refresh_max_age_secs: self.refresh_ttl_days * 24 * 3600,
            tenant_id: tenant.map(|t| t.tenant_id),
        })
    }

    // --------------------------------------------------------- refresh/switch

    /// Rotates the refresh token (AUTH-R06) and issues a new access token for
    /// the session's tenant. If the membership ended, the new token has no tenant.
    pub async fn refresh(&self, pool: &MySqlPool, refresh_token: &str, meta: &RequestMeta) -> AppResult<Tokens> {
        self.rotate(pool, refresh_token, None, TenantChoice::Keep, meta).await
    }

    /// Signs the session in to another business the user belongs to (or out of
    /// any business with `None`), rotating the refresh token.
    pub async fn switch_tenant(
        &self,
        pool: &MySqlPool,
        refresh_token: &str,
        user_id: Uuid,
        tenant_id: Option<Uuid>,
        meta: &RequestMeta,
    ) -> AppResult<Tokens> {
        self.rotate(pool, refresh_token, Some(user_id), TenantChoice::Switch(tenant_id), meta).await
    }

    async fn rotate(
        &self,
        pool: &MySqlPool,
        refresh_token: &str,
        expected_user: Option<Uuid>,
        choice: TenantChoice,
        meta: &RequestMeta,
    ) -> AppResult<Tokens> {
        let mut tx = pool.begin().await.map_err(map_db_error)?;
        let Some(current) = session::lock_by_token(&mut tx, refresh_token).await? else {
            return Err(session::invalid());
        };
        if current.revoked {
            if current.revoked_reason.as_deref() == Some(session::REASON_ROTATED) {
                // AC-AUTH-02: an old token came back, so someone else may hold the family.
                let revoked =
                    session::revoke_family(&mut tx, current.family_id, current.user_id, session::REASON_REUSE).await?;
                let subject =
                    EventSubject { user_id: Some(current.user_id), tenant_id: current.tenant_id, identifier: None };
                events::record(&mut tx, events::REFRESH_REUSE, subject, meta, json!({ "revoked_sessions": revoked }))
                    .await?;
                tx.commit().await.map_err(map_db_error)?;
                tracing::warn!(user_id = %current.user_id, family_id = %current.family_id, "refresh token reuse detected");
                return Err(AppError::AuthFailed {
                    code: "REFRESH_REUSED",
                    message: "this session was used elsewhere and has been signed out".into(),
                });
            }
            return Err(session::invalid());
        }
        if current.expired || expected_user.is_some_and(|u| u != current.user_id) {
            return Err(session::invalid());
        }

        let status: Option<String> = sqlx::query_scalar("SELECT status FROM users WHERE id = ?")
            .bind(current.user_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_db_error)?;
        if status.as_deref() != Some("active") {
            session::revoke_all_for_user(&mut tx, current.user_id, session::REASON_ACCOUNT_DISABLED).await?;
            tx.commit().await.map_err(map_db_error)?;
            return Err(AppError::Denied { code: "ACCOUNT_DISABLED", message: "this account is disabled".into() });
        }

        let tenant = match choice {
            TenantChoice::Keep => match current.tenant_id {
                Some(tid) => active_membership(&mut tx, tid, current.user_id).await?.ok(),
                None => None,
            },
            TenantChoice::Switch(None) => None,
            TenantChoice::Switch(Some(tid)) => Some(active_membership(&mut tx, tid, current.user_id).await??),
        };

        let issued = session::create(
            &mut tx,
            current.user_id,
            tenant.map(|t| t.tenant_id),
            current.family_id,
            meta,
            self.refresh_ttl_days,
        )
        .await?;
        session::mark_rotated(&mut tx, current.id, issued.session_id).await?;
        if let TenantChoice::Switch(tenant_id) = choice {
            let subject = EventSubject { user_id: Some(current.user_id), tenant_id, identifier: None };
            events::record(&mut tx, events::TENANT_SWITCHED, subject, meta, json!({})).await?;
        }
        tx.commit().await.map_err(map_db_error)?;
        self.tokens(current.user_id, issued, tenant)
    }

    // ------------------------------------------------------------------ logout

    /// Ends the session of this device. Unknown tokens are ignored.
    pub async fn logout(&self, pool: &MySqlPool, refresh_token: &str, meta: &RequestMeta) -> AppResult<()> {
        let mut tx = pool.begin().await.map_err(map_db_error)?;
        if let Some(current) = session::lock_by_token(&mut tx, refresh_token).await? {
            session::revoke_family(&mut tx, current.family_id, current.user_id, session::REASON_LOGOUT).await?;
            let subject =
                EventSubject { user_id: Some(current.user_id), tenant_id: current.tenant_id, identifier: None };
            events::record(&mut tx, events::LOGOUT, subject, meta, json!({})).await?;
        }
        tx.commit().await.map_err(map_db_error)
    }

    // -------------------------------------------------------- password reset

    /// US-AUTH-03 step 1. Same answer whether or not the account exists.
    pub async fn forgot_password(&self, pool: &MySqlPool, raw_identifier: &str) -> AppResult<OtpChallenge> {
        let identifier = Identifier::parse(raw_identifier, "identifier")?;
        let user = find_user(pool, &identifier).await?.filter(|u| u.status == "active");
        let Some(user) = user else {
            return Ok(self.challenge(identifier.as_str(), None));
        };
        let mut tx = pool.begin().await.map_err(map_db_error)?;
        let issued =
            otp::issue(&mut tx, &self.otp_pepper, Some(user.id), identifier.as_str(), OtpPurpose::ResetPassword)
                .await?;
        tx.commit().await.map_err(map_db_error)?;
        self.deliver(identifier.as_str(), OtpPurpose::ResetPassword, &issued);
        Ok(self.challenge(identifier.as_str(), Some(issued)))
    }

    /// US-AUTH-03 step 2. Sets the new password and signs out every session.
    pub async fn reset_password(
        &self,
        pool: &MySqlPool,
        raw_identifier: &str,
        code: &str,
        new_password: String,
        meta: &RequestMeta,
    ) -> AppResult<()> {
        let identifier = Identifier::parse(raw_identifier, "identifier")?;
        let Some(user) = find_user(pool, &identifier).await?.filter(|u| u.status == "active") else {
            return Err(AppError::Unprocessable {
                code: "OTP_INVALID",
                message: "the code is wrong or has expired".into(),
            });
        };
        let personal =
            [user.full_name.as_str(), user.phone.as_deref().unwrap_or(""), user.email.as_deref().unwrap_or("")];
        password::validate_policy(&new_password, "new_password", &personal)?;
        let matched = otp::check(pool, &self.otp_pepper, identifier.as_str(), OtpPurpose::ResetPassword, code).await?;
        if matched.user_id != Some(user.id) {
            return Err(AppError::Unprocessable {
                code: "OTP_INVALID",
                message: "the code is wrong or has expired".into(),
            });
        }
        let password_hash = password::hash(new_password).await?;

        let mut tx = pool.begin().await.map_err(map_db_error)?;
        otp::consume(&mut tx, matched).await?;
        // Receiving the code proves the user controls that phone or mailbox.
        let verify_sql = match identifier {
            Identifier::Phone(_) => {
                "UPDATE users SET password_hash = ?, phone_verified_at = COALESCE(phone_verified_at, NOW(6)) WHERE id = ?"
            }
            Identifier::Email(_) => {
                "UPDATE users SET password_hash = ?, email_verified_at = COALESCE(email_verified_at, NOW(6)) WHERE id = ?"
            }
        };
        sqlx::query(verify_sql).bind(&password_hash).bind(user.id).execute(&mut *tx).await.map_err(map_db_error)?;
        let revoked = session::revoke_all_for_user(&mut tx, user.id, session::REASON_PASSWORD_RESET).await?;
        events::record(
            &mut tx,
            events::PASSWORD_RESET,
            EventSubject::user(user.id),
            meta,
            json!({ "revoked_sessions": revoked }),
        )
        .await?;
        tx.commit().await.map_err(map_db_error)
    }

    // ---------------------------------------------------------------- sessions

    pub async fn list_sessions(&self, pool: &MySqlPool, claims: &AccessClaims) -> AppResult<Vec<SessionInfo>> {
        session::list_active(pool, claims.sub, claims.sid).await
    }

    /// US-AUTH-05 for the user's own devices.
    pub async fn revoke_session(
        &self,
        pool: &MySqlPool,
        user_id: Uuid,
        family_id: Uuid,
        meta: &RequestMeta,
    ) -> AppResult<()> {
        let mut tx = pool.begin().await.map_err(map_db_error)?;
        if session::revoke_family(&mut tx, family_id, user_id, session::REASON_USER_REVOKED).await? == 0 {
            return Err(AppError::NotFound);
        }
        events::record(
            &mut tx,
            events::SESSION_REVOKED,
            EventSubject::user(user_id),
            meta,
            json!({ "family_id": family_id }),
        )
        .await?;
        tx.commit().await.map_err(map_db_error)
    }
}

fn locked_out(retry_after: Option<i64>) -> AppError {
    AppError::RateLimited {
        code: "TOO_MANY_ATTEMPTS",
        message: "too many failed sign-in attempts, try again later".into(),
        retry_after_secs: retry_after.unwrap_or(LOCKOUT_MINUTES * 60).max(1).unsigned_abs(),
    }
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    full_name: String,
    phone: Option<String>,
    email: Option<String>,
    password_hash: String,
    status: String,
}

async fn find_user(pool: &MySqlPool, identifier: &Identifier) -> AppResult<Option<UserRow>> {
    let sql = match identifier {
        Identifier::Phone(_) => "SELECT id, full_name, phone, email, password_hash, status FROM users WHERE phone = ?",
        Identifier::Email(_) => "SELECT id, full_name, phone, email, password_hash, status FROM users WHERE email = ?",
    };
    sqlx::query_as(sql).bind(identifier.as_str()).fetch_optional(pool).await.map_err(map_db_error)
}

/// The membership to sign in to: `Ok(claim)`, or `Err` if it is not usable.
async fn active_membership(
    conn: &mut MySqlConnection,
    tenant_id: Uuid,
    user_id: Uuid,
) -> AppResult<AppResult<TenantClaim>> {
    let row: Option<(Uuid, i32, String, String)> = sqlx::query_as(
        "SELECT m.id, m.perms_version, m.status, t.status FROM tenant_memberships m \
         JOIN tenants t ON t.id = m.tenant_id WHERE m.tenant_id = ? AND m.user_id = ?",
    )
    .bind(tenant_id)
    .bind(user_id)
    .fetch_optional(conn)
    .await
    .map_err(map_db_error)?;
    Ok(match row {
        Some((membership_id, perms_version, member_status, tenant_status)) if member_status == "active" => {
            check_tenant_status(&tenant_status).map(|()| TenantClaim { tenant_id, membership_id, perms_version })
        }
        // Not a member (or no longer): indistinguishable from a tenant that does not exist.
        _ => Err(AppError::NotFound),
    })
}

/// Signs straight in to the only business of a user with exactly one.
async fn default_tenant(conn: &mut MySqlConnection, user_id: Uuid) -> AppResult<Option<TenantClaim>> {
    let rows: Vec<(Uuid, Uuid, i32)> = sqlx::query_as(
        "SELECT m.tenant_id, m.id, m.perms_version FROM tenant_memberships m JOIN tenants t ON t.id = m.tenant_id \
         WHERE m.user_id = ? AND m.status = 'active' AND t.status = 'active' LIMIT 2 \
         /* tenant-lint: allow the caller's own memberships across tenants (DB design §3.4) */",
    )
    .bind(user_id)
    .fetch_all(conn)
    .await
    .map_err(map_db_error)?;
    Ok(match rows.as_slice() {
        [(tenant_id, membership_id, perms_version)] => {
            Some(TenantClaim { tenant_id: *tenant_id, membership_id: *membership_id, perms_version: *perms_version })
        }
        _ => None,
    })
}

pub async fn user_profile(pool: &MySqlPool, user_id: Uuid) -> AppResult<UserProfile> {
    let row: Option<UserProfile> = sqlx::query_as(
        "SELECT id, full_name, phone, email, phone_verified_at IS NOT NULL AS phone_verified, locale FROM users \
         WHERE id = ? AND status = 'active'",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(map_db_error)?;
    row.ok_or_else(|| AppError::AuthFailed { code: "ACCOUNT_DISABLED", message: "this account is disabled".into() })
}

/// The businesses the user can sign in to (DB design §3.4 special case).
pub async fn memberships(pool: &MySqlPool, user_id: Uuid) -> AppResult<Vec<MembershipSummary>> {
    let rows: Vec<(Uuid, Uuid, String, String, String, bool)> = sqlx::query_as(
        "SELECT m.tenant_id, m.id, t.name, t.slug, t.status, m.is_owner FROM tenant_memberships m \
         JOIN tenants t ON t.id = m.tenant_id \
         WHERE m.user_id = ? AND m.status = 'active' AND t.status IN ('active', 'suspended') ORDER BY t.name \
         /* tenant-lint: allow the caller's own memberships across tenants (DB design §3.4) */",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(map_db_error)?;
    Ok(rows
        .into_iter()
        .map(|(tenant_id, membership_id, tenant_name, tenant_slug, tenant_status, is_owner)| MembershipSummary {
            tenant_id,
            membership_id,
            tenant_name,
            tenant_slug,
            tenant_status,
            is_owner,
        })
        .collect())
}
