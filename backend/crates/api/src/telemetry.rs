use pos_core::config::{LogConfig, LogFormat};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

/// Structured logging (NFR-OBS-01): JSON in deployed environments, readable
/// output in development. Never log secrets or request bodies with PII.
pub fn init(cfg: &LogConfig) {
    let filter = EnvFilter::try_new(&cfg.filter).unwrap_or_else(|_| EnvFilter::new("info"));
    let registry = tracing_subscriber::registry().with(filter);
    match cfg.format {
        LogFormat::Json => registry.with(fmt::layer().json().flatten_event(true).with_current_span(true)).init(),
        LogFormat::Pretty => registry.with(fmt::layer().compact()).init(),
    }
}
