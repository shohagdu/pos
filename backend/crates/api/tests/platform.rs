//! The SaaS side over HTTP (Phase 5: E5.2–E5.5): the owner requests an
//! invoice and reports a bKash payment; platform staff verify it (the
//! subscription becomes active for the paid period), reject another, set an
//! override, suspend and reactivate the business; staff roles are enforced;
//! the owner exports their data; the lifecycle job stores an ended trial.

mod common;

use std::io::Read;

use actix_web::{http::StatusCode, test};
use chrono::Utc;
use common::{Req, call, create_business, sign_up};
use pos_api::{AppState, app};
use pos_platform::StaffRole;
use serde_json::json;
use sqlx::MySqlPool;

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn billing_claims_admin_and_export(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool.clone(), common::config()))).await;
    let mut owner = sign_up(&srv, "Owner", "01711000951").await;
    let tenant_id = create_business(&srv, &mut owner, "Rahim Store").await;
    let staff = sign_up(&srv, "Admin", "01711000952").await;
    let support = sign_up(&srv, "Helper", "01711000953").await;

    // Not staff yet.
    let denied = call(&srv, Req::get("/api/v1/admin/me").bearer(&staff.token).build()).await;
    assert_eq!((denied.status, denied.code()), (StatusCode::FORBIDDEN, "PLATFORM_STAFF_ONLY"));
    pos_platform::staff::grant(&pool, "01711000952", StaffRole::PlatformOwner).await.expect("grant owner");
    pos_platform::staff::grant(&pool, "01711000953", StaffRole::Support).await.expect("grant support");
    let me = call(&srv, Req::get("/api/v1/admin/me").bearer(&staff.token).build()).await;
    assert_eq!(me.body["role"], "platform_owner", "{}", me.body);

    // The owner sees plans and opens an invoice; asking twice returns the same one.
    let billing = call(&srv, Req::get("/api/v1/subscription/billing").bearer(&owner.token).build()).await;
    assert_eq!(billing.status, StatusCode::OK, "{}", billing.body);
    assert!(billing.body["plans"].as_array().is_some_and(|p| p.iter().any(|p| p["code"] == "starter")));
    let ask = json!({ "plan_code": "starter", "billing_cycle": "monthly" });
    let invoice =
        call(&srv, Req::post("/api/v1/subscription/invoices", ask.clone()).bearer(&owner.token).build()).await;
    assert_eq!(invoice.status, StatusCode::CREATED, "{}", invoice.body);
    assert!(invoice.body["invoice_no"].as_str().is_some_and(|n| n.starts_with("SUB-")), "{}", invoice.body);
    assert_eq!(invoice.body["total"], "990.00");
    let again = call(&srv, Req::post("/api/v1/subscription/invoices", ask).bearer(&owner.token).build()).await;
    assert_eq!(again.body["id"], invoice.body["id"]);
    let invoice_id = invoice.body["id"].as_str().expect("invoice id").to_owned();

    let claim_body =
        json!({ "method": "bkash", "amount": "990", "transaction_ref": "TRX9X8Y7", "payer_account": "01711000951" });
    let claim = call(
        &srv,
        Req::post(&format!("/api/v1/subscription/invoices/{invoice_id}/claims"), claim_body.clone())
            .bearer(&owner.token)
            .build(),
    )
    .await;
    assert_eq!(claim.status, StatusCode::CREATED, "{}", claim.body);
    assert_eq!(claim.body["status"], "pending");
    let twice = call(
        &srv,
        Req::post(&format!("/api/v1/subscription/invoices/{invoice_id}/claims"), claim_body)
            .bearer(&owner.token)
            .build(),
    )
    .await;
    assert_eq!(twice.code(), "CLAIM_PENDING", "{}", twice.body);
    let claim_id = claim.body["id"].as_str().expect("claim").to_owned();

    // The queue; support may look but not approve.
    let queue = call(&srv, Req::get("/api/v1/admin/claims").bearer(&staff.token).build()).await;
    assert_eq!(queue.body["pending_count"], 1, "{}", queue.body);
    let forbidden = call(
        &srv,
        Req::post(&format!("/api/v1/admin/claims/{claim_id}/approve"), json!({})).bearer(&support.token).build(),
    )
    .await;
    assert_eq!(forbidden.status, StatusCode::FORBIDDEN);
    let approved = call(
        &srv,
        Req::post(&format!("/api/v1/admin/claims/{claim_id}/approve"), json!({})).bearer(&staff.token).build(),
    )
    .await;
    assert_eq!(approved.status, StatusCode::OK, "{}", approved.body);
    assert_eq!(approved.body["status"], "approved");
    let sub = call(&srv, Req::get("/api/v1/subscription").bearer(&owner.token).build()).await;
    assert_eq!(sub.body["subscription"]["status"], "active", "{}", sub.body);
    assert_eq!(sub.body["subscription"]["plan_code"], "starter");
    let days = sub.body["subscription"]["days_left"].as_i64().unwrap_or_default();
    assert!((27..=31).contains(&days), "a month paid: {}", sub.body);

    // A second invoice whose claim is rejected; a reused transaction id is refused.
    let quarterly = call(
        &srv,
        Req::post("/api/v1/subscription/invoices", json!({ "plan_code": "business", "billing_cycle": "quarterly" }))
            .bearer(&owner.token)
            .build(),
    )
    .await;
    let quarterly_id = quarterly.body["id"].as_str().expect("invoice").to_owned();
    let reused = call(
        &srv,
        Req::post(
            &format!("/api/v1/subscription/invoices/{quarterly_id}/claims"),
            json!({ "method": "bkash", "amount": "7096.50", "transaction_ref": "TRX9X8Y7" }),
        )
        .bearer(&owner.token)
        .build(),
    )
    .await;
    assert_eq!(reused.code(), "DUPLICATE_TRANSACTION", "{}", reused.body);
    let second = call(
        &srv,
        Req::post(
            &format!("/api/v1/subscription/invoices/{quarterly_id}/claims"),
            json!({ "method": "nagad", "amount": "100", "transaction_ref": "NGD55555" }),
        )
        .bearer(&owner.token)
        .build(),
    )
    .await;
    let second_id = second.body["id"].as_str().expect("claim").to_owned();
    let no_reason = call(
        &srv,
        Req::post(&format!("/api/v1/admin/claims/{second_id}/reject"), json!({ "reason": " " }))
            .bearer(&staff.token)
            .build(),
    )
    .await;
    assert_eq!(no_reason.status, StatusCode::BAD_REQUEST);
    let rejected = call(
        &srv,
        Req::post(
            &format!("/api/v1/admin/claims/{second_id}/reject"),
            json!({ "reason": "Amount does not match the invoice" }),
        )
        .bearer(&staff.token)
        .build(),
    )
    .await;
    assert_eq!(rejected.body["status"], "rejected", "{}", rejected.body);

    // Override: the starter plan's 3 users become 10.
    let override_set = call(
        &srv,
        Req::put(
            &format!("/api/v1/admin/tenants/{tenant_id}/overrides"),
            json!({ "feature_code": "users.max", "limit_value": 10, "reason": "Pilot customer" }),
        )
        .bearer(&staff.token)
        .build(),
    )
    .await;
    assert_eq!(override_set.status, StatusCode::OK, "{}", override_set.body);
    let detail = call(&srv, Req::get(&format!("/api/v1/admin/tenants/{tenant_id}")).bearer(&staff.token).build()).await;
    assert_eq!(detail.status, StatusCode::OK, "{}", detail.body);
    assert_eq!(detail.body["subscription_status"], "active");
    let users = detail.body["usage"].as_array().into_iter().flatten().find(|u| u["code"] == "users.max").cloned();
    assert_eq!(users.map(|u| u["limit"].clone()), Some(json!(10)), "{}", detail.body);
    assert!(detail.body["events"].as_array().is_some_and(|e| e.iter().any(|e| e["event_type"] == "payment_verified")));

    // Suspend: the owner is shut out until reactivated.
    let suspended = call(
        &srv,
        Req::post(
            &format!("/api/v1/admin/tenants/{tenant_id}/suspend"),
            json!({ "reason": "Chargeback investigation" }),
        )
        .bearer(&staff.token)
        .build(),
    )
    .await;
    assert_eq!(suspended.status, StatusCode::OK, "{}", suspended.body);
    let blocked = call(&srv, Req::get("/api/v1/products").bearer(&owner.token).build()).await;
    assert!(blocked.status == StatusCode::FORBIDDEN || blocked.status == StatusCode::UNAUTHORIZED, "{}", blocked.body);
    let back = call(
        &srv,
        Req::post(&format!("/api/v1/admin/tenants/{tenant_id}/reactivate"), json!({ "reason": "Resolved" }))
            .bearer(&staff.token)
            .build(),
    )
    .await;
    assert_eq!(back.status, StatusCode::OK, "{}", back.body);
    let open_again = call(&srv, Req::get("/api/v1/products").bearer(&owner.token).build()).await;
    assert_eq!(open_again.status, StatusCode::OK, "{}", open_again.body);

    let tenants = call(&srv, Req::get("/api/v1/admin/tenants?q=Rahim").bearer(&staff.token).build()).await;
    assert_eq!(tenants.body["data"].as_array().map(Vec::len), Some(1), "{}", tenants.body);
    let metrics = call(&srv, Req::get("/api/v1/admin/metrics").bearer(&staff.token).build()).await;
    assert_eq!(metrics.body["mrr"], "990.00", "{}", metrics.body);

    // Export: a ZIP with a README and one CSV per table.
    let export = test::call_service(&srv, Req::get("/api/v1/subscription/export").bearer(&owner.token).build()).await;
    assert_eq!(export.status(), StatusCode::OK);
    let bytes = test::read_body(export).await;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes.to_vec())).expect("a zip");
    let mut readme = String::new();
    archive.by_name("README.txt").expect("readme").read_to_string(&mut readme).expect("text");
    assert!(readme.contains(&tenant_id));
    let mut customers = String::new();
    archive.by_name("customers.csv").expect("customers").read_to_string(&mut customers).expect("text");
    assert!(customers.starts_with("tenant_id,id,code,name"), "{customers}");
    assert!(customers.contains("Walk-in Customer"));

    // Lifecycle: a trial that ended yesterday is stored as grace.
    let mut other = sign_up(&srv, "Late", "01711000954").await;
    let late_id = create_business(&srv, &mut other, "Late Shop").await;
    // Raw writes pass the tenant guard triggers only as the platform.
    let mut conn = pool.acquire().await.expect("connection");
    sqlx::query("SET @app_platform = 1").execute(&mut *conn).await.expect("platform session");
    sqlx::query(
        "UPDATE subscriptions SET trial_ends_at = UTC_TIMESTAMP(6) - INTERVAL 1 DAY, current_period_end = UTC_TIMESTAMP(6) - INTERVAL 1 DAY \
         WHERE tenant_id = UUID_TO_BIN(?)",
    )
    .bind(&late_id)
    .execute(&mut *conn)
    .await
    .expect("backdate");
    sqlx::query("SET @app_platform = NULL").execute(&mut *conn).await.expect("reset session");
    drop(conn);
    let outcome = pos_platform::lifecycle::run(&pool, Utc::now()).await.expect("lifecycle");
    assert!(outcome.transitions >= 1, "{outcome:?}");
    let stored: String = sqlx::query_scalar("SELECT status FROM subscriptions WHERE tenant_id = UUID_TO_BIN(?)")
        .bind(&late_id)
        .fetch_one(&pool)
        .await
        .expect("status");
    assert_eq!(stored, "grace");

    // Licences sold offline: support staff may not grant one; billing staff reopen
    // the trial, then sell a one-time licence that never ends.
    let licence = format!("/api/v1/admin/tenants/{late_id}/licence");
    let trial = json!({ "kind": "trial", "days": 10, "reason": "Demo extension" });
    let refused = call(&srv, Req::post(&licence, trial.clone()).bearer(&support.token).build()).await;
    assert_eq!(refused.status, StatusCode::FORBIDDEN, "{}", refused.body);
    let reopened = call(&srv, Req::post(&licence, trial).bearer(&staff.token).build()).await;
    assert_eq!(reopened.status, StatusCode::OK, "{}", reopened.body);
    assert_eq!(
        (reopened.body["subscription_status"].clone(), reopened.body["days_left"].clone()),
        (json!("trialing"), json!(10))
    );
    let lifetime =
        json!({ "kind": "lifetime", "plan_code": "starter", "price_amount": "25000", "reason": "MR-1042 cash" });
    let sold = call(&srv, Req::post(&licence, lifetime).bearer(&staff.token).build()).await;
    assert_eq!(sold.status, StatusCode::OK, "{}", sold.body);
    assert_eq!(sold.body["subscription_status"], "active", "{}", sold.body);
    assert_eq!(sold.body["billing_cycle"], "lifetime");
    assert_eq!(sold.body["current_period_end"], serde_json::Value::Null);
    assert!(
        sold.body["events"].as_array().is_some_and(|e| e
            .iter()
            .filter(|e| e["event_type"] == "licence_granted")
            .count()
            == 2)
    );
    pos_platform::lifecycle::run(&pool, Utc::now() + chrono::Duration::days(3650)).await.expect("lifecycle");
    let stored: String = sqlx::query_scalar("SELECT status FROM subscriptions WHERE tenant_id = UUID_TO_BIN(?)")
        .bind(&late_id)
        .fetch_one(&pool)
        .await
        .expect("status");
    assert_eq!(stored, "active", "a lifetime licence never lapses");
    let yearly = json!({ "kind": "annual", "plan_code": "starter", "years": 11, "reason": "x" });
    let too_long = call(&srv, Req::post(&licence, yearly).bearer(&staff.token).build()).await;
    assert_eq!((too_long.status, too_long.code()), (StatusCode::BAD_REQUEST, "VALIDATION_FAILED"), "{}", too_long.body);
}
