//! Invitations, members and roles over HTTP (E1.5, E1.10): AUTH-R07/R08/R09,
//! AT-16 (plan limit), owner/admin protections and privilege escalation.

mod common;

use actix_http::Request;
use actix_web::{
    Error,
    body::MessageBody,
    dev::{Service, ServiceResponse},
    http::StatusCode,
    test,
};
use common::{Req, Session, call, create_business, invite, join, role_id, sign_up, switch, token_of};
use pos_api::{AppState, app};
use serde_json::json;
use sqlx::MySqlPool;

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
        .expect(phone_e164)
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn invitation_round_trip(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool.clone(), common::config()))).await;
    let mut owner = sign_up(&srv, "Owner", "01711000201").await;
    let tenant = create_business(&srv, &mut owner, "Rahim Store").await;

    let created = invite(&srv, &owner, "01711-000202", &["cashier"]).await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
    assert_eq!(created.body["invitation"]["destination"], "+8801711000202");
    assert_eq!(created.body["invitation"]["status"], "pending");
    assert_eq!(created.body["invitation"]["roles"][0]["name"], "Cashier");
    let token = token_of(&created.body);
    let stored: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM invitations WHERE token_hash = UNHEX(SHA2(?, 256))")
        .bind(&token)
        .fetch_one(&pool)
        .await
        .expect("hash lookup");
    assert_eq!(stored, 1, "only the SHA-256 of the token is stored");

    // The setup checklist item is done once someone is invited.
    let onboarding = call(&srv, Req::get("/api/v1/onboarding").bearer(&owner.token).build()).await;
    assert!(
        onboarding.body["steps"].as_array().into_iter().flatten().any(|s| s["key"] == "staff" && s["done"] == true)
    );

    let preview = call(&srv, Req::post("/api/v1/invitations/preview", json!({ "token": token })).build()).await;
    assert_eq!(preview.status, StatusCode::OK);
    assert_eq!(preview.body["business_name"], "Rahim Store");
    assert_eq!(preview.body["invited_by"], "Owner");
    assert_eq!(preview.body["destination"], "+880*******202");
    assert_eq!(preview.body["roles"], json!(["Cashier"]));
    let bogus = call(&srv, Req::post("/api/v1/invitations/preview", json!({ "token": "nope" })).build()).await;
    assert_eq!(bogus.status, StatusCode::NOT_FOUND);

    // Someone else cannot use the link.
    let stranger = sign_up(&srv, "Stranger", "01711000203").await;
    let wrong =
        call(&srv, Req::post("/api/v1/invitations/accept", json!({ "token": token })).bearer(&stranger.token).build())
            .await;
    assert_eq!((wrong.status, wrong.code()), (StatusCode::FORBIDDEN, "INVITATION_MISMATCH"));

    let mut cashier = sign_up(&srv, "Cashier", "01711000202").await;
    let accepted =
        call(&srv, Req::post("/api/v1/invitations/accept", json!({ "token": token })).bearer(&cashier.token).build())
            .await;
    assert_eq!(accepted.status, StatusCode::OK, "{}", accepted.body);
    assert_eq!(accepted.body["tenant_id"], tenant.as_str());
    switch(&srv, &mut cashier, Some(&tenant)).await;
    let me = call(&srv, Req::get("/api/v1/auth/me").bearer(&cashier.token).build()).await;
    let perms = me.body["tenant"]["permissions"].as_array().cloned().unwrap_or_default();
    assert!(perms.contains(&json!("pos.sell")) && !perms.contains(&json!("users.view")));
    assert_eq!(me.body["tenant"]["is_owner"], false);

    // Single use (AUTH-R09).
    let again =
        call(&srv, Req::post("/api/v1/invitations/accept", json!({ "token": token })).bearer(&cashier.token).build())
            .await;
    assert_eq!(again.code(), "INVITATION_CLOSED");

    let members = call(&srv, Req::get("/api/v1/members").bearer(&owner.token).build()).await;
    let list = members.body.as_array().cloned().unwrap_or_default();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0]["is_owner"], true, "owner first");
    assert_eq!(list[1]["roles"][0]["code"], "cashier");
    let invitations = call(&srv, Req::get("/api/v1/invitations").bearer(&owner.token).build()).await;
    assert_eq!(invitations.body[0]["status"], "accepted");
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn invitations_expire_and_can_be_resent_or_revoked(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool.clone(), common::config()))).await;
    let mut owner = sign_up(&srv, "Owner", "01711000211").await;
    create_business(&srv, &mut owner, "Rahim Store").await;
    let guest = sign_up(&srv, "Guest", "01711000212").await;

    let created = invite(&srv, &owner, "01711000212", &["salesperson"]).await;
    let id = created.body["invitation"]["id"].as_str().unwrap_or_default().to_owned();
    let old_token = token_of(&created.body);
    let ctx = pos_core::PlatformCtx { actor_id: None, reason: "test".into(), meta: Default::default() };
    let mut ptx = pos_db::begin_platform(&pool, &ctx).await.expect("tx");
    sqlx::query("UPDATE invitations SET expires_at = NOW(6) - INTERVAL 1 SECOND WHERE id = UUID_TO_BIN(?)")
        .bind(&id)
        .execute(&mut *ptx)
        .await
        .expect("expire");
    ptx.commit().await.expect("commit");

    let expired =
        call(&srv, Req::post("/api/v1/invitations/accept", json!({ "token": old_token })).bearer(&guest.token).build())
            .await;
    assert_eq!(expired.code(), "INVITATION_EXPIRED");
    let listed = call(&srv, Req::get("/api/v1/invitations").bearer(&owner.token).build()).await;
    assert_eq!(listed.body[0]["status"], "expired");

    let resent =
        call(&srv, Req::post(&format!("/api/v1/invitations/{id}/resend"), json!({})).bearer(&owner.token).build())
            .await;
    assert_eq!(resent.status, StatusCode::OK, "{}", resent.body);
    let new_token = token_of(&resent.body);
    assert_ne!(new_token, old_token);
    let old = call(&srv, Req::post("/api/v1/invitations/preview", json!({ "token": old_token })).build()).await;
    assert_eq!(old.status, StatusCode::NOT_FOUND, "the old link stops working");

    let revoked = call(&srv, Req::delete(&format!("/api/v1/invitations/{id}")).bearer(&owner.token).build()).await;
    assert_eq!(revoked.status, StatusCode::NO_CONTENT);
    let closed =
        call(&srv, Req::post("/api/v1/invitations/accept", json!({ "token": new_token })).bearer(&guest.token).build())
            .await;
    assert_eq!(closed.code(), "INVITATION_CLOSED");
}

