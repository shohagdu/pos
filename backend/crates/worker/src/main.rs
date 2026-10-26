//! `pos-worker`: runs scheduled maintenance on an interval, and notification
//! checks and delivery on a shorter one, until SIGINT/SIGTERM.

use std::time::Duration;

use anyhow::Context;
use chrono::Utc;
use pos_core::config::{AppConfig, LogFormat, load_dotenv};
use pos_worker::{cleanup, partitions, reconcile};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    load_dotenv();
    let config = AppConfig::from_env().context("invalid configuration")?;
    init_tracing(&config);

    let tick_secs: u64 = std::env::var("WORKER_TICK_SECS").ok().and_then(|v| v.parse().ok()).unwrap_or(3600);
    // Notification checks and email/SMS delivery need a shorter cycle (NOT-R02 retries).
    let notify_secs: u64 = std::env::var("NOTIFY_TICK_SECS").ok().and_then(|v| v.parse().ok()).unwrap_or(120);
    let pool = pos_db::connect(&config.database).await.context("database connection failed")?;
    let gateways = pos_notify::delivery::Gateways::from_env().context("invalid notification gateway settings")?;

    tracing::info!(tick_secs, notify_secs, "pos-worker started");
    let mut ticker = tokio::time::interval(Duration::from_secs(tick_secs));
    let mut notify_ticker = tokio::time::interval(Duration::from_secs(notify_secs));
    // Reconciliation runs at start-up and then once per (UTC) day.
    let mut reconciled_on = None;
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                run_maintenance(&pool).await;
                let today = Utc::now().date_naive();
                if reconciled_on != Some(today) {
                    if let Err(err) = reconcile::run(&pool).await {
                        tracing::error!(error = %err, job = "reconciliation", "maintenance job failed");
                    }
                    reconciled_on = Some(today);
                }
                // Subscription states and reminders (SUB-R02, SUB-R06); cheap, so every tick.
                match pos_platform::lifecycle::run(&pool, Utc::now()).await {
                    Ok(o) if o.transitions + o.reminders > 0 => {
                        tracing::info!(transitions = o.transitions, reminders = o.reminders, "subscription lifecycle ran");
                    }
                    Ok(_) => {}
                    Err(err) => tracing::error!(error = %err, job = "subscription_lifecycle", "maintenance job failed"),
                }
            }
            _ = notify_ticker.tick() => run_notifications(&pool, &gateways).await,
            _ = shutdown_signal() => break,
        }
    }
    tracing::info!("pos-worker stopped");
    Ok(())
}

/// One failing job must not stop the others; failures are logged for alerting.
async fn run_maintenance(pool: &sqlx::MySqlPool) {
    let today = Utc::now().date_naive();
    if let Err(err) = partitions::ensure_all(pool, today, partitions::MONTHS_AHEAD).await {
        tracing::error!(error = %err, job = "partitions", "maintenance job failed");
    }
    if let Err(err) = cleanup::purge_expired_idempotency_keys(pool).await {
        tracing::error!(error = %err, job = "idempotency_cleanup", "maintenance job failed");
    }
}

/// Scheduled notification checks, then delivery of what is due.
async fn run_notifications(pool: &sqlx::MySqlPool, gateways: &pos_notify::delivery::Gateways) {
    let now = Utc::now();
    match pos_notify::jobs::run(pool, now).await {
        Ok(0) => {}
        Ok(created) => tracing::info!(created, "notifications created"),
        Err(err) => tracing::error!(error = %err, job = "notifications", "maintenance job failed"),
    }
    match pos_notify::delivery::dispatch(pool, gateways, now).await {
        Ok(o) if o == pos_notify::delivery::Outcome::default() => {}
        Ok(o) => tracing::info!(sent = o.sent, failed = o.failed, gave_up = o.gave_up, "notifications delivered"),
        Err(err) => tracing::error!(error = %err, job = "notification_delivery", "maintenance job failed"),
    }
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut term = match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(s) => s,
            Err(_) => return std::future::pending().await,
        };
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = term.recv() => {}
        }
    }
    #[cfg(not(unix))]
    let _ = tokio::signal::ctrl_c().await;
}

fn init_tracing(config: &AppConfig) {
    let filter = EnvFilter::try_new(&config.log.filter).unwrap_or_else(|_| EnvFilter::new("info"));
    let registry = tracing_subscriber::registry().with(filter);
    match config.log.format {
        LogFormat::Json => registry.with(fmt::layer().json().flatten_event(true)).init(),
        LogFormat::Pretty => registry.with(fmt::layer().compact()).init(),
    }
}
