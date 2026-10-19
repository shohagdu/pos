//! `pos-api` binary.
//!
//! Commands:
//!   serve         run the HTTP server (default)
//!   migrate       apply pending database migrations
//!   check-schema  verify tenant-isolation schema rules (CI)
//!   openapi       print the OpenAPI document (no database needed)
//!   platform-staff <phone|email> <platform_owner|billing|support>
//!                 grant a Super Admin role to an existing user

use actix_web::HttpServer;
use anyhow::Context;
use pos_api::{AppState, app, openapi::ApiDoc, telemetry};
use pos_core::config::{AppConfig, AppEnv, load_dotenv};
use utoipa::OpenApi;

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let command = std::env::args().nth(1).unwrap_or_else(|| "serve".to_owned());
    if command == "openapi" {
        println!("{}", ApiDoc::openapi().to_pretty_json()?);
        return Ok(());
    }

    load_dotenv();
    let config = AppConfig::from_env().context("invalid configuration")?;
    telemetry::init(&config.log);

    match command.as_str() {
        "migrate" => {
            let pool = pos_db::connect(&config.database).await.context("database connection failed")?;
            pos_db::MIGRATOR.run(&pool).await.context("migration failed")?;
            tracing::info!("migrations applied");
        }
        "check-schema" => {
            let pool = pos_db::connect(&config.database).await.context("database connection failed")?;
            let report = pos_db::guards::check_schema(&pool).await?;
            if !report.is_clean() {
                tracing::error!(?report, "schema guard violations");
                std::process::exit(1);
            }
            tracing::info!("schema guards passed");
        }
        "platform-staff" => {
            let usage = "usage: pos-api platform-staff <phone|email> <platform_owner|billing|support>";
            let identifier = std::env::args().nth(2).context(usage)?;
            let role = std::env::args().nth(3).as_deref().and_then(pos_platform::StaffRole::parse).context(usage)?;
            let pool = pos_db::connect(&config.database).await.context("database connection failed")?;
            let user_id =
                pos_platform::staff::grant(&pool, &identifier, role).await.map_err(|e| anyhow::anyhow!("{e}"))?;
            tracing::info!(%user_id, role = role.as_str(), "platform staff role granted");
        }
        "serve" => serve(config).await?,
        other => {
            anyhow::bail!("unknown command '{other}' (expected serve|migrate|check-schema|openapi|platform-staff)")
        }
    }
    Ok(())
}

async fn serve(config: AppConfig) -> anyhow::Result<()> {
    if config.env == AppEnv::Production {
        // OTP codes and invitation links would only reach the log (pos_auth::notify::LogNotifier).
        anyhow::bail!("no SMS gateway is integrated yet; pos-api cannot serve production traffic");
    }
    if config.run_migrations_on_start {
        let pool = pos_db::connect(&config.database).await.context("database connection failed")?;
        pos_db::MIGRATOR.run(&pool).await.context("migration failed")?;
        tracing::info!("migrations applied on start-up");
    }

    // Lazy pool: the server starts even if MySQL is still booting; /readyz reports it.
    let pool = pos_db::connect_lazy(&config.database).context("invalid database configuration")?;
    let bind = (config.http.host.clone(), config.http.port);
    let state = AppState::new(pool, config);

    tracing::info!(host = %bind.0, port = bind.1, env = ?state.config.env, "starting pos-api");
    HttpServer::new(move || app(state.clone())).bind(bind)?.shutdown_timeout(30).run().await?;
    Ok(())
}