/// AT-16: the trial and Starter plans allow 3 users; the 4th is refused with 402.
#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn plan_user_limit(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool, common::config()))).await;
    let mut owner = sign_up(&srv, "Owner", "01711000221").await;
    create_business(&srv, &mut owner, "Rahim Store").await;

    for phone in ["01711000222", "01711000223"] {
        assert_eq!(invite(&srv, &owner, phone, &["cashier"]).await.status, StatusCode::CREATED);
    }
    let fourth = invite(&srv, &owner, "01711000224", &["cashier"]).await;
    assert_eq!((fourth.status, fourth.code()), (StatusCode::PAYMENT_REQUIRED, "PLAN_LIMIT_REACHED"));
    assert_eq!((fourth.body["limit"].as_str(), fourth.body["max"].as_i64()), (Some("users.max"), Some(3)));

    // Re-inviting the same person replaces the open invitation instead of taking a seat.
    assert_eq!(invite(&srv, &owner, "01711000223", &["salesperson"]).await.status, StatusCode::CREATED);
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn invitation_validation(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool, common::config()))).await;
    let mut owner = sign_up(&srv, "Owner", "01711000231").await;
    create_business(&srv, &mut owner, "Rahim Store").await;

    let owner_role = invite(&srv, &owner, "01711000232", &["owner"]).await;
    assert_eq!(owner_role.body["errors"][0]["code"], "OWNER_ROLE_RESERVED");
    let unknown = call(
        &srv,
        Req::post("/api/v1/invitations", json!({ "destination": "01711000232", "role_ids": [uuid::Uuid::now_v7()] }))
            .bearer(&owner.token)
            .build(),
    )
    .await;
    assert_eq!(unknown.body["errors"][0]["code"], "UNKNOWN_ROLE");
    let none = call(
        &srv,
        Req::post("/api/v1/invitations", json!({ "destination": "01711000232", "role_ids": [] }))
            .bearer(&owner.token)
            .build(),
    )
    .await;
    assert_eq!(none.body["errors"][0]["code"], "NO_ROLES");
    let bad_dest = invite(&srv, &owner, "12", &["cashier"]).await;
    assert_eq!(bad_dest.body["errors"][0]["field"], "destination");
    let myself = invite(&srv, &owner, "01711000231", &["cashier"]).await;
    assert_eq!((myself.status, myself.code()), (StatusCode::CONFLICT, "ALREADY_MEMBER"));

    // Email invitations work the same way.
    let by_email = invite(&srv, &owner, "Karim@Example.com", &["accountant"]).await;
    assert_eq!(by_email.status, StatusCode::CREATED);
    assert_eq!(by_email.body["invitation"]["destination"], "karim@example.com");
}

