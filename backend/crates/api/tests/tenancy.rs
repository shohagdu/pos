//! Business provisioning, settings, branches, onboarding and tenant access
//! rules over HTTP (E1.4, E1.5, E1.9; AC-ONB-02, AC-AUTH-03/04, AUTH-R07/R08).

mod common;

use actix_web::{http::StatusCode, test};
use common::{PASSWORD, Req, call, create_business, sign_up, switch};
use pos_api::{AppState, app};
use pos_core::id::new_id;
use serde_json::json;
use sqlx::MySqlPool;
use uuid::Uuid;

async fn count(pool: &MySqlPool, sql: &str, tenant_id: Uuid) -> i64 {
    sqlx::query_scalar(sqlx::AssertSqlSafe(sql.to_owned())).bind(tenant_id).fetch_one(pool).await.expect(sql)
}

/// Runs an admin-style change on tenant data (what Super Admin tooling will do).
async fn platform_update(pool: &MySqlPool, sql: &'static str, tenant_id: Uuid, id: Uuid) {
    let ctx = pos_core::PlatformCtx { actor_id: None, reason: "test".into(), meta: Default::default() };
    let mut tx = pos_db::begin_platform(pool, &ctx).await.expect("platform tx");
    sqlx::query(sql).bind(tenant_id).bind(id).execute(&mut *tx).await.expect(sql);
    tx.commit().await.expect("commit");
}

fn uuid(s: &str) -> Uuid {
    Uuid::parse_str(s).expect("uuid")
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn provisioning_creates_all_defaults(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool.clone(), common::config()))).await;
    let mut session = sign_up(&srv, "Rahim Uddin", "01711000101").await;

    let created = call(
        &srv,
        Req::post(
            "/api/v1/tenants",
            json!({ "name": "Rahim General Store", "business_type": "grocery", "address": "Mirpur 10, Dhaka", "district": "Dhaka" }),
        )
        .bearer(&session.token)
        .build(),
    )
    .await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
    assert_eq!(created.body["slug"], "rahim-general-store");
    let tenant = uuid(created.body["tenant_id"].as_str().unwrap_or_default());

    for (sql, expected) in [
        ("SELECT COUNT(*) FROM tenants WHERE id = ? AND status = 'active'", 1),
        ("SELECT COUNT(*) FROM tenant_settings WHERE tenant_id = ?", 1),
        ("SELECT COUNT(*) FROM branches WHERE tenant_id = ? AND is_default AND default_location_id IS NOT NULL", 1),
        ("SELECT COUNT(*) FROM stock_locations WHERE tenant_id = ?", 2),
        ("SELECT COUNT(*) FROM stock_locations WHERE tenant_id = ? AND code = 'DAMAGED' AND NOT is_sellable", 1),
        ("SELECT COUNT(*) FROM roles WHERE tenant_id = ? AND is_system", 8),
        (
            "SELECT COUNT(*) FROM role_permissions rp JOIN roles r ON r.tenant_id = rp.tenant_id AND r.id = rp.role_id \
          WHERE rp.tenant_id = ? AND r.code = 'owner'",
            64,
        ),
        ("SELECT COUNT(*) FROM tenant_memberships WHERE tenant_id = ? AND is_owner AND status = 'active'", 1),
        ("SELECT COUNT(*) FROM membership_roles WHERE tenant_id = ? AND branch_id IS NULL", 1),
        ("SELECT COUNT(*) FROM document_sequences WHERE tenant_id = ?", 12),
        ("SELECT COUNT(*) FROM document_sequences WHERE tenant_id = ? AND branch_id IS NULL AND NOT include_period", 3),
        (
            "SELECT COUNT(*) FROM subscriptions WHERE tenant_id = ? AND status = 'trialing' \
          AND trial_ends_at BETWEEN NOW(6) + INTERVAL 13 DAY AND NOW(6) + INTERVAL 15 DAY",
            1,
        ),
        ("SELECT COUNT(*) FROM subscription_events WHERE tenant_id = ? AND event_type = 'trial_started'", 1),
        ("SELECT COUNT(*) FROM audit_logs WHERE tenant_id = ? AND action = 'tenant.provisioned'", 1),
    ] {
        assert_eq!(count(&pool, sql, tenant).await, expected, "{sql}");
    }

    // Same name again: the slug gets a suffix.
    let second = call(
        &srv,
        Req::post("/api/v1/tenants", json!({ "name": "Rahim General Store", "business_type": "retail" }))
            .bearer(&session.token)
            .build(),
    )
    .await;
    assert_eq!(second.status, StatusCode::CREATED);
    let slug = second.body["slug"].as_str().unwrap_or_default();
    assert!(slug.starts_with("rahim-general-store-") && slug.len() == "rahim-general-store-".len() + 5, "{slug}");

    // Two businesses: login no longer picks one; /me lists both.
    let login = call(
        &srv,
        Req::post("/api/v1/auth/login", json!({ "identifier": "01711000101", "password": PASSWORD })).build(),
    )
    .await;
    assert!(login.body["tenant_id"].is_null());
    let me = call(&srv, Req::get("/api/v1/auth/me").bearer(&login.token()).build()).await;
    assert_eq!(me.body["memberships"].as_array().map(Vec::len), Some(2));

    switch(&srv, &mut session, Some(&tenant.to_string())).await;
    let me = call(&srv, Req::get("/api/v1/auth/me").bearer(&session.token).build()).await;
    assert_eq!(me.body["tenant"]["name"], "Rahim General Store");
    assert_eq!(me.body["tenant"]["is_owner"], true);
    assert_eq!(me.body["tenant"]["permissions"].as_array().map(Vec::len), Some(64));
    assert_eq!(me.body["tenant"]["subscription"]["status"], "trialing");
    assert_eq!(me.body["tenant"]["subscription"]["plan_code"], "trial");
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn provisioning_validates_the_profile(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool, common::config()))).await;
    let session = sign_up(&srv, "Rahim", "01711000102").await;
    for (body, field) in [
        (json!({ "name": "R", "business_type": "retail" }), "name"),
        (json!({ "name": "Rahim Store", "business_type": "casino" }), "business_type"),
        (json!({ "name": "Rahim Store", "business_type": "retail", "email": "bad" }), "email"),
    ] {
        let res = call(&srv, Req::post("/api/v1/tenants", body).bearer(&session.token).build()).await;
        assert_eq!(res.status, StatusCode::BAD_REQUEST);
        assert_eq!(res.body["errors"][0]["field"], field);
    }
}

