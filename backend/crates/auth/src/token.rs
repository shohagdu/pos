//! Access tokens (JWT, AUTH-R05) and opaque refresh tokens (AUTH-R06).

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, errors::ErrorKind};
use pos_core::{AppError, AppResult, config::AuthConfig};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const ISSUER: &str = "pos-api";
const AUDIENCE: &str = "pos";

/// Claims of an access token. `tid`/`mid`/`pv` are present once the user has
/// picked a business; the server reads the tenant only from here (R-TEN-02).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccessClaims {
    pub sub: Uuid,
    /// Session (refresh-token row) that issued this token.
    pub sid: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tid: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mid: Option<Uuid>,
    /// Membership `perms_version` at issue time (AUTH-R07).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pv: Option<i32>,
    pub iat: u64,
    pub exp: u64,
    pub iss: String,
    pub aud: String,
}

/// The tenant part of a token: active tenant, membership and permission version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TenantClaim {
    pub tenant_id: Uuid,
    pub membership_id: Uuid,
    pub perms_version: i32,
}

pub struct TokenKeys {
    encoding: EncodingKey,
    decoding: Vec<DecodingKey>,
    ttl_secs: u64,
}

impl TokenKeys {
    pub fn from_config(cfg: &AuthConfig) -> Self {
        let mut decoding = vec![DecodingKey::from_secret(cfg.jwt_secret.expose().as_bytes())];
        if let Some(previous) = &cfg.jwt_secret_previous {
            decoding.push(DecodingKey::from_secret(previous.expose().as_bytes()));
        }
        Self {
            encoding: EncodingKey::from_secret(cfg.jwt_secret.expose().as_bytes()),
            decoding,
            ttl_secs: cfg.access_token_ttl_secs,
        }
    }

    pub fn ttl_secs(&self) -> u64 {
        self.ttl_secs
    }

    pub fn issue(&self, user_id: Uuid, session_id: Uuid, tenant: Option<TenantClaim>) -> AppResult<String> {
        let now = jsonwebtoken::get_current_timestamp();
        let claims = AccessClaims {
            sub: user_id,
            sid: session_id,
            tid: tenant.map(|t| t.tenant_id),
            mid: tenant.map(|t| t.membership_id),
            pv: tenant.map(|t| t.perms_version),
            iat: now,
            exp: now + self.ttl_secs,
            iss: ISSUER.to_owned(),
            aud: AUDIENCE.to_owned(),
        };
        jsonwebtoken::encode(&Header::new(Algorithm::HS256), &claims, &self.encoding).map_err(AppError::internal)
    }

    pub fn verify(&self, token: &str) -> AppResult<AccessClaims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[ISSUER]);
        validation.set_audience(&[AUDIENCE]);
        validation.set_required_spec_claims(&["exp", "sub", "iss", "aud"]);
        validation.leeway = 30;

        let mut last = AppError::AuthFailed { code: "TOKEN_INVALID", message: "the access token is invalid".into() };
        for key in &self.decoding {
            match jsonwebtoken::decode::<AccessClaims>(token, key, &validation) {
                Ok(data) => return Ok(data.claims),
                Err(err) if matches!(err.kind(), ErrorKind::ExpiredSignature) => {
                    last =
                        AppError::AuthFailed { code: "TOKEN_EXPIRED", message: "the access token has expired".into() };
                }
                Err(_) => {}
            }
        }
        Err(last)
    }
}

/// Cryptographically random bytes from the OS.
pub fn random_bytes<const N: usize>() -> AppResult<[u8; N]> {
    let mut buf = [0u8; N];
    getrandom::fill(&mut buf).map_err(|e| AppError::internal(e.to_string()))?;
    Ok(buf)
}

pub fn sha256(parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part);
    }
    hasher.finalize().into()
}

/// A new opaque secret token (256 random bits, URL-safe) and the SHA-256
/// hash that is stored instead of it. Used for refresh tokens and invitation links.
pub fn new_opaque_token() -> AppResult<(String, [u8; 32])> {
    let token = URL_SAFE_NO_PAD.encode(random_bytes::<32>()?);
    let hash = hash_opaque_token(&token);
    Ok((token, hash))
}

