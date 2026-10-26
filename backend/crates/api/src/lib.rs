//! HTTP API (Actix Web). Route modules register themselves in [`configure`];
//! cross-cutting concerns (request id, problem responses, security headers,
//! CORS) are applied in [`app`].

pub mod auth;
pub mod error;
pub mod middleware;
pub mod openapi;
pub mod posting;
pub mod routes;
pub mod state;
pub mod telemetry;

use actix_cors::Cors;
use actix_web::{
    App, Error,
    body::MessageBody,
    dev::{ServiceFactory, ServiceRequest, ServiceResponse},
    http::header,
    middleware::{DefaultHeaders, from_fn},
    web,
};
use pos_core::AppError;

pub use error::ApiError;
pub use state::AppState;

/// Max JSON request body (bulk endpoints opt into larger limits explicitly).
const JSON_LIMIT_BYTES: usize = 1024 * 1024;

pub fn app(
    state: AppState,
) -> App<
    impl ServiceFactory<
        ServiceRequest,
        Config = (),
        Response = ServiceResponse<impl MessageBody>,
        Error = Error,
        InitError = (),
    >,
> {
    let cors = cors(&state.config.http.cors_allowed_origins);

    App::new()
        .app_data(web::Data::new(state))
        .app_data(
            web::JsonConfig::default()
                .limit(JSON_LIMIT_BYTES)
                .error_handler(|err, _req| ApiError::from(AppError::validation(err.to_string(), vec![])).into()),
        )
        .app_data(
            web::PathConfig::default()
                .error_handler(|err, _req| ApiError::from(AppError::validation(err.to_string(), vec![])).into()),
        )
        .app_data(
            web::QueryConfig::default()
                .error_handler(|err, _req| ApiError::from(AppError::validation(err.to_string(), vec![])).into()),
        )
        .wrap(security_headers())
        .wrap(cors)
        // Outermost: every response (including CORS rejections) gets a request id.
        .wrap(from_fn(middleware::request_context))
        .configure(configure)
        .default_service(web::to(|| async { Err::<actix_web::HttpResponse, _>(ApiError::from(AppError::NotFound)) }))
}

/// Registers all routes. Probes live at the root; the versioned API under `/api/v1`.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(routes::health::healthz).service(routes::health::readyz).service(
        web::scope("/api/v1")
            .service(routes::meta::meta)
            .service(routes::meta::openapi_json)
            .configure(routes::auth::configure)
            .configure(routes::tenancy::configure)
            .configure(routes::users::configure)
            .configure(routes::subscription::configure)
            .configure(routes::audit::configure)
            .configure(routes::catalog::configure)
            .configure(routes::files::configure)
            .configure(routes::inventory::configure)
            .configure(routes::parties::configure)
            .configure(routes::money::configure)
            .configure(routes::sales::configure)
            .configure(routes::registers::configure)
            .configure(routes::purchasing::configure)
            .configure(routes::reports::configure)
            .configure(routes::accounting::configure)
            .configure(routes::notifications::configure)
            .configure(routes::admin::configure),
    );
}

fn security_headers() -> DefaultHeaders {
    // HSTS is set by the TLS-terminating proxy (Nginx); these apply to every API response.
    DefaultHeaders::new()
        .add((header::X_CONTENT_TYPE_OPTIONS, "nosniff"))
        .add((header::X_FRAME_OPTIONS, "DENY"))
        .add((header::REFERRER_POLICY, "no-referrer"))
        .add((header::CONTENT_SECURITY_POLICY, "default-src 'none'; frame-ancestors 'none'"))
        .add((header::CACHE_CONTROL, "no-store"))
}

fn cors(allowed_origins: &[String]) -> Cors {
    // No configured origins = same-origin only (SPA served behind the same host/proxy).
    let mut cors = Cors::default()
        .allowed_methods(["GET", "POST", "PUT", "PATCH", "DELETE"])
        .allowed_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::IF_MATCH,
            header::HeaderName::from_static("idempotency-key"),
            header::HeaderName::from_static("x-request-id"),
            header::HeaderName::from_static("x-branch-id"),
        ])
        .expose_headers([header::ETAG, header::RETRY_AFTER, header::HeaderName::from_static("x-request-id")])
        .supports_credentials()
        .max_age(3600);
    for origin in allowed_origins {
        cors = cors.allowed_origin(origin);
    }
    cors
}
