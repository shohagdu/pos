//! Password policy and Argon2id hashing (AUTH-R01, AUTH-R02).

use std::sync::OnceLock;

use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::{PasswordHasher, PasswordVerifier},
};
use pos_core::{AppError, AppResult};

pub const MIN_LENGTH: usize = 8;
/// Upper bound so a huge password cannot tie up a hashing thread.
pub const MAX_LENGTH: usize = 128;

// OWASP minimum for Argon2id: 19 MiB, 2 iterations, 1 lane.
const MEMORY_KIB: u32 = 19 * 1024;
const ITERATIONS: u32 = 2;
const LANES: u32 = 1;

/// Hash of a throwaway password, verified for unknown users so that a login
/// for a missing account takes as long as one with a wrong password.
static DUMMY_HASH: OnceLock<String> = OnceLock::new();

/// Frequently used passwords (subset of public breach lists), compared case-insensitively.
const COMMON: &[&str] = &[
    "password",
    "password1",
    "password12",
    "password123",
    "password1234",
    "passw0rd",
    "p@ssw0rd",
    "p@ssword",
    "12345678",
    "123456789",
    "1234567890",
    "0123456789",
    "87654321",
    "11111111",
    "00000000",
    "12341234",
    "11223344",
    "12121212",
    "123123123",
    "88888888",
    "99999999",
    "abcd1234",
    "abc12345",
    "1q2w3e4r",
    "1qaz2wsx",
    "qwertyui",
    "qwerty123",
    "qwerty12",
    "qwertyuiop",
    "asdfghjk",
    "asdf1234",
    "zxcvbnm1",
    "iloveyou",
    "iloveyou1",
    "sunshine",
    "princess",
    "football",
    "baseball",
    "superman",
    "trustno1",
    "welcome1",
    "welcome123",
    "letmein1",
    "admin123",
    "administrator",
    "changeme",
    "computer",
    "internet",
    "starwars",
    "whatever",
    "dragon12",
    "monkey12",
    "michael1",
    "shadow12",
    "master12",
    "jennifer",
    "bangladesh",
    "bangladesh1",
    "dhaka123",
    "dhaka1234",
    "allah786",
    "bismillah",
    "bismillah1",
    "786786786",
    "pakistan",
    "india123",
    "cricket1",
    "shakib75",
    "tamim123",
    "mashrafe",
    "mypassword",
    "secret12",
    "default1",
    "test1234",
    "testtest",
    "guest123",
    "user1234",
    "login123",
    "pos12345",
    "shop1234",
];

pub fn validate_policy(password: &str, field: &str, personal: &[&str]) -> AppResult<()> {
    let len = password.chars().count();
    if len < MIN_LENGTH {
        return Err(AppError::field(field, "PASSWORD_TOO_SHORT", format!("use at least {MIN_LENGTH} characters")));
    }
    if len > MAX_LENGTH {
        return Err(AppError::field(field, "PASSWORD_TOO_LONG", format!("use at most {MAX_LENGTH} characters")));
    }
    let lower = password.to_lowercase();
    if COMMON.contains(&lower.as_str()) || lower.chars().all(|c| lower.starts_with(c)) {
        return Err(AppError::field(field, "PASSWORD_TOO_COMMON", "this password is too easy to guess"));
    }
    let digits: String = password.chars().filter(char::is_ascii_digit).collect();
    for value in personal.iter().map(|v| v.trim().to_lowercase()).filter(|v| v.len() >= 4) {
        let value_digits: String = value.chars().filter(char::is_ascii_digit).collect();
        let is_phone = value_digits.len() >= 8;
        if lower == value || (is_phone && digits.len() >= 8 && value_digits.ends_with(&digits)) {
            return Err(AppError::field(field, "PASSWORD_PERSONAL", "do not use your name, phone or email"));
        }
    }
    Ok(())
}

fn hasher() -> AppResult<Argon2<'static>> {
    let params = Params::new(MEMORY_KIB, ITERATIONS, LANES, None).map_err(AppError::internal)?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
}

fn dummy_hash() -> AppResult<&'static str> {
    if let Some(hash) = DUMMY_HASH.get() {
        return Ok(hash);
    }
    let hash = hasher()?.hash_password(b"no such user, no such password").map_err(AppError::internal)?;
    Ok(DUMMY_HASH.get_or_init(|| hash.to_string()))
}

/// Hashes on the blocking pool: Argon2 takes tens of milliseconds by design.
pub async fn hash(password: String) -> AppResult<String> {
    tokio::task::spawn_blocking(move || {
        let hash = hasher()?.hash_password(password.as_bytes()).map_err(AppError::internal)?;
        Ok(hash.to_string())
    })
    .await
    .map_err(AppError::internal)?
}

/// `false` for a wrong password; errors only for a malformed stored hash.
pub async fn verify(password: String, stored: Option<String>) -> AppResult<bool> {
    tokio::task::spawn_blocking(move || {
        let known = stored.is_some();
        let stored = match stored {
            Some(hash) => hash,
            None => dummy_hash()?.to_owned(),
        };
        // Parameters come from the stored PHC string, not from `hasher()`.
        match hasher()?.verify_password(password.as_bytes(), stored.as_str()) {
            Ok(()) => Ok(known),
            Err(argon2::password_hash::Error::PasswordInvalid) => Ok(false),
            Err(err) => Err(AppError::internal(err)),
        }
    })
    .await
    .map_err(AppError::internal)?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code(result: AppResult<()>) -> Option<String> {
        result.err().and_then(|e| e.field_errors().first().map(|f| f.code.clone()))
    }

    #[test]
    fn enforces_policy() {
        assert_eq!(code(validate_policy("short1", "password", &[])).as_deref(), Some("PASSWORD_TOO_SHORT"));
        assert_eq!(code(validate_policy("Password123", "password", &[])).as_deref(), Some("PASSWORD_TOO_COMMON"));
        assert_eq!(code(validate_policy("aaaaaaaaaa", "password", &[])).as_deref(), Some("PASSWORD_TOO_COMMON"));
        assert_eq!(
            code(validate_policy("01712345678", "password", &["+8801712345678"])).as_deref(),
            Some("PASSWORD_PERSONAL")
        );
        assert_eq!(
            code(validate_policy("rahim@shop.bd", "password", &["Rahim@Shop.bd"])).as_deref(),
            Some("PASSWORD_PERSONAL")
        );
        assert!(validate_policy("correct horse battery", "password", &["Rahim", "+8801712345678"]).is_ok());
        assert_eq!(code(validate_policy(&"x1".repeat(65), "password", &[])).as_deref(), Some("PASSWORD_TOO_LONG"));
    }

    #[tokio::test]
    async fn hashes_and_verifies() {
        let hash = hash("correct horse battery".into()).await.expect("hash");
        assert!(hash.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"));
        assert!(verify("correct horse battery".into(), Some(hash.clone())).await.expect("verify"));
        assert!(!verify("wrong horse battery".into(), Some(hash)).await.expect("verify"));
    }

    #[tokio::test]
    async fn unknown_user_never_verifies() {
        assert!(!verify("anything at all".into(), None).await.expect("dummy verify"));
    }
}
