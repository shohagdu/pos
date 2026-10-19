//! Identity flows end to end over HTTP (E1.3): registration and phone
//! verification, login lockout (AC-AUTH-01), refresh rotation and reuse
//! detection (AC-AUTH-02), logout, password reset and sessions.

mod common;

use actix_web::{http::StatusCode, test};
use common::{PASSWORD, Req, call, create_business, sign_up};
use pos_api::{AppState, app};
use serde_json::json;
use sqlx::MySqlPool;

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn register_verify_and_me(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool.clone(), common::config()))).await;

    let reg = call(
        &srv,
        Req::post(
            "/api/v1/auth/register",
            json!({ "full_name": "Rahim Uddin", "phone": "01711-000001", "email": "Rahim@Example.com", "password": PASSWORD }),
        )
        .build(),
    )
    .await;
    assert_eq!(reg.status, StatusCode::CREATED);
    assert_eq!(reg.body["destination"], "+880*******001");
    assert_eq!(reg.body["expires_in"], 300);

    // A wrong code is rejected and counted.
    let wrong =
        call(&srv, Req::post("/api/v1/auth/verify-phone", json!({ "phone": "01711000001", "code": "000000" })).build())
            .await;
    let code = reg.body["dev_code"].as_str().expect("code");
    if code != "000000" {
        assert_eq!((wrong.status, wrong.code()), (StatusCode::UNPROCESSABLE_ENTITY, "OTP_INVALID"));
    }

    let ok =
        call(&srv, Req::post("/api/v1/auth/verify-phone", json!({ "phone": "+8801711000001", "code": code })).build())
            .await;
    assert_eq!(ok.status, StatusCode::OK, "{}", ok.body);
    assert_eq!(ok.body["token_type"], "Bearer");
    assert!(ok.body["tenant_id"].is_null());
    assert!(ok.body.get("refresh_token").is_none(), "refresh token must only be in the cookie");
    let cookie =
        ok.headers.get_all("set-cookie").map(|v| v.to_str().unwrap_or_default().to_owned()).collect::<Vec<_>>();
    assert!(
        cookie
            .iter()
            .any(|c| c.contains("HttpOnly") && c.contains("SameSite=Strict") && c.contains("Path=/api/v1/auth"))
    );

    // The code is single-use.
    let again =
        call(&srv, Req::post("/api/v1/auth/verify-phone", json!({ "phone": "01711000001", "code": code })).build())
            .await;
    assert_eq!(again.code(), "OTP_INVALID");

    let me = call(&srv, Req::get("/api/v1/auth/me").bearer(&ok.token()).build()).await;
    assert_eq!(me.status, StatusCode::OK);
    assert_eq!(me.body["user"]["phone"], "+8801711000001");
    assert_eq!(me.body["user"]["email"], "rahim@example.com");
    assert_eq!(me.body["user"]["phone_verified"], true);
    assert_eq!(me.body["memberships"], json!([]));
    assert!(me.body["tenant"].is_null());

    // A verified number cannot be registered again.
    let dup = call(
        &srv,
        Req::post(
            "/api/v1/auth/register",
            json!({ "full_name": "Other", "phone": "01711000001", "password": PASSWORD }),
        )
        .build(),
    )
    .await;
    assert_eq!((dup.status, dup.code()), (StatusCode::CONFLICT, "PHONE_TAKEN"));

    let stored: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE phone = '+8801711000001'")
        .fetch_one(&pool)
        .await
        .expect("user");
    assert!(stored.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"), "AUTH-R02 parameters");
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn registration_validates_input(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool, common::config()))).await;
    let cases = [
        (json!({ "full_name": "R", "phone": "01711000002", "password": PASSWORD }), "full_name", "INVALID_NAME"),
        (json!({ "full_name": "Rahim", "phone": "12345", "password": PASSWORD }), "phone", "INVALID_PHONE"),
        (
            json!({ "full_name": "Rahim", "phone": "01711000002", "password": "password123" }),
            "password",
            "PASSWORD_TOO_COMMON",
        ),
        (
            json!({ "full_name": "Rahim", "phone": "01711000002", "password": "short" }),
            "password",
            "PASSWORD_TOO_SHORT",
        ),
        (
            json!({ "full_name": "Rahim", "phone": "01711000002", "email": "nope", "password": PASSWORD }),
            "email",
            "INVALID_EMAIL",
        ),
    ];
    for (body, field, code) in cases {
        let res = call(&srv, Req::post("/api/v1/auth/register", body).build()).await;
        assert_eq!(res.status, StatusCode::BAD_REQUEST);
        assert_eq!(
            (res.body["errors"][0]["field"].as_str(), res.body["errors"][0]["code"].as_str()),
            (Some(field), Some(code))
        );
    }
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn unverified_users_can_sign_in_but_not_create_a_business(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool, common::config()))).await;
    let reg = call(
        &srv,
        Req::post(
            "/api/v1/auth/register",
            json!({ "full_name": "Karim", "phone": "01811000003", "password": PASSWORD }),
        )
        .build(),
    )
    .await;
    assert_eq!(reg.status, StatusCode::CREATED);

    let login = call(
        &srv,
        Req::post("/api/v1/auth/login", json!({ "identifier": "01811000003", "password": PASSWORD })).build(),
    )
    .await;
    assert_eq!(login.status, StatusCode::OK);
    let create = call(
        &srv,
        Req::post("/api/v1/tenants", json!({ "name": "Karim Store", "business_type": "retail" }))
            .bearer(&login.token())
            .build(),
    )
    .await;
    assert_eq!((create.status, create.code()), (StatusCode::FORBIDDEN, "PHONE_NOT_VERIFIED"));

    // Resending replaces the old code; the old one stops working.
    let resend =
        call(&srv, Req::post("/api/v1/auth/verify-phone/resend", json!({ "phone": "01811000003" })).build()).await;
    assert_eq!(resend.status, StatusCode::ACCEPTED);
    let old = call(
        &srv,
        Req::post("/api/v1/auth/verify-phone", json!({ "phone": "01811000003", "code": reg.body["dev_code"] })).build(),
    )
    .await;
    assert_eq!(old.code(), "OTP_INVALID");
    let new = call(
        &srv,
        Req::post("/api/v1/auth/verify-phone", json!({ "phone": "01811000003", "code": resend.body["dev_code"] }))
            .build(),
    )
    .await;
    assert_eq!(new.status, StatusCode::OK);

    // A third send within 15 minutes is the limit (AUTH-R04); the fourth is refused.
    let reg2 = call(
        &srv,
        Req::post(
            "/api/v1/auth/register",
            json!({ "full_name": "Babul", "phone": "01911000004", "password": PASSWORD }),
        )
        .build(),
    )
    .await;
    assert_eq!(reg2.status, StatusCode::CREATED);
    for _ in 0..2 {
        let r =
            call(&srv, Req::post("/api/v1/auth/verify-phone/resend", json!({ "phone": "01911000004" })).build()).await;
        assert_eq!(r.status, StatusCode::ACCEPTED);
    }
    let limited =
        call(&srv, Req::post("/api/v1/auth/verify-phone/resend", json!({ "phone": "01911000004" })).build()).await;
    assert_eq!((limited.status, limited.code()), (StatusCode::TOO_MANY_REQUESTS, "OTP_RATE_LIMITED"));
    assert!(limited.headers.contains_key("retry-after"));

    // Unknown numbers get the same answer as real ones, without a code.
    let unknown =
        call(&srv, Req::post("/api/v1/auth/verify-phone/resend", json!({ "phone": "01511000099" })).build()).await;
    assert_eq!(unknown.status, StatusCode::ACCEPTED);
    assert!(unknown.body.get("dev_code").is_none());
}