/// AUTH-R07/R08: role changes, suspension and removal apply on the next request.
#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn member_changes_apply_immediately(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool, common::config()))).await;
    let mut owner = sign_up(&srv, "Owner", "01711000241").await;
    let tenant = create_business(&srv, &mut owner, "Rahim Store").await;
    let staff = join(&srv, &owner, &tenant, "Staff", "01711000242", &["cashier"]).await;
    let id = membership_id(&srv, &owner, "+8801711000242").await;

    let accountant = role_id(&srv, &owner, "accountant").await;
    let changed = call(
        &srv,
        Req::put(&format!("/api/v1/members/{id}/roles"), json!({ "assignments": [{ "role_id": accountant }] }))
            .bearer(&owner.token)
            .build(),
    )
    .await;
    assert_eq!(changed.status, StatusCode::OK, "{}", changed.body);
    assert_eq!(changed.body["roles"][0]["code"], "accountant");
    let stale = call(&srv, Req::get("/api/v1/branches").bearer(&staff.token).build()).await;
    assert_eq!(stale.code(), "TOKEN_STALE");
    let refreshed = call(&srv, Req::post("/api/v1/auth/refresh", json!({})).cookie(&staff.cookie).build()).await;
    let me = call(&srv, Req::get("/api/v1/auth/me").bearer(&refreshed.token()).build()).await;
    assert!(me.body["tenant"]["permissions"].as_array().into_iter().flatten().any(|p| p == "payments.void"));

    let suspended =
        call(&srv, Req::post(&format!("/api/v1/members/{id}/suspend"), json!({})).bearer(&owner.token).build()).await;
    assert_eq!(suspended.body["status"], "suspended");
    let blocked = call(&srv, Req::get("/api/v1/branches").bearer(&refreshed.token()).build()).await;
    assert_eq!(blocked.status, StatusCode::UNAUTHORIZED);
    // Their session in this business was ended too.
    let session = call(&srv, Req::post("/api/v1/auth/refresh", json!({})).cookie(&refreshed.cookie()).build()).await;
    assert_eq!(session.status, StatusCode::UNAUTHORIZED);

    let again =
        call(&srv, Req::post(&format!("/api/v1/members/{id}/suspend"), json!({})).bearer(&owner.token).build()).await;
    assert_eq!(again.code(), "INVALID_STATUS");
    let back =
        call(&srv, Req::post(&format!("/api/v1/members/{id}/reactivate"), json!({})).bearer(&owner.token).build())
            .await;
    assert_eq!(back.body["status"], "active");

    let removed = call(&srv, Req::delete(&format!("/api/v1/members/{id}")).bearer(&owner.token).build()).await;
    assert_eq!(removed.status, StatusCode::NO_CONTENT);
    let list = call(&srv, Req::get("/api/v1/members").bearer(&owner.token).build()).await;
    assert_eq!(list.body.as_array().map(Vec::len), Some(1));

    // A removed member can be invited and join again with the same account.
    let created = invite(&srv, &owner, "01711000242", &["salesperson"]).await;
    let login = call(
        &srv,
        Req::post("/api/v1/auth/login", json!({ "identifier": "01711000242", "password": common::PASSWORD })).build(),
    )
    .await;
    let mut rejoined = Session { token: login.token(), cookie: login.cookie(), phone: "01711000242".into() };
    let accepted = call(
        &srv,
        Req::post("/api/v1/invitations/accept", json!({ "token": token_of(&created.body) }))
            .bearer(&rejoined.token)
            .build(),
    )
    .await;
    assert_eq!(accepted.status, StatusCode::OK, "{}", accepted.body);
    switch(&srv, &mut rejoined, Some(&tenant)).await;
    let me = call(&srv, Req::get("/api/v1/auth/me").bearer(&rejoined.token).build()).await;
    let perms = me.body["tenant"]["permissions"].as_array().cloned().unwrap_or_default();
    assert!(perms.contains(&json!("pos.sell")) && !perms.contains(&json!("payments.void")), "old roles are gone");
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn owner_and_admin_protections(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool, common::config()))).await;
    let mut owner = sign_up(&srv, "Owner", "01711000251").await;
    let tenant = create_business(&srv, &mut owner, "Rahim Store").await;
    let admin = join(&srv, &owner, &tenant, "Admin", "01711000252", &["admin"]).await;
    let owner_id = membership_id(&srv, &owner, "+8801711000251").await;
    let admin_id = membership_id(&srv, &owner, "+8801711000252").await;

    let self_change =
        call(&srv, Req::post(&format!("/api/v1/members/{owner_id}/suspend"), json!({})).bearer(&owner.token).build())
            .await;
    assert_eq!(self_change.code(), "CANNOT_CHANGE_SELF");
    let owner_target =
        call(&srv, Req::post(&format!("/api/v1/members/{owner_id}/suspend"), json!({})).bearer(&admin.token).build())
            .await;
    assert_eq!(owner_target.code(), "OWNER_PROTECTED");
    let admin_self = call(&srv, Req::delete(&format!("/api/v1/members/{admin_id}")).bearer(&admin.token).build()).await;
    assert_eq!(admin_self.code(), "CANNOT_CHANGE_SELF");

    // An admin can invite, but not create other admins.
    let make_admin = invite(&srv, &admin, "01711000253", &["admin"]).await;
    assert_eq!((make_admin.status, make_admin.code()), (StatusCode::FORBIDDEN, "ADMIN_PROTECTED"));
    let cashier = invite(&srv, &admin, "01711000253", &["cashier"]).await;
    assert_eq!(cashier.status, StatusCode::CREATED);

    // subscription.manage stays with the owner.
    let owner_only = call(
        &srv,
        Req::post("/api/v1/roles", json!({ "name": "Billing", "permissions": ["subscription.manage"] }))
            .bearer(&owner.token)
            .build(),
    )
    .await;
    assert_eq!(owner_only.body["errors"][0]["code"], "OWNER_ONLY_PERMISSION");

    // Only the owner removes an admin.
    let by_owner = call(&srv, Req::delete(&format!("/api/v1/members/{admin_id}")).bearer(&owner.token).build()).await;
    assert_eq!(by_owner.status, StatusCode::NO_CONTENT);
}

