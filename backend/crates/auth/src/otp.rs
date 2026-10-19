//! One-time codes for phone verification and password reset (AUTH-R04):
//! 6 digits, valid 5 minutes, at most 5 attempts, at most 3 sends per 15
//! minutes per destination. Only a peppered hash is stored.

use pos_core::{AppError, AppResult, id::new_id};
use pos_db::map_db_error;
use sqlx::{MySqlConnection, MySqlPool};
use uuid::Uuid;

use crate::token::{constant_time_eq, random_bytes, sha256};

pub const CODE_TTL_SECS: u64 = 5 * 60;
pub const MAX_ATTEMPTS: i16 = 5;
pub const MAX_SENDS_PER_WINDOW: i64 = 3;
pub const SEND_WINDOW_MINUTES: i64 = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OtpPurpose {
    VerifyPhone,
    ResetPassword,
}

impl OtpPurpose {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::VerifyPhone => "verify_phone",
            Self::ResetPassword => "reset_password",
        }
    }
}

/// A code that was stored and must now be delivered.
#[derive(Debug, Clone)]
pub struct IssuedOtp {
    pub code: String,
    pub expires_in: u64,
}

/// A code that matched. The caller consumes it in the transaction that acts on it.
#[derive(Debug, Clone, Copy)]
pub struct MatchedOtp {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
}

fn code_hash(pepper: &[u8; 32], otp_id: Uuid, code: &str) -> [u8; 32] {
    sha256(&[pepper, otp_id.as_bytes(), code.as_bytes()])
}

/// Uniform 6-digit code (rejection sampling avoids modulo bias).
fn generate_code() -> AppResult<String> {
    const RANGE: u32 = 1_000_000;
    const LIMIT: u32 = u32::MAX - (u32::MAX % RANGE);
    loop {
        let n = u32::from_le_bytes(random_bytes::<4>()?);
        if n < LIMIT {
            return Ok(format!("{:06}", n % RANGE));
        }
    }
}

/// Stores a new code for `destination`, replacing any earlier unused one.
pub async fn issue(
    conn: &mut MySqlConnection,
    pepper: &[u8; 32],
    user_id: Option<Uuid>,
    destination: &str,
    purpose: OtpPurpose,
) -> AppResult<IssuedOtp> {
    let (sent, retry_after): (i64, Option<i64>) = sqlx::query_as(
        "SELECT COUNT(*), TIMESTAMPDIFF(SECOND, NOW(6), MIN(created_at) + INTERVAL ? MINUTE) \
         FROM otp_codes WHERE destination = ? AND purpose = ? AND created_at > NOW(6) - INTERVAL ? MINUTE",
    )
    .bind(SEND_WINDOW_MINUTES)
    .bind(destination)
    .bind(purpose.as_str())
    .bind(SEND_WINDOW_MINUTES)
    .fetch_one(&mut *conn)
    .await
    .map_err(map_db_error)?;
    if sent >= MAX_SENDS_PER_WINDOW {
        return Err(AppError::RateLimited {
            code: "OTP_RATE_LIMITED",
            message: "too many codes requested, try again later".into(),
            retry_after_secs: retry_after.unwrap_or(60).max(1).unsigned_abs(),
        });
    }

    sqlx::query(
        "UPDATE otp_codes SET expires_at = NOW(6) \
         WHERE destination = ? AND purpose = ? AND consumed_at IS NULL AND expires_at > NOW(6)",
    )
    .bind(destination)
    .bind(purpose.as_str())
    .execute(&mut *conn)
    .await
    .map_err(map_db_error)?;

    let id = new_id();
    let code = generate_code()?;
    sqlx::query(
        "INSERT INTO otp_codes (id, user_id, destination, purpose, code_hash, expires_at) \
         VALUES (?, ?, ?, ?, ?, NOW(6) + INTERVAL ? SECOND)",
    )
    .bind(id)
    .bind(user_id)
    .bind(destination)
    .bind(purpose.as_str())
    .bind(code_hash(pepper, id, &code).as_slice())
    .bind(CODE_TTL_SECS)
    .execute(&mut *conn)
    .await
    .map_err(map_db_error)?;
    Ok(IssuedOtp { code, expires_in: CODE_TTL_SECS })
}

fn invalid_code() -> AppError {
    AppError::Unprocessable { code: "OTP_INVALID", message: "the code is wrong or has expired".into() }
}

/// Checks `code` against the latest live code for `destination`. A wrong code
/// counts as an attempt (committed even though the call fails).
pub async fn check(
    pool: &MySqlPool,
    pepper: &[u8; 32],
    destination: &str,
    purpose: OtpPurpose,
    code: &str,
) -> AppResult<MatchedOtp> {
    let code = code.trim();
    if code.len() != 6 || !code.bytes().all(|b| b.is_ascii_digit()) {
        return Err(AppError::field("code", "OTP_FORMAT", "enter the 6-digit code"));
    }

    let mut tx = pool.begin().await.map_err(map_db_error)?;
    let row: Option<(Uuid, Option<Uuid>, Vec<u8>, i16)> = sqlx::query_as(
        "SELECT id, user_id, code_hash, attempts FROM otp_codes \
         WHERE destination = ? AND purpose = ? AND consumed_at IS NULL AND expires_at > NOW(6) \
         ORDER BY created_at DESC LIMIT 1 FOR UPDATE",
    )
    .bind(destination)
    .bind(purpose.as_str())
    .fetch_optional(&mut *tx)
    .await
    .map_err(map_db_error)?;

    let Some((id, user_id, stored, attempts)) = row else {
        return Err(invalid_code());
    };
    if attempts >= MAX_ATTEMPTS {
        return Err(AppError::Unprocessable {
            code: "OTP_ATTEMPTS_EXCEEDED",
            message: "too many wrong codes, request a new one".into(),
        });
    }
    if !constant_time_eq(&stored, &code_hash(pepper, id, code)) {
        sqlx::query("UPDATE otp_codes SET attempts = attempts + 1 WHERE id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        tx.commit().await.map_err(map_db_error)?;
        return Err(invalid_code());
    }
    tx.commit().await.map_err(map_db_error)?;
    Ok(MatchedOtp { id, user_id })
}

/// Marks a matched code as used. Fails if a concurrent request used it first.
pub async fn consume(conn: &mut MySqlConnection, otp: MatchedOtp) -> AppResult<()> {
    let done = sqlx::query("UPDATE otp_codes SET consumed_at = NOW(6) WHERE id = ? AND consumed_at IS NULL")
        .bind(otp.id)
        .execute(conn)
        .await
        .map_err(map_db_error)?;
    if done.rows_affected() == 1 { Ok(()) } else { Err(invalid_code()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_six_digits() {
        for _ in 0..200 {
            let code = generate_code().expect("code");
            assert_eq!(code.len(), 6);
            assert!(code.bytes().all(|b| b.is_ascii_digit()));
        }
    }

    #[test]
    fn hash_depends_on_pepper_id_and_code() {
        let id = Uuid::now_v7();
        let base = code_hash(&[1; 32], id, "123456");
        assert_ne!(base, code_hash(&[2; 32], id, "123456"));
        assert_ne!(base, code_hash(&[1; 32], Uuid::now_v7(), "123456"));
        assert_ne!(base, code_hash(&[1; 32], id, "123457"));
    }
}
