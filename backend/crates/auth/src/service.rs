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
/// Failed sign-ins for one account from all IPs together before it locks for
/// [`LOCKOUT_MINUTES`] (the per-IP limit is [`MAX_LOGIN_FAILURES`]).
pub const MAX_ACCOUNT_FAILURES: i64 = 20;

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

/// Where a code was sent. `dev_code` is only filled when `AUTH_EXPOSE_OTP` is
/// on, or for sign-up when `AUTH_PHONE_VERIFICATION` is off (`auto_verify`).
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct OtpChallenge {
    pub destination: String,
    pub expires_in: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dev_code: Option<String>,
    /// Phone verification is off: the client submits `dev_code` itself.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub auto_verify: bool,
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
    phone_verification: bool,
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
            phone_verification: cfg.phone_verification,
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
            auto_verify: false,
        }
    }

    /// Challenge for the sign-up code. With phone verification off, the code
    /// is handed back so the client can confirm the number without an SMS.
    /// Only this purpose: a password-reset code must never be returned.
    fn verify_phone_challenge(&self, destination: &str, issued: Option<otp::IssuedOtp>) -> OtpChallenge {
        if self.phone_verification {
            return self.challenge(destination, issued);
        }
        let mut challenge = self.challenge(destination, None);
        challenge.auto_verify = issued.is_some();
        challenge.dev_code = issued.map(|i| i.code);
        challenge
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
            // An unverified account proves nothing yet; whoever verifies the
            // phone owns it. Whoever started it loses any session it had.
            session::revoke_all_for_user(&mut tx, user_id, "account_reclaimed").await?;
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
        Ok(self.verify_phone_challenge(&phone, Some(issued)))
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
        Ok(self.verify_phone_challenge(&phone, Some(issued)))
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
        // The number is proven now: sessions from before (when the account
        // proved nothing) must not carry over to its real owner.
        session::revoke_all_for_user(&mut tx, user_id, "phone_verified").await?;
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
        // One check per identifier at a time: the failure count is read before
        // the (slow) password check and written after it, so parallel guesses
        // would all see the old count and slip past the lockout.
        let _in_flight = LoginInFlight::claim(identifier.as_str())?;

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
        // The per-IP limit alone is beaten by changing IPs (botnet, rotating
        // proxies), so the account also locks after more failures from anywhere.
        let (account_failures, account_retry): (i64, Option<i64>) = sqlx::query_as(
            "SELECT COUNT(*), TIMESTAMPDIFF(SECOND, NOW(6), MAX(created_at) + INTERVAL ? MINUTE) FROM auth_events \
             WHERE identifier = ? AND event = ? AND created_at > NOW(6) - INTERVAL ? MINUTE \
             /* tenant-lint: allow security log, not tenant data */",
        )
        .bind(LOCKOUT_MINUTES)
        .bind(identifier.as_str())
        .bind(events::LOGIN_FAILURE)
        .bind(LOCKOUT_MINUTES)
        .fetch_one(pool)
        .await
        .map_err(map_db_error)?;
        if account_failures >= MAX_ACCOUNT_FAILURES {
            return Err(locked_out(account_retry));
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
        // An unverified sign-up proves nothing about the number: it gets no
        // session (else someone could register a stranger's number, sign in
        // and keep that session after the real owner takes the number over).
        // Only reached with the right password, so it reveals nothing to a
        // guesser; a fresh code is sent so the holder can finish verifying.
        if self.phone_verification && !user.phone_verified {
            // Best effort: past the send limit the earlier code still works.
            if let Some(phone) = user.phone.as_deref() {
                match otp::issue(&mut tx, &self.otp_pepper, Some(user.id), phone, OtpPurpose::VerifyPhone).await {
                    Ok(issued) => {
                        tx.commit().await.map_err(map_db_error)?;
                        self.deliver(phone, OtpPurpose::VerifyPhone, &issued);
                    }
                    // Usually the send limit; not an error for the caller.
                    Err(err) => tracing::info!(user_id = %user.id, error = %err, "no fresh verification code on login"),
                }
            }
            return Err(AppError::Denied {
                code: "PHONE_NOT_VERIFIED",
                message: "confirm your mobile number with the code we sent before signing in".into(),
            });
        }

        // With phone verification off, signing in with the password is enough;
        // otherwise accounts made before it was turned off stay stuck on /verify.
        let update = if self.phone_verification {
            "UPDATE users SET last_login_at = NOW(6) WHERE id = ?"
        } else {
            "UPDATE users SET last_login_at = NOW(6), \
             phone_verified_at = IF(phone IS NULL, NULL, COALESCE(phone_verified_at, NOW(6))) WHERE id = ?"
        };
        sqlx::query(update).bind(user.id).execute(&mut *tx).await.map_err(map_db_error)?;
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
            session::create(conn, user_id, tenant.map(|t| t.tenant_id), new_id(), None, meta, self.refresh_ttl_days)
                .await?;
        self.tokens(user_id, issued, tenant, None)
    }

    fn tokens(
        &self,
        user_id: Uuid,
        issued: session::IssuedSession,
        tenant: Option<TenantClaim>,
        impersonator: Option<Uuid>,
    ) -> AppResult<Tokens> {
        Ok(Tokens {
            access_token: self.keys.issue(user_id, issued.session_id, tenant, impersonator)?,
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
        if current.impersonator_id.is_some() && matches!(choice, TenantChoice::Switch(_)) {
            return Err(impersonating());
        }
        if let Some(owner_id) = current.impersonator_id
            && !impersonation_still_valid(&mut tx, &current, owner_id).await?
        {
            session::revoke_family(&mut tx, current.family_id, current.user_id, session::REASON_IMPERSONATION_ENDED)
                .await?;
            tx.commit().await.map_err(map_db_error)?;
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

        // An impersonation lives only inside its business: once the staff member
        // has lost access there, it ends instead of continuing without a tenant.
        if current.impersonator_id.is_some() && tenant.is_none() {
            session::revoke_family(&mut tx, current.family_id, current.user_id, session::REASON_IMPERSONATION_ENDED)
                .await?;
            tx.commit().await.map_err(map_db_error)?;
            return Err(session::invalid());
        }

        let issued = session::create(
            &mut tx,
            current.user_id,
            tenant.map(|t| t.tenant_id),
            current.family_id,
            current.impersonator_id,
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
        self.tokens(current.user_id, issued, tenant, current.impersonator_id)
    }

    // ---------------------------------------------------------- impersonation

    /// The owner of the token's business signs in as one of its staff
    /// (`membership_id`). The owner's session on this device ends and a new
    /// one starts for the staff member, marked with the owner, limited to this
    /// business; [`Self::stop_impersonation`] goes back. Owners and the
    /// caller themself cannot be chosen; other businesses' members are 404.
    pub async fn impersonate(
        &self,
        pool: &MySqlPool,
        refresh_token: &str,
        claims: &AccessClaims,
        membership_id: Uuid,
        meta: &RequestMeta,
    ) -> AppResult<Tokens> {
        if claims.imp.is_some() {
            return Err(impersonating());
        }
        let Some(tenant_id) = claims.tid else {
            return Err(AppError::Denied { code: "TENANT_REQUIRED", message: "choose a business first".into() });
        };
        let mut tx = pool.begin().await.map_err(map_db_error)?;
        let Some(current) = session::lock_by_token(&mut tx, refresh_token).await? else {
            return Err(session::invalid());
        };
        if current.revoked || current.expired || current.user_id != claims.sub || current.tenant_id != Some(tenant_id) {
            return Err(session::invalid());
        }

        let owner: Option<(bool, String)> =
            sqlx::query_as("SELECT is_owner, status FROM tenant_memberships WHERE tenant_id = ? AND user_id = ?")
                .bind(tenant_id)
                .bind(claims.sub)
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_db_error)?;
        if !matches!(owner, Some((true, ref status)) if status == "active") {
            return Err(AppError::Denied {
                code: "OWNER_ONLY",
                message: "only the business owner can sign in as a staff member".into(),
            });
        }

        let target: Option<(Uuid, bool, String, String)> = sqlx::query_as(
            "SELECT m.user_id, m.is_owner, m.status, u.status FROM tenant_memberships m \
             JOIN users u ON u.id = m.user_id WHERE m.tenant_id = ? AND m.id = ? AND m.status <> 'removed'",
        )
        .bind(tenant_id)
        .bind(membership_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let Some((target_user, target_is_owner, member_status, user_status)) = target else {
            return Err(AppError::NotFound);
        };
        if target_user == claims.sub || target_is_owner {
            return Err(AppError::Denied {
                code: "CANNOT_IMPERSONATE",
                message: "you cannot sign in as yourself or as an owner".into(),
            });
        }
        if member_status != "active" || user_status != "active" {
            return Err(AppError::Conflict {
                code: "MEMBER_NOT_ACTIVE",
                message: "this staff member is not active; reactivate them first".into(),
            });
        }
        let tenant = active_membership(&mut tx, tenant_id, target_user).await??;

        session::revoke_family(&mut tx, current.family_id, claims.sub, session::REASON_IMPERSONATION_STARTED).await?;
        let issued = session::create(
            &mut tx,
            target_user,
            Some(tenant_id),
            new_id(),
            Some(claims.sub),
            meta,
            self.refresh_ttl_days,
        )
        .await?;
        let subject = EventSubject { user_id: Some(claims.sub), tenant_id: Some(tenant_id), identifier: None };
        events::record(
            &mut tx,
            events::IMPERSONATION_STARTED,
            subject,
            meta,
            json!({ "target_user_id": target_user, "membership_id": membership_id }),
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        self.tokens(target_user, issued, Some(tenant), Some(claims.sub))
    }

    /// Platform support: a platform staff member opens a business by signing in
    /// as its owner (the same impersonation machinery as an owner signing in as
    /// staff). The caller has already checked the staff role; this re-checks it
    /// inside the transaction. The staff member's own session is replaced and
    /// `POST /auth/impersonate/stop` brings them back with no business chosen
    /// (the admin console). Audit entries written meanwhile name the staff
    /// member as impersonator. Refused for the platform tenant itself.
    pub async fn open_business(
        &self,
        pool: &MySqlPool,
        refresh_token: &str,
        claims: &AccessClaims,
        tenant_id: Uuid,
        meta: &RequestMeta,
    ) -> AppResult<Tokens> {
        if claims.imp.is_some() {
            return Err(impersonating());
        }
        let mut tx = pool.begin().await.map_err(map_db_error)?;
        let Some(current) = session::lock_by_token(&mut tx, refresh_token).await? else {
            return Err(session::invalid());
        };
        if current.revoked || current.expired || current.user_id != claims.sub || current.impersonator_id.is_some() {
            return Err(session::invalid());
        }
        if !platform_support_staff(&mut tx, claims.sub).await? {
            return Err(AppError::Forbidden);
        }

        let slug: Option<String> = sqlx::query_scalar("SELECT slug FROM tenants WHERE id = ?")
            .bind(tenant_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_db_error)?;
        if slug.is_none_or(|s| s == PLATFORM_TENANT_SLUG) {
            return Err(AppError::NotFound);
        }
        let owner: Option<(Uuid, String, String)> = sqlx::query_as(
            "SELECT m.user_id, m.status, u.status FROM tenant_memberships m JOIN users u ON u.id = m.user_id \
             WHERE m.tenant_id = ? AND m.is_owner ORDER BY m.status = 'active' DESC, m.created_at LIMIT 1",
        )
        .bind(tenant_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let Some((owner_id, member_status, user_status)) = owner else {
            return Err(AppError::Conflict {
                code: "NO_OWNER",
                message: "this business has no owner to sign in as".into(),
            });
        };
        if member_status != "active" || user_status != "active" {
            return Err(AppError::Conflict {
                code: "OWNER_NOT_ACTIVE",
                message: "the owner of this business is not active".into(),
            });
        }
        if owner_id == claims.sub {
            return Err(AppError::Denied {
                code: "CANNOT_IMPERSONATE",
                message: "you own this business; switch to it instead".into(),
            });
        }
        let tenant = active_membership(&mut tx, tenant_id, owner_id).await??;

        session::revoke_family(&mut tx, current.family_id, claims.sub, session::REASON_IMPERSONATION_STARTED).await?;
        let issued = session::create(
            &mut tx,
            owner_id,
            Some(tenant_id),
            new_id(),
            Some(claims.sub),
            meta,
            self.refresh_ttl_days,
        )
        .await?;
        let subject = EventSubject { user_id: Some(claims.sub), tenant_id: Some(tenant_id), identifier: None };
        events::record(
            &mut tx,
            events::IMPERSONATION_STARTED,
            subject,
            meta,
            json!({ "target_user_id": owner_id, "platform_support": true }),
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        self.tokens(owner_id, issued, Some(tenant), Some(claims.sub))
    }

    /// Ends an impersonation and signs the owner back in to the same business
    /// (or with no business if they are no longer its active owner).
    pub async fn stop_impersonation(
        &self,
        pool: &MySqlPool,
        refresh_token: &str,
        claims: &AccessClaims,
        meta: &RequestMeta,
    ) -> AppResult<Tokens> {
        let mut tx = pool.begin().await.map_err(map_db_error)?;
        let Some(current) = session::lock_by_token(&mut tx, refresh_token).await? else {
            return Err(session::invalid());
        };
        let Some(owner_id) = current.impersonator_id else {
            return Err(AppError::Conflict {
                code: "NOT_IMPERSONATING",
                message: "you are not signed in as someone else".into(),
            });
        };
        if current.revoked || current.expired || current.user_id != claims.sub || claims.imp != Some(owner_id) {
            return Err(session::invalid());
        }
        let still_valid = impersonation_still_valid(&mut tx, &current, owner_id).await?;
        session::revoke_family(&mut tx, current.family_id, current.user_id, session::REASON_IMPERSONATION_ENDED)
            .await?;
        // Going back mints an owner session without a password, so it is only
        // allowed while the owner is still the active owner here and within the
        // impersonation's lifetime; otherwise the owner signs in again.
        if !still_valid {
            tx.commit().await.map_err(map_db_error)?;
            return Err(session::invalid());
        }
        let tenant = match current.tenant_id {
            Some(tid) => active_membership(&mut tx, tid, owner_id).await?.ok(),
            None => None,
        };
        let issued = session::create(
            &mut tx,
            owner_id,
            tenant.map(|t| t.tenant_id),
            new_id(),
            None,
            meta,
            self.refresh_ttl_days,
        )
        .await?;
        let subject = EventSubject { user_id: Some(owner_id), tenant_id: current.tenant_id, identifier: None };
        events::record(
            &mut tx,
            events::IMPERSONATION_ENDED,
            subject,
            meta,
            json!({ "target_user_id": current.user_id }),
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        self.tokens(owner_id, issued, tenant, None)
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
        // The general rules come first and the personal ones only after the code
        // matched: otherwise "password too short" for a real account versus
        // "wrong code" for an unknown one would tell anyone who has an account.
        password::validate_policy(&new_password, "new_password", &[])?;
        let Some(user) = find_user(pool, &identifier).await?.filter(|u| u.status == "active") else {
            return Err(AppError::Unprocessable {
                code: "OTP_INVALID",
                message: "the code is wrong or has expired".into(),
            });
        };
        let matched = otp::check(pool, &self.otp_pepper, identifier.as_str(), OtpPurpose::ResetPassword, code).await?;
        if matched.user_id != Some(user.id) {
            return Err(AppError::Unprocessable {
                code: "OTP_INVALID",
                message: "the code is wrong or has expired".into(),
            });
        }
        let personal =
            [user.full_name.as_str(), user.phone.as_deref().unwrap_or(""), user.email.as_deref().unwrap_or("")];
        password::validate_policy(&new_password, "new_password", &personal)?;
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

    /// Changes the signed-in user's password after checking the current one,
    /// and signs out their other devices (this one stays signed in). Wrong
    /// current passwords count like failed sign-ins: five in 15 minutes lock it.
    pub async fn change_password(
        &self,
        pool: &MySqlPool,
        claims: &AccessClaims,
        current_password: String,
        new_password: String,
        meta: &RequestMeta,
    ) -> AppResult<u64> {
        let user_id = claims.sub;
        let (failures, retry_after): (i64, Option<i64>) = sqlx::query_as(
            "SELECT COUNT(*), TIMESTAMPDIFF(SECOND, NOW(6), MAX(created_at) + INTERVAL ? MINUTE) FROM auth_events \
             WHERE user_id = ? AND event = ? AND created_at > NOW(6) - INTERVAL ? MINUTE \
             /* tenant-lint: allow security log, not tenant data */",
        )
        .bind(LOCKOUT_MINUTES)
        .bind(user_id)
        .bind(events::PASSWORD_CHANGE_FAILURE)
        .bind(LOCKOUT_MINUTES)
        .fetch_one(pool)
        .await
        .map_err(map_db_error)?;
        if failures >= MAX_LOGIN_FAILURES {
            return Err(locked_out(retry_after));
        }
        let user: Option<(String, String, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT password_hash, full_name, phone, email FROM users WHERE id = ? AND status = 'active'",
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(map_db_error)?;
        let (hash, full_name, phone, email) = user.ok_or(AppError::Unauthorized)?;
        if !password::verify(current_password.clone(), Some(hash)).await? {
            let mut tx = pool.begin().await.map_err(map_db_error)?;
            events::record(&mut tx, events::PASSWORD_CHANGE_FAILURE, EventSubject::user(user_id), meta, json!({}))
                .await?;
            tx.commit().await.map_err(map_db_error)?;
            return Err(AppError::field("current_password", "WRONG_PASSWORD", "the current password is wrong"));
        }
        if new_password == current_password {
            return Err(AppError::field(
                "new_password",
                "SAME_PASSWORD",
                "choose a password different from the current one",
            ));
        }
        let personal = [full_name.as_str(), phone.as_deref().unwrap_or(""), email.as_deref().unwrap_or("")];
        password::validate_policy(&new_password, "new_password", &personal)?;
        let password_hash = password::hash(new_password).await?;

        let mut tx = pool.begin().await.map_err(map_db_error)?;
        sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
            .bind(&password_hash)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        let revoked = session::revoke_others(&mut tx, user_id, claims.sid, session::REASON_PASSWORD_CHANGED).await?;
        events::record(
            &mut tx,
            events::PASSWORD_CHANGED,
            EventSubject::user(user_id),
            meta,
            json!({ "revoked_sessions": revoked }),
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(revoked)
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

/// Refused while the owner is signed in as a staff member.
pub fn impersonating() -> AppError {
    AppError::Denied {
        code: "IMPERSONATING",
        message: "not available while signed in as a staff member; return to your account first".into(),
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
    phone_verified: bool,
}

async fn find_user(pool: &MySqlPool, identifier: &Identifier) -> AppResult<Option<UserRow>> {
    let sql = match identifier {
        Identifier::Phone(_) => {
            "SELECT id, full_name, phone, email, password_hash, status, phone_verified_at IS NOT NULL AS phone_verified \
             FROM users WHERE phone = ?"
        }
        Identifier::Email(_) => {
            "SELECT id, full_name, phone, email, password_hash, status, phone_verified_at IS NOT NULL AS phone_verified \
             FROM users WHERE email = ?"
        }
    };
    sqlx::query_as(sql).bind(identifier.as_str()).fetch_optional(pool).await.map_err(map_db_error)
}

/// The membership to sign in to: `Ok(claim)`, or `Err` if it is not usable.
/// How long a "signed in as staff" session may last in total, refreshes included.
const IMPERSONATION_MAX_HOURS: i64 = 8;

/// An impersonation continues only while whoever started it is still an
/// active user and the active owner of that business (or active platform
/// support staff, see [`AuthService::open_business`]), and for at most
/// [`IMPERSONATION_MAX_HOURS`] from its start.
async fn impersonation_still_valid(
    conn: &mut MySqlConnection,
    current: &session::SessionRow,
    owner_id: Uuid,
) -> AppResult<bool> {
    let Some(tenant_id) = current.tenant_id else { return Ok(false) };
    let owner: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM tenant_memberships m JOIN users u ON u.id = m.user_id \
         WHERE m.tenant_id = ? AND m.user_id = ? AND m.is_owner AND m.status = 'active' AND u.status = 'active'",
    )
    .bind(tenant_id)
    .bind(owner_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(map_db_error)?;
    // Or a platform staff member who opened the business for support.
    if owner.is_none() && !platform_support_staff(conn, owner_id).await? {
        return Ok(false);
    }
    let started = session::family_started_at(conn, current.family_id).await?;
    Ok(started.is_some_and(|at| chrono::Utc::now() - at < chrono::Duration::hours(IMPERSONATION_MAX_HOURS)))
}

/// The platform tenant's slug (`pos_platform::tenants::PLATFORM_SLUG`); it is
/// never opened as a business.
const PLATFORM_TENANT_SLUG: &str = "platform";

/// Whether the user is active platform staff allowed to open businesses.
pub async fn platform_support_staff(conn: &mut MySqlConnection, user_id: Uuid) -> AppResult<bool> {
    // Roles allowed to open a business (ADM-R11): `StaffRole::can_open_business`.
    let found: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM platform_staff s JOIN users u ON u.id = s.user_id \
         WHERE s.user_id = ? AND s.status = 'active' AND u.status = 'active' \
         AND s.staff_role IN ('platform_owner', 'support')",
    )
    .bind(user_id)
    .fetch_optional(conn)
    .await
    .map_err(map_db_error)?;
    Ok(found.is_some())
}

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

/// Identifiers whose login is being checked right now (see [`Auth::login`]).
static LOGINS_IN_FLIGHT: std::sync::LazyLock<std::sync::Mutex<std::collections::HashSet<String>>> =
    std::sync::LazyLock::new(Default::default);

/// Holds an identifier in [`LOGINS_IN_FLIGHT`] until dropped.
struct LoginInFlight(String);

impl LoginInFlight {
    fn claim(identifier: &str) -> AppResult<Self> {
        let mut busy = LOGINS_IN_FLIGHT.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if !busy.insert(identifier.to_owned()) {
            return Err(AppError::RateLimited {
                code: "LOGIN_IN_PROGRESS",
                message: "a sign-in for this account is already being checked; try again".into(),
                retry_after_secs: 1,
            });
        }
        Ok(Self(identifier.to_owned()))
    }
}

impl Drop for LoginInFlight {
    fn drop(&mut self) {
        LOGINS_IN_FLIGHT.lock().unwrap_or_else(std::sync::PoisonError::into_inner).remove(&self.0);
    }
}
