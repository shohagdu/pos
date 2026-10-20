//! Email and SMS delivery (NOT-R02), run by the worker.
//!
//! Due deliveries are leased for ten minutes (so a crashed worker's batch is
//! retried), sent outside any transaction, then marked `sent`, or `failed`
//! with the next attempt after 2, 4, 8 and 16 minutes; the fifth failure is
//! `gave_up`: five tries within an hour. Errors are kept in `last_error` and logged.
//!
//! Gateways come from the environment; without them messages are only
//! logged (development):
//!
//! - `NOTIFY_SMS_URL`, `NOTIFY_SMS_API_KEY`, `NOTIFY_SMS_SENDER_ID`: an HTTP
//!   SMS gateway taking a form POST of `api_key`, `senderid`, `number`,
//!   `message` (the common shape of Bangladeshi bulk-SMS providers).
//! - `SMTP_HOST`, `SMTP_PORT` (587 STARTTLS, 465 TLS), `SMTP_USERNAME`,
//!   `SMTP_PASSWORD`, `SMTP_FROM` ("Shop POS <no-reply@example.com>").

use std::time::Duration as StdDuration;

use chrono::{DateTime, Duration, Utc};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor, message::Mailbox,
    transport::smtp::authentication::Credentials,
};
use pos_core::{AppError, AppResult, PlatformCtx, RequestMeta};
use pos_db::map_db_error;
use sqlx::MySqlPool;
use uuid::Uuid;

/// Tries before a delivery is given up.
pub const MAX_ATTEMPTS: i16 = 5;
const BATCH: i64 = 50;
const LEASE_MINUTES: i64 = 10;

/// Wait before the next try after `attempts` failures (2, 4, 8, 16 minutes).
pub fn backoff(attempts: i16) -> Option<Duration> {
    (attempts < MAX_ATTEMPTS).then(|| Duration::minutes(2_i64.pow(u32::from(attempts.clamp(1, 4).unsigned_abs()))))
}

enum Sms {
    Log,
    Http { client: reqwest::Client, url: String, api_key: String, sender_id: String },
}

enum Email {
    Log,
    Smtp { transport: AsyncSmtpTransport<Tokio1Executor>, from: Mailbox },
}

pub struct Gateways {
    sms: Sms,
    email: Email,
}

fn env(key: &str) -> Option<String> {
    std::env::var(key).ok().map(|v| v.trim().to_owned()).filter(|v| !v.is_empty())
}

fn config_error(key: &'static str, message: impl Into<String>) -> AppError {
    AppError::internal(format!("{key}: {}", message.into()))
}

impl Gateways {
    /// Log-only gateways (development and tests).
    pub fn log_only() -> Self {
        Self { sms: Sms::Log, email: Email::Log }
    }