/// AC-AUTH-01: 5 wrong passwords lock the identifier + IP; the 6th attempt is
/// refused with 429 and `retry_after` even with the right password.
#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn login_lockout_after_five_failures(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool.clone(), common::config()))).await;
    sign_up(&srv, "Rahim", "01711000005").await;

    let attempt = |password: &'static str| {
        Req::post("/api/v1/auth/login", json!({ "identifier": "01711000005", "password": password })).build()
    };
    for i in 1..=4 {
        let res = call(&srv, attempt("wrong password!")).await;
        assert_eq!((res.status, res.code()), (StatusCode::UNAUTHORIZED, "INVALID_CREDENTIALS"), "attempt {i}");
    }
    let fifth = call(&srv, attempt("wrong password!")).await;
    assert_eq!(fifth.status, StatusCode::TOO_MANY_REQUESTS);
    let sixth = call(&srv, attempt(PASSWORD)).await;
    assert_eq!((sixth.status, sixth.code()), (StatusCode::TOO_MANY_REQUESTS, "TOO_MANY_ATTEMPTS"));
    let retry_after = sixth.body["retry_after"].as_u64().expect("retry_after");
    assert!((800..=900).contains(&retry_after), "retry_after = {retry_after}");
    assert_eq!(sixth.headers.get("retry-after").and_then(|v| v.to_str().ok()), Some(retry_after.to_string().as_str()));

    let lockouts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM auth_events WHERE event = 'auth.lockout'")
        .fetch_one(&pool)
        .await
        .expect("events");
    assert_eq!(lockouts, 1);

    // Unknown accounts fail exactly like wrong passwords.
    let unknown = call(
        &srv,
        Req::post("/api/v1/auth/login", json!({ "identifier": "nobody@example.com", "password": PASSWORD })).build(),
    )
    .await;
    assert_eq!((unknown.status, unknown.code()), (StatusCode::UNAUTHORIZED, "INVALID_CREDENTIALS"));
}

