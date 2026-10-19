//! HTTP behaviour of the platform skeleton (E1.1). Tests that need MySQL use
//! `#[sqlx::test]`; the others use a lazy pool pointing at a closed port.

mod common;

use std::time::Duration;

use actix_web::{http::StatusCode, test};
use pos_api::{AppState, app};
use serde_json::Value;
use sqlx::{MySqlPool, mysql::MySqlPoolOptions};

fn header(h: &actix_web::http::header::HeaderMap, name: &str) -> String {
    h.get(name).and_then(|v| v.to_str().ok()).unwrap_or_default().to_owned()
}

fn unreachable_db_state() -> AppState {
    let pool = MySqlPoolOptions::new()
        .acquire_timeout(Duration::from_millis(500))
        .connect_lazy("mysql://nobody@127.0.0.1:1/none")
        .expect("lazy pool");
    AppState::new(pool, common::config())
}

#[actix_web::test]
async fn healthz_is_ok_without_database() {
    let srv = test::init_service(app(unreachable_db_state())).await;
    let res = test::call_service(&srv, test::TestRequest::get().uri("/healthz").to_request()).await;
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = test::read_body_json(res).await;
    assert_eq!(body["status"], "ok");
}

#[actix_web::test]
async fn responses_carry_security_headers_and_request_id() {
    let srv = test::init_service(app(unreachable_db_state())).await;
    let res = test::call_service(&srv, test::TestRequest::get().uri("/healthz").to_request()).await;
    let h = res.headers();
    assert_eq!(header(h, "x-content-type-options"), "nosniff");
    assert_eq!(header(h, "x-frame-options"), "DENY");
    assert_eq!(header(h, "cache-control"), "no-store");
    assert!(uuid::Uuid::parse_str(&header(h, "x-request-id")).is_ok());
}

#[actix_web::test]
async fn client_request_id_is_propagated_and_unsafe_ids_replaced() {
    let srv = test::init_service(app(unreachable_db_state())).await;
    let req = test::TestRequest::get().uri("/healthz").insert_header(("x-request-id", "client-abc_123")).to_request();
    let res = test::call_service(&srv, req).await;
    assert_eq!(header(res.headers(), "x-request-id"), "client-abc_123");

    let req =
        test::TestRequest::get().uri("/healthz").insert_header(("x-request-id", "bad id with spaces")).to_request();
    let res = test::call_service(&srv, req).await;
    assert_ne!(header(res.headers(), "x-request-id"), "bad id with spaces");
}

#[actix_web::test]
async fn unknown_route_returns_problem_details() {
    let srv = test::init_service(app(unreachable_db_state())).await;
    let req = test::TestRequest::get().uri("/api/v1/nope").insert_header(("x-request-id", "req-404")).to_request();
    let res = test::call_service(&srv, req).await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    assert_eq!(header(res.headers(), "content-type"), "application/problem+json");
    let body: Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "NOT_FOUND");
    assert_eq!(body["status"], 404);
    assert_eq!(body["request_id"], "req-404");
}

#[actix_web::test]
async fn readyz_reports_unavailable_when_database_is_down() {
    let srv = test::init_service(app(unreachable_db_state())).await;
    let res = test::call_service(&srv, test::TestRequest::get().uri("/readyz").to_request()).await;
    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body: Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "SERVICE_UNAVAILABLE");
    // internal details (host, driver error) must not leak to clients
    assert!(!body.to_string().contains("127.0.0.1"));
}

#[actix_web::test]
async fn cors_allows_configured_origin_only() {
    let srv = test::init_service(app(unreachable_db_state())).await;
    let preflight = |origin: &'static str| {
        test::TestRequest::default()
            .method(actix_web::http::Method::OPTIONS)
            .uri("/api/v1/meta")
            .insert_header(("origin", origin))
            .insert_header(("access-control-request-method", "GET"))
            .to_request()
    };
    let ok = test::call_service(&srv, preflight("http://localhost:5173")).await;
    assert_eq!(header(ok.headers(), "access-control-allow-origin"), "http://localhost:5173");
    let denied = test::call_service(&srv, preflight("https://evil.example")).await;
    assert!(denied.headers().get("access-control-allow-origin").is_none());
}

#[actix_web::test]
async fn openapi_document_lists_paths() {
    let srv = test::init_service(app(unreachable_db_state())).await;
    let res = test::call_service(&srv, test::TestRequest::get().uri("/api/v1/openapi.json").to_request()).await;
    let doc: Value = test::read_body_json(res).await;
    for path in ["/healthz", "/readyz", "/api/v1/meta", "/api/v1/auth/login", "/api/v1/tenants", "/api/v1/settings"] {
        assert!(doc["paths"].get(path).is_some(), "missing {path}");
    }
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn readyz_is_ok_with_database(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool, common::config()))).await;
    let res = test::call_service(&srv, test::TestRequest::get().uri("/readyz").to_request()).await;
    assert_eq!(res.status(), StatusCode::OK);
}
