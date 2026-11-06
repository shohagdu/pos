//! Product warranty over HTTP: a product carries a tenure in days, months or
//! years; a sale line keeps the tenure it was sold with and the date the
//! warranty runs out, even after the product changes; the sales list sums
//! up each invoice's warranty.
//!
//! One test function: every `#[sqlx::test]` migrates a fresh database.

mod common;

use actix_web::{http::StatusCode, test};
use chrono::{Months, NaiveDate};
use common::{Req, Res, Session, call, create_business, sign_up};
use pos_api::{AppState, app};
use serde_json::{Value, json};
use sqlx::MySqlPool;
use uuid::Uuid;

fn s(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or_else(|| panic!("{key} in {v}")).to_owned()
}

async fn get<S, B>(srv: &S, who: &Session, uri: &str) -> Res
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    call(srv, Req::get(uri).bearer(&who.token).build()).await
}

async fn send<S, B>(srv: &S, who: &Session, req: test::TestRequest, body: Value) -> Res
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let req = req
        .set_json(body)
        .insert_header(("authorization", format!("Bearer {}", who.token)))
        .insert_header(("idempotency-key", Uuid::now_v7().to_string()))
        .to_request();
    call(srv, req).await
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn warranty_is_kept_on_the_sale_line(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool, common::config()))).await;
    let mut owner = sign_up(&srv, "Owner", "01711000961").await;
    create_business(&srv, &mut owner, "Gadget Corner").await;

    let methods = get(&srv, &owner, "/api/v1/payment-methods").await;
    let cash = methods.body.as_array().into_iter().flatten().find(|m| m["name"] == "Cash").map(|m| s(m, "id"));
    let units = get(&srv, &owner, "/api/v1/units").await;
    let pcs = units.body.as_array().into_iter().flatten().find(|u| u["short_name"] == "pcs").map(|u| s(u, "id"));
    let post = test::TestRequest::post;

    // Both halves of the tenure are needed, within range.
    for (body, field) in [
        (json!({ "warranty_period": 3 }), "warranty_unit"),
        (json!({ "warranty_period": 0, "warranty_unit": "month" }), "warranty_period"),
        (json!({ "warranty_period": 2, "warranty_unit": "week" }), "warranty_unit"),
    ] {
        let mut input = json!({ "name": "Bad", "unit_id": pcs, "selling_price": "1" });
        input.as_object_mut().unwrap().extend(body.as_object().unwrap().clone());
        let res = send(&srv, &owner, post().uri("/api/v1/products"), input).await;
        assert_eq!(res.status, StatusCode::BAD_REQUEST, "{}", res.body);
        assert_eq!(res.body["errors"][0]["field"], field, "{}", res.body);
    }

    let made = send(
        &srv,
        &owner,
        post().uri("/api/v1/products"),
        json!({ "name": "Power Bank", "unit_id": pcs, "selling_price": "1500", "track_inventory": false,
                "barcodes": ["PB-123"], "warranty_period": 3, "warranty_unit": "Month" }),
    )
    .await;
    assert_eq!(made.status, StatusCode::CREATED, "{}", made.body);
    assert_eq!(made.body["warranty_period"], 3);
    assert_eq!(made.body["warranty_unit"], "month");
    let plain = send(
        &srv,
        &owner,
        post().uri("/api/v1/products"),
        json!({ "name": "Bag", "unit_id": pcs, "selling_price": "50", "track_inventory": false }),
    )
    .await;
    assert_eq!(plain.body["warranty_period"], Value::Null, "{}", plain.body);

    let sale = send(
        &srv,
        &owner,
        post().uri("/api/v1/sales"),
        json!({ "lines": [{ "variant_id": s(&made.body, "variant_id"), "quantity": "1" },
                          { "variant_id": s(&plain.body, "variant_id"), "quantity": "1" }],
                "payments": [{ "method_id": cash, "amount": "1550" }] }),
    )
    .await;
    assert_eq!(sale.status, StatusCode::CREATED, "{}", sale.body);
    let day: NaiveDate = s(&sale.body, "business_date").parse().unwrap();
    let expires = (day + Months::new(3)).to_string();
    let line = &sale.body["items"][0];
    assert_eq!((&line["warranty_period"], &line["warranty_unit"]), (&json!(3), &json!("month")), "{line}");
    assert_eq!(line["warranty_expires_on"], expires.as_str());
    assert_eq!(sale.body["items"][1]["warranty_expires_on"], Value::Null);

    // The sales list (also the customer's history) sums up each invoice's
    // warranty and can show only sales with one.
    let plain_sale = send(
        &srv,
        &owner,
        post().uri("/api/v1/sales"),
        json!({ "lines": [{ "variant_id": s(&plain.body, "variant_id"), "quantity": "1" }],
                "payments": [{ "method_id": cash, "amount": "50" }] }),
    )
    .await;
    assert_eq!(plain_sale.status, StatusCode::CREATED, "{}", plain_sale.body);
    let list = get(&srv, &owner, "/api/v1/sales").await;
    let row = |id: &str| list.body["data"].as_array().unwrap().iter().find(|r| r["id"] == id).cloned().unwrap();
    let with = row(&s(&sale.body, "id"));
    assert_eq!((&with["warranty_items"], &with["warranty_until"]), (&json!(1), &json!(expires)), "{with}");
    let without = row(&s(&plain_sale.body, "id"));
    assert_eq!((&without["warranty_items"], &without["warranty_until"]), (&json!(0), &Value::Null), "{without}");
    let only = get(&srv, &owner, "/api/v1/sales?with_warranty=true").await;
    let ids: Vec<String> = only.body["data"].as_array().unwrap().iter().map(|r| s(r, "id")).collect();
    assert_eq!(ids, [s(&sale.body, "id")], "{}", only.body);
    assert_eq!(only.body["summary"]["count"], 1);

    // One row per warranty line, for the customer's history and claims.
    let customer = s(&sale.body, "customer_id");
    let items = get(&srv, &owner, &format!("/api/v1/sale-warranties?customer_id={customer}")).await;
    assert_eq!(items.status, StatusCode::OK, "{}", items.body);
    assert_eq!(items.body["today"], s(&sale.body, "business_date"));
    let rows = items.body["data"].as_array().unwrap();
    assert_eq!(rows.len(), 1, "{}", items.body);
    assert_eq!(rows[0]["product_name"], "Power Bank");
    assert_eq!(rows[0]["invoice_no"], sale.body["invoice_no"]);
    assert_eq!(rows[0]["warranty_expires_on"], expires.as_str());
    let active = get(&srv, &owner, &format!("/api/v1/sale-warranties?customer_id={customer}&status=active")).await;
    assert_eq!(active.body["data"].as_array().map(Vec::len), Some(1), "{}", active.body);
    let expired = get(&srv, &owner, "/api/v1/sale-warranties?status=expired").await;
    assert_eq!(expired.body["data"].as_array().map(Vec::len), Some(0), "{}", expired.body);
    let other = get(&srv, &owner, &format!("/api/v1/sale-warranties?customer_id={}", Uuid::now_v7())).await;
    assert_eq!(other.body["data"].as_array().map(Vec::len), Some(0), "{}", other.body);
    // Warranty lookup: by invoice number, or by scanning the item's barcode.
    let invoice = s(&sale.body, "invoice_no");
    for q in [invoice.as_str(), "PB-123", "power"] {
        let found = get(&srv, &owner, &format!("/api/v1/sale-warranties?q={q}")).await;
        assert_eq!(found.body["data"].as_array().map(Vec::len), Some(1), "{q}: {}", found.body);
    }
    let none = get(&srv, &owner, "/api/v1/sale-warranties?q=PB-12").await;
    assert_eq!(none.body["data"].as_array().map(Vec::len), Some(0), "a barcode matches exactly: {}", none.body);
    let bad = get(&srv, &owner, "/api/v1/sale-warranties?status=old").await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST, "{}", bad.body);

    // Changing the product later leaves the sold warranty alone; dropping
    // both fields removes the product's warranty.
    let id = s(&made.body, "id");
    let body = json!({ "name": "Power Bank", "unit_id": pcs, "selling_price": "1500", "track_inventory": false });
    let edited =
        call(&srv, Req::put(&format!("/api/v1/products/{id}"), body).bearer(&owner.token).if_match(1).build()).await;
    assert_eq!(edited.status, StatusCode::OK, "{}", edited.body);
    assert_eq!(edited.body["warranty_unit"], Value::Null);
    let again = get(&srv, &owner, &format!("/api/v1/sales/{}", s(&sale.body, "id"))).await;
    assert_eq!(again.body["items"][0]["warranty_expires_on"], expires.as_str(), "{}", again.body);
}
