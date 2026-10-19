//! Entitlements and subscription state over HTTP (E1.6): SUB-R02 on-request
//! checks, SUB-R03 `can`/`within_limit`, AT-09 (read-only), overrides, cache.

mod common;

use actix_web::{http::StatusCode, test};
use common::{Req, call, create_business, sign_up};
use pos_api::{AppState, app};
use serde_json::{Value, json};
use sqlx::MySqlPool;
use uuid::Uuid;

/// Runs a platform-level change, as Super Admin tooling and the scheduler will.
async fn platform(pool: &MySqlPool, sql: &'static str, tenant_id: Uuid) {
    let ctx = pos_core::PlatformCtx { actor_id: None, reason: "test".into(), meta: Default::default() };
    let mut tx = pos_db::begin_platform(pool, &ctx).await.expect("platform tx");
    sqlx::query(sql).bind(tenant_id).execute(&mut *tx).await.expect(sql);
    tx.commit().await.expect("commit");
}

/// Moves the trial end `days` into the past.
async fn trial_ended(pool: &MySqlPool, state: &AppState, tenant_id: Uuid, days: i64) {
    let ctx = pos_core::PlatformCtx { actor_id: None, reason: "test".into(), meta: Default::default() };
    let mut tx = pos_db::begin_platform(pool, &ctx).await.expect("platform tx");
    sqlx::query(
        "UPDATE subscriptions SET trial_ends_at = NOW(6) - INTERVAL ? DAY, current_period_end = NOW(6) - INTERVAL ? DAY \
         WHERE tenant_id = ?",
    )
    .bind(days)
    .bind(days)
    .bind(tenant_id)
    .execute(&mut *tx)
    .await
    .expect("move trial");
    tx.commit().await.expect("commit");
    state.entitlements.invalidate(tenant_id);
}

fn meter<'a>(body: &'a Value, feature: &str) -> &'a Value {
    body["usage"].as_array().and_then(|u| u.iter().find(|m| m["feature"] == feature)).expect(feature)
}

fn feature<'a>(body: &'a Value, code: &str) -> &'a Value {
    body["features"].as_array().and_then(|f| f.iter().find(|x| x["code"] == code)).expect(code)
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn new_business_is_on_a_trial(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool, common::config()))).await;
    let mut owner = sign_up(&srv, "Owner", "01711000301").await;
    create_business(&srv, &mut owner, "Rahim Store").await;

    let res = call(&srv, Req::get("/api/v1/subscription").bearer(&owner.token).build()).await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.body);
    let sub = &res.body["subscription"];
    assert_eq!((sub["status"].as_str(), sub["plan_code"].as_str()), (Some("trialing"), Some("trial")));
    assert_eq!(sub["days_left"], 14);
    assert_eq!(
        (meter(&res.body, "users.max")["used"].as_i64(), meter(&res.body, "users.max")["limit"].as_i64()),
        (Some(1), Some(3))
    );
    assert_eq!(
        (meter(&res.body, "branches.max")["used"].as_i64(), meter(&res.body, "branches.max")["limit"].as_i64()),
        (Some(1), Some(1))
    );
    assert!(meter(&res.body, "products.max")["used"].is_null(), "counted once the catalogue exists");
    assert_eq!(feature(&res.body, "pos.core")["enabled"], true);
    assert_eq!(feature(&res.body, "inventory.transfers")["enabled"], false);

    let me = call(&srv, Req::get("/api/v1/auth/me").bearer(&owner.token).build()).await;
    let features = me.body["tenant"]["features"].as_array().cloned().unwrap_or_default();
    assert!(features.contains(&json!("pos.core")) && !features.contains(&json!("inventory.transfers")));
    assert_eq!(me.body["tenant"]["subscription"]["days_left"], 14);
}

