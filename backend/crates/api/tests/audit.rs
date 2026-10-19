//! Audit log viewer over HTTP (E1.7, AUD-R03): filters, cursor pagination,
//! `audit.view` and tenant isolation.

mod common;

use actix_web::{http::StatusCode, test};
use common::{Req, call, create_business, invite, join, sign_up};
use pos_api::{AppState, app};
use serde_json::{Value, json};
use sqlx::MySqlPool;

fn actions(page: &Value) -> Vec<String> {
    page["data"].as_array().into_iter().flatten().map(|e| e["action"].as_str().unwrap_or_default().to_owned()).collect()
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn owner_reads_filters_and_pages_the_log(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool, common::config()))).await;
    let mut owner = sign_up(&srv, "Owner", "01711000301").await;
    create_business(&srv, &mut owner, "Rahim Store").await;
    let first = invite(&srv, &owner, "01711000302", &["cashier"]).await;
    let second = invite(&srv, &owner, "01711000303", &["cashier"]).await;
    let revoked_id = second.body["invitation"]["id"].as_str().expect("invitation id").to_owned();
    let revoked =
        call(&srv, Req::delete(&format!("/api/v1/invitations/{revoked_id}")).bearer(&owner.token).build()).await;
    assert_eq!(revoked.status, StatusCode::NO_CONTENT);
    assert_eq!(first.status, StatusCode::CREATED);

    let get = |query: &str| Req::get(&format!("/api/v1/audit-logs{query}")).bearer(&owner.token).build();

    let all = call(&srv, get("")).await;
    assert_eq!(all.status, StatusCode::OK, "{}", all.body);
    let all_actions = actions(&all.body);
    assert_eq!(&all_actions[..3], ["invitation.revoked", "invitation.created", "invitation.created"], "newest first");
    assert!(all_actions.contains(&"tenant.provisioned".to_owned()));
    assert!(all.body["next_cursor"].is_null());
    let newest = &all.body["data"][0];
    assert_eq!(newest["actor_type"], "user");
    assert_eq!(newest["actor_name"], "Owner");
    assert_eq!(newest["entity_id"], revoked_id.as_str());
    assert!(newest["request_id"].is_string());

    let prefixed = call(&srv, get("?action=invitation.*")).await;
    assert_eq!(actions(&prefixed.body), ["invitation.revoked", "invitation.created", "invitation.created"]);
    let exact = call(&srv, get("?action=invitation.created")).await;
    assert_eq!(actions(&exact.body).len(), 2);
    let by_entity = call(&srv, get(&format!("?entity_type=invitation&entity_id={revoked_id}"))).await;
    assert_eq!(actions(&by_entity.body), ["invitation.revoked", "invitation.created"]);
    let actor = newest["actor_id"].as_str().expect("actor_id");
    let by_actor = call(&srv, get(&format!("?actor_id={actor}&action=invitation.*"))).await;
    assert_eq!(actions(&by_actor.body).len(), 3);
    let future = call(&srv, get("?from=2999-01-01T00:00:00Z")).await;
    assert_eq!(future.body["data"], json!([]));
    let past = call(&srv, get("?to=2000-01-01T00:00:00Z")).await;
    assert_eq!(past.body["data"], json!([]));

    // Walking one entry per page visits every entry once, in order.
    let mut walked = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let query = match &cursor {
            Some(c) => format!("?limit=1&cursor={c}"),
            None => "?limit=1".to_owned(),
        };
        let page = call(&srv, get(&query)).await;
        assert_eq!(page.status, StatusCode::OK, "{}", page.body);
        walked.extend(actions(&page.body));
        match page.body["next_cursor"].as_str() {
            Some(next) => cursor = Some(next.to_owned()),
            None => break,
        }
        assert!(walked.len() <= all_actions.len(), "pagination does not end");
    }
    assert_eq!(walked, all_actions);
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn bad_queries_are_rejected(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool, common::config()))).await;
    let mut owner = sign_up(&srv, "Owner", "01711000311").await;
    create_business(&srv, &mut owner, "Rahim Store").await;

    for query in [
        "?limit=0",
        "?limit=201",
        "?cursor=not-a-cursor",
        "?action=member%25",
        "?action=*",
        "?from=2026-10-02T00:00:00Z&to=2026-10-01T00:00:00Z",
        "?actor_id=nope",
        "?unknown=1",
    ] {
        let res = call(&srv, Req::get(&format!("/api/v1/audit-logs{query}")).bearer(&owner.token).build()).await;
        assert_eq!(res.status, StatusCode::BAD_REQUEST, "{query}: {}", res.body);
        assert_eq!(res.code(), "VALIDATION_FAILED", "{query}");
    }
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn only_audit_viewers_of_the_same_business(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool, common::config()))).await;
    let mut owner = sign_up(&srv, "Owner", "01711000321").await;
    let tenant = create_business(&srv, &mut owner, "Rahim Store").await;
    let cashier = join(&srv, &owner, &tenant, "Cashier", "01711000322", &["cashier"]).await;
    let admin = join(&srv, &owner, &tenant, "Admin", "01711000323", &["admin"]).await;

    let denied = call(&srv, Req::get("/api/v1/audit-logs").bearer(&cashier.token).build()).await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN);
    let allowed = call(&srv, Req::get("/api/v1/audit-logs?action=member.joined").bearer(&admin.token).build()).await;
    assert_eq!(allowed.status, StatusCode::OK, "{}", allowed.body);
    let joined: Vec<&str> =
        allowed.body["data"].as_array().into_iter().flatten().filter_map(|e| e["actor_name"].as_str()).collect();
    assert_eq!(joined, ["Admin", "Cashier"]);

    // Another business sees none of these entries.
    let mut other = sign_up(&srv, "Other", "01711000324").await;
    create_business(&srv, &mut other, "Karim Traders").await;
    let theirs = call(&srv, Req::get("/api/v1/audit-logs").bearer(&other.token).build()).await;
    assert_eq!(theirs.status, StatusCode::OK);
    assert!(!actions(&theirs.body).iter().any(|a| a.starts_with("member.") || a.starts_with("invitation.")));
    let no_auth = call(&srv, Req::get("/api/v1/audit-logs").build()).await;
    assert_eq!(no_auth.status, StatusCode::UNAUTHORIZED);
}
