//! Phase 6 over HTTP: a second branch with its own locations, numbering and
//! cash drawer; `X-Branch-Id` scoping of postings and lists; a transfer with
//! a transit loss; a stock count; a register shift with X/Z figures; other
//! income and its void; the new reports; and the books still balance.
//!
//! One test function: every `#[sqlx::test]` migrates a fresh database.

mod common;

use actix_web::{http::StatusCode, test};
use common::{Res, Session, call, create_business, sign_up};
use pos_api::{AppState, app};
use serde_json::{Value, json};
use sqlx::MySqlPool;

fn s(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or_else(|| panic!("{key} in {v}")).to_owned()
}

fn num(v: &Value, key: &str) -> f64 {
    s(v, key).parse::<f64>().unwrap_or_else(|_| panic!("{key} is a number in {v}"))
}

fn find<'a>(list: &'a Value, key: &str, value: &str) -> &'a Value {
    list.as_array().into_iter().flatten().find(|v| v[key] == value).unwrap_or_else(|| panic!("{value} in {list}"))
}

/// A request as `who`, optionally working in `branch`.
async fn send<S, B>(srv: &S, who: &Session, method: &str, uri: &str, body: Option<Value>, branch: Option<&str>) -> Res
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let req = match method {
        "get" => test::TestRequest::get(),
        "post" => test::TestRequest::post(),
        "put" => test::TestRequest::put(),
        _ => unreachable!("{method}"),
    };
    let mut req = req
        .uri(uri)
        .insert_header(("authorization", format!("Bearer {}", who.token)))
        .insert_header(("idempotency-key", uuid::Uuid::now_v7().to_string()));
    if let Some(branch) = branch {
        req = req.insert_header(("x-branch-id", branch.to_owned()));
    }
    if let Some(body) = body {
        req = req.set_json(body);
    }
    call(srv, req.to_request()).await
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn branches_transfers_counts_shifts_and_income(pool: MySqlPool) {
    // Room for a second branch, transfers and registers on the trial plan.
    sqlx::query(
        "UPDATE plan_features pf JOIN plans p ON p.id = pf.plan_id \
         SET pf.enabled = TRUE, pf.limit_value = IF(pf.feature_code IN ('branches.max', 'registers.max'), 3, pf.limit_value) \
         WHERE p.code = 'trial' AND pf.feature_code IN ('branches.max', 'registers.max', 'inventory.transfers')",
    )
    .execute(&pool)
    .await
    .expect("plan limits");
    let srv = test::init_service(app(AppState::new(pool.clone(), common::config()))).await;
    let mut owner = sign_up(&srv, "Owner", "01711000951").await;
    create_business(&srv, &mut owner, "Karim Traders").await;
    macro_rules! get {
        ($uri:expr) => {
            send(&srv, &owner, "get", $uri, None, None)
        };
    }

    let units = get!("/api/v1/units").await;
    let pcs = s(find(&units.body, "short_name", "pcs"), "id");
    let methods = get!("/api/v1/payment-methods").await;
    let cash = s(find(&methods.body, "name", "Cash"), "id");
    let product = send(
        &srv,
        &owner,
        "post",
        "/api/v1/products",
        Some(json!({ "name": "Soap", "unit_id": pcs, "selling_price": "100", "purchase_price": "60" })),
        None,
    )
    .await;
    assert_eq!(product.status, StatusCode::CREATED, "{}", product.body);
    let soap = s(&product.body, "variant_id");
    let opening = send(
        &srv,
        &owner,
        "post",
        "/api/v1/stock-adjustments",
        Some(json!({ "reason": "opening", "lines": [{ "variant_id": soap, "quantity": "10", "unit_cost": "60" }] })),
        None,
    )
    .await;
    assert_eq!(opening.status, StatusCode::CREATED, "{}", opening.body);

    // --- a second branch (BRN-R01)
    let branches = get!("/api/v1/branches").await;
    let main = s(&branches.body[0], "id");
    let created =
        send(&srv, &owner, "post", "/api/v1/branches", Some(json!({ "code": "mir", "name": "Mirpur" })), None).await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
    assert_eq!(created.body["code"], "MIR");
    let mirpur = s(&created.body, "id");
    let duplicate =
        send(&srv, &owner, "post", "/api/v1/branches", Some(json!({ "code": "MIR", "name": "Again" })), None).await;
    assert_eq!(duplicate.status, StatusCode::CONFLICT, "{}", duplicate.body);
    let close_main = send(&srv, &owner, "post", &format!("/api/v1/branches/{main}/deactivate"), None, None).await;
    assert_eq!(close_main.code(), "DEFAULT_BRANCH", "{}", close_main.body);

    let locations = get!("/api/v1/stock-locations").await;
    let mir_main = s(find(&locations.body, "code", "MIR-MAIN"), "id");
    find(&locations.body, "code", "MIR-DMG");
    let accounts = get!("/api/v1/payment-accounts").await;
    let mir_cash = s(find(&accounts.body, "name", "Cash - Mirpur"), "id");
    let bad_branch = send(&srv, &owner, "get", "/api/v1/sales", None, Some("not-a-uuid")).await;
    assert_eq!(bad_branch.status, StatusCode::BAD_REQUEST, "{}", bad_branch.body);

    // --- transfer 4 from the main branch; 3 arrive (transit loss 60)
    let transfer = send(
        &srv,
        &owner,
        "post",
        "/api/v1/stock-transfers",
        Some(
            json!({ "to_location_id": mir_main, "dispatch": true, "lines": [{ "variant_id": soap, "quantity": "4" }] }),
        ),
        Some(&main),
    )
    .await;
    assert_eq!(transfer.status, StatusCode::CREATED, "{}", transfer.body);
    assert_eq!(transfer.body["status"], "dispatched");
    assert_eq!(num(&transfer.body, "total_value"), 240.0);
    let transfer_id = s(&transfer.body, "id");
    let item = s(&transfer.body["items"][0], "id");
    let wrong = send(
        &srv,
        &owner,
        "post",
        &format!("/api/v1/stock-transfers/{transfer_id}/receive"),
        Some(json!({})),
        Some(&main),
    )
    .await;
    assert_eq!(wrong.code(), "BRANCH_NOT_ALLOWED", "{}", wrong.body);
    let received = send(
        &srv,
        &owner,
        "post",
        &format!("/api/v1/stock-transfers/{transfer_id}/receive"),
        Some(json!({ "lines": [{ "item_id": item, "quantity_received": "3" }] })),
        Some(&mirpur),
    )
    .await;
    assert_eq!(received.status, StatusCode::OK, "{}", received.body);
    assert_eq!(received.body["status"], "received");
    assert_eq!(num(&received.body, "loss_value"), 60.0);
    let stock_at = |branch: String| {
        let srv = &srv;
        let owner = &owner;
        let soap = soap.clone();
        async move {
            let res = send(srv, owner, "get", &format!("/api/v1/stock?variant_id={soap}"), None, Some(&branch)).await;
            assert_eq!(res.status, StatusCode::OK, "{}", res.body);
            num(&res.body["data"][0], "quantity")
        }
    };
    assert_eq!(stock_at(main.clone()).await, 6.0);
    assert_eq!(stock_at(mirpur.clone()).await, 3.0);

    // --- a sale in Mirpur goes to its drawer and its own list
    let sale = send(
        &srv,
        &owner,
        "post",
        "/api/v1/sales",
        Some(json!({ "lines": [{ "variant_id": soap, "quantity": "1" }], "payments": [{ "method_id": cash, "amount": "100" }] })),
        Some(&mirpur),
    )
    .await;
    assert_eq!(sale.status, StatusCode::CREATED, "{}", sale.body);
    assert!(s(&sale.body, "invoice_no").starts_with("INV-MIR"), "{}", sale.body);
    let mir_sales = send(&srv, &owner, "get", "/api/v1/sales", None, Some(&mirpur)).await;
    assert_eq!(mir_sales.body["data"].as_array().map(Vec::len), Some(1), "{}", mir_sales.body);
    let main_sales = send(&srv, &owner, "get", "/api/v1/sales", None, Some(&main)).await;
    assert_eq!(main_sales.body["data"].as_array().map(Vec::len), Some(0), "{}", main_sales.body);
    let account = get!(&format!("/api/v1/payment-accounts/{mir_cash}")).await;
    assert_eq!(num(&account.body, "current_balance"), 100.0, "{}", account.body);

    // --- a register shift: float 500, one cash sale of 100, counted 590
    let register =
        send(&srv, &owner, "post", "/api/v1/registers", Some(json!({ "name": "Till 1" })), Some(&mirpur)).await;
    assert_eq!(register.status, StatusCode::CREATED, "{}", register.body);
    assert_eq!(register.body["cash_account_id"], mir_cash.as_str());
    let shift = send(
        &srv,
        &owner,
        "post",
        "/api/v1/shifts",
        Some(json!({ "register_id": s(&register.body, "id"), "opening_cash": "500" })),
        Some(&mirpur),
    )
    .await;
    assert_eq!(shift.status, StatusCode::CREATED, "{}", shift.body);
    let shift_id = s(&shift.body, "id");
    let current = send(&srv, &owner, "get", "/api/v1/shifts/current", None, Some(&mirpur)).await;
    assert_eq!(current.body["id"], shift_id.as_str());
    let in_shift = send(
        &srv,
        &owner,
        "post",
        "/api/v1/sales",
        Some(json!({ "lines": [{ "variant_id": soap, "quantity": "1" }], "payments": [{ "method_id": cash, "amount": "100" }] })),
        Some(&mirpur),
    )
    .await;
    assert_eq!(in_shift.status, StatusCode::CREATED, "{}", in_shift.body);
    let x = get!(&format!("/api/v1/shifts/{shift_id}/report")).await;
    assert_eq!(x.body["kind"], "X", "{}", x.body);
    assert_eq!(x.body["sales_count"], 1);
    assert_eq!(num(&x.body, "expected_cash"), 600.0);
    let z = send(
        &srv,
        &owner,
        "post",
        &format!("/api/v1/shifts/{shift_id}/close"),
        Some(json!({ "counted_cash": "590", "note": "Short 10" })),
        Some(&mirpur),
    )
    .await;
    assert_eq!(z.status, StatusCode::OK, "{}", z.body);
    assert_eq!(z.body["kind"], "Z");
    assert_eq!(num(&z.body["shift"], "difference"), -10.0);

    // --- a count in Mirpur finds 3 where the system has 1
    let count = send(&srv, &owner, "post", "/api/v1/stock-counts", Some(json!({})), Some(&mirpur)).await;
    assert_eq!(count.status, StatusCode::CREATED, "{}", count.body);
    let count_id = s(&count.body, "id");
    let line = find(&count.body["items"], "variant_id", &soap);
    assert_eq!(num(line, "system_qty"), 1.0);
    let entered = send(
        &srv,
        &owner,
        "put",
        &format!("/api/v1/stock-counts/{count_id}/items"),
        Some(json!({ "lines": [{ "item_id": s(line, "id"), "counted_qty": "3" }] })),
        Some(&mirpur),
    )
    .await;
    assert_eq!(entered.status, StatusCode::OK, "{}", entered.body);
    let posted =
        send(&srv, &owner, "post", &format!("/api/v1/stock-counts/{count_id}/post"), None, Some(&mirpur)).await;
    assert_eq!(posted.status, StatusCode::OK, "{}", posted.body);
    assert_eq!(posted.body["status"], "posted");
    assert_eq!(num(&posted.body, "variance_value"), 120.0);
    assert_eq!(stock_at(mirpur.clone()).await, 3.0);

    // --- other income, then its void
    let categories = get!("/api/v1/income-categories").await;
    let interest = s(find(&categories.body, "name", "Interest"), "id");
    let income = send(
        &srv,
        &owner,
        "post",
        "/api/v1/incomes",
        Some(json!({ "category_id": interest, "amount": "25", "source": "City Bank" })),
        None,
    )
    .await;
    assert_eq!(income.status, StatusCode::CREATED, "{}", income.body);
    let voided = send(
        &srv,
        &owner,
        "post",
        &format!("/api/v1/incomes/{}/void", s(&income.body, "id")),
        Some(json!({ "reason": "Entered twice" })),
        None,
    )
    .await;
    assert_eq!(voided.body["status"], "voided", "{}", voided.body);
    let kept =
        send(&srv, &owner, "post", "/api/v1/incomes", Some(json!({ "category_id": interest, "amount": "40" })), None)
            .await;
    assert_eq!(kept.status, StatusCode::CREATED, "{}", kept.body);
    let incomes = get!("/api/v1/incomes").await;
    assert_eq!(num(&incomes.body, "total_amount"), 40.0, "{}", incomes.body);

    // --- reports
    for uri in [
        "/api/v1/reports/aging?party=customers",
        "/api/v1/reports/aging?party=suppliers",
        "/api/v1/reports/vat-summary",
        "/api/v1/reports/discounts",
    ] {
        let res = get!(uri).await;
        assert_eq!(res.status, StatusCode::OK, "{uri}: {}", res.body);
    }
    let movement = get!("/api/v1/reports/product-movement").await;
    assert_eq!(movement.status, StatusCode::OK, "{}", movement.body);
    assert_eq!(find(&movement.body["rows"], "variant_id", &soap)["class"], "fast", "{}", movement.body);
    assert_eq!(num(find(&movement.body["rows"], "variant_id", &soap), "quantity_sold"), 2.0);

    // --- the books still balance (transit loss and count journals included)
    let tb = get!("/api/v1/reports/trial-balance").await;
    assert_eq!(tb.body["balanced"], true, "{}", tb.body);
    let bs = get!("/api/v1/reports/balance-sheet").await;
    assert_eq!(bs.body["balanced"], true, "{}", bs.body);
}
