//! Owner impersonation: the owner signs in as a staff member of the business
//! and returns to their own account. Owner only, same business only, no
//! account actions while impersonating, audited with the owner's id.
//! Platform support staff can also open a business as its owner and return to
//! the admin console.

mod common;

use actix_http::Request;
use actix_web::{
    Error,
    body::MessageBody,
    dev::{Service, ServiceResponse},
    http::StatusCode,
    test,
};
use common::{Req, Res, Session, call, create_business, join, sign_up};
use pos_api::{AppState, app};
use serde_json::json;
use sqlx::MySqlPool;
use uuid::Uuid;

async fn membership_id<S, B>(srv: &S, owner: &Session, phone_e164: &str) -> String
where
    S: Service<Request, Response = ServiceResponse<B>, Error = Error>,
    B: MessageBody,
{
    let list = call(srv, Req::get("/api/v1/members").bearer(&owner.token).build()).await;
    list.body
        .as_array()
        .into_iter()
        .flatten()
        .find(|m| m["phone"] == phone_e164)
        .map(|m| m["id"].as_str().unwrap_or_default().to_owned())
        .expect("member")
}

async fn impersonate<S, B>(srv: &S, session: &Session, membership_id: &str) -> Res
where
    S: Service<Request, Response = ServiceResponse<B>, Error = Error>,
    B: MessageBody,
{
    call(
        srv,
        Req::post("/api/v1/auth/impersonate", json!({ "membership_id": membership_id }))
            .bearer(&session.token)
            .cookie(&session.cookie)
            .build(),
    )
    .await
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn owner_signs_in_as_staff_and_returns(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool.clone(), common::config()))).await;
    let mut owner = sign_up(&srv, "Owner", "01711000501").await;
    let tenant = create_business(&srv, &mut owner, "Rahim Store").await;
    let admin = join(&srv, &owner, &tenant, "Admin", "01711000502", &["admin"]).await;
    let cashier = join(&srv, &owner, &tenant, "Cashier", "01711000503", &["cashier"]).await;
    let cashier_id = membership_id(&srv, &owner, "+8801711000503").await;
    let owner_id = membership_id(&srv, &owner, "+8801711000501").await;
    let admin_id = membership_id(&srv, &owner, "+8801711000502").await;

    // The cashier also owns a business of their own, which the owner must not reach.
    let mut cashier_own = sign_up(&srv, "Other Owner", "01711000504").await;
    let other_tenant = create_business(&srv, &mut cashier_own, "Other Store").await;
    let _ = cashier;

    // Not the owner, not yourself, not another business's member.
    let res = impersonate(&srv, &admin, &cashier_id).await;
    assert_eq!((res.status, res.code()), (StatusCode::FORBIDDEN, "OWNER_ONLY"), "{}", res.body);
    let res = impersonate(&srv, &owner, &owner_id).await;
    assert_eq!((res.status, res.code()), (StatusCode::FORBIDDEN, "CANNOT_IMPERSONATE"), "{}", res.body);
    let res = impersonate(&srv, &cashier_own, &cashier_id).await;
    assert_eq!(res.status, StatusCode::NOT_FOUND, "{}", res.body);
    let res = impersonate(&srv, &owner, &Uuid::now_v7().to_string()).await;
    assert_eq!(res.status, StatusCode::NOT_FOUND, "{}", res.body);

    // Suspended staff cannot be chosen.
    let suspended =
        call(&srv, Req::post(&format!("/api/v1/members/{admin_id}/suspend"), json!({})).bearer(&owner.token).build())
            .await;
    assert_eq!(suspended.status, StatusCode::OK, "{}", suspended.body);
    let res = impersonate(&srv, &owner, &admin_id).await;
    assert_eq!((res.status, res.code()), (StatusCode::CONFLICT, "MEMBER_NOT_ACTIVE"), "{}", res.body);

    // Sign in as the cashier.
    let res = impersonate(&srv, &owner, &cashier_id).await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.body);
    assert_eq!(res.body["tenant_id"], tenant.as_str());
    let mut imp = Session { token: res.token(), cookie: res.cookie(), phone: String::new() };

    let me = call(&srv, Req::get("/api/v1/auth/me").bearer(&imp.token).build()).await;
    assert_eq!(me.status, StatusCode::OK, "{}", me.body);
    assert_eq!(me.body["user"]["full_name"], "Cashier");
    assert_eq!(me.body["impersonator"]["full_name"], "Owner");
    assert_eq!(me.body["tenant"]["is_owner"], false);
    let perms = me.body["tenant"]["permissions"].as_array().cloned().unwrap_or_default();
    assert!(perms.contains(&json!("pos.sell")) && !perms.contains(&json!("users.view")));
    let memberships = me.body["memberships"].as_array().cloned().unwrap_or_default();
    assert_eq!(memberships.len(), 1, "only this business: {memberships:?}");

    // The owner's own session on this device ended.
    let old = call(&srv, Req::post("/api/v1/auth/refresh", json!({})).cookie(&owner.cookie).build()).await;
    assert_eq!(old.status, StatusCode::UNAUTHORIZED, "{}", old.body);

    // The cashier's permissions apply.
    let members = call(&srv, Req::get("/api/v1/members").bearer(&imp.token).build()).await;
    assert_eq!(members.status, StatusCode::FORBIDDEN);

    // No account actions and no other business while impersonating.
    let switch = call(
        &srv,
        Req::post("/api/v1/auth/switch-tenant", json!({ "tenant_id": other_tenant }))
            .bearer(&imp.token)
            .cookie(&imp.cookie)
            .build(),
    )
    .await;
    assert_eq!((switch.status, switch.code()), (StatusCode::FORBIDDEN, "IMPERSONATING"), "{}", switch.body);
    let pw = call(
        &srv,
        Req::post(
            "/api/v1/auth/password/change",
            json!({ "current_password": common::PASSWORD, "new_password": "another long passphrase" }),
        )
        .bearer(&imp.token)
        .build(),
    )
    .await;
    assert_eq!((pw.status, pw.code()), (StatusCode::FORBIDDEN, "IMPERSONATING"));
    let sessions = call(&srv, Req::get("/api/v1/auth/sessions").bearer(&imp.token).build()).await;
    assert_eq!(sessions.status, StatusCode::FORBIDDEN);
    let new_business = call(
        &srv,
        Req::post("/api/v1/tenants", json!({ "name": "Sneaky", "business_type": "grocery" }))
            .bearer(&imp.token)
            .build(),
    )
    .await;
    assert_eq!(new_business.status, StatusCode::FORBIDDEN);
    let nested = impersonate(&srv, &imp, &cashier_id).await;
    assert_eq!(nested.code(), "IMPERSONATING");

    // Work done is recorded for the cashier, naming the owner.
    let customer = call(
        &srv,
        Req::post("/api/v1/customers", json!({ "name": "Jamal", "phone": "01911000501" })).bearer(&imp.token).build(),
    )
    .await;
    assert_eq!(customer.status, StatusCode::CREATED, "{}", customer.body);

    // Refreshing keeps the impersonation.
    let refreshed = call(&srv, Req::post("/api/v1/auth/refresh", json!({})).cookie(&imp.cookie).build()).await;
    assert_eq!(refreshed.status, StatusCode::OK, "{}", refreshed.body);
    imp.token = refreshed.token();
    imp.cookie = refreshed.cookie();
    let me = call(&srv, Req::get("/api/v1/auth/me").bearer(&imp.token).build()).await;
    assert_eq!(me.body["impersonator"]["full_name"], "Owner");

    // Return to the owner's account in the same business.
    let back = call(
        &srv,
        Req::post("/api/v1/auth/impersonate/stop", json!({})).bearer(&imp.token).cookie(&imp.cookie).build(),
    )
    .await;
    assert_eq!(back.status, StatusCode::OK, "{}", back.body);
    assert_eq!(back.body["tenant_id"], tenant.as_str());
    let owner_again = Session { token: back.token(), cookie: back.cookie(), phone: String::new() };
    let me = call(&srv, Req::get("/api/v1/auth/me").bearer(&owner_again.token).build()).await;
    assert_eq!(me.body["user"]["full_name"], "Owner");
    assert!(me.body.get("impersonator").is_none(), "{}", me.body);
    assert_eq!(me.body["tenant"]["is_owner"], true);
    let dead = call(&srv, Req::post("/api/v1/auth/refresh", json!({})).cookie(&imp.cookie).build()).await;
    assert_eq!(dead.status, StatusCode::UNAUTHORIZED);
    let again = call(
        &srv,
        Req::post("/api/v1/auth/impersonate/stop", json!({}))
            .bearer(&owner_again.token)
            .cookie(&owner_again.cookie)
            .build(),
    )
    .await;
    assert_eq!(again.code(), "NOT_IMPERSONATING");

    // The audit log names the owner on the cashier's entries.
    let audit = call(&srv, Req::get("/api/v1/audit-logs?action=customer.*").bearer(&owner_again.token).build()).await;
    assert_eq!(audit.status, StatusCode::OK, "{}", audit.body);
    let entry = &audit.body["data"][0];
    assert_eq!(entry["actor_name"], "Cashier", "{}", audit.body);
    assert_eq!(entry["impersonator_name"], "Owner", "{}", audit.body);
    let started =
        call(&srv, Req::get("/api/v1/audit-logs?action=member.impersonated").bearer(&owner_again.token).build()).await;
    assert_eq!(started.body["data"].as_array().map(Vec::len), Some(1), "{}", started.body);

    // Someone holding the owner's account starts an impersonation on their own
    // device; the real owner changes the password on another. The impersonation
    // must end with it: it can neither be refreshed nor turned back into an
    // owner session without the new password.
    let res = impersonate(&srv, &owner_again, &cashier_id).await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.body);
    let stolen = Session { token: res.token(), cookie: res.cookie(), phone: String::new() };
    let device_b = call(
        &srv,
        Req::post("/api/v1/auth/login", json!({ "identifier": "01711000501", "password": common::PASSWORD })).build(),
    )
    .await;
    assert_eq!(device_b.status, StatusCode::OK, "{}", device_b.body);
    let changed = call(
        &srv,
        Req::post(
            "/api/v1/auth/password/change",
            json!({ "current_password": common::PASSWORD, "new_password": "a brand new passphrase 2026" }),
        )
        .bearer(&device_b.token())
        .build(),
    )
    .await;
    assert_eq!(changed.status, StatusCode::OK, "{}", changed.body);
    let refresh = call(&srv, Req::post("/api/v1/auth/refresh", json!({})).cookie(&stolen.cookie).build()).await;
    assert_eq!(refresh.status, StatusCode::UNAUTHORIZED, "{}", refresh.body);
    let back = call(
        &srv,
        Req::post("/api/v1/auth/impersonate/stop", json!({})).bearer(&stolen.token).cookie(&stolen.cookie).build(),
    )
    .await;
    assert_eq!(back.status, StatusCode::UNAUTHORIZED, "{}", back.body);

    // ---- Platform support opens the business as its owner.
    let open = |session: &Session, tenant_id: &str, reason: &str| {
        Req::post("/api/v1/auth/open-business", json!({ "tenant_id": tenant_id, "reason": reason }))
            .bearer(&session.token)
            .cookie(&session.cookie)
            .build()
    };
    let support = sign_up(&srv, "Helper", "01711000505").await;
    let billing = sign_up(&srv, "Biller", "01711000506").await;
    let res = call(&srv, open(&support, &tenant, "customer asked for help")).await;
    assert_eq!((res.status, res.code()), (StatusCode::FORBIDDEN, "PLATFORM_STAFF_ONLY"), "{}", res.body);
    pos_platform::staff::grant(&pool, "01711000505", pos_platform::StaffRole::Support).await.expect("grant support");
    pos_platform::staff::grant(&pool, "01711000506", pos_platform::StaffRole::Billing).await.expect("grant billing");
    let res = call(&srv, open(&billing, &tenant, "customer asked for help")).await;
    assert_eq!(res.status, StatusCode::FORBIDDEN, "billing staff may not open: {}", res.body);
    let res = call(&srv, open(&support, &tenant, "  ")).await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST, "{}", res.body);
    let res = call(&srv, open(&support, &Uuid::now_v7().to_string(), "help")).await;
    assert_eq!(res.status, StatusCode::NOT_FOUND, "{}", res.body);

    let res = call(&srv, open(&support, &tenant, "customer asked for help")).await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.body);
    assert_eq!(res.body["tenant_id"], tenant.as_str());
    let mut opened = Session { token: res.token(), cookie: res.cookie(), phone: String::new() };
    let me = call(&srv, Req::get("/api/v1/auth/me").bearer(&opened.token).build()).await;
    assert_eq!(me.body["user"]["full_name"], "Owner", "{}", me.body);
    assert_eq!(me.body["tenant"]["is_owner"], true);
    assert_eq!(me.body["impersonator"]["full_name"], "Helper");
    assert_eq!(me.body["impersonator"]["platform_support"], true);
    // The owner's other businesses stay hidden; the console is closed meanwhile.
    assert_eq!(me.body["memberships"].as_array().map(Vec::len), Some(1), "{}", me.body);
    let console = call(&srv, Req::get("/api/v1/admin/me").bearer(&opened.token).build()).await;
    assert_eq!(console.code(), "IMPERSONATING");
    let nested = impersonate(&srv, &opened, &cashier_id).await;
    assert_eq!(nested.code(), "IMPERSONATING");

    let customer = call(
        &srv,
        Req::post("/api/v1/customers", json!({ "name": "Karim", "phone": "01911000502" }))
            .bearer(&opened.token)
            .build(),
    )
    .await;
    assert_eq!(customer.status, StatusCode::CREATED, "{}", customer.body);
    let refreshed = call(&srv, Req::post("/api/v1/auth/refresh", json!({})).cookie(&opened.cookie).build()).await;
    assert_eq!(refreshed.status, StatusCode::OK, "refresh keeps the support session: {}", refreshed.body);
    opened.token = refreshed.token();
    opened.cookie = refreshed.cookie();

    // Back to the console: no business chosen, the staff member again.
    let back = call(
        &srv,
        Req::post("/api/v1/auth/impersonate/stop", json!({})).bearer(&opened.token).cookie(&opened.cookie).build(),
    )
    .await;
    assert_eq!(back.status, StatusCode::OK, "{}", back.body);
    assert!(back.body["tenant_id"].is_null(), "{}", back.body);
    let staff_again = Session { token: back.token(), cookie: back.cookie(), phone: String::new() };
    let console = call(&srv, Req::get("/api/v1/admin/me").bearer(&staff_again.token).build()).await;
    assert_eq!(console.status, StatusCode::OK, "{}", console.body);
    assert_eq!(console.body["full_name"], "Helper");

    // The owner sees the support work in the audit log, naming the staff member.
    let audit = call(&srv, Req::get("/api/v1/audit-logs?action=customer.*").bearer(&device_b.token()).build()).await;
    assert_eq!(audit.status, StatusCode::OK, "{}", audit.body);
    let entry = &audit.body["data"][0];
    assert_eq!(entry["actor_name"], "Owner", "{}", audit.body);
    assert_eq!(entry["impersonator_name"], "Helper", "{}", audit.body);

    // Losing the support role ends a live support session at its next refresh.
    let res = call(&srv, open(&staff_again, &tenant, "follow-up")).await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.body);
    let second = Session { token: res.token(), cookie: res.cookie(), phone: String::new() };
    sqlx::query("UPDATE platform_staff SET status = 'disabled' WHERE user_id = (SELECT id FROM users WHERE phone = ?)")
        .bind("+8801711000505")
        .execute(&pool)
        .await
        .expect("disable staff");
    let refresh = call(&srv, Req::post("/api/v1/auth/refresh", json!({})).cookie(&second.cookie).build()).await;
    assert_eq!(refresh.status, StatusCode::UNAUTHORIZED, "{}", refresh.body);
}
