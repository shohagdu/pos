//! Helpers shared by the API integration tests.
#![allow(dead_code)]

use actix_http::Request;
use actix_web::{
    Error,
    body::MessageBody,
    dev::{Service, ServiceResponse},
    http::{StatusCode, header::HeaderMap},
    test,
};
use pos_api::auth::REFRESH_COOKIE;
use pos_core::config::{AppConfig, AppEnv, AuthConfig, DatabaseConfig, HttpConfig, LogConfig, LogFormat, Secret};
use serde_json::{Value, json};

pub fn config() -> AppConfig {
    AppConfig {
        env: AppEnv::Test,
        http: HttpConfig {
            host: "127.0.0.1".into(),
            port: 0,
            cors_allowed_origins: vec!["http://localhost:5173".into()],
            trust_proxy_headers: false,
            app_url: "http://localhost:5173".into(),
        },
        database: DatabaseConfig {
            url: Secret::new("mysql://nobody@127.0.0.1:1/none"),
            max_connections: 1,
            min_connections: 0,
            acquire_timeout_secs: 1,
        },
        log: LogConfig { format: LogFormat::Pretty, filter: "warn".into() },
        auth: AuthConfig {
            jwt_secret: Secret::new("integration-test-secret-0123456789abcdef"),
            jwt_secret_previous: None,
            access_token_ttl_secs: 900,
            refresh_token_ttl_days: 30,
            cookie_secure: false,
            expose_otp: true,
        },
        run_migrations_on_start: false,
    }
}

pub struct Res {
    pub status: StatusCode,
    pub body: Value,
    pub headers: HeaderMap,
    /// Value of the refresh cookie set by the response (empty = cleared).
    pub refresh_cookie: Option<String>,
}

impl Res {
    pub fn code(&self) -> &str {
        self.body["code"].as_str().unwrap_or_default()
    }

    pub fn token(&self) -> String {
        self.body["access_token"].as_str().expect("access_token in response").to_owned()
    }

    pub fn cookie(&self) -> String {
        self.refresh_cookie.clone().filter(|c| !c.is_empty()).expect("refresh cookie set")
    }
}

pub async fn call<S, B>(srv: &S, req: Request) -> Res
where
    S: Service<Request, Response = ServiceResponse<B>, Error = Error>,
    B: MessageBody,
{
    let res = test::call_service(srv, req).await;
    let status = res.status();
    let headers = res.headers().clone();
    let refresh_cookie = res.response().cookies().find(|c| c.name() == REFRESH_COOKIE).map(|c| c.value().to_owned());
    let bytes = test::read_body(res).await;
    let body = if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap_or(Value::Null) };
    Res { status, body, headers, refresh_cookie }
}

pub struct Req {
    inner: test::TestRequest,
}

impl Req {
    pub fn get(uri: &str) -> Self {
        Self { inner: test::TestRequest::get().uri(uri) }
    }

    pub fn post(uri: &str, body: Value) -> Self {
        Self { inner: test::TestRequest::post().uri(uri).set_json(body) }
    }

    pub fn patch(uri: &str, body: Value) -> Self {
        Self { inner: test::TestRequest::patch().uri(uri).set_json(body) }
    }

    pub fn put(uri: &str, body: Value) -> Self {
        Self { inner: test::TestRequest::put().uri(uri).set_json(body) }
    }

    pub fn delete(uri: &str) -> Self {
        Self { inner: test::TestRequest::delete().uri(uri) }
    }

    pub fn bearer(self, token: &str) -> Self {
        Self { inner: self.inner.insert_header(("authorization", format!("Bearer {token}"))) }
    }

    pub fn cookie(self, value: &str) -> Self {
        Self { inner: self.inner.cookie(actix_web::cookie::Cookie::new(REFRESH_COOKIE, value.to_owned())) }
    }

    pub fn header(self, name: &'static str, value: &str) -> Self {
        Self { inner: self.inner.insert_header((name, value.to_owned())) }
    }

    pub fn if_match(self, version: i64) -> Self {
        Self { inner: self.inner.insert_header(("if-match", format!("\"{version}\""))) }
    }

    pub fn build(self) -> Request {
        self.inner.to_request()
    }
}

pub const PASSWORD: &str = "correct horse battery";

/// A signed-in, verified user without a business.
pub struct Session {
    pub token: String,
    pub cookie: String,
    pub phone: String,
}