/// AC-ONB-02: if any provisioning insert fails, no tenant rows remain.
#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn failed_provisioning_leaves_no_rows(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool.clone(), common::config()))).await;
    let session = sign_up(&srv, "Rahim", "01711000103").await;
    // Remove the trial plan so the subscription step (near the end) fails.
    sqlx::query("DELETE FROM plans WHERE code = 'trial'").execute(&pool).await.expect("drop trial plan");

    let res = call(
        &srv,
        Req::post("/api/v1/tenants", json!({ "name": "Doomed Store", "business_type": "retail" }))
            .bearer(&session.token)
            .build(),
    )
    .await;
    assert_eq!(res.status, StatusCode::INTERNAL_SERVER_ERROR);

    for table in [
        "tenant_settings",
        "branches",
        "stock_locations",
        "roles",
        "role_permissions",
        "tenant_memberships",
        "document_sequences",
    ] {
        let rows: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT COUNT(*) FROM {table}")))
            .fetch_one(&pool)
            .await
            .expect(table);
        assert_eq!(rows, 0, "{table} has leftover rows");
    }
    let tenants: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tenants WHERE slug <> 'platform'")
        .fetch_one(&pool)
        .await
        .expect("tenants");
    assert_eq!(tenants, 0);
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn settings_use_optimistic_concurrency(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool, common::config()))).await;
    let mut session = sign_up(&srv, "Rahim", "01711000104").await;
    create_business(&srv, &mut session, "Rahim Store").await;

    let get = call(&srv, Req::get("/api/v1/settings").bearer(&session.token).build()).await;
    assert_eq!(get.status, StatusCode::OK);
    assert_eq!(get.headers.get("etag").and_then(|v| v.to_str().ok()), Some("\"1\""));
    assert_eq!(get.body["timezone"], "Asia/Dhaka");
    assert_eq!(get.body["currency_code"], "BDT");
    assert_eq!(get.body["fiscal_year_start_month"], 7);
    assert_eq!(get.body["invoice_prefix"], "INV");

    let patch = json!({ "fiscal_year_start_month": 1, "invoice_prefix": "mir", "receipt_paper": "58mm", "is_vat_registered": true });
    let missing = call(&srv, Req::patch("/api/v1/settings", patch.clone()).bearer(&session.token).build()).await;
    assert_eq!(missing.status, StatusCode::BAD_REQUEST);

    let updated =
        call(&srv, Req::patch("/api/v1/settings", patch.clone()).bearer(&session.token).if_match(1).build()).await;
    assert_eq!(updated.status, StatusCode::OK, "{}", updated.body);
    assert_eq!(updated.body["version"], 2);
    assert_eq!(updated.body["invoice_prefix"], "MIR");
    assert_eq!(updated.headers.get("etag").and_then(|v| v.to_str().ok()), Some("\"2\""));

    let stale = call(
        &srv,
        Req::patch("/api/v1/settings", json!({ "locale": "bn" })).bearer(&session.token).if_match(1).build(),
    )
    .await;
    assert_eq!((stale.status, stale.code()), (StatusCode::PRECONDITION_FAILED, "VERSION_MISMATCH"));

    let unknown_field = call(
        &srv,
        Req::patch("/api/v1/settings", json!({ "tenant_id": "x" })).bearer(&session.token).if_match(2).build(),
    )
    .await;
    assert_eq!(unknown_field.status, StatusCode::BAD_REQUEST, "tenant ids are never taken from the body");

    // An Italian shop: Partita IVA and codice fiscale are checked, and the receipt prints them.
    let italy = json!({ "country_code": "IT", "legal_name": "di Uddin Md Jashim", "bin_number": "IT03580690984",
        "tax_code": "ddnmjs87t15z249i", "rea_number": "546330", "registry_office": "Brescia",
        "timezone": "Europe/Rome", "currency_code": "EUR" });
    let wrong = call(
        &srv,
        Req::patch("/api/v1/settings", json!({ "country_code": "IT", "bin_number": "03580690985" }))
            .bearer(&session.token)
            .if_match(2)
            .build(),
    )
    .await;
    assert_eq!(
        (wrong.status, wrong.body["errors"][0]["code"].as_str()),
        (StatusCode::BAD_REQUEST, Some("INVALID_VAT_NUMBER"))
    );
    let italian = call(&srv, Req::patch("/api/v1/settings", italy).bearer(&session.token).if_match(2).build()).await;
    assert_eq!(italian.status, StatusCode::OK, "{}", italian.body);
    assert_eq!(italian.body["bin_number"], "03580690984");
    assert_eq!(italian.body["tax_code"], "DDNMJS87T15Z249I");
    let pos = call(&srv, Req::get("/api/v1/pos/context").bearer(&session.token).build()).await;
    assert_eq!(pos.status, StatusCode::OK, "{}", pos.body);
    let receipt = &pos.body["receipt"];
    assert_eq!(
        (&receipt["country_code"], &receipt["legal_name"], &receipt["bin_number"], &receipt["tax_code"]),
        (&json!("IT"), &json!("di Uddin Md Jashim"), &json!("03580690984"), &json!("DDNMJS87T15Z249I"))
    );
    assert_eq!((&receipt["rea_number"], &receipt["registry_office"]), (&json!("546330"), &json!("Brescia")));
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn branch_update_and_onboarding_progress(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool, common::config()))).await;
    let mut session = sign_up(&srv, "Rahim", "01711000105").await;
    create_business(&srv, &mut session, "Rahim Store").await;

    let branches = call(&srv, Req::get("/api/v1/branches").bearer(&session.token).build()).await;
    let branch = branches.body[0].clone();
    assert_eq!((branch["code"].as_str(), branch["is_default"].as_bool()), (Some("MAIN"), Some(true)));
    let id = branch["id"].as_str().unwrap_or_default();

    let renamed = call(
        &srv,
        Req::patch(&format!("/api/v1/branches/{id}"), json!({ "name": "Mirpur Outlet", "phone": "01711000105" }))
            .bearer(&session.token)
            .if_match(1)
            .build(),
    )
    .await;
    assert_eq!(renamed.status, StatusCode::OK, "{}", renamed.body);
    assert_eq!((renamed.body["name"].as_str(), renamed.body["version"].as_i64()), (Some("Mirpur Outlet"), Some(2)));
    assert_eq!(renamed.body["phone"], "+8801711000105");

    let status = call(&srv, Req::get("/api/v1/onboarding").bearer(&session.token).build()).await;
    let done: Vec<_> = status.body["steps"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|s| s["done"] == true)
        .map(|s| s["key"].clone())
        .collect();
    assert_eq!(done, vec![json!("business")]);

    for step in ["regional", "branch", "regional"] {
        let res = call(
            &srv,
            Req::post(&format!("/api/v1/onboarding/steps/{step}"), json!({})).bearer(&session.token).build(),
        )
        .await;
        assert_eq!(res.status, StatusCode::OK);
    }
    let unknown =
        call(&srv, Req::post("/api/v1/onboarding/steps/nope", json!({})).bearer(&session.token).build()).await;
    assert_eq!(unknown.status, StatusCode::NOT_FOUND);

    for step in ["invoice", "products", "staff"] {
        call(&srv, Req::post(&format!("/api/v1/onboarding/steps/{step}"), json!({})).bearer(&session.token).build())
            .await;
    }
    let status = call(&srv, Req::get("/api/v1/onboarding").bearer(&session.token).build()).await;
    assert_eq!(status.body["complete"], true);
    let dismissed = call(&srv, Req::post("/api/v1/onboarding/dismiss", json!({})).bearer(&session.token).build()).await;
    assert_eq!(dismissed.body["dismissed"], true);

    // Onboarding progress does not bump the settings version.
    let settings = call(&srv, Req::get("/api/v1/settings").bearer(&session.token).build()).await;
    assert_eq!(settings.body["version"], 1);
}