    pub fn from_env() -> AppResult<Self> {
        // reqwest and lettre use rustls without a built-in crypto provider; use ring like sqlx.
        let _ = rustls::crypto::ring::default_provider().install_default();
        let sms = match env("NOTIFY_SMS_URL") {
            None => Sms::Log,
            Some(url) => Sms::Http {
                client: reqwest::Client::builder()
                    .timeout(StdDuration::from_secs(15))
                    .build()
                    .map_err(|e| config_error("NOTIFY_SMS_URL", e.to_string()))?,
                url,
                api_key: env("NOTIFY_SMS_API_KEY").unwrap_or_default(),
                sender_id: env("NOTIFY_SMS_SENDER_ID").unwrap_or_default(),
            },
        };
        let email = match env("SMTP_HOST") {
            None => Email::Log,
            Some(host) => {
                let port: u16 = env("SMTP_PORT").and_then(|p| p.parse().ok()).unwrap_or(587);
                let builder = if port == 465 {
                    AsyncSmtpTransport::<Tokio1Executor>::relay(&host)
                } else {
                    AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host)
                }
                .map_err(|e| config_error("SMTP_HOST", e.to_string()))?
                .port(port)
                .timeout(Some(StdDuration::from_secs(20)));
                let builder = match (env("SMTP_USERNAME"), env("SMTP_PASSWORD")) {
                    (Some(user), Some(pass)) => builder.credentials(Credentials::new(user, pass)),
                    _ => builder,
                };
                let from = env("SMTP_FROM")
                    .ok_or_else(|| config_error("SMTP_FROM", "required with SMTP_HOST"))?
                    .parse()
                    .map_err(|_| config_error("SMTP_FROM", "not an email address"))?;
                Email::Smtp { transport: builder.build(), from }
            }
        };
        Ok(Self { sms, email })
    }

    async fn send(&self, d: &Due) -> Result<Option<String>, String> {
        let text = match &d.body {
            Some(body) => format!("{}\n\n{body}", d.title),
            None => d.title.clone(),
        };
        match d.channel.as_str() {
            "sms" => match &self.sms {
                Sms::Log => {
                    tracing::info!(to = %mask(&d.destination), message = %text, "SMS (no gateway configured)");
                    Ok(None)
                }
                Sms::Http { client, url, api_key, sender_id } => {
                    let message = format!("{}: {text}", d.business_name);
                    let res = client
                        .post(url)
                        .form(&[
                            ("api_key", api_key.as_str()),
                            ("senderid", sender_id.as_str()),
                            ("number", d.destination.trim_start_matches('+')),
                            ("message", message.as_str()),
                        ])
                        .send()
                        .await
                        .map_err(|e| e.to_string())?;
                    let status = res.status();
                    let body = res.text().await.unwrap_or_default();
                    if status.is_success() {
                        Ok(Some(body.chars().take(100).collect()))
                    } else {
                        Err(format!("gateway answered {status}: {}", body.chars().take(300).collect::<String>()))
                    }
                }
            },
            "email" => match &self.email {
                Email::Log => {
                    tracing::info!(to = %mask(&d.destination), subject = %d.title, "email (no SMTP configured)");
                    Ok(None)
                }
                Email::Smtp { transport, from } => {
                    let to: Mailbox = d.destination.parse().map_err(|_| "not an email address".to_owned())?;
                    let message = Message::builder()
                        .from(from.clone())
                        .to(to)
                        .subject(format!("{} — {}", d.title, d.business_name))
                        .body(text)
                        .map_err(|e| e.to_string())?;
                    let response = transport.send(message).await.map_err(|e| e.to_string())?;
                    Ok(response.message().next().map(|m| m.chars().take(100).collect()))
                }
            },
            other => Err(format!("unknown channel {other}")),
        }
    }
}

/// `+8801711000001` → `+88017******01`; `rahim@example.com` → `ra***@example.com`.
fn mask(destination: &str) -> String {
    if let Some((user, domain)) = destination.split_once('@') {
        return format!("{}***@{domain}", user.chars().take(2).collect::<String>());
    }
    let chars: Vec<char> = destination.chars().collect();
    if chars.len() <= 6 {
        return "******".into();
    }
    chars.iter().enumerate().map(|(i, c)| if i < 6 || i >= chars.len() - 2 { *c } else { '*' }).collect()
}

#[derive(sqlx::FromRow)]
struct Due {
    tenant_id: Uuid,
    id: Uuid,
    channel: String,
    destination: String,
    attempts: i16,
    title: String,
    body: Option<String>,
    business_name: String,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub sent: usize,
    pub failed: usize,
    pub gave_up: usize,
}

fn ctx() -> PlatformCtx {
    PlatformCtx { actor_id: None, reason: "notification delivery".into(), meta: RequestMeta::default() }
}