/// Registers and verifies `phone`, returning the signed-in session.
pub async fn sign_up<S, B>(srv: &S, name: &str, phone: &str) -> Session
where
    S: Service<Request, Response = ServiceResponse<B>, Error = Error>,
    B: MessageBody,
{
    let reg = call(
        srv,
        Req::post("/api/v1/auth/register", json!({ "full_name": name, "phone": phone, "password": PASSWORD })).build(),
    )
    .await;
    assert_eq!(reg.status, StatusCode::CREATED, "register: {}", reg.body);
    let code = reg.body["dev_code"].as_str().expect("dev_code");
    let verified =
        call(srv, Req::post("/api/v1/auth/verify-phone", json!({ "phone": phone, "code": code })).build()).await;
    assert_eq!(verified.status, StatusCode::OK, "verify: {}", verified.body);
    Session { token: verified.token(), cookie: verified.cookie(), phone: phone.to_owned() }
}

/// Creates a business and signs the session in to it.
pub async fn create_business<S, B>(srv: &S, session: &mut Session, name: &str) -> String
where
    S: Service<Request, Response = ServiceResponse<B>, Error = Error>,
    B: MessageBody,
{
    let created = call(
        srv,
        Req::post("/api/v1/tenants", json!({ "name": name, "business_type": "grocery" }))
            .bearer(&session.token)
            .build(),
    )
    .await;
    assert_eq!(created.status, StatusCode::CREATED, "create tenant: {}", created.body);
    let tenant_id = created.body["tenant_id"].as_str().expect("tenant_id").to_owned();
    switch(srv, session, Some(&tenant_id)).await;
    tenant_id
}

pub async fn switch<S, B>(srv: &S, session: &mut Session, tenant_id: Option<&str>)
where
    S: Service<Request, Response = ServiceResponse<B>, Error = Error>,
    B: MessageBody,
{
    let res = call(
        srv,
        Req::post("/api/v1/auth/switch-tenant", json!({ "tenant_id": tenant_id }))
            .bearer(&session.token)
            .cookie(&session.cookie)
            .build(),
    )
    .await;
    assert_eq!(res.status, StatusCode::OK, "switch tenant: {}", res.body);
    session.token = res.token();
    session.cookie = res.cookie();
}

pub async fn role_id<S, B>(srv: &S, session: &Session, code: &str) -> String
where
    S: Service<Request, Response = ServiceResponse<B>, Error = Error>,
    B: MessageBody,
{
    let roles = call(srv, Req::get("/api/v1/roles").bearer(&session.token).build()).await;
    roles
        .body
        .as_array()
        .into_iter()
        .flatten()
        .find(|r| r["code"] == code)
        .map(|r| r["id"].as_str().unwrap_or_default().to_owned())
        .expect(code)
}

pub fn token_of(created: &Value) -> String {
    let link = created["dev_link"].as_str().expect("dev_link");
    link.split_once("/invite#").map(|(_, t)| t.to_owned()).expect("token in fragment")
}

pub async fn invite<S, B>(srv: &S, owner: &Session, phone: &str, roles: &[&str]) -> Res
where
    S: Service<Request, Response = ServiceResponse<B>, Error = Error>,
    B: MessageBody,
{
    let mut ids = Vec::new();
    for code in roles {
        ids.push(role_id(srv, owner, code).await);
    }
    call(
        srv,
        Req::post("/api/v1/invitations", json!({ "destination": phone, "role_ids": ids })).bearer(&owner.token).build(),
    )
    .await
}

/// Invites `phone` with `roles`, signs the person up, accepts and signs in to the business.
pub async fn join<S, B>(srv: &S, owner: &Session, tenant_id: &str, name: &str, phone: &str, roles: &[&str]) -> Session
where
    S: Service<Request, Response = ServiceResponse<B>, Error = Error>,
    B: MessageBody,
{
    let created = invite(srv, owner, phone, roles).await;
    assert_eq!(created.status, StatusCode::CREATED, "invite: {}", created.body);
    let mut member = sign_up(srv, name, phone).await;
    let accepted = call(
        srv,
        Req::post("/api/v1/invitations/accept", json!({ "token": token_of(&created.body) }))
            .bearer(&member.token)
            .build(),
    )
    .await;
    assert_eq!(accepted.status, StatusCode::OK, "accept: {}", accepted.body);
    switch(srv, &mut member, Some(tenant_id)).await;
    member
}