pub fn hash_opaque_token(token: &str) -> [u8; 32] {
    sha256(&[token.as_bytes()])
}

pub fn new_refresh_token() -> AppResult<(String, [u8; 32])> {
    new_opaque_token()
}

pub fn hash_refresh_token(token: &str) -> [u8; 32] {
    hash_opaque_token(token)
}

/// Constant-time comparison for secrets of equal length.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use pos_core::config::Secret;

    use super::*;

    fn cfg(secret: &str, previous: Option<&str>, ttl: u64) -> AuthConfig {
        AuthConfig {
            jwt_secret: Secret::new(secret),
            jwt_secret_previous: previous.map(Secret::new),
            access_token_ttl_secs: ttl,
            refresh_token_ttl_days: 30,
            cookie_secure: true,
            expose_otp: false,
        }
    }

    const KEY_A: &str = "a-test-secret-that-is-long-enough-000";
    const KEY_B: &str = "b-test-secret-that-is-long-enough-111";

    #[test]
    fn round_trips_claims() {
        let keys = TokenKeys::from_config(&cfg(KEY_A, None, 900));
        let tenant = TenantClaim { tenant_id: Uuid::now_v7(), membership_id: Uuid::now_v7(), perms_version: 3 };
        let (user, session) = (Uuid::now_v7(), Uuid::now_v7());
        let token = keys.issue(user, session, Some(tenant)).expect("issue");
        let claims = keys.verify(&token).expect("verify");
        assert_eq!((claims.sub, claims.sid, claims.tid, claims.pv), (user, session, Some(tenant.tenant_id), Some(3)));
        assert_eq!(claims.exp - claims.iat, 900);
    }

    #[test]
    fn rejects_other_keys_and_tampering() {
        let keys = TokenKeys::from_config(&cfg(KEY_A, None, 900));
        let other = TokenKeys::from_config(&cfg(KEY_B, None, 900));
        let token = other.issue(Uuid::now_v7(), Uuid::now_v7(), None).expect("issue");
        assert_eq!(keys.verify(&token).map_err(|e| e.code()).err(), Some("TOKEN_INVALID"));
        assert!(keys.verify("not.a.jwt").is_err());
    }

    #[test]
    fn accepts_previous_key_during_rotation() {
        let old = TokenKeys::from_config(&cfg(KEY_A, None, 900));
        let rotated = TokenKeys::from_config(&cfg(KEY_B, Some(KEY_A), 900));
        let token = old.issue(Uuid::now_v7(), Uuid::now_v7(), None).expect("issue");
        assert!(rotated.verify(&token).is_ok());
    }

    #[test]
    fn reports_expiry() {
        let keys = TokenKeys::from_config(&cfg(KEY_A, None, 900));
        let now = jsonwebtoken::get_current_timestamp();
        let claims = AccessClaims {
            sub: Uuid::now_v7(),
            sid: Uuid::now_v7(),
            tid: None,
            mid: None,
            pv: None,
            iat: now - 7200,
            exp: now - 3600,
            iss: ISSUER.into(),
            aud: AUDIENCE.into(),
        };
        let token =
            jsonwebtoken::encode(&Header::new(Algorithm::HS256), &claims, &EncodingKey::from_secret(KEY_A.as_bytes()))
                .expect("encode");
        assert_eq!(keys.verify(&token).map_err(|e| e.code()).err(), Some("TOKEN_EXPIRED"));
    }

    #[test]
    fn refresh_tokens_are_random_and_hashed() {
        let (a, hash_a) = new_refresh_token().expect("token");
        let (b, _) = new_refresh_token().expect("token");
        assert_ne!(a, b);
        assert_eq!(a.len(), 43);
        assert_eq!(hash_refresh_token(&a), hash_a);
        assert!(constant_time_eq(&hash_a, &hash_refresh_token(&a)));
        assert!(!constant_time_eq(&hash_a, &hash_refresh_token(&b)));
    }
}
