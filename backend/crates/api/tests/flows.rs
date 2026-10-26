//! The buy → stock → sell → collect → return → expense cycle over HTTP
//! (Roadmap §17.4), with the acceptance checks AC-PUR-01/02, AC-POS-02…04,
//! idempotency (POS-R06), FIFO collection, returns, cancellation, and the
//! books: the trial balance and balance sheet balance and the P&L agrees
//! with the profit estimate. Chart-of-accounts edits (ACC-R01). Also
//! cross-tenant isolation of the new ids.
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

fn amount(v: &Value, key: &str) -> f64 {
    s(v, key).parse::<f64>().unwrap_or_else(|_| panic!("{key} is a number in {v}"))
}

fn find<'a>(list: &'a Value, key: &str, value: &str) -> &'a Value {
    list.as_array().into_iter().flatten().find(|v| v[key] == value).unwrap_or_else(|| panic!("{value} in {list}"))
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

async fn post<S, B>(srv: &S, who: &Session, uri: &str, body: Value) -> Res
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    call(srv, Req::post(uri, body).bearer(&who.token).build()).await
}

async fn post_key<S, B>(srv: &S, who: &Session, uri: &str, key: &str, body: Value) -> Res
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
        .insert_header(("idempotency-key", key.to_owned()))
        .to_request();
    call(srv, req).await
}