/// Sends the deliveries that are due.
pub async fn dispatch(pool: &MySqlPool, gateways: &Gateways, now: DateTime<Utc>) -> AppResult<Outcome> {
    // Cheap check first: platform transactions are logged, so open one only with work to do.
    let any: Option<i32> = sqlx::query_scalar(
        "SELECT 1 FROM notification_deliveries WHERE status IN ('queued', 'failed') AND next_attempt_at <= ? LIMIT 1 \
         /* tenant-lint: allow the dispatcher serves every business */",
    )
    .bind(now)
    .fetch_optional(pool)
    .await
    .map_err(map_db_error)?;
    if any.is_none() {
        return Ok(Outcome::default());
    }

    let mut tx = pos_db::begin_platform(pool, &ctx()).await?;
    let due: Vec<Due> = sqlx::query_as(
        "SELECT d.tenant_id, d.id, d.channel, d.destination, d.attempts, n.title, n.body, t.name AS business_name \
         FROM notification_deliveries d \
         JOIN notifications n ON n.tenant_id = d.tenant_id AND n.id = d.notification_id \
         JOIN tenants t ON t.id = d.tenant_id \
         WHERE d.status IN ('queued', 'failed') AND d.next_attempt_at <= ? \
         ORDER BY d.next_attempt_at LIMIT ? FOR UPDATE OF d SKIP LOCKED \
         /* tenant-lint: allow the dispatcher serves every business */",
    )
    .bind(now)
    .bind(BATCH)
    .fetch_all(&mut *tx)
    .await
    .map_err(map_db_error)?;
    for d in &due {
        sqlx::query("UPDATE notification_deliveries SET next_attempt_at = ? WHERE tenant_id = ? AND id = ?")
            .bind(now + Duration::minutes(LEASE_MINUTES))
            .bind(d.tenant_id)
            .bind(d.id)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
    }
    tx.commit().await?;

    let mut outcome = Outcome::default();
    let mut results = Vec::with_capacity(due.len());
    for d in &due {
        results.push(gateways.send(d).await);
    }

    let mut tx = pos_db::begin_platform(pool, &ctx()).await?;
    for (d, result) in due.iter().zip(results) {
        let attempts = d.attempts + 1;
        match result {
            Ok(provider_ref) => {
                outcome.sent += 1;
                sqlx::query(
                    "UPDATE notification_deliveries SET status = 'sent', attempts = ?, provider_ref = ?, last_error = NULL, \
                     sent_at = UTC_TIMESTAMP(6) WHERE tenant_id = ? AND id = ?",
                )
                .bind(attempts)
                .bind(provider_ref)
                .bind(d.tenant_id)
                .bind(d.id)
                .execute(&mut *tx)
                .await
                .map_err(map_db_error)?;
            }
            Err(error) => {
                let next = backoff(attempts);
                tracing::warn!(delivery = %d.id, channel = %d.channel, attempts, error = %error, gave_up = next.is_none(), "notification delivery failed");
                if next.is_some() {
                    outcome.failed += 1;
                } else {
                    outcome.gave_up += 1;
                }
                sqlx::query(
                    "UPDATE notification_deliveries SET status = ?, attempts = ?, last_error = ?, next_attempt_at = ? \
                     WHERE tenant_id = ? AND id = ?",
                )
                .bind(if next.is_some() { "failed" } else { "gave_up" })
                .bind(attempts)
                .bind(error.chars().take(1000).collect::<String>())
                .bind(now + next.unwrap_or_else(Duration::zero))
                .bind(d.tenant_id)
                .bind(d.id)
                .execute(&mut *tx)
                .await
                .map_err(map_db_error)?;
            }
        }
    }
    tx.commit().await?;
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retries_back_off_and_stop_after_five_tries() {
        let waits: Vec<_> = (1..=5).map(|n| backoff(n).map(|d| d.num_minutes())).collect();
        assert_eq!(waits, vec![Some(2), Some(4), Some(8), Some(16), None]);
        // Under an hour from the first try to the last.
        assert!(waits.iter().flatten().sum::<i64>() <= 60);
    }

    #[test]
    fn masks_destinations_for_logs() {
        assert_eq!(mask("+8801711000001"), "+88017******01");
        assert_eq!(mask("rahim@example.com"), "ra***@example.com");
    }
}
