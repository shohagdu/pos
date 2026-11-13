//! Adding staff directly (`POST /members`): the account signs in at once
//! with the password the owner set, no invitation. A number that already has
//! an account joins with its own password, which the owner cannot change.
//! Staff then replace the owner-set password with `POST /auth/password/change`.
//!
//! One test function: every `#[sqlx::test]` migrates a fresh database.

mod common;

use actix_web::{http::StatusCode, test};
use common::{PASSWORD, Req, call, create_business, role_id, sign_up, switch, token_of};
use pos_api::{AppState, app};
use serde_json::json;
use sqlx::MySqlPool;

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn owner_adds_staff_with_a_password(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool, common::config()))).await;
    let mut owner = sign_up(&srv, "Owner", "01711000971").await;
    let tenant_id = create_business(&srv, &mut owner, "Corner Shop").await;
    let cashier = role_id(&srv, &owner, "cashier").await;
    let add = |body: serde_json::Value| Req::post("/api/v1/members", body).bearer(&owner.token).build();

    // The policy applies to the first password, and a role is needed.
    let weak = call(
        &srv,
        add(json!({ "full_name": "Karim", "phone": "01711000972", "password": "123", "role_ids": [&cashier] })),
    )
    .await;
    assert_eq!(weak.status, StatusCode::BAD_REQUEST, "{}", weak.body);
    assert_eq!(weak.body["errors"][0]["field"], "password");
    let no_role = call(
        &srv,
        add(json!({ "full_name": "Karim", "phone": "01711000972", "password": "Shop-pass-2026", "role_ids": [] })),
    )
    .await;
    assert_eq!(no_role.body["errors"][0]["field"], "role_ids", "{}", no_role.body);
    let owner_role = role_id(&srv, &owner, "owner").await;
    let as_owner = call(&srv, add(json!({ "full_name": "Karim", "phone": "01711000972", "password": "Shop-pass-2026", "role_ids": [owner_role] }))).await;
    assert!(as_owner.status.is_client_error(), "the owner role is never handed out: {}", as_owner.body);

    let created = call(
        &srv,
        add(json!({ "full_name": "Karim Cashier", "phone": "01711000972", "password": "Shop-pass-2026", "role_ids": [&cashier] })),
    )
    .await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
    assert_eq!(created.body["existing_account"], false);
    assert_eq!(created.body["member"]["status"], "active");
    assert_eq!(created.body["member"]["roles"][0]["code"], "cashier");
    let again = call(
        &srv,
        add(json!({ "full_name": "Karim Cashier", "phone": "01711000972", "password": "Shop-pass-2026", "role_ids": [&cashier] })),
    )
    .await;
    assert_eq!((again.status, again.code()), (StatusCode::CONFLICT, "ALREADY_MEMBER"), "{}", again.body);

    // The new account signs in straight away (no SMS code) and works in the business.
    let login = call(
        &srv,
        Req::post("/api/v1/auth/login", json!({ "identifier": "01711000972", "password": "Shop-pass-2026" })).build(),
    )
    .await;
    assert_eq!(login.status, StatusCode::OK, "{}", login.body);
    let mut karim = common::Session { token: login.token(), cookie: login.cookie(), phone: "01711000972".into() };
    switch(&srv, &mut karim, Some(&tenant_id)).await;
    let sales = call(&srv, Req::get("/api/v1/sales").bearer(&karim.token).build()).await;
    assert_eq!(sales.status, StatusCode::OK, "{}", sales.body);

    // Karim replaces the password the owner set. His other device is signed
    // out; this one stays signed in.
    let other = call(
        &srv,
        Req::post("/api/v1/auth/login", json!({ "identifier": "01711000972", "password": "Shop-pass-2026" })).build(),
    )
    .await;
    let change = |current: &str, new: &str| {
        Req::post("/api/v1/auth/password/change", json!({ "current_password": current, "new_password": new }))
            .bearer(&karim.token)
            .build()
    };
    let wrong = call(&srv, change("not-my-password", "Karim-own-2026")).await;
    assert_eq!(wrong.status, StatusCode::BAD_REQUEST, "{}", wrong.body);
    assert_eq!(wrong.body["errors"][0]["code"], "WRONG_PASSWORD");
    let same = call(&srv, change("Shop-pass-2026", "Shop-pass-2026")).await;
    assert_eq!(same.body["errors"][0]["field"], "new_password", "{}", same.body);
    let changed = call(&srv, change("Shop-pass-2026", "Karim-own-2026")).await;
    assert_eq!(changed.status, StatusCode::OK, "{}", changed.body);
    assert_eq!(changed.body["signed_out_sessions"], 1);
    let stale = call(&srv, Req::post("/api/v1/auth/refresh", json!({})).cookie(&other.cookie()).build()).await;
    assert_eq!(stale.status, StatusCode::UNAUTHORIZED, "the other device is signed out: {}", stale.body);
    let kept = call(&srv, Req::post("/api/v1/auth/refresh", json!({})).cookie(&karim.cookie).build()).await;
    assert_eq!(kept.status, StatusCode::OK, "this device stays signed in: {}", kept.body);
    karim.token = kept.token();
    let old = call(
        &srv,
        Req::post("/api/v1/auth/login", json!({ "identifier": "01711000972", "password": "Shop-pass-2026" })).build(),
    )
    .await;
    assert_eq!(old.status, StatusCode::UNAUTHORIZED);
    let new = call(
        &srv,
        Req::post("/api/v1/auth/login", json!({ "identifier": "01711000972", "password": "Karim-own-2026" })).build(),
    )
    .await;
    assert_eq!(new.status, StatusCode::OK, "{}", new.body);

    // Someone who already has an account is invited, not added: the link
    // goes to their number and they join only by accepting it. The typed
    // password is ignored and their own one stays.
    let mut rina = sign_up(&srv, "Rina", "01711000973").await;
    let joined = call(
        &srv,
        add(json!({ "full_name": "Rina", "phone": "01711000973", "password": "Owner-chosen-1", "role_ids": [&cashier] })),
    )
    .await;
    assert_eq!(joined.status, StatusCode::CREATED, "{}", joined.body);
    assert_eq!(joined.body["existing_account"], true);
    assert!(joined.body["member"].is_null(), "not a member yet: {}", joined.body);
    assert_eq!(joined.body["invitation"]["status"], "pending");
    let not_yet = call(
        &srv,
        Req::post("/api/v1/auth/switch-tenant", json!({ "tenant_id": &tenant_id })).cookie(&rina.cookie).build(),
    )
    .await;
    assert_ne!(not_yet.status, StatusCode::OK, "no access before accepting: {}", not_yet.body);
    // The holder accepts with the link sent to the number (a fresh one here).
    let invitation_id = joined.body["invitation"]["id"].as_str().expect("invitation id").to_owned();
    let resent = call(
        &srv,
        Req::post(&format!("/api/v1/invitations/{invitation_id}/resend"), json!({})).bearer(&owner.token).build(),
    )
    .await;
    assert_eq!(resent.status, StatusCode::OK, "{}", resent.body);
    let accepted = call(
        &srv,
        Req::post("/api/v1/invitations/accept", json!({ "token": token_of(&resent.body) })).bearer(&rina.token).build(),
    )
    .await;
    assert_eq!(accepted.status, StatusCode::OK, "{}", accepted.body);
    let typed = call(
        &srv,
        Req::post("/api/v1/auth/login", json!({ "identifier": "01711000973", "password": "Owner-chosen-1" })).build(),
    )
    .await;
    assert_eq!(typed.status, StatusCode::UNAUTHORIZED, "{}", typed.body);
    switch(&srv, &mut rina, Some(&tenant_id)).await;
    let own = call(
        &srv,
        Req::post("/api/v1/auth/login", json!({ "identifier": "01711000973", "password": PASSWORD })).build(),
    )
    .await;
    assert_eq!(own.status, StatusCode::OK, "{}", own.body);

    // Staff without `users.invite` cannot add anyone.
    let denied = call(
        &srv,
        Req::post(
            "/api/v1/members",
            json!({ "full_name": "X", "phone": "01711000974", "password": "Shop-pass-2026", "role_ids": [&cashier] }),
        )
        .bearer(&karim.token)
        .build(),
    )
    .await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN, "{}", denied.body);
}
