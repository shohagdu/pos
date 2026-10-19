//! Configuration from environment variables (12-factor). `.env` is loaded in
//! development only; production injects variables from the secret manager.

use std::{env, fmt, str::FromStr};

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("missing required environment variable {0}")]
    Missing(&'static str),
    #[error("invalid value for {key}: {message}")]
    Invalid { key: &'static str, message: String },
}

/// A value that must never appear in logs or `Debug` output.
#[derive(Clone)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(***)")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEnv {
    Development,
    Test,
    Staging,
    Production,
}

impl FromStr for AppEnv {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "development" | "dev" => Ok(Self::Development),
            "test" => Ok(Self::Test),
            "staging" => Ok(Self::Staging),
            "production" | "prod" => Ok(Self::Production),
            other => Err(format!("unknown environment '{other}'")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Json,
    Pretty,
}

impl FromStr for LogFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "json" => Ok(Self::Json),
            "pretty" => Ok(Self::Pretty),
            other => Err(format!("expected json|pretty, got '{other}'")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: Secret,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_secs: u64,
}

#[derive(Debug, Clone)]
pub struct HttpConfig {
    pub host: String,
    pub port: u16,
    pub cors_allowed_origins: Vec<String>,
    /// Read the client IP from `X-Forwarded-For` (only behind a trusted proxy).
    pub trust_proxy_headers: bool,
    /// Public URL of the web app, used in links sent to users (invitations).
    pub app_url: String,
}

#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// HS256 signing key for access tokens (at least 32 bytes).
    pub jwt_secret: Secret,
    /// Previous key, still accepted for verification during a key rotation.
    pub jwt_secret_previous: Option<Secret>,
    pub access_token_ttl_secs: u64,
    pub refresh_token_ttl_days: u64,
    /// `Secure` attribute on the refresh-token cookie. Only off for plain-HTTP development.
    pub cookie_secure: bool,
    /// Return OTP codes in API responses. Development and test only.
    pub expose_otp: bool,
}

#[derive(Debug, Clone)]
pub struct LogConfig {
    pub format: LogFormat,
    pub filter: String,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub env: AppEnv,
    pub http: HttpConfig,
    pub database: DatabaseConfig,
    pub log: LogConfig,
    pub auth: AuthConfig,
    pub run_migrations_on_start: bool,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let env: AppEnv = parse_or("APP_ENV", AppEnv::Development)?;
        let default_format = if env == AppEnv::Development { LogFormat::Pretty } else { LogFormat::Json };
        let local = matches!(env, AppEnv::Development | AppEnv::Test);

        let jwt_secret = required("JWT_SECRET")?;
        if jwt_secret.len() < 32 {
            return Err(ConfigError::Invalid { key: "JWT_SECRET", message: "must be at least 32 bytes".into() });
        }
        let expose_otp: bool = parse_or("AUTH_EXPOSE_OTP", false)?;
        if expose_otp && !local {
            return Err(ConfigError::Invalid {
                key: "AUTH_EXPOSE_OTP",
                message: "only allowed in development and test".into(),
            });
        }

        Ok(Self {
            env,
            http: HttpConfig {
                host: string_or("HTTP_HOST", "0.0.0.0"),
                port: parse_or("HTTP_PORT", 8080)?,
                cors_allowed_origins: list("CORS_ALLOWED_ORIGINS"),
                trust_proxy_headers: parse_or("HTTP_TRUST_PROXY_HEADERS", false)?,
                app_url: string_or("APP_URL", "http://localhost:5173").trim_end_matches('/').to_owned(),
            },
            database: DatabaseConfig {
                url: Secret::new(required("DATABASE_URL")?),
                max_connections: parse_or("DB_MAX_CONNECTIONS", 20)?,
                min_connections: parse_or("DB_MIN_CONNECTIONS", 1)?,
                acquire_timeout_secs: parse_or("DB_ACQUIRE_TIMEOUT_SECS", 5)?,
            },
            log: LogConfig {
                format: parse_or("LOG_FORMAT", default_format)?,
                filter: string_or("RUST_LOG", "info,sqlx=warn"),
            },
            auth: AuthConfig {
                jwt_secret: Secret::new(jwt_secret),
                jwt_secret_previous: env::var("JWT_SECRET_PREVIOUS").ok().filter(|v| !v.is_empty()).map(Secret::new),
                access_token_ttl_secs: parse_or("ACCESS_TOKEN_TTL_SECS", 900)?,
                refresh_token_ttl_days: parse_or("REFRESH_TOKEN_TTL_DAYS", 30)?,
                cookie_secure: parse_or("AUTH_COOKIE_SECURE", true)?,
                expose_otp,
            },
            run_migrations_on_start: parse_or("RUN_MIGRATIONS_ON_START", false)?,
        })
    }
}

/// Loads `.env` if present. Call once at binary start-up.
pub fn load_dotenv() {
    let _ = dotenvy::dotenv();
}

fn required(key: &'static str) -> Result<String, ConfigError> {
    env::var(key).ok().filter(|v| !v.is_empty()).ok_or(ConfigError::Missing(key))
}

fn string_or(key: &str, default: &str) -> String {
    env::var(key).ok().filter(|v| !v.is_empty()).unwrap_or_else(|| default.to_owned())
}

fn list(key: &str) -> Vec<String> {
    env::var(key).unwrap_or_default().split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned).collect()
}

fn parse_or<T>(key: &'static str, default: T) -> Result<T, ConfigError>
where
    T: FromStr,
    T::Err: fmt::Display,
{
    match env::var(key) {
        Ok(v) if !v.is_empty() => v.parse().map_err(|e: T::Err| ConfigError::Invalid { key, message: e.to_string() }),
        _ => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_is_redacted_in_debug() {
        let s = Secret::new("mysql://pos:hunter2@db/pos");
        assert_eq!(format!("{s:?}"), "Secret(***)");
        assert!(s.expose().contains("hunter2"));
    }

    #[test]
    fn parses_environment_names() {
        assert_eq!("prod".parse::<AppEnv>(), Ok(AppEnv::Production));
        assert!("qa".parse::<AppEnv>().is_err());
    }
}