/// Nobody can grant permissions they do not hold themselves.
#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn no_privilege_escalation(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool, common::config()))).await;
    let mut owner = sign_up(&srv, "Owner", "01711000261").await;
    let tenant = create_business(&srv, &mut owner, "Rahim Store").await;

    let supervisor = call(
        &srv,
        Req::post(
            "/api/v1/roles",
            json!({ "name": "Supervisor", "permissions": ["users.view", "users.invite", "users.manage_roles", "pos.sell"] }),
        )
        .bearer(&owner.token)
        .build(),
    )
    .await;
    assert_eq!(supervisor.status, StatusCode::CREATED, "{}", supervisor.body);
    let code = supervisor.body["code"].as_str().unwrap_or_default().to_owned();
    let sup = join(&srv, &owner, &tenant, "Supervisor", "01711000262", &[&code]).await;

    let wider = call(
        &srv,
        Req::post("/api/v1/roles", json!({ "name": "Wider", "permissions": ["settings.manage"] }))
            .bearer(&sup.token)
            .build(),
    )
    .await;
    assert_eq!((wider.status, wider.code()), (StatusCode::FORBIDDEN, "PRIVILEGE_ESCALATION"));
    let accountant = invite(&srv, &sup, "01711000263", &["accountant"]).await;
    assert_eq!(accountant.code(), "PRIVILEGE_ESCALATION");
    let narrow = call(
        &srv,
        Req::post("/api/v1/roles", json!({ "name": "Seller", "permissions": ["pos.sell"] })).bearer(&sup.token).build(),
    )
    .await;
    assert_eq!(narrow.status, StatusCode::CREATED);
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn custom_roles(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool, common::config()))).await;
    let mut owner = sign_up(&srv, "Owner", "01711000271").await;
    let tenant = create_business(&srv, &mut owner, "Rahim Store").await;

    let perms = call(&srv, Req::get("/api/v1/permissions").bearer(&owner.token).build()).await;
    assert_eq!(perms.body.as_array().map(Vec::len), Some(64));
    assert_eq!(perms.body[0]["code"], "dashboard.view");

    let created = call(
        &srv,
        Req::post(
            "/api/v1/roles",
            json!({ "name": "Senior Cashier", "permissions": ["pos.sell", "sales.view", "pos.sell"], "max_discount_pct": "15.5" }),
        )
        .bearer(&owner.token)
        .build(),
    )
    .await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
    assert_eq!(created.body["permissions"], json!(["pos.sell", "sales.view"]));
    assert_eq!(created.body["max_discount_pct"], "15.5");
    assert_eq!(created.body["is_system"], false);
    let id = created.body["id"].as_str().unwrap_or_default().to_owned();

    let dup = call(
        &srv,
        Req::post("/api/v1/roles", json!({ "name": "Senior Cashier", "permissions": ["pos.sell"] }))
            .bearer(&owner.token)
            .build(),
    )
    .await;
    assert_eq!(dup.body["errors"][0]["code"], "ROLE_NAME_TAKEN");
    let unknown = call(
        &srv,
        Req::post("/api/v1/roles", json!({ "name": "X Role", "permissions": ["pos.fly"] }))
            .bearer(&owner.token)
            .build(),
    )
    .await;
    assert_eq!(unknown.body["errors"][0]["code"], "UNKNOWN_PERMISSION");

    // Someone holds the role: an edit bumps their permission version.
    let code = created.body["code"].as_str().unwrap_or_default().to_owned();
    let holder = join(&srv, &owner, &tenant, "Holder", "01711000272", &[&code]).await;
    let update = |version: i64| {
        Req::put(
            &format!("/api/v1/roles/{id}"),
            json!({ "name": "Senior Cashier", "permissions": ["pos.sell", "sales.view", "sales.return"] }),
        )
        .bearer(&owner.token)
        .if_match(version)
        .build()
    };
    let updated = call(&srv, update(1)).await;
    assert_eq!(updated.status, StatusCode::OK, "{}", updated.body);
    assert_eq!(updated.body["version"], 2);
    let stale_version = call(&srv, update(1)).await;
    assert_eq!(stale_version.status, StatusCode::PRECONDITION_FAILED);
    let stale_token = call(&srv, Req::get("/api/v1/branches").bearer(&holder.token).build()).await;
    assert_eq!(stale_token.code(), "TOKEN_STALE");

    let cashier = role_id(&srv, &owner, "cashier").await;
    let system = call(
        &srv,
        Req::put(&format!("/api/v1/roles/{cashier}"), json!({ "name": "Cashier", "permissions": ["pos.sell"] }))
            .bearer(&owner.token)
            .if_match(1)
            .build(),
    )
    .await;
    assert_eq!(system.code(), "SYSTEM_ROLE_READ_ONLY");

    let in_use = call(&srv, Req::delete(&format!("/api/v1/roles/{id}")).bearer(&owner.token).build()).await;
    assert_eq!(in_use.code(), "ROLE_IN_USE");
    let spare = call(
        &srv,
        Req::post("/api/v1/roles", json!({ "name": "Spare", "permissions": ["pos.sell"] }))
            .bearer(&owner.token)
            .build(),
    )
    .await;
    let spare_id = spare.body["id"].as_str().unwrap_or_default();
    let deleted = call(&srv, Req::delete(&format!("/api/v1/roles/{spare_id}")).bearer(&owner.token).build()).await;
    assert_eq!(deleted.status, StatusCode::NO_CONTENT);
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn staff_sessions_and_permissions(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool, common::config()))).await;
    let mut owner = sign_up(&srv, "Owner", "01711000281").await;
    let tenant = create_business(&srv, &mut owner, "Rahim Store").await;
    let staff = join(&srv, &owner, &tenant, "Staff", "01711000282", &["cashier"]).await;
    let id = membership_id(&srv, &owner, "+8801711000282").await;

    let denied = call(&srv, Req::get("/api/v1/members").bearer(&staff.token).build()).await;
    assert_eq!((denied.status, denied.code()), (StatusCode::FORBIDDEN, "PERMISSION_DENIED"));

    let sessions = call(&srv, Req::get(&format!("/api/v1/members/{id}/sessions")).bearer(&owner.token).build()).await;
    assert_eq!(sessions.body.as_array().map(Vec::len), Some(1), "{}", sessions.body);
    let revoked = call(&srv, Req::delete(&format!("/api/v1/members/{id}/sessions")).bearer(&owner.token).build()).await;
    assert_eq!(revoked.body["revoked"], 1);
    let refresh = call(&srv, Req::post("/api/v1/auth/refresh", json!({})).cookie(&staff.cookie).build()).await;
    assert_eq!(refresh.status, StatusCode::UNAUTHORIZED);
}