/// AT-09: once read-only, writes get 402 while reads keep working.
#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn read_only_blocks_writes(pool: MySqlPool) {
    let state = AppState::new(pool.clone(), common::config());
    let srv = test::init_service(app(state.clone())).await;
    let mut owner = sign_up(&srv, "Owner", "01711000311").await;
    let tenant = Uuid::parse_str(&create_business(&srv, &mut owner, "Rahim Store").await).expect("uuid");
    let cashier = call(&srv, Req::get("/api/v1/roles").bearer(&owner.token).build())
        .await
        .body
        .as_array()
        .into_iter()
        .flatten()
        .find(|r| r["code"] == "cashier")
        .map(|r| r["id"].clone())
        .expect("cashier");
    let invitation = call(
        &srv,
        Req::post("/api/v1/invitations", json!({ "destination": "01711000312", "role_ids": [cashier] }))
            .bearer(&owner.token)
            .build(),
    )
    .await;
    let invitation_id = invitation.body["invitation"]["id"].as_str().unwrap_or_default().to_owned();

    // 14-day trial ended 10 days ago: grace (7 days) is over.
    trial_ended(&pool, &state, tenant, 10).await;

    let read = call(&srv, Req::get("/api/v1/branches").bearer(&owner.token).build()).await;
    assert_eq!(read.status, StatusCode::OK);
    let write =
        call(&srv, Req::patch("/api/v1/settings", json!({ "locale": "bn" })).bearer(&owner.token).if_match(1).build())
            .await;
    assert_eq!((write.status, write.code()), (StatusCode::PAYMENT_REQUIRED, "SUBSCRIPTION_READ_ONLY"));
    let invite = call(
        &srv,
        Req::post("/api/v1/invitations", json!({ "destination": "01711000313", "role_ids": [cashier] }))
            .bearer(&owner.token)
            .build(),
    )
    .await;
    assert_eq!(invite.code(), "SUBSCRIPTION_READ_ONLY");
    // Removing things stays possible (SUB-R04).
    let revoke =
        call(&srv, Req::delete(&format!("/api/v1/invitations/{invitation_id}")).bearer(&owner.token).build()).await;
    assert_eq!(revoke.status, StatusCode::NO_CONTENT);

    let sub = call(&srv, Req::get("/api/v1/subscription").bearer(&owner.token).build()).await;
    assert_eq!(sub.body["subscription"]["status"], "read_only");
    assert_eq!(sub.body["subscription"]["stored_status"], "trialing", "the scheduler has not run");
    assert_eq!(sub.body["subscription"]["days_left"], 27);
    let me = call(&srv, Req::get("/api/v1/auth/me").bearer(&owner.token).build()).await;
    assert_eq!(me.body["tenant"]["subscription"]["status"], "read_only");
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn grace_keeps_full_access(pool: MySqlPool) {
    let state = AppState::new(pool.clone(), common::config());
    let srv = test::init_service(app(state.clone())).await;
    let mut owner = sign_up(&srv, "Owner", "01711000321").await;
    let tenant = Uuid::parse_str(&create_business(&srv, &mut owner, "Rahim Store").await).expect("uuid");
    trial_ended(&pool, &state, tenant, 2).await;

    let write =
        call(&srv, Req::patch("/api/v1/settings", json!({ "locale": "bn" })).bearer(&owner.token).if_match(1).build())
            .await;
    assert_eq!(write.status, StatusCode::OK);
    let sub = call(&srv, Req::get("/api/v1/subscription").bearer(&owner.token).build()).await;
    assert_eq!(
        (sub.body["subscription"]["status"].as_str(), sub.body["subscription"]["days_left"].as_i64()),
        (Some("grace"), Some(5))
    );
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn suspended_only_shows_the_plan(pool: MySqlPool) {
    let state = AppState::new(pool.clone(), common::config());
    let srv = test::init_service(app(state.clone())).await;
    let mut owner = sign_up(&srv, "Owner", "01711000331").await;
    let tenant = Uuid::parse_str(&create_business(&srv, &mut owner, "Rahim Store").await).expect("uuid");
    trial_ended(&pool, &state, tenant, 40).await;

    let read = call(&srv, Req::get("/api/v1/branches").bearer(&owner.token).build()).await;
    assert_eq!((read.status, read.code()), (StatusCode::PAYMENT_REQUIRED, "SUBSCRIPTION_SUSPENDED"));
    let sub = call(&srv, Req::get("/api/v1/subscription").bearer(&owner.token).build()).await;
    assert_eq!(sub.status, StatusCode::OK);
    assert_eq!(sub.body["subscription"]["status"], "suspended");
    let me = call(&srv, Req::get("/api/v1/auth/me").bearer(&owner.token).build()).await;
    assert_eq!(me.status, StatusCode::OK);
    assert_eq!(me.body["tenant"]["subscription"]["status"], "suspended");
}

/// Overrides beat the plan; expired ones are ignored; the cache is invalidated on change.
#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn overrides_and_cache(pool: MySqlPool) {
    let state = AppState::new(pool.clone(), common::config());
    let srv = test::init_service(app(state.clone())).await;
    let mut owner = sign_up(&srv, "Owner", "01711000341").await;
    let tenant = Uuid::parse_str(&create_business(&srv, &mut owner, "Rahim Store").await).expect("uuid");
    let owner_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE phone = '+8801711000341'")
        .fetch_one(&pool)
        .await
        .expect("owner");

    // Fill the cache, then change the override behind its back.
    let before = call(&srv, Req::get("/api/v1/auth/me").bearer(&owner.token).build()).await;
    assert!(!before.body["tenant"]["features"].as_array().into_iter().flatten().any(|f| f == "inventory.transfers"));
    let ctx = pos_core::PlatformCtx { actor_id: Some(owner_id), reason: "test".into(), meta: Default::default() };
    let mut tx = pos_db::begin_platform(&pool, &ctx).await.expect("tx");
    for (feature, enabled, limit, expired) in [
        ("inventory.transfers", Some(true), None, false),
        ("users.max", None, Some(5), false),
        ("api.access", Some(true), None, true),
    ] {
        sqlx::query(
            "INSERT INTO tenant_feature_overrides (tenant_id, id, feature_code, enabled, limit_value, expires_at, reason, created_by) \
             VALUES (?, ?, ?, ?, ?, IF(?, NOW(6) - INTERVAL 1 DAY, NULL), 'pilot customer', ?)",
        )
        .bind(tenant)
        .bind(pos_core::id::new_id())
        .bind(feature)
        .bind(enabled)
        .bind(limit)
        .bind(expired)
        .bind(owner_id)
        .execute(&mut *tx)
        .await
        .expect(feature);
    }
    tx.commit().await.expect("commit");

    let cached = call(&srv, Req::get("/api/v1/auth/me").bearer(&owner.token).build()).await;
    assert!(
        !cached.body["tenant"]["features"].as_array().into_iter().flatten().any(|f| f == "inventory.transfers"),
        "served from cache"
    );
    state.entitlements.invalidate(tenant);
    let fresh = call(&srv, Req::get("/api/v1/auth/me").bearer(&owner.token).build()).await;
    let features = fresh.body["tenant"]["features"].as_array().cloned().unwrap_or_default();
    assert!(features.contains(&json!("inventory.transfers")));
    assert!(!features.contains(&json!("api.access")), "expired override is ignored");

    let sub = call(&srv, Req::get("/api/v1/subscription").bearer(&owner.token).build()).await;
    assert_eq!(meter(&sub.body, "users.max")["limit"], 5);
    assert_eq!(feature(&sub.body, "users.max")["overridden"], true);

    // An unlimited override: limit NULL with enabled unset.
    platform(
        &pool,
        "UPDATE tenant_feature_overrides SET limit_value = NULL WHERE tenant_id = ? AND feature_code = 'users.max'",
        tenant,
    )
    .await;
    let sub = call(&srv, Req::get("/api/v1/subscription").bearer(&owner.token).build()).await;
    assert!(meter(&sub.body, "users.max")["limit"].is_null(), "{}", sub.body);

    // Write paths read exact limits in their transaction (no cache): 4 more invites fit.
    let cashier = call(&srv, Req::get("/api/v1/roles").bearer(&owner.token).build())
        .await
        .body
        .as_array()
        .into_iter()
        .flatten()
        .find(|r| r["code"] == "cashier")
        .map(|r| r["id"].clone())
        .expect("cashier");
    for phone in ["01711000342", "01711000343", "01711000344", "01711000345"] {
        let res = call(
            &srv,
            Req::post("/api/v1/invitations", json!({ "destination": phone, "role_ids": [cashier] }))
                .bearer(&owner.token)
                .build(),
        )
        .await;
        assert_eq!(res.status, StatusCode::CREATED, "{phone}: {}", res.body);
    }
}
