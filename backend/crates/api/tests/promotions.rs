//! Discounts and promotions over HTTP (Final_docs/05_discount-system.md):
//! product discounts priced and snapshotted on the sale, the lock after first
//! use, flash quantity running out, Buy 1 Get 1, coupons (validation, minimum
//! purchase, best-offer rule, single use, given back on cancel) and removing
//! an offer at the POS (permission + audit).
//!
//! One test function: every `#[sqlx::test]` migrates a fresh database.

mod common;

use actix_web::{http::StatusCode, test};
use common::{Req, Res, Session, call, create_business, join, sign_up};
use pos_api::{AppState, app};
use serde_json::{Value, json};
use sqlx::MySqlPool;
use uuid::Uuid;

fn s(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or_else(|| panic!("{key} in {v}")).to_owned()
}

fn find<'a>(list: &'a Value, key: &str, value: &str) -> &'a Value {
    list.as_array().into_iter().flatten().find(|v| v[key] == value).unwrap_or_else(|| panic!("{value} in {list}"))
}

/// The code of a validation error's first field (coupon errors are on `coupon_code`).
fn field_code(res: &Res) -> &str {
    res.body["errors"][0]["code"].as_str().unwrap_or_else(|| res.code())
}

/// Decimal strings compared as numbers ("200.00" == "200").
fn num(v: &Value) -> f64 {
    v.as_str().unwrap_or_else(|| panic!("number in {v}")).parse().unwrap_or_else(|_| panic!("number in {v}"))
}

macro_rules! service_bounds {
    () => {
        impl actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<impl actix_web::body::MessageBody>,
            Error = actix_web::Error,
        >
    };
}

async fn get(srv: &service_bounds!(), who: &Session, uri: &str) -> Res {
    call(srv, Req::get(uri).bearer(&who.token).build()).await
}

async fn post(srv: &service_bounds!(), who: &Session, uri: &str, body: Value) -> Res {
    call(srv, Req::post(uri, body).bearer(&who.token).build()).await
}

async fn sell(srv: &service_bounds!(), who: &Session, body: Value) -> Res {
    let req = test::TestRequest::post()
        .uri("/api/v1/sales")
        .set_json(body)
        .insert_header(("authorization", format!("Bearer {}", who.token)))
        .insert_header(("idempotency-key", Uuid::now_v7().to_string()))
        .to_request();
    call(srv, req).await
}