/// SRS-TEN-005 for the new routes: another tenant's ids are 404.
#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn other_tenants_users_and_roles_are_not_found(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool, common::config()))).await;
    let mut alice = sign_up(&srv, "Alice", "01711000291").await;
    let alice_tenant = create_business(&srv, &mut alice, "Alice Store").await;
    join(&srv, &alice, &alice_tenant, "Staff", "01711000292", &["cashier"]).await;
    let member = membership_id(&srv, &alice, "+8801711000292").await;
    let invitation = invite(&srv, &alice, "01711000293", &["cashier"]).await.body["invitation"]["id"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let role = call(
        &srv,
        Req::post("/api/v1/roles", json!({ "name": "Alice Role", "permissions": ["pos.sell"] }))
            .bearer(&alice.token)
            .build(),
    )
    .await
    .body["id"]
        .as_str()
        .unwrap_or_default()
        .to_owned();

    let mut bob = sign_up(&srv, "Bob", "01711000294").await;
    create_business(&srv, &mut bob, "Bob Store").await;
    for req in [
        Req::get(&format!("/api/v1/members/{member}")),
        Req::post(&format!("/api/v1/members/{member}/suspend"), json!({})),
        Req::delete(&format!("/api/v1/members/{member}")),
        Req::get(&format!("/api/v1/members/{member}/sessions")),
        Req::delete(&format!("/api/v1/invitations/{invitation}")),
        Req::post(&format!("/api/v1/invitations/{invitation}/resend"), json!({})),
        Req::get(&format!("/api/v1/roles/{role}")),
        Req::delete(&format!("/api/v1/roles/{role}")),
    ] {
        let res = call(&srv, req.bearer(&bob.token).build()).await;
        assert_eq!(res.status, StatusCode::NOT_FOUND, "{}", res.body);
    }
    let put = call(
        &srv,
        Req::put(&format!("/api/v1/roles/{role}"), json!({ "name": "x y", "permissions": ["pos.sell"] }))
            .bearer(&bob.token)
            .if_match(1)
            .build(),
    )
    .await;
    assert_eq!(put.status, StatusCode::NOT_FOUND);
    let members = call(&srv, Req::get("/api/v1/members").bearer(&bob.token).build()).await;
    assert_eq!(members.body.as_array().map(Vec::len), Some(1));
}