/// AC-AUTH-02: refresh rotates the token; replaying an old one revokes the family.
#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn refresh_rotation_and_reuse_detection(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool, common::config()))).await;
    let session = sign_up(&srv, "Rahim", "01711000006").await;
    let first = session.cookie.clone();

    let r1 = call(&srv, Req::post("/api/v1/auth/refresh", json!({})).cookie(&first).build()).await;
    assert_eq!(r1.status, StatusCode::OK, "{}", r1.body);
    let second = r1.cookie();
    assert_ne!(first, second);

    let r2 = call(&srv, Req::post("/api/v1/auth/refresh", json!({})).cookie(&second).build()).await;
    assert_eq!(r2.status, StatusCode::OK);
    let third = r2.cookie();

    // The rotated first token comes back: someone copied it.
    let reuse = call(&srv, Req::post("/api/v1/auth/refresh", json!({})).cookie(&first).build()).await;
    assert_eq!((reuse.status, reuse.code()), (StatusCode::UNAUTHORIZED, "REFRESH_REUSED"));
    assert_eq!(reuse.refresh_cookie.as_deref(), Some(""), "cookie is cleared");

    // The whole family is gone, including the newest token.
    let latest = call(&srv, Req::post("/api/v1/auth/refresh", json!({})).cookie(&third).build()).await;
    assert_eq!((latest.status, latest.code()), (StatusCode::UNAUTHORIZED, "SESSION_INVALID"));

    let missing = call(&srv, Req::post("/api/v1/auth/refresh", json!({})).build()).await;
    assert_eq!(missing.status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn logout_ends_the_session(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool, common::config()))).await;
    let session = sign_up(&srv, "Rahim", "01711000007").await;

    let out = call(&srv, Req::post("/api/v1/auth/logout", json!({})).cookie(&session.cookie).build()).await;
    assert_eq!(out.status, StatusCode::NO_CONTENT);
    assert_eq!(out.refresh_cookie.as_deref(), Some(""));
    let after = call(&srv, Req::post("/api/v1/auth/refresh", json!({})).cookie(&session.cookie).build()).await;
    assert_eq!(after.status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn password_reset_signs_out_everywhere(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool, common::config()))).await;
    let session = sign_up(&srv, "Rahim", "01711000008").await;

    let forgot =
        call(&srv, Req::post("/api/v1/auth/password/forgot", json!({ "identifier": "01711000008" })).build()).await;
    assert_eq!(forgot.status, StatusCode::ACCEPTED);
    let code = forgot.body["dev_code"].as_str().expect("code").to_owned();

    let weak = call(
        &srv,
        Req::post(
            "/api/v1/auth/password/reset",
            json!({ "identifier": "01711000008", "code": code, "new_password": "12345678" }),
        )
        .build(),
    )
    .await;
    assert_eq!(weak.status, StatusCode::BAD_REQUEST);

    let reset = call(
        &srv,
        Req::post(
            "/api/v1/auth/password/reset",
            json!({ "identifier": "01711000008", "code": code, "new_password": "a brand new secret" }),
        )
        .build(),
    )
    .await;
    assert_eq!(reset.status, StatusCode::NO_CONTENT, "{}", reset.body);

    let old_session = call(&srv, Req::post("/api/v1/auth/refresh", json!({})).cookie(&session.cookie).build()).await;
    assert_eq!(old_session.status, StatusCode::UNAUTHORIZED);
    let old_password = call(
        &srv,
        Req::post("/api/v1/auth/login", json!({ "identifier": "01711000008", "password": PASSWORD })).build(),
    )
    .await;
    assert_eq!(old_password.status, StatusCode::UNAUTHORIZED);
    let new_password = call(
        &srv,
        Req::post("/api/v1/auth/login", json!({ "identifier": "+8801711000008", "password": "a brand new secret" }))
            .build(),
    )
    .await;
    assert_eq!(new_password.status, StatusCode::OK);

    // Unknown accounts get the same 202 with no code.
    let unknown =
        call(&srv, Req::post("/api/v1/auth/password/forgot", json!({ "identifier": "ghost@example.com" })).build())
            .await;
    assert_eq!(unknown.status, StatusCode::ACCEPTED);
    assert!(unknown.body.get("dev_code").is_none());
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn list_and_revoke_own_sessions(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool, common::config()))).await;
    let phone_session = sign_up(&srv, "Rahim", "01711000009").await;
    let laptop = call(
        &srv,
        Req::post("/api/v1/auth/login", json!({ "identifier": "01711000009", "password": PASSWORD })).build(),
    )
    .await;

    let list = call(&srv, Req::get("/api/v1/auth/sessions").bearer(&laptop.token()).build()).await;
    let sessions = list.body.as_array().expect("array").clone();
    assert_eq!(sessions.len(), 2);
    let current: Vec<_> = sessions.iter().filter(|s| s["current"] == true).collect();
    assert_eq!(current.len(), 1);
    let other = sessions.iter().find(|s| s["current"] == false).expect("other session");

    let revoke = call(
        &srv,
        Req::delete(&format!("/api/v1/auth/sessions/{}", other["id"].as_str().unwrap_or_default()))
            .bearer(&laptop.token())
            .build(),
    )
    .await;
    assert_eq!(revoke.status, StatusCode::NO_CONTENT);
    let revoked = call(&srv, Req::post("/api/v1/auth/refresh", json!({})).cookie(&phone_session.cookie).build()).await;
    assert_eq!(revoked.status, StatusCode::UNAUTHORIZED);

    // Revoking twice (or someone else's id) is a 404.
    let again = call(
        &srv,
        Req::delete(&format!("/api/v1/auth/sessions/{}", other["id"].as_str().unwrap_or_default()))
            .bearer(&laptop.token())
            .build(),
    )
    .await;
    assert_eq!(again.status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn protected_routes_need_a_valid_token(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool, common::config()))).await;
    let none = call(&srv, Req::get("/api/v1/auth/me").build()).await;
    assert_eq!((none.status, none.code()), (StatusCode::UNAUTHORIZED, "UNAUTHORIZED"));
    let garbage = call(&srv, Req::get("/api/v1/auth/me").bearer("not-a-token").build()).await;
    assert_eq!((garbage.status, garbage.code()), (StatusCode::UNAUTHORIZED, "TOKEN_INVALID"));

    let mut session = sign_up(&srv, "Rahim", "01711000010").await;
    let no_tenant = call(&srv, Req::get("/api/v1/settings").bearer(&session.token).build()).await;
    assert_eq!((no_tenant.status, no_tenant.code()), (StatusCode::FORBIDDEN, "TENANT_REQUIRED"));

    create_business(&srv, &mut session, "Rahim Store").await;
    let ok = call(&srv, Req::get("/api/v1/settings").bearer(&session.token).build()).await;
    assert_eq!(ok.status, StatusCode::OK);
}
