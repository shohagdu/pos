//! Batch and expiry tracking (STK-R11) over HTTP: opening stock and purchases
//! put stock into batches, sales take the earliest expiry first, returns and
//! cancellations put it back into the batches it came from, supplier returns
//! take from the batches the purchase brought in, write-offs name a batch, and
//! the expiry report lists what is expired or expiring.
//!
//! One test function: every `#[sqlx::test]` migrates a fresh database.

mod common;

use actix_web::{http::StatusCode, test};
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

/// POST with a fresh Idempotency-Key (posting endpoints accept one; POS sales need it).
async fn post<S, B>(srv: &S, who: &Session, uri: &str, body: Value) -> Res
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let req = test::TestRequest::post()
        .uri(uri)
        .set_json(body)
        .insert_header(("authorization", format!("Bearer {}", who.token)))
        .insert_header(("idempotency-key", Uuid::now_v7().to_string()))
        .to_request();
    call(srv, req).await
}

/// `(batch_no, quantity)` of the variant's batches in stock, earliest expiry first.
async fn batches_of<S, B>(srv: &S, who: &Session, variant_id: &str) -> Vec<(String, String)>
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let res = get(srv, who, &format!("/api/v1/stock/batches?variant_id={variant_id}")).await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.body);
    res.body["data"].as_array().into_iter().flatten().map(|b| (s(b, "batch_no"), s(b, "quantity"))).collect()
}

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter().map(|(a, b)| ((*a).to_owned(), (*b).to_owned())).collect()
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn batches_follow_stock_earliest_expiry_first(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool, common::config()))).await;
    let mut owner = sign_up(&srv, "Owner", "01711000951").await;
    create_business(&srv, &mut owner, "Shifa Pharmacy").await;

    let methods = get(&srv, &owner, "/api/v1/payment-methods").await;
    let cash = methods.body.as_array().into_iter().flatten().find(|m| m["name"] == "Cash").map(|m| s(m, "id"));
    let units = get(&srv, &owner, "/api/v1/units").await;
    let pcs = units.body.as_array().into_iter().flatten().find(|u| u["short_name"] == "pcs").map(|u| s(u, "id"));
    let product = post(
        &srv,
        &owner,
        "/api/v1/products",
        json!({ "name": "Napa 500mg", "unit_id": pcs, "selling_price": "20", "purchase_price": "15" }),
    )
    .await;
    assert_eq!(product.status, StatusCode::CREATED, "{}", product.body);
    assert_eq!(product.body["track_batches"], false);
    let napa = s(&product.body, "variant_id");
    let product_id = s(&product.body, "id");

    // Opening stock arrives as batch A; the product now tracks batches.
    let opening = post(
        &srv,
        &owner,
        "/api/v1/stock-adjustments",
        json!({ "reason": "opening", "lines": [{ "variant_id": napa, "quantity": "10", "batch_no": "A", "expiry_date": "2030-01-31" }] }),
    )
    .await;
    assert_eq!(opening.status, StatusCode::CREATED, "{}", opening.body);
    assert_eq!(opening.body["items"][0]["batch_no"], "A");
    assert_eq!(opening.body["items"][0]["expiry_date"], "2030-01-31");
    let p = get(&srv, &owner, &format!("/api/v1/products/{product_id}")).await;
    assert_eq!(p.body["track_batches"], true, "{}", p.body);

    // A batch number keeps its expiry date.
    let clash = post(
        &srv,
        &owner,
        "/api/v1/stock-adjustments",
        json!({ "reason": "found", "lines": [{ "variant_id": napa, "quantity": "1", "batch_no": "A", "expiry_date": "2031-01-01" }] }),
    )
    .await;
    assert_eq!(clash.status, StatusCode::BAD_REQUEST, "{}", clash.body);
    assert_eq!(clash.body["errors"][0]["field"], "lines[0].expiry_date", "{}", clash.body);

    // A purchase brings batch B, which expires earlier.
    let supplier = post(&srv, &owner, "/api/v1/suppliers", json!({ "name": "Beximco", "phone": "01811000051" })).await;
    let draft = post(
        &srv,
        &owner,
        "/api/v1/purchases",
        json!({ "supplier_id": s(&supplier.body, "id"),
                "lines": [{ "variant_id": napa, "quantity": "5", "unit_cost": "15", "batch_no": "B", "expiry_date": "2029-06-30" }] }),
    )
    .await;
    assert_eq!(draft.status, StatusCode::CREATED, "{}", draft.body);
    assert_eq!(draft.body["items"][0]["batch_no"], "B");
    let purchase_id = s(&draft.body, "id");
    let purchase_item = s(&draft.body["items"][0], "id");
    let posted = post(&srv, &owner, &format!("/api/v1/purchases/{purchase_id}/post"), json!({})).await;
    assert_eq!(posted.status, StatusCode::OK, "{}", posted.body);
    assert_eq!(batches_of(&srv, &owner, &napa).await, pairs(&[("B", "5.000"), ("A", "10.000")]));

    // A sale of 7 takes all of B, then 2 of A.
    let sale = post(&srv, &owner, "/api/v1/sales", json!({ "lines": [{ "variant_id": napa, "quantity": "7" }], "payments": [{ "method_id": cash, "amount": "140" }] })).await;
    assert_eq!(sale.status, StatusCode::CREATED, "{}", sale.body);
    assert_eq!(batches_of(&srv, &owner, &napa).await, pairs(&[("A", "8.000")]));

    // Returning 3 puts back A's 2 (latest expiry first), then 1 into B.
    let returned = post(
        &srv,
        &owner,
        "/api/v1/sale-returns",
        json!({ "sale_id": s(&sale.body, "id"), "reason": "Wrong strength", "refund_mode": "cash",
                "lines": [{ "sale_item_id": s(&sale.body["items"][0], "id"), "quantity": "3" }] }),
    )
    .await;
    assert_eq!(returned.status, StatusCode::CREATED, "{}", returned.body);
    assert_eq!(batches_of(&srv, &owner, &napa).await, pairs(&[("B", "1.000"), ("A", "10.000")]));

    // A cancelled sale puts its batch back.
    let one = post(&srv, &owner, "/api/v1/sales", json!({ "lines": [{ "variant_id": napa, "quantity": "1" }], "payments": [{ "method_id": cash, "amount": "20" }] })).await;
    assert_eq!(one.status, StatusCode::CREATED, "{}", one.body);
    assert_eq!(batches_of(&srv, &owner, &napa).await, pairs(&[("A", "10.000")]));
    let cancelled =
        post(&srv, &owner, &format!("/api/v1/sales/{}/cancel", s(&one.body, "id")), json!({ "reason": "Mistake" }))
            .await;
    assert_eq!(cancelled.status, StatusCode::OK, "{}", cancelled.body);
    assert_eq!(batches_of(&srv, &owner, &napa).await, pairs(&[("B", "1.000"), ("A", "10.000")]));

    // A supplier return takes from the batch that purchase brought in.
    let back = post(
        &srv,
        &owner,
        "/api/v1/purchase-returns",
        json!({ "purchase_id": purchase_id, "reason": "Damaged strip",
                "lines": [{ "purchase_item_id": purchase_item, "quantity": "1" }] }),
    )
    .await;
    assert_eq!(back.status, StatusCode::CREATED, "{}", back.body);
    assert_eq!(batches_of(&srv, &owner, &napa).await, pairs(&[("A", "10.000")]));

    // An expired batch shows on the report; a write-off names it.
    let found = post(
        &srv,
        &owner,
        "/api/v1/stock-adjustments",
        json!({ "reason": "found", "lines": [{ "variant_id": napa, "quantity": "2", "expiry_date": "2020-01-31" }] }),
    )
    .await;
    assert_eq!(found.status, StatusCode::CREATED, "{}", found.body);
    assert_eq!(found.body["items"][0]["batch_no"], "2020-01-31", "a blank number takes the date");
    let report = get(&srv, &owner, "/api/v1/stock/batches?status=expired").await;
    assert_eq!(report.status, StatusCode::OK, "{}", report.body);
    assert_eq!(report.body["summary"]["expired"], 1, "{}", report.body);
    assert_eq!(report.body["summary"]["expiring"], 0, "{}", report.body);
    assert_eq!(report.body["data"][0]["batch_no"], "2020-01-31");
    assert!(report.body["data"][0]["days_left"].as_i64().is_some_and(|d| d < 0), "{}", report.body);
    let expired_id = s(&report.body["data"][0], "batch_id");
    let options = get(&srv, &owner, &format!("/api/v1/stock/batch-options?variant_ids={napa}")).await;
    assert_eq!(options.status, StatusCode::OK, "{}", options.body);
    assert_eq!(options.body.as_array().map(Vec::len), Some(2), "{}", options.body);
    let write_off = post(
        &srv,
        &owner,
        "/api/v1/stock-adjustments",
        json!({ "reason": "expired", "lines": [{ "variant_id": napa, "quantity": "-2", "batch_id": expired_id }] }),
    )
    .await;
    assert_eq!(write_off.status, StatusCode::CREATED, "{}", write_off.body);
    assert_eq!(write_off.body["items"][0]["batches"][0]["quantity"], "-2.000", "{}", write_off.body);
    let report = get(&srv, &owner, "/api/v1/stock/batches?status=expired").await;
    assert_eq!(report.body["summary"]["expired"], 0, "{}", report.body);
    let soon = get(&srv, &owner, "/api/v1/stock/batches?status=expiring&days=36500").await;
    assert_eq!(soon.status, StatusCode::BAD_REQUEST, "{}", soon.body);

    // Stock without a batch is the remainder: a "found" without a batch adds no batch.
    let loose = post(
        &srv,
        &owner,
        "/api/v1/stock-adjustments",
        json!({ "reason": "found", "lines": [{ "variant_id": napa, "quantity": "4" }] }),
    )
    .await;
    assert_eq!(loose.status, StatusCode::CREATED, "{}", loose.body);
    let stock = get(&srv, &owner, &format!("/api/v1/stock?variant_id={napa}")).await;
    assert_eq!(stock.body["data"][0]["quantity"], "14.000", "{}", stock.body);
    assert_eq!(batches_of(&srv, &owner, &napa).await, pairs(&[("A", "10.000")]));

    // Product import: opening stock with a batch and expiry.
    let imported = post(
        &srv,
        &owner,
        "/api/v1/products/import",
        json!({ "products": [{ "name": "Seclo 20mg", "unit_id": pcs, "selling_price": "7", "generate_barcode": true }],
                "opening_stock": [{ "row": 0, "quantity": "30", "batch_no": "S-1", "expiry_date": "2028-02-29" }] }),
    )
    .await;
    assert_eq!(imported.status, StatusCode::CREATED, "{}", imported.body);
    let seclo = s(&imported.body[0], "variant_id");
    assert_eq!(batches_of(&srv, &owner, &seclo).await, pairs(&[("S-1", "30.000")]));
}