async fn stock_of<S, B>(srv: &S, who: &Session, variant_id: &str) -> (f64, f64)
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let res = get(srv, who, &format!("/api/v1/stock?variant_id={variant_id}")).await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.body);
    let row = &res.body["data"][0];
    (amount(row, "quantity"), amount(row, "avg_cost"))
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn trade_cycle_keeps_stock_money_parties_and_books_consistent(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool.clone(), common::config()))).await;
    let mut owner = sign_up(&srv, "Owner", "01711000901").await;
    create_business(&srv, &mut owner, "Rahim Store").await;

    // Defaults from provisioning.
    let methods = get(&srv, &owner, "/api/v1/payment-methods").await;
    let cash = s(find(&methods.body, "name", "Cash"), "id");
    let bkash = s(find(&methods.body, "name", "bKash"), "id");
    let accounts = get(&srv, &owner, "/api/v1/payment-accounts").await;
    let cash_account = s(find(&accounts.body, "name", "Cash"), "id");
    let walk_in = get(&srv, &owner, "/api/v1/customers/walk-in").await;
    assert_eq!(walk_in.body["is_walk_in"], true);

    let units = get(&srv, &owner, "/api/v1/units").await;
    let pcs = s(find(&units.body, "short_name", "pcs"), "id");
    let product = post(
        &srv,
        &owner,
        "/api/v1/products",
        json!({ "name": "Rice 5kg", "unit_id": pcs, "selling_price": "150", "purchase_price": "90" }),
    )
    .await;
    assert_eq!(product.status, StatusCode::CREATED, "{}", product.body);
    let rice = s(&product.body, "variant_id");

    // Opening stock 10 @ 90, then a purchase of 10 @ 100 + 50 shipping (AC-PUR-01).
    let opening = post(
        &srv,
        &owner,
        "/api/v1/stock-adjustments",
        json!({ "reason": "opening", "lines": [{ "variant_id": rice, "quantity": "10", "unit_cost": "90" }] }),
    )
    .await;
    assert_eq!(opening.status, StatusCode::CREATED, "{}", opening.body);
    assert_eq!(stock_of(&srv, &owner, &rice).await, (10.0, 90.0));

    let supplier =
        post(&srv, &owner, "/api/v1/suppliers", json!({ "name": "Karim Traders", "phone": "01811000001" })).await;
    assert_eq!(supplier.status, StatusCode::CREATED, "{}", supplier.body);
    let supplier_id = s(&supplier.body, "id");
    let draft = post(
        &srv,
        &owner,
        "/api/v1/purchases",
        json!({
            "supplier_id": supplier_id, "supplier_invoice_no": "KT-77", "shipping_cost": "50",
            "lines": [{ "variant_id": rice, "quantity": "10", "unit_cost": "100" }],
        }),
    )
    .await;
    assert_eq!(draft.status, StatusCode::CREATED, "{}", draft.body);
    assert_eq!(draft.body["status"], "draft");
    assert_eq!(draft.body["grand_total"], "1050.00");
    let purchase_id = s(&draft.body, "id");
    let posted = post(
        &srv,
        &owner,
        &format!("/api/v1/purchases/{purchase_id}/post"),
        json!({ "payment": { "method_id": cash, "amount": "400" } }),
    )
    .await;
    assert_eq!(posted.status, StatusCode::OK, "{}", posted.body);
    assert_eq!(posted.body["status"], "received");
    assert_eq!((amount(&posted.body, "paid_total"), amount(&posted.body, "due_total")), (400.0, 650.0));
    assert_eq!(stock_of(&srv, &owner, &rice).await, (20.0, 97.5), "AC-PUR-01");
    // AC-PUR-02: supplier balance = bill − paid.
    let supplier_now = get(&srv, &owner, &format!("/api/v1/suppliers/{supplier_id}")).await;
    assert_eq!(supplier_now.body["current_balance"], "650.00");
    // The same supplier bill number cannot be entered twice.
    let dup = post(
        &srv,
        &owner,
        "/api/v1/purchases",
        json!({ "supplier_id": supplier_id, "supplier_invoice_no": "KT-77",
                "lines": [{ "variant_id": rice, "quantity": "1", "unit_cost": "100" }] }),
    )
    .await;
    assert_eq!(dup.code(), "DUPLICATE_SUPPLIER_INVOICE", "{}", dup.body);

    // A walk-in sale paid in cash with change; replay with the same key (AC-POS-02).
    let key = Uuid::now_v7().to_string();
    let cash_sale = json!({ "lines": [{ "variant_id": rice, "quantity": "2" }],
                            "payments": [{ "method_id": cash, "amount": "500" }], "expected_total": "300" });
    let sale = post_key(&srv, &owner, "/api/v1/sales", &key, cash_sale.clone()).await;
    assert_eq!(sale.status, StatusCode::CREATED, "{}", sale.body);
    assert_eq!(
        (sale.body["grand_total"].as_str(), sale.body["change_given"].as_str()),
        (Some("300.00"), Some("200.00"))
    );
    assert_eq!(sale.body["payment_status"], "paid");
    let again = post_key(&srv, &owner, "/api/v1/sales", &key, cash_sale).await;
    assert_eq!(again.status, StatusCode::CREATED);
    assert_eq!(again.body["id"], sale.body["id"], "same sale returned");
    let reused = post_key(
        &srv,
        &owner,
        "/api/v1/sales",
        &key,
        json!({ "lines": [{ "variant_id": rice, "quantity": "3" }], "payments": [{ "method_id": cash, "amount": "450" }] }),
    )
    .await;
    assert_eq!(reused.code(), "IDEMPOTENCY_KEY_REUSED");
    assert_eq!(stock_of(&srv, &owner, &rice).await.0, 18.0, "one stock deduction");

    // AC-POS-04: a due needs a named customer; a key is required.
    let walk_in_due = post_key(
        &srv,
        &owner,
        "/api/v1/sales",
        &Uuid::now_v7().to_string(),
        json!({ "lines": [{ "variant_id": rice, "quantity": "1" }], "payments": [] }),
    )
    .await;
    assert_eq!(walk_in_due.code(), "DUE_REQUIRES_CUSTOMER");
    let no_key =
        post(&srv, &owner, "/api/v1/sales", json!({ "lines": [{ "variant_id": rice, "quantity": "1" }] })).await;
    assert_eq!(no_key.status, StatusCode::BAD_REQUEST);
    // A mismatching client total is refused (POS-R01).
    let mismatch = post_key(
        &srv,
        &owner,
        "/api/v1/sales",
        &Uuid::now_v7().to_string(),
        json!({ "lines": [{ "variant_id": rice, "quantity": "1" }], "payments": [{ "method_id": cash, "amount": "150" }],
                "expected_total": "149" }),
    )
    .await;
    assert_eq!(mismatch.code(), "TOTAL_MISMATCH");

    // AC-POS-03: a named customer leaves a due; split payment cash + bKash.
    let customer = post(&srv, &owner, "/api/v1/customers", json!({ "name": "Jamal", "phone": "01911000001" })).await;
    assert_eq!(customer.status, StatusCode::CREATED, "{}", customer.body);
    let customer_id = s(&customer.body, "id");
    let credit_sale = post_key(
        &srv,
        &owner,
        "/api/v1/sales",
        &Uuid::now_v7().to_string(),
        json!({ "customer_id": customer_id, "lines": [{ "variant_id": rice, "quantity": "4" }],
                "payments": [{ "method_id": cash, "amount": "200" }, { "method_id": bkash, "amount": "100", "reference": "TRX1" }] }),
    )
    .await;
    assert_eq!(credit_sale.status, StatusCode::CREATED, "{}", credit_sale.body);
    assert_eq!(credit_sale.body["grand_total"], "600.00");
    assert_eq!(
        (credit_sale.body["paid_total"].as_str(), credit_sale.body["due_total"].as_str()),
        (Some("300.00"), Some("300.00"))
    );
    assert_eq!(credit_sale.body["payment_status"], "partial");
    let credit_sale_id = s(&credit_sale.body, "id");
    let jamal = get(&srv, &owner, &format!("/api/v1/customers/{customer_id}")).await;
    assert_eq!(jamal.body["current_balance"], "300.00");
    // bKash needs its transaction id.
    let no_ref = post_key(
        &srv,
        &owner,
        "/api/v1/sales",
        &Uuid::now_v7().to_string(),
        json!({ "lines": [{ "variant_id": rice, "quantity": "1" }], "payments": [{ "method_id": bkash, "amount": "150" }] }),
    )
    .await;
    assert_eq!(no_ref.body["errors"][0]["code"], "REFERENCE_REQUIRED", "{}", no_ref.body);

    // Collection pays the invoice (FIFO) and leaves an advance.
    let collected = post(
        &srv,
        &owner,
        "/api/v1/collections",
        json!({ "customer_id": customer_id, "amount": "350", "method_id": cash }),
    )
    .await;
    assert_eq!(collected.status, StatusCode::CREATED, "{}", collected.body);
    assert_eq!(collected.body["unallocated_amount"], "50.00");
    let paid_sale = get(&srv, &owner, &format!("/api/v1/sales/{credit_sale_id}")).await;
    assert_eq!(paid_sale.body["payment_status"], "paid");
    let jamal = get(&srv, &owner, &format!("/api/v1/customers/{customer_id}")).await;
    assert_eq!(jamal.body["current_balance"], "-50.00", "advance");

    // Return 1 of the 4 as customer credit; stock comes back at the sale cost.
    let item_id = s(&paid_sale.body["items"][0], "id");
    let returned = post_key(
        &srv,
        &owner,
        "/api/v1/sale-returns",
        &Uuid::now_v7().to_string(),
        json!({ "sale_id": credit_sale_id, "reason": "Torn bag", "refund_mode": "customer_credit",
                "lines": [{ "sale_item_id": item_id, "quantity": "1" }] }),
    )
    .await;
    assert_eq!(returned.status, StatusCode::CREATED, "{}", returned.body);
    assert_eq!(
        (returned.body["total"].as_str(), returned.body["credited_amount"].as_str()),
        (Some("150.00"), Some("150.00"))
    );
    let too_many = post_key(
        &srv,
        &owner,
        "/api/v1/sale-returns",
        &Uuid::now_v7().to_string(),
        json!({ "sale_id": credit_sale_id, "reason": "Again", "refund_mode": "cash",
                "lines": [{ "sale_item_id": item_id, "quantity": "4" }] }),
    )
    .await;
    assert_eq!(too_many.code(), "RETURN_QTY_EXCEEDED");
    assert_eq!(stock_of(&srv, &owner, &rice).await.0, 15.0);
    let jamal = get(&srv, &owner, &format!("/api/v1/customers/{customer_id}")).await;
    assert_eq!(jamal.body["current_balance"], "-200.00");
    let statement = get(&srv, &owner, &format!("/api/v1/customers/{customer_id}/statement")).await;
    assert_eq!(statement.body["closing_balance"], "-200.00", "{}", statement.body);

    // Same-day cancellation puts the stock back and voids the money.
    let to_cancel = post_key(
        &srv,
        &owner,
        "/api/v1/sales",
        &Uuid::now_v7().to_string(),
        json!({ "lines": [{ "variant_id": rice, "quantity": "1" }], "payments": [{ "method_id": cash, "amount": "150" }] }),
    )
    .await;
    assert_eq!(to_cancel.status, StatusCode::CREATED, "{}", to_cancel.body);
    let cancelled = post(
        &srv,
        &owner,
        &format!("/api/v1/sales/{}/cancel", s(&to_cancel.body, "id")),
        json!({ "reason": "Customer changed their mind" }),
    )
    .await;
    assert_eq!(cancelled.status, StatusCode::OK, "{}", cancelled.body);
    assert_eq!(cancelled.body["status"], "cancelled");
    assert_eq!(stock_of(&srv, &owner, &rice).await.0, 15.0);

    // Expense and a manual check of the cash book.
    let categories = get(&srv, &owner, "/api/v1/expense-categories").await;
    let rent = s(find(&categories.body, "name", "Rent"), "id");
    let expense =
        post(&srv, &owner, "/api/v1/expenses", json!({ "category_id": rent, "amount": "120", "payee": "Landlord" }))
            .await;
    assert_eq!(expense.status, StatusCode::CREATED, "{}", expense.body);
    // Cash: −400 (supplier) +300 (sale) +200 (credit sale) +350 (collection) +150 −150 (cancelled) −120 (rent).
    let book = get(&srv, &owner, &format!("/api/v1/payment-accounts/{cash_account}/book")).await;
    assert_eq!(book.body["closing_balance"], "330.00", "{}", book.body);
    assert_eq!(book.body["account"]["current_balance"], "330.00");

    // The books balance and agree with the documents.
    let tb = get(&srv, &owner, "/api/v1/reports/trial-balance").await;
    assert_eq!(tb.status, StatusCode::OK, "{}", tb.body);
    assert_eq!(tb.body["balanced"], true, "{}", tb.body);
    let bs = get(&srv, &owner, "/api/v1/reports/balance-sheet").await;
    assert_eq!(bs.body["balanced"], true, "{}", bs.body);
    let pl = get(&srv, &owner, "/api/v1/reports/profit-and-loss").await;
    let estimate = get(&srv, &owner, "/api/v1/reports/profit").await;
    assert_eq!(estimate.status, StatusCode::OK, "{}", estimate.body);
    // Net sales: 300 + 600 − 150 (return) = 750; COGS: 5 sold × 97.5 = 487.50; rent 120.
    assert_eq!(estimate.body["net_sales"], "750.00", "{}", estimate.body);
    assert_eq!(estimate.body["cogs"], "487.50");
    assert_eq!(estimate.body["net_profit"], "142.50");
    assert_eq!(pl.body["net_profit"], estimate.body["net_profit"], "{}", pl.body);
    let inventory_line =
        bs.body["assets"]["lines"].as_array().into_iter().flatten().find(|l| l["code"] == "1200").cloned();
    // 15 × 97.5 = 1462.50 in stock and in the ledger.
    assert_eq!(inventory_line.map(|l| l["amount"].clone()), Some(json!("1462.50")), "{}", bs.body);
    let summary = get(&srv, &owner, "/api/v1/reports/sales-summary").await;
    assert_eq!(summary.body["totals"]["net_sales"], "750.00", "{}", summary.body);
    let dashboard = get(&srv, &owner, "/api/v1/dashboard").await;
    assert_eq!(dashboard.status, StatusCode::OK, "{}", dashboard.body);
    assert_eq!(dashboard.body["payables"], "650.00");

    // Chart of accounts (ACC-R01): own groups and accounts; system accounts
    // can only be renamed; accounts in use keep their shape.
    let ledger = get(&srv, &owner, "/api/v1/ledger-accounts").await;
    let cash_ledger = find(&ledger.body, "code", "1010").clone();
    let rent_ledger = find(&ledger.body, "code", "6010").clone();
    assert_eq!(rent_ledger["has_entries"], true, "{rent_ledger}");
    let group = post(
        &srv,
        &owner,
        "/api/v1/ledger-accounts",
        json!({ "code": "7000", "name": "Other Expenses", "account_type": "expense", "is_postable": false }),
    )
    .await;
    assert_eq!(group.status, StatusCode::CREATED, "{}", group.body);
    assert_eq!(group.body["normal_side"], "debit");
    let group_id = s(&group.body, "id");
    let donations = post(
        &srv,
        &owner,
        "/api/v1/ledger-accounts",
        json!({ "code": "7010", "name": "Donations", "parent_id": group_id }),
    )
    .await;
    assert_eq!(donations.status, StatusCode::CREATED, "{}", donations.body);
    assert_eq!(
        (donations.body["account_type"].as_str(), donations.body["is_postable"].as_bool()),
        (Some("expense"), Some(true))
    );
    for (body, field, code) in [
        (json!({ "code": "7010", "name": "Again", "parent_id": group_id }), "code", "DUPLICATE_CODE"),
        (
            json!({ "code": "7020", "name": "Under cash", "parent_id": s(&cash_ledger, "id") }),
            "parent_id",
            "PARENT_NOT_GROUP",
        ),
        (
            json!({ "code": "7030", "name": "Wrong type", "parent_id": group_id, "account_type": "income" }),
            "account_type",
            "TYPE_MISMATCH",
        ),
        (json!({ "code": "7040", "name": "No type" }), "account_type", "REQUIRED"),
    ] {
        let res = post(&srv, &owner, "/api/v1/ledger-accounts", body).await;
        assert_eq!(res.status, StatusCode::BAD_REQUEST, "{}", res.body);
        let errors = res.body["errors"].as_array().cloned().unwrap_or_default();
        assert!(errors.iter().any(|e| e["field"] == field && e["code"] == code), "{field} {code}: {}", res.body);
    }
    let put = |uri: String, version: i64, body: Value| {
        call(&srv, Req::put(&uri, body).bearer(&owner.token).if_match(version).build())
    };
    let cash_uri = format!("/api/v1/ledger-accounts/{}", s(&cash_ledger, "id"));
    let renamed = put(cash_uri.clone(), 1, json!({ "code": "1010", "name": "Cash Box" })).await;
    assert_eq!(renamed.status, StatusCode::OK, "{}", renamed.body);
    assert_eq!((renamed.body["name"].as_str(), renamed.body["version"].as_i64()), (Some("Cash Box"), Some(2)));
    let stale = put(cash_uri.clone(), 1, json!({ "code": "1010", "name": "Till" })).await;
    assert_eq!(stale.status, StatusCode::PRECONDITION_FAILED, "{}", stale.body);
    let recoded = put(cash_uri, 2, json!({ "code": "1011", "name": "Cash Box" })).await;
    assert_eq!(recoded.status, StatusCode::BAD_REQUEST, "{}", recoded.body);
    let rent_uri = format!("/api/v1/ledger-accounts/{}", s(&rent_ledger, "id"));
    let regroup = put(rent_uri.clone(), 1, json!({ "code": "6010", "name": "Rent", "is_postable": false })).await;
    assert_eq!(regroup.status, StatusCode::BAD_REQUEST, "{}", regroup.body);
    let linked = put(rent_uri, 1, json!({ "code": "6010", "name": "Rent", "status": "inactive" })).await;
    assert_eq!(linked.body["code"], "ACCOUNT_LINKED", "{}", linked.body);
    let retired = put(
        format!("/api/v1/ledger-accounts/{}", s(&donations.body, "id")),
        1,
        json!({ "code": "7011", "name": "Charity", "parent_id": group_id, "status": "inactive" }),
    )
    .await;
    assert_eq!(retired.status, StatusCode::OK, "{}", retired.body);
    assert_eq!((retired.body["code"].as_str(), retired.body["status"].as_str()), (Some("7011"), Some("inactive")));
    let to_retired = post(
        &srv,
        &owner,
        "/api/v1/journals",
        json!({ "memo": "Gift", "lines": [{ "account_id": s(&donations.body, "id"), "debit": "10" }, { "account_id": s(&cash_ledger, "id"), "credit": "10" }] }),
    )
    .await;
    assert_eq!(to_retired.status, StatusCode::BAD_REQUEST, "{}", to_retired.body);

    // Another business cannot see any of it.
    let mut other = sign_up(&srv, "Other", "01711000902").await;
    create_business(&srv, &mut other, "Other Store").await;
    for uri in [
        format!("/api/v1/sales/{credit_sale_id}"),
        format!("/api/v1/purchases/{purchase_id}"),
        format!("/api/v1/customers/{customer_id}"),
        format!("/api/v1/suppliers/{supplier_id}"),
        format!("/api/v1/payment-accounts/{cash_account}"),
        format!("/api/v1/sale-returns/{}", s(&returned.body, "id")),
        format!("/api/v1/expenses/{}", s(&expense.body, "id")),
        format!("/api/v1/payments/{}", s(&collected.body, "id")),
    ] {
        let res = get(&srv, &other, &uri).await;
        assert_eq!(res.status, StatusCode::NOT_FOUND, "{uri}: {}", res.body);
    }
    let theirs = get(&srv, &other, "/api/v1/stock").await;
    assert_eq!(theirs.body["data"].as_array().map(Vec::len), Some(0));
}