/// AC-AUTH-04 and the isolation rule for these routes (SRS-TEN-005): another
/// tenant's ids are 404, also for a user who is a member of both.
#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn other_tenants_resources_are_not_found(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool, common::config()))).await;
    let mut alice = sign_up(&srv, "Alice", "01711000106").await;
    let alice_tenant = create_business(&srv, &mut alice, "Alice Fashion").await;
    let alice_branch =
        call(&srv, Req::get("/api/v1/branches").bearer(&alice.token).build()).await.body[0]["id"].clone();

    let mut bob = sign_up(&srv, "Bob", "01711000107").await;
    let bob_tenant = create_business(&srv, &mut bob, "Bob Electronics").await;
    let attempt = call(
        &srv,
        Req::patch(
            &format!("/api/v1/branches/{}", alice_branch.as_str().unwrap_or_default()),
            json!({ "name": "pwned" }),
        )
        .bearer(&bob.token)
        .if_match(1)
        .build(),
    )
    .await;
    assert_eq!(attempt.status, StatusCode::NOT_FOUND);
    let bob_branches = call(&srv, Req::get("/api/v1/branches").bearer(&bob.token).build()).await;
    assert_eq!(bob_branches.body.as_array().map(Vec::len), Some(1));
    assert_ne!(bob_branches.body[0]["id"], alice_branch);

    // Alice also owns a second business; with the first active, the second's ids are 404.
    let second = call(
        &srv,
        Req::post("/api/v1/tenants", json!({ "name": "Alice Grocery", "business_type": "grocery" }))
            .bearer(&alice.token)
            .build(),
    )
    .await;
    let second_id = second.body["tenant_id"].as_str().unwrap_or_default().to_owned();
    let mut alice_second =
        common::Session { token: alice.token.clone(), cookie: alice.cookie.clone(), phone: alice.phone.clone() };
    switch(&srv, &mut alice_second, Some(&second_id)).await;
    let second_branch =
        call(&srv, Req::get("/api/v1/branches").bearer(&alice_second.token).build()).await.body[0]["id"].clone();
    // The switch rotated the shared refresh token, so sign in again for the first business.
    let relogin = call(
        &srv,
        Req::post("/api/v1/auth/login", json!({ "identifier": "01711000106", "password": PASSWORD })).build(),
    )
    .await;
    let mut alice_first =
        common::Session { token: relogin.token(), cookie: relogin.cookie(), phone: alice.phone.clone() };
    switch(&srv, &mut alice_first, Some(&alice_tenant)).await;
    let cross = call(
        &srv,
        Req::patch(&format!("/api/v1/branches/{}", second_branch.as_str().unwrap_or_default()), json!({ "name": "x" }))
            .bearer(&alice_first.token)
            .if_match(1)
            .build(),
    )
    .await;
    assert_eq!(cross.status, StatusCode::NOT_FOUND);

    // Switching to a business you do not belong to is a 404, not a 403.
    let foreign = call(
        &srv,
        Req::post("/api/v1/auth/switch-tenant", json!({ "tenant_id": bob_tenant }))
            .bearer(&alice_first.token)
            .cookie(&alice_first.cookie)
            .build(),
    )
    .await;
    assert_eq!(foreign.status, StatusCode::NOT_FOUND);
}