/// The business's local time (Asia/Dhaka, UTC+6) as the API takes it.
fn local(offset_minutes: i64) -> String {
    (chrono::Utc::now().naive_utc() + chrono::Duration::hours(6) + chrono::Duration::minutes(offset_minutes))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string()
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn promotions_price_sales_and_keep_their_counters(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool.clone(), common::config()))).await;
    let mut owner = sign_up(&srv, "Owner", "01711000951").await;
    let tenant = create_business(&srv, &mut owner, "Promo Store").await;
    let cashier = join(&srv, &owner, &tenant, "Cashier", "01711000952", &["cashier"]).await;

    let methods = get(&srv, &owner, "/api/v1/payment-methods").await;
    let cash = s(find(&methods.body, "name", "Cash"), "id");
    let units = get(&srv, &owner, "/api/v1/units").await;
    let pcs = s(find(&units.body, "short_name", "pcs"), "id");
    let electric = post(&srv, &owner, "/api/v1/categories", json!({ "name": "Electric" })).await;
    assert_eq!(electric.status, StatusCode::CREATED, "{}", electric.body);
    let electric = s(&electric.body, "id");

    let mut ids = Vec::new();
    for (name, price, category) in
        [("Battery", "500", Some(electric.clone())), ("Rice", "150", None), ("Oil", "120", None)]
    {
        let body = json!({ "name": name, "unit_id": pcs, "selling_price": price, "purchase_price": "50", "category_id": category });
        let res = post(&srv, &owner, "/api/v1/products", body).await;
        assert_eq!(res.status, StatusCode::CREATED, "{}", res.body);
        ids.push((s(&res.body, "id"), s(&res.body, "variant_id")));
    }
    let battery = ids[0].1.clone();
    let (rice_p, rice) = ids[1].clone();
    let (oil_p, oil) = ids[2].clone();
    let opening = post(
        &srv,
        &owner,
        "/api/v1/stock-adjustments",
        json!({ "reason": "opening", "lines": [
            { "variant_id": battery, "quantity": "50", "unit_cost": "300" },
            { "variant_id": rice, "quantity": "50", "unit_cost": "100" },
            { "variant_id": oil, "quantity": "50", "unit_cost": "80" },
        ] }),
    )
    .await;
    assert_eq!(opening.status, StatusCode::CREATED, "{}", opening.body);
    let pay = |total: &str| json!([{ "method_id": cash, "amount": total }]);

    // --- Product discount: 20% on the Electric category.
    let promo = post(
        &srv,
        &owner,
        "/api/v1/promotions",
        json!({ "name": "Eid offer", "type": "product_discount", "discount_type": "percent", "discount_value": "20",
                "targets": [{ "target_type": "category", "target_id": electric }], "status": "active" }),
    )
    .await;
    assert_eq!(promo.status, StatusCode::CREATED, "{}", promo.body);
    assert_eq!(promo.body["matched_products"], 1);
    assert_eq!(promo.body["state"], "running");
    let eid = s(&promo.body, "id");
    let live = get(&srv, &cashier, "/api/v1/pos/promotions").await;
    assert_eq!(live.status, StatusCode::OK, "{}", live.body);
    assert!(live.body.as_array().is_some_and(|l| l.iter().any(|p| p["id"] == eid.as_str())), "{}", live.body);

    // The POS must have priced it: the undiscounted total is refused.
    let wrong = sell(&srv, &cashier, json!({ "lines": [{ "variant_id": battery, "quantity": "2" }], "payments": pay("1000"), "expected_total": "1000" })).await;
    assert_eq!(wrong.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", wrong.body);
    assert_eq!(wrong.code(), "TOTAL_MISMATCH");
    let sale = sell(&srv, &cashier, json!({ "lines": [{ "variant_id": battery, "quantity": "2" }], "payments": pay("800"), "expected_total": "800" })).await;
    assert_eq!(sale.status, StatusCode::CREATED, "{}", sale.body);
    assert_eq!(num(&sale.body["promotion_discount_total"]), 200.0);
    let line = &sale.body["items"][0];
    assert_eq!(num(&line["promotion_discount"]), 200.0);
    assert_eq!(num(&line["line_total"]), 800.0);
    assert_eq!(line["discounts"][0]["name"], "Eid offer");

    // Used: benefit and products are locked; the name is not.
    let detail = get(&srv, &owner, &format!("/api/v1/promotions/{eid}")).await;
    assert_eq!(detail.body["locked"], true);
    assert_eq!(detail.body["usage_count"], 1);
    let version = detail.body["version"].as_i64().expect("version");
    let edit = |value: &str, name: &str| {
        json!({ "name": name, "type": "product_discount", "discount_type": "percent", "discount_value": value,
                "targets": [{ "target_type": "category", "target_id": electric }] })
    };
    let locked = call(
        &srv,
        Req::put(&format!("/api/v1/promotions/{eid}"), edit("30", "Eid offer"))
            .bearer(&owner.token)
            .if_match(version)
            .build(),
    )
    .await;
    assert_eq!(locked.status, StatusCode::CONFLICT, "{}", locked.body);
    assert_eq!(locked.code(), "PROMOTION_LOCKED");
    let renamed = call(
        &srv,
        Req::put(&format!("/api/v1/promotions/{eid}"), edit("20", "Eid mubarak"))
            .bearer(&owner.token)
            .if_match(version)
            .build(),
    )
    .await;
    assert_eq!(renamed.status, StatusCode::OK, "{}", renamed.body);
    let deleted = call(&srv, Req::delete(&format!("/api/v1/promotions/{eid}")).bearer(&owner.token).build()).await;
    assert_eq!(deleted.code(), "PROMOTION_USED");

    // --- Removing an offer at the POS needs the permission and is audited.
    let line_off = json!({ "lines": [{ "variant_id": battery, "quantity": "1", "no_promotions": true }], "payments": pay("500"), "expected_total": "500" });
    let denied = sell(&srv, &cashier, line_off.clone()).await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN, "{}", denied.body);
    assert_eq!(denied.code(), "PROMOTION_REMOVAL_DENIED");
    let removed = sell(&srv, &owner, line_off).await;
    assert_eq!(removed.status, StatusCode::CREATED, "{}", removed.body);
    let audit = get(&srv, &owner, "/api/v1/audit-logs?action=promotion.removed_at_pos").await;
    assert!(audit.body.to_string().contains("promotion.removed_at_pos"), "{}", audit.body);

    // --- Flash: 50% off rice, 3 units only.
    let flash = post(
        &srv,
        &owner,
        "/api/v1/promotions",
        json!({ "name": "Rice flash", "type": "flash", "discount_type": "percent", "discount_value": "50",
                "starts_at": local(-60), "ends_at": local(60), "quantity_limit": "3",
                "targets": [{ "target_type": "product", "target_id": rice_p }], "status": "active" }),
    )
    .await;
    assert_eq!(flash.status, StatusCode::CREATED, "{}", flash.body);
    let flash_id = s(&flash.body, "id");
    let first = sell(
        &srv,
        &cashier,
        json!({ "lines": [{ "variant_id": rice, "quantity": "2" }], "payments": pay("150"), "expected_total": "150" }),
    )
    .await;
    assert_eq!(first.status, StatusCode::CREATED, "{}", first.body);
    // One flash unit left: 75 + 150.
    let second = sell(
        &srv,
        &cashier,
        json!({ "lines": [{ "variant_id": rice, "quantity": "2" }], "payments": pay("225"), "expected_total": "225" }),
    )
    .await;
    assert_eq!(second.status, StatusCode::CREATED, "{}", second.body);
    let sold_out = get(&srv, &owner, &format!("/api/v1/promotions/{flash_id}")).await;
    assert_eq!(num(&sold_out.body["quantity_used"]), 3.0);
    assert_eq!(sold_out.body["state"], "sold_out");
    let full = sell(
        &srv,
        &cashier,
        json!({ "lines": [{ "variant_id": rice, "quantity": "1" }], "payments": pay("150"), "expected_total": "150" }),
    )
    .await;
    assert_eq!(full.status, StatusCode::CREATED, "{}", full.body);

    // --- Buy 1 Get 1 on oil: 2 for the price of 1, the free unit recorded.
    let bogo = post(
        &srv,
        &owner,
        "/api/v1/promotions",
        json!({ "name": "Oil BOGO", "type": "bxgy", "targets": [{ "target_type": "product", "target_id": oil_p }],
                "bxgy": { "buy_quantity": "1", "get_quantity": "1", "get_target_type": "same_product", "get_discount_type": "free" },
                "status": "active" }),
    )
    .await;
    assert_eq!(bogo.status, StatusCode::CREATED, "{}", bogo.body);
    let two = sell(
        &srv,
        &cashier,
        json!({ "lines": [{ "variant_id": oil, "quantity": "2" }], "payments": pay("120"), "expected_total": "120" }),
    )
    .await;
    assert_eq!(two.status, StatusCode::CREATED, "{}", two.body);
    assert_eq!(num(&two.body["items"][0]["free_quantity"]), 2.0 - 1.0);
    // Returning the paid unit while keeping the free one refunds nothing (§8);
    // returning the other one then refunds the line.
    let item_id = s(&two.body["items"][0], "id");
    let ret = |qty: &str| {
        let body = json!({ "sale_id": s(&two.body, "id"), "reason": "test", "refund_mode": "cash",
                           "lines": [{ "sale_item_id": item_id, "quantity": qty }] });
        let req = test::TestRequest::post()
            .uri("/api/v1/sale-returns")
            .set_json(body)
            .insert_header(("authorization", format!("Bearer {}", owner.token)))
            .insert_header(("idempotency-key", Uuid::now_v7().to_string()))
            .to_request();
        call(&srv, req)
    };
    let first_back = ret("1").await;
    assert_eq!(first_back.status, StatusCode::CREATED, "{}", first_back.body);
    assert_eq!(num(&first_back.body["total"]), 0.0);
    let second_back = ret("1").await;
    assert_eq!(num(&second_back.body["total"]), 120.0, "{}", second_back.body);

    // --- Coupon: 10% off, minimum 100, one use.
    let coupon = post(
        &srv,
        &owner,
        "/api/v1/promotions",
        json!({ "name": "Ten off", "type": "coupon", "discount_type": "percent", "discount_value": "10",
                "min_purchase_amount": "100", "targets": [{ "target_type": "all_products" }], "status": "active" }),
    )
    .await;
    assert_eq!(coupon.status, StatusCode::CREATED, "{}", coupon.body);
    let coupon_id = s(&coupon.body, "id");
    let code = post(
        &srv,
        &owner,
        &format!("/api/v1/promotions/{coupon_id}/coupons"),
        json!({ "code": "eid10", "usage_limit": 1 }),
    )
    .await;
    assert_eq!(code.status, StatusCode::CREATED, "{}", code.body);
    assert_eq!(code.body["code"], "EID10");
    let check = |code: &str| json!({ "code": code });
    let unknown = post(&srv, &cashier, "/api/v1/pos/coupons/validate", check("NOPE")).await;
    assert_eq!(field_code(&unknown), "COUPON_NOT_FOUND", "{}", unknown.body);
    let ok = post(&srv, &cashier, "/api/v1/pos/coupons/validate", check("Eid10")).await;
    assert_eq!(ok.status, StatusCode::OK, "{}", ok.body);
    assert_eq!(ok.body["coupon_code"], "EID10");
    // The battery's 20% gives more than 10%: the coupon does not stack by default.
    let not_better = sell(&srv, &cashier, json!({ "lines": [{ "variant_id": battery, "quantity": "1" }], "coupon_code": "EID10", "payments": pay("400"), "expected_total": "400" })).await;
    assert_eq!(field_code(&not_better), "COUPON_NOT_BETTER", "{}", not_better.body);
    let too_small = sell(&srv, &cashier, json!({ "lines": [{ "variant_id": oil, "quantity": "1" }], "coupon_code": "EID10", "payments": pay("108"), "expected_total": "108" })).await;
    assert_eq!(too_small.status, StatusCode::CREATED, "{}", too_small.body);
    let used = get(&srv, &owner, &format!("/api/v1/promotions/{coupon_id}/redemptions")).await;
    assert_eq!(used.body.as_array().map(Vec::len), Some(1), "{}", used.body);
    let again = post(&srv, &cashier, "/api/v1/pos/coupons/validate", check("EID10")).await;
    assert_eq!(field_code(&again), "COUPON_USED_UP", "{}", again.body);
    let below = sell(
        &srv,
        &owner,
        json!({ "lines": [{ "variant_id": rice, "quantity": "1" }], "payments": pay("150"), "expected_total": "150" }),
    )
    .await;
    assert_eq!(below.status, StatusCode::CREATED);

    // Cancelling the sale gives the coupon use back.
    let sale_id = s(&too_small.body, "id");
    let cancel = post(&srv, &owner, &format!("/api/v1/sales/{sale_id}/cancel"), json!({ "reason": "test" })).await;
    assert_eq!(cancel.status, StatusCode::OK, "{}", cancel.body);
    let back = post(&srv, &cashier, "/api/v1/pos/coupons/validate", check("EID10")).await;
    assert_eq!(back.status, StatusCode::OK, "{}", back.body);
    let redemptions = get(&srv, &owner, &format!("/api/v1/promotions/{coupon_id}/redemptions")).await;
    assert_eq!(redemptions.body[0]["status"], "reversed");

    // Limits stay editable after use: raise the minimum, then it is not met.
    let current = get(&srv, &owner, &format!("/api/v1/promotions/{coupon_id}")).await;
    let version = current.body["version"].as_i64().expect("version");
    let raised = call(
        &srv,
        Req::put(
            &format!("/api/v1/promotions/{coupon_id}"),
            json!({ "name": "Ten off", "type": "coupon", "discount_type": "percent", "discount_value": "10",
                    "min_purchase_amount": "1000", "targets": [{ "target_type": "all_products" }] }),
        )
        .bearer(&owner.token)
        .if_match(version)
        .build(),
    )
    .await;
    assert_eq!(raised.status, StatusCode::OK, "{}", raised.body);
    let min = sell(&srv, &cashier, json!({ "lines": [{ "variant_id": rice, "quantity": "1" }], "coupon_code": "EID10", "payments": pay("135"), "expected_total": "135" })).await;
    assert_eq!(field_code(&min), "COUPON_MIN_PURCHASE", "{}", min.body);

    // Cross-product Buy X Get Y: buy sugar, get salt free. Returning the sugar
    // while keeping the salt charges the salt; returning the salt gives it back.
    let mut cross = Vec::new();
    for (name, price) in [("Sugar", "100"), ("Salt", "40")] {
        let res = post(
            &srv,
            &owner,
            "/api/v1/products",
            json!({ "name": name, "unit_id": pcs, "selling_price": price, "purchase_price": "20" }),
        )
        .await;
        assert_eq!(res.status, StatusCode::CREATED, "{}", res.body);
        cross.push((s(&res.body, "id"), s(&res.body, "variant_id")));
    }
    let (sugar_p, sugar) = cross[0].clone();
    let (salt_p, salt) = cross[1].clone();
    let stocked = post(
        &srv,
        &owner,
        "/api/v1/stock-adjustments",
        json!({ "reason": "opening", "lines": [{ "variant_id": sugar, "quantity": "10", "unit_cost": "20" }, { "variant_id": salt, "quantity": "10", "unit_cost": "20" }] }),
    )
    .await;
    assert_eq!(stocked.status, StatusCode::CREATED, "{}", stocked.body);
    let salt_free = post(
        &srv,
        &owner,
        "/api/v1/promotions",
        json!({ "name": "Salt free", "type": "bxgy", "targets": [{ "target_type": "product", "target_id": sugar_p }],
                "bxgy": { "buy_quantity": "1", "get_quantity": "1", "get_target_type": "product", "get_target_id": salt_p,
                          "get_discount_type": "free", "repeat": false }, "status": "active" }),
    )
    .await;
    assert_eq!(salt_free.status, StatusCode::CREATED, "{}", salt_free.body);
    let both = sell(
        &srv,
        &cashier,
        json!({ "lines": [{ "variant_id": sugar, "quantity": "1" }, { "variant_id": salt, "quantity": "1" }], "payments": pay("100"), "expected_total": "100" }),
    )
    .await;
    assert_eq!(both.status, StatusCode::CREATED, "{}", both.body);
    let line_of = |variant: &str| s(find(&both.body["items"], "variant_id", variant), "id");
    let give_back = |item: String| {
        let body = json!({ "sale_id": s(&both.body, "id"), "reason": "test", "refund_mode": "cash",
                           "lines": [{ "sale_item_id": item, "quantity": "1" }] });
        let req = test::TestRequest::post()
            .uri("/api/v1/sale-returns")
            .set_json(body)
            .insert_header(("authorization", format!("Bearer {}", owner.token)))
            .insert_header(("idempotency-key", Uuid::now_v7().to_string()))
            .to_request();
        call(&srv, req)
    };
    let sugar_back = give_back(line_of(&sugar)).await;
    assert_eq!(sugar_back.status, StatusCode::CREATED, "{}", sugar_back.body);
    assert_eq!(num(&sugar_back.body["total"]), 60.0, "{}", sugar_back.body);
    let salt_back = give_back(line_of(&salt)).await;
    assert_eq!(salt_back.status, StatusCode::CREATED, "{}", salt_back.body);
    assert_eq!(num(&salt_back.body["total"]), 40.0, "{}", salt_back.body);

    // The promotions report sees all of it.
    let report = get(&srv, &owner, "/api/v1/reports/promotions").await;
    assert_eq!(report.status, StatusCode::OK, "{}", report.body);
    let names: Vec<&str> =
        report.body["promotions"].as_array().into_iter().flatten().filter_map(|p| p["name"].as_str()).collect();
    assert!(names.contains(&"Rice flash") && names.contains(&"Oil BOGO"), "{}", report.body);
    assert!(report.body["flash_sales"][0]["sold_out_at"].is_string(), "{}", report.body);
}