/// RBAC guard, AUTH-R07 (stale permission version) and AUTH-R08 (deactivation).
#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn permissions_and_membership_changes_apply_immediately(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool.clone(), common::config()))).await;
    let mut owner = sign_up(&srv, "Owner", "01711000108").await;
    let tenant_id = uuid(&create_business(&srv, &mut owner, "Rahim Store").await);

    // Add a cashier directly (the invitation flow is E1.10).
    let mut cashier = sign_up(&srv, "Cashier", "01711000109").await;
    let cashier_user: Uuid =
        sqlx::query_scalar("SELECT id FROM users WHERE phone = '+8801711000109'").fetch_one(&pool).await.expect("user");
    let membership_id = new_id();
    let ctx = pos_core::TenantCtx { tenant_id, user_id: cashier_user, meta: Default::default() };
    let mut tx = pos_db::begin_tenant(&pool, &ctx).await.expect("tx");
    sqlx::query("INSERT INTO tenant_memberships (tenant_id, id, user_id, status) VALUES (?, ?, ?, 'active')")
        .bind(tenant_id)
        .bind(membership_id)
        .bind(cashier_user)
        .execute(&mut *tx)
        .await
        .expect("membership");
    sqlx::query(
        "INSERT INTO membership_roles (tenant_id, id, membership_id, role_id) \
         SELECT tenant_id, ?, ?, id FROM roles WHERE tenant_id = ? AND code = 'cashier'",
    )
    .bind(new_id())
    .bind(membership_id)
    .bind(tenant_id)
    .execute(&mut *tx)
    .await
    .expect("role");
    tx.commit().await.expect("commit");

    switch(&srv, &mut cashier, Some(&tenant_id.to_string())).await;
    let me = call(&srv, Req::get("/api/v1/auth/me").bearer(&cashier.token).build()).await;
    let perms: Vec<String> = serde_json::from_value(me.body["tenant"]["permissions"].clone()).unwrap_or_default();
    assert!(perms.contains(&"pos.sell".to_owned()) && !perms.contains(&"settings.view".to_owned()), "{perms:?}");

    let settings = call(&srv, Req::get("/api/v1/settings").bearer(&cashier.token).build()).await;
    assert_eq!((settings.status, settings.code()), (StatusCode::FORBIDDEN, "PERMISSION_DENIED"));
    let branches = call(&srv, Req::get("/api/v1/branches").bearer(&cashier.token).build()).await;
    assert_eq!(branches.status, StatusCode::OK);

    // AUTH-R07: a permission change bumps perms_version; the old token is stale.
    platform_update(
        &pool,
        "UPDATE tenant_memberships SET perms_version = perms_version + 1 WHERE tenant_id = ? AND id = ?",
        tenant_id,
        membership_id,
    )
    .await;
    let stale = call(&srv, Req::get("/api/v1/branches").bearer(&cashier.token).build()).await;
    assert_eq!((stale.status, stale.code()), (StatusCode::UNAUTHORIZED, "TOKEN_STALE"));
    let refreshed = call(&srv, Req::post("/api/v1/auth/refresh", json!({})).cookie(&cashier.cookie).build()).await;
    assert_eq!(refreshed.status, StatusCode::OK);
    let fresh = call(&srv, Req::get("/api/v1/branches").bearer(&refreshed.token()).build()).await;
    assert_eq!(fresh.status, StatusCode::OK);

    // AUTH-R08: a suspended membership loses access on the next request.
    platform_update(
        &pool,
        "UPDATE tenant_memberships SET status = 'suspended' WHERE tenant_id = ? AND id = ?",
        tenant_id,
        membership_id,
    )
    .await;
    let suspended = call(&srv, Req::get("/api/v1/branches").bearer(&refreshed.token()).build()).await;
    assert_eq!((suspended.status, suspended.code()), (StatusCode::UNAUTHORIZED, "MEMBERSHIP_INACTIVE"));
    // The next refresh drops the business from the token instead of failing.
    let after = call(&srv, Req::post("/api/v1/auth/refresh", json!({})).cookie(&refreshed.cookie()).build()).await;
    assert_eq!(after.status, StatusCode::OK);
    assert!(after.body["tenant_id"].is_null());
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn suspended_tenants_are_refused(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool.clone(), common::config()))).await;
    let mut owner = sign_up(&srv, "Owner", "01711000110").await;
    let tenant_id = uuid(&create_business(&srv, &mut owner, "Rahim Store").await);
    sqlx::query("UPDATE tenants SET status = 'suspended' WHERE id = ?")
        .bind(tenant_id)
        .execute(&pool)
        .await
        .expect("suspend");

    let res = call(&srv, Req::get("/api/v1/branches").bearer(&owner.token).build()).await;
    assert_eq!((res.status, res.code()), (StatusCode::FORBIDDEN, "TENANT_SUSPENDED"));
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn permission_catalogue_matches_the_seed(pool: MySqlPool) {
    let mut seeded: Vec<String> =
        sqlx::query_scalar("SELECT code FROM permissions").fetch_all(&pool).await.expect("permissions");
    seeded.sort();
    let mut catalogue: Vec<String> = pos_auth::rbac::PERMISSIONS.iter().map(|p| (*p).to_owned()).collect();
    catalogue.sort();
    assert_eq!(seeded, catalogue);
}
