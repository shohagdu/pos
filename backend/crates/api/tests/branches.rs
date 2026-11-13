//! Phase 6 over HTTP: a second branch with its own locations, numbering and
//! cash drawer; `X-Branch-Id` scoping of postings and lists; a transfer with
//! a transit loss; a stock count; a register shift with X/Z figures; other
//! income and its void; the new reports; and the books still balance.
//!
//! One test function: every `#[sqlx::test]` migrates a fresh database.

mod common;

use actix_web::{http::StatusCode, test};
use common::{Req, Res, Session, call, create_business, role_id, sign_up, switch};
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

/// Sets the business currency (`PATCH /settings` with the current version).
async fn set_currency<S, B>(srv: &S, who: &Session, code: &str) -> Res
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let settings = send(srv, who, "get", "/api/v1/settings", None, None).await;
    let version = settings.body["version"].as_i64().expect("settings version");
    let req = test::TestRequest::patch()
        .uri("/api/v1/settings")
        .insert_header(("authorization", format!("Bearer {}", who.token)))
        .insert_header(("if-match", format!("\"{version}\"")))
        .set_json(json!({ "currency_code": code }))
        .to_request();
    call(srv, req).await
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn branches_transfers_counts_shifts_and_income(pool: MySqlPool) {
    // Room for a second branch, transfers and registers on the trial plan.
    sqlx::query(
        "UPDATE plan_features pf JOIN plans p ON p.id = pf.plan_id \
         SET pf.enabled = TRUE, pf.limit_value = IF(pf.feature_code IN ('branches.max', 'registers.max'), 3, pf.limit_value) \
         WHERE p.code = 'trial' AND pf.feature_code IN ('branches.max', 'registers.max', 'inventory.transfers', 'notifications.sms')",
    )
    .execute(&pool)
    .await
    .expect("plan limits");
    let srv = test::init_service(app(AppState::new(pool.clone(), common::config()))).await;
    let mut owner = sign_up(&srv, "Owner", "01711000951").await;
    let tenant_id = create_business(&srv, &mut owner, "Karim Traders").await;
    // A new business may pick its currency; it is reported with the session.
    let euro = set_currency(&srv, &owner, "EUR").await;
    assert_eq!(euro.body["currency_code"], "EUR", "{}", euro.body);
    let me = send(&srv, &owner, "get", "/api/v1/auth/me", None, None).await;
    assert_eq!(me.body["tenant"]["currency_code"], "EUR", "{}", me.body);
    let unknown = set_currency(&srv, &owner, "XYZ").await;
    assert_eq!(unknown.status, StatusCode::BAD_REQUEST, "{}", unknown.body);
    assert_eq!(set_currency(&srv, &owner, "BDT").await.body["currency_code"], "BDT");
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

    // --- a manager limited to the main branch cannot reach Mirpur by id
    // (BRN-R02): not its sale, not its stock location, and not by dropping
    // the branch header either.
    let manager_role = role_id(&srv, &owner, "branch_manager").await;
    let added = call(
        &srv,
        Req::post(
            "/api/v1/members",
            json!({ "full_name": "Main Manager", "phone": "01711000981", "password": "Shop-pass-2026",
                    "role_ids": [&manager_role], "branch_ids": [&main] }),
        )
        .bearer(&owner.token)
        .build(),
    )
    .await;
    assert_eq!(added.status, StatusCode::CREATED, "{}", added.body);
    let login = call(
        &srv,
        Req::post("/api/v1/auth/login", json!({ "identifier": "01711000981", "password": "Shop-pass-2026" })).build(),
    )
    .await;
    let mut manager = Session { token: login.token(), cookie: login.cookie(), phone: "01711000981".into() };
    switch(&srv, &mut manager, Some(&tenant_id)).await;
    let mir_sale = s(&sale.body, "id");
    let peek = send(&srv, &manager, "get", &format!("/api/v1/sales/{mir_sale}"), None, None).await;
    assert_eq!(peek.code(), "BRANCH_NOT_ALLOWED", "{}", peek.body);
    let cancel =
        send(&srv, &manager, "post", &format!("/api/v1/sales/{mir_sale}/cancel"), Some(json!({ "reason": "x" })), None)
            .await;
    assert_eq!(cancel.code(), "BRANCH_NOT_ALLOWED", "{}", cancel.body);
    let adjust = send(
        &srv,
        &manager,
        "post",
        "/api/v1/stock-adjustments",
        Some(json!({ "reason": "damaged", "location_id": mir_main, "lines": [{ "variant_id": soap, "quantity": "-1" }] })),
        None,
    )
    .await;
    assert_eq!(adjust.code(), "BRANCH_NOT_ALLOWED", "{}", adjust.body);
    let own = send(&srv, &manager, "get", "/api/v1/sales", None, None).await;
    assert_eq!(own.status, StatusCode::OK, "{}", own.body);

    // --- the minimum price holds after discounts too: the manager may give
    // discounts but not sell below the minimum (no pos.below_min_price).
    let bulb = send(
        &srv,
        &owner,
        "post",
        "/api/v1/products",
        Some(json!({ "name": "Bulb", "unit_id": pcs, "selling_price": "100", "purchase_price": "60",
                     "min_selling_price": "80" })),
        None,
    )
    .await;
    assert_eq!(bulb.status, StatusCode::CREATED, "{}", bulb.body);
    let bulb = s(&bulb.body, "variant_id");
    let bulb_stock = send(
        &srv,
        &owner,
        "post",
        "/api/v1/stock-adjustments",
        Some(json!({ "reason": "opening", "lines": [{ "variant_id": bulb, "quantity": "5", "unit_cost": "60" }] })),
        Some(&main),
    )
    .await;
    assert_eq!(bulb_stock.status, StatusCode::CREATED, "{}", bulb_stock.body);
    let sell_bulb = |extra: Value| {
        let mut body = json!({ "lines": [{ "variant_id": bulb, "quantity": "1" }],
                               "payments": [{ "method_id": cash, "amount": "100" }] });
        for (k, v) in extra.as_object().expect("object") {
            if k == "line" {
                for (lk, lv) in v.as_object().expect("line") {
                    body["lines"][0][lk] = lv.clone();
                }
            } else {
                body[k] = v.clone();
            }
        }
        body
    };
    let line_disc = send(
        &srv,
        &manager,
        "post",
        "/api/v1/sales",
        Some(sell_bulb(json!({ "line": { "discount_type": "percent", "discount_value": "30" } }))),
        None,
    )
    .await;
    assert_eq!(line_disc.code(), "BELOW_MIN_PRICE", "{}", line_disc.body);
    let invoice_disc = send(
        &srv,
        &manager,
        "post",
        "/api/v1/sales",
        Some(sell_bulb(json!({ "invoice_discount_type": "amount", "invoice_discount_value": "25" }))),
        None,
    )
    .await;
    assert_eq!(invoice_disc.code(), "BELOW_MIN_PRICE", "{}", invoice_disc.body);
    let within = send(
        &srv,
        &manager,
        "post",
        "/api/v1/sales",
        Some(sell_bulb(json!({ "line": { "discount_type": "amount", "discount_value": "20" } }))),
        None,
    )
    .await;
    assert_eq!(within.status, StatusCode::CREATED, "{}", within.body);
    // The owner may sell below the minimum.
    let owner_below = send(
        &srv,
        &owner,
        "post",
        "/api/v1/sales",
        Some(sell_bulb(json!({ "line": { "discount_type": "percent", "discount_value": "30" } }))),
        Some(&main),
    )
    .await;
    assert_eq!(owner_below.status, StatusCode::CREATED, "{}", owner_below.body);

    // --- returns to the supplier stay in the member's branches: a buyer
    // limited to the main branch cannot return a Mirpur bill.
    let supplier = send(&srv, &owner, "post", "/api/v1/suppliers", Some(json!({ "name": "Mirpur Wholesale" })), None)
        .await;
    assert_eq!(supplier.status, StatusCode::CREATED, "{}", supplier.body);
    let mir_bill = send(
        &srv,
        &owner,
        "post",
        "/api/v1/purchases",
        Some(json!({ "supplier_id": s(&supplier.body, "id"),
                     "lines": [{ "variant_id": bulb, "quantity": "2", "unit_cost": "60" }] })),
        Some(&mirpur),
    )
    .await;
    assert_eq!(mir_bill.status, StatusCode::CREATED, "{}", mir_bill.body);
    let mir_bill =
        send(&srv, &owner, "post", &format!("/api/v1/purchases/{}/post", s(&mir_bill.body, "id")), None, Some(&mirpur))
            .await;
    assert_eq!(mir_bill.body["status"], "received", "{}", mir_bill.body);
    let buyer_role = call(
        &srv,
        Req::post("/api/v1/roles", json!({ "name": "Buyer", "permissions": ["purchases.view", "purchases.return"] }))
            .bearer(&owner.token)
            .build(),
    )
    .await;
    assert_eq!(buyer_role.status, StatusCode::CREATED, "{}", buyer_role.body);
    let buyer_added = call(
        &srv,
        Req::post(
            "/api/v1/members",
            json!({ "full_name": "Main Buyer", "phone": "01711000985", "password": "Shop-pass-2026",
                    "role_ids": [s(&buyer_role.body, "id")], "branch_ids": [&main] }),
        )
        .bearer(&owner.token)
        .build(),
    )
    .await;
    assert_eq!(buyer_added.status, StatusCode::CREATED, "{}", buyer_added.body);
    let buyer_login = call(
        &srv,
        Req::post("/api/v1/auth/login", json!({ "identifier": "01711000985", "password": "Shop-pass-2026" })).build(),
    )
    .await;
    let mut buyer = Session { token: buyer_login.token(), cookie: buyer_login.cookie(), phone: "01711000985".into() };
    switch(&srv, &mut buyer, Some(&tenant_id)).await;
    let foreign_return = send(
        &srv,
        &buyer,
        "post",
        "/api/v1/purchase-returns",
        Some(json!({ "purchase_id": s(&mir_bill.body, "id"), "reason": "Damaged",
                     "lines": [{ "purchase_item_id": s(&mir_bill.body["items"][0], "id"), "quantity": "1" }] })),
        None,
    )
    .await;
    assert_eq!(foreign_return.code(), "BRANCH_NOT_ALLOWED", "{}", foreign_return.body);
    // The owner, whatever branch is selected, still opens it.
    let owner_view = send(&srv, &owner, "get", &format!("/api/v1/sales/{mir_sale}"), None, Some(&main)).await;
    assert_eq!(owner_view.status, StatusCode::OK, "{}", owner_view.body);

    // --- no branch escalation: someone who may add staff only in the main
    // branch cannot hand out the same role for all branches or for Mirpur.
    let hr_role = call(
        &srv,
        Req::post(
            "/api/v1/roles",
            json!({ "name": "Branch HR", "permissions": ["users.view", "users.invite", "users.manage_roles", "sales.view"] }),
        )
        .bearer(&owner.token)
        .build(),
    )
    .await;
    assert_eq!(hr_role.status, StatusCode::CREATED, "{}", hr_role.body);
    let hr_role = s(&hr_role.body, "id");
    let hr_added = call(
        &srv,
        Req::post(
            "/api/v1/members",
            json!({ "full_name": "Main HR", "phone": "01711000982", "password": "Shop-pass-2026",
                    "role_ids": [&hr_role], "branch_ids": [&main] }),
        )
        .bearer(&owner.token)
        .build(),
    )
    .await;
    assert_eq!(hr_added.status, StatusCode::CREATED, "{}", hr_added.body);
    let hr_login = call(
        &srv,
        Req::post("/api/v1/auth/login", json!({ "identifier": "01711000982", "password": "Shop-pass-2026" })).build(),
    )
    .await;
    let mut hr = Session { token: hr_login.token(), cookie: hr_login.cookie(), phone: "01711000982".into() };
    switch(&srv, &mut hr, Some(&tenant_id)).await;
    let grant = |phone: &str, branches: serde_json::Value| {
        Req::post(
            "/api/v1/members",
            json!({ "full_name": "New Staff", "phone": phone, "password": "Shop-pass-2026",
                    "role_ids": [&hr_role], "branch_ids": branches }),
        )
        .bearer(&hr.token)
        .build()
    };
    let everywhere = call(&srv, grant("01711000983", json!([]))).await;
    assert_eq!(
        (everywhere.status, everywhere.code()),
        (StatusCode::FORBIDDEN, "BRANCH_ESCALATION"),
        "{}",
        everywhere.body
    );
    let other_branch = call(&srv, grant("01711000983", json!([&mirpur]))).await;
    assert_eq!(other_branch.code(), "BRANCH_ESCALATION", "{}", other_branch.body);
    let own_branch = call(&srv, grant("01711000983", json!([&main]))).await;
    assert_eq!(own_branch.status, StatusCode::CREATED, "{}", own_branch.body);
    // Nor widen a role that someone holds in every branch: the new
    // permission would reach branches outside the editor's own.
    let floor = call(
        &srv,
        Req::post("/api/v1/roles", json!({ "name": "Floor", "permissions": ["sales.view"] })).bearer(&owner.token).build(),
    )
    .await;
    assert_eq!(floor.status, StatusCode::CREATED, "{}", floor.body);
    let (floor_id, floor_version) = (s(&floor.body, "id"), floor.body["version"].as_i64().expect("version"));
    let floor_staff = call(
        &srv,
        Req::post(
            "/api/v1/members",
            json!({ "full_name": "Floor Staff", "phone": "01711000984", "password": "Shop-pass-2026",
                    "role_ids": [&floor_id], "branch_ids": [] }),
        )
        .bearer(&owner.token)
        .build(),
    )
    .await;
    assert_eq!(floor_staff.status, StatusCode::CREATED, "{}", floor_staff.body);
    let widen = |permissions: serde_json::Value| {
        Req::put(&format!("/api/v1/roles/{floor_id}"), json!({ "name": "Floor", "permissions": permissions }))
            .bearer(&hr.token)
            .if_match(floor_version)
            .build()
    };
    let widened = call(&srv, widen(json!(["sales.view", "users.view"]))).await;
    assert_eq!(
        (widened.status, widened.code()),
        (StatusCode::FORBIDDEN, "BRANCH_ESCALATION"),
        "{}",
        widened.body
    );
    // ...or suspend a member who works in other branches.
    let floor_member = s(&floor_staff.body["member"], "id");
    let suspended = call(
        &srv,
        Req::post(&format!("/api/v1/members/{floor_member}/suspend"), json!({})).bearer(&hr.token).build(),
    )
    .await;
    assert_eq!(suspended.code(), "BRANCH_ESCALATION", "{}", suspended.body);
    let own_member = s(&own_branch.body["member"], "id");
    let own_suspended = call(
        &srv,
        Req::post(&format!("/api/v1/members/{own_member}/suspend"), json!({})).bearer(&hr.token).build(),
    )
    .await;
    assert_eq!(own_suspended.status, StatusCode::OK, "{}", own_suspended.body);

    // --- per-branch permissions: "Floor everywhere" + "Branch HR in main".
    // The HR permissions work in main only, not in Mirpur and not across the
    // whole business (no branch selected).
    let rewired = call(
        &srv,
        Req::put(
            &format!("/api/v1/members/{floor_member}/roles"),
            json!({ "assignments": [{ "role_id": &floor_id, "branch_id": null }, { "role_id": &hr_role, "branch_id": &main }] }),
        )
        .bearer(&owner.token)
        .build(),
    )
    .await;
    assert_eq!(rewired.status, StatusCode::OK, "{}", rewired.body);
    let floor_login = call(
        &srv,
        Req::post("/api/v1/auth/login", json!({ "identifier": "01711000984", "password": "Shop-pass-2026" })).build(),
    )
    .await;
    let mut floor_user = Session { token: floor_login.token(), cookie: floor_login.cookie(), phone: "01711000984".into() };
    switch(&srv, &mut floor_user, Some(&tenant_id)).await;
    let in_main = send(&srv, &floor_user, "get", "/api/v1/members", None, Some(&main)).await;
    assert_eq!(in_main.status, StatusCode::OK, "{}", in_main.body);
    let in_mirpur = send(&srv, &floor_user, "get", "/api/v1/members", None, Some(&mirpur)).await;
    assert_eq!(in_mirpur.status, StatusCode::FORBIDDEN, "{}", in_mirpur.body);
    let business_wide = send(&srv, &floor_user, "get", "/api/v1/members", None, None).await;
    assert_eq!(business_wide.status, StatusCode::FORBIDDEN, "{}", business_wide.body);
    // The everywhere role still works in Mirpur.
    let sales_in_mirpur = send(&srv, &floor_user, "get", "/api/v1/sales", None, Some(&mirpur)).await;
    assert_eq!(sales_in_mirpur.status, StatusCode::OK, "{}", sales_in_mirpur.body);

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

    // --- a purchase order: 10 ordered (+20 shipping), approved, 4 then 2 arrive, closed short
    let supplier =
        send(&srv, &owner, "post", "/api/v1/suppliers", Some(json!({ "name": "Rahman Distributors" })), None).await;
    assert_eq!(supplier.status, StatusCode::CREATED, "{}", supplier.body);
    let supplier_id = s(&supplier.body, "id");
    let draft = send(
        &srv,
        &owner,
        "post",
        "/api/v1/purchases",
        Some(json!({ "supplier_id": supplier_id, "shipping_cost": "20",
                     "lines": [{ "variant_id": soap, "quantity": "10", "unit_cost": "50" }] })),
        None,
    )
    .await;
    assert_eq!(draft.status, StatusCode::CREATED, "{}", draft.body);
    let po = s(&draft.body, "id");
    let po_item = s(&draft.body["items"][0], "id");
    let po_uri = |action: &str| format!("/api/v1/purchases/{po}/{action}");
    let ordered = send(&srv, &owner, "post", &po_uri("order"), None, None).await;
    assert_eq!(ordered.body["status"], "ordered", "{}", ordered.body);
    assert!(ordered.body["purchase_no"].is_string());
    assert_eq!(num(&ordered.body, "due_total"), 0.0);
    let early = send(&srv, &owner, "post", &po_uri("receive"), None, None).await;
    assert_eq!(early.code(), "PURCHASE_NOT_APPROVED", "{}", early.body);
    let approved = send(&srv, &owner, "post", &po_uri("approve"), None, None).await;
    assert_eq!(approved.body["status"], "approved", "{}", approved.body);
    let first = send(
        &srv,
        &owner,
        "post",
        &po_uri("receive"),
        Some(json!({ "lines": [{ "purchase_item_id": po_item, "quantity": "4" }], "note": "First lorry",
                     "payment": { "method_id": cash, "amount": "100" } })),
        None,
    )
    .await;
    assert_eq!(first.body["status"], "partially_received", "{}", first.body);
    assert_eq!(num(&first.body, "received_total"), 208.0, "{}", first.body);
    assert_eq!(num(&first.body, "due_total"), 108.0);
    assert_eq!(first.body["payment_status"], "partial");
    let cancel = send(&srv, &owner, "post", &po_uri("cancel"), Some(json!({ "reason": "x" })), None).await;
    assert_eq!(cancel.code(), "PURCHASE_NOT_CANCELLABLE", "{}", cancel.body);
    let too_many = send(
        &srv,
        &owner,
        "post",
        &po_uri("receive"),
        Some(json!({ "lines": [{ "purchase_item_id": po_item, "quantity": "7" }] })),
        None,
    )
    .await;
    assert!(!too_many.status.is_success(), "{}", too_many.body);
    let second = send(
        &srv,
        &owner,
        "post",
        &po_uri("receive"),
        Some(json!({ "lines": [{ "purchase_item_id": po_item, "quantity": "2" }] })),
        None,
    )
    .await;
    assert_eq!(second.body["status"], "partially_received", "{}", second.body);
    assert_eq!(num(&second.body, "received_total"), 312.0);
    assert_eq!(second.body["receipts"].as_array().map(Vec::len), Some(2));
    assert_eq!(num(&second.body["items"][0], "received_quantity"), 6.0);
    let closed =
        send(&srv, &owner, "post", &po_uri("close"), Some(json!({ "reason": "Supplier out of stock" })), None).await;
    assert_eq!(closed.body["status"], "received", "{}", closed.body);
    assert!(closed.body["closed_at"].is_string());
    assert_eq!(num(&closed.body, "grand_total"), 520.0);
    assert_eq!(num(&closed.body, "due_total"), 212.0);
    let owed = get!(&format!("/api/v1/suppliers/{supplier_id}")).await;
    assert_eq!(num(&owed.body, "current_balance"), 212.0, "{}", owed.body);
    // A one-step bill still receives everything at once.
    let bill = send(
        &srv,
        &owner,
        "post",
        "/api/v1/purchases",
        Some(json!({ "supplier_id": supplier_id, "lines": [{ "variant_id": soap, "quantity": "1", "unit_cost": "50" }] })),
        None,
    )
    .await;
    let bill = send(&srv, &owner, "post", &format!("/api/v1/purchases/{}/post", s(&bill.body, "id")), None, None).await;
    assert_eq!(bill.body["status"], "received", "{}", bill.body);
    assert_eq!(num(&bill.body, "received_total"), 50.0);
    assert_eq!(num(&bill.body, "due_total"), 50.0);

    // --- bank reconciliation: 100 moved to the bank, a 15 charge; the statement says 85
    let accounts = get!("/api/v1/payment-accounts").await;
    let bank = s(find(&accounts.body, "name", "Bank Account"), "id");
    let main_cash = s(find(&accounts.body, "name", "Cash"), "id");
    let moved = send(
        &srv,
        &owner,
        "post",
        "/api/v1/account-transfers",
        Some(json!({ "from_account_id": main_cash, "to_account_id": bank, "amount": "100" })),
        None,
    )
    .await;
    assert_eq!(moved.status, StatusCode::CREATED, "{}", moved.body);
    let expense_categories = get!("/api/v1/expense-categories").await;
    let charges = s(find(&expense_categories.body, "name", "Bank & MFS Charges"), "id");
    let fee = send(
        &srv,
        &owner,
        "post",
        "/api/v1/expenses",
        Some(json!({ "category_id": charges, "amount": "15", "account_id": bank })),
        None,
    )
    .await;
    assert_eq!(fee.status, StatusCode::CREATED, "{}", fee.body);
    let today = moved.body["business_date"].as_str().expect("business date").to_owned();
    let cash_rec = send(
        &srv,
        &owner,
        "post",
        "/api/v1/bank-reconciliations",
        Some(json!({ "account_id": main_cash, "statement_date": today, "statement_balance": "0" })),
        None,
    )
    .await;
    assert_eq!(cash_rec.status, StatusCode::BAD_REQUEST, "{}", cash_rec.body);
    let rec = send(
        &srv,
        &owner,
        "post",
        "/api/v1/bank-reconciliations",
        Some(json!({ "account_id": bank, "statement_date": today, "statement_balance": "85" })),
        None,
    )
    .await;
    assert_eq!(rec.status, StatusCode::CREATED, "{}", rec.body);
    assert_eq!(num(&rec.body, "difference"), 85.0, "{}", rec.body);
    let lines: Vec<String> = rec.body["lines"].as_array().expect("lines").iter().map(|l| s(l, "id")).collect();
    assert_eq!(lines.len(), 2, "{}", rec.body);
    let rec_uri = |action: &str| format!("/api/v1/bank-reconciliations/{}/{action}", s(&rec.body, "id"));
    let again = send(
        &srv,
        &owner,
        "post",
        "/api/v1/bank-reconciliations",
        Some(json!({ "account_id": bank, "statement_date": today, "statement_balance": "85" })),
        None,
    )
    .await;
    assert_eq!(again.code(), "RECONCILIATION_OPEN", "{}", again.body);
    let early = send(&srv, &owner, "post", &rec_uri("complete"), None, None).await;
    assert_eq!(early.code(), "RECONCILIATION_DIFFERENCE", "{}", early.body);
    let half = send(&srv, &owner, "put", &rec_uri("lines"), Some(json!({ "clear": [lines[0]] })), None).await;
    assert_eq!(num(&half.body, "difference").abs(), 15.0, "{}", half.body);
    let all = send(&srv, &owner, "put", &rec_uri("lines"), Some(json!({ "clear": lines })), None).await;
    assert_eq!(num(&all.body, "difference"), 0.0, "{}", all.body);
    let done = send(&srv, &owner, "post", &rec_uri("complete"), None, None).await;
    assert_eq!(done.body["status"], "completed", "{}", done.body);
    assert_eq!(num(&done.body, "cleared_balance"), 85.0);
    let next = send(
        &srv,
        &owner,
        "post",
        "/api/v1/bank-reconciliations",
        Some(json!({ "account_id": bank, "statement_date": today, "statement_balance": "85" })),
        None,
    )
    .await;
    assert_eq!(num(&next.body, "opening_balance"), 85.0, "{}", next.body);
    assert_eq!(next.body["lines"].as_array().map(Vec::len), Some(0), "{}", next.body);
    let not_latest = send(&srv, &owner, "post", &rec_uri("reopen"), Some(json!({ "reason": "Typo" })), None).await;
    assert_eq!(not_latest.code(), "RECONCILIATION_OPEN", "{}", not_latest.body);
    let next_cancel = format!("/api/v1/bank-reconciliations/{}/cancel", s(&next.body, "id"));
    let cancelled = send(&srv, &owner, "post", &next_cancel, None, None).await;
    assert_eq!(cancelled.body["status"], "cancelled", "{}", cancelled.body);
    let reopened = send(&srv, &owner, "post", &rec_uri("reopen"), Some(json!({ "reason": "Typo" })), None).await;
    assert_eq!(reopened.body["status"], "in_progress", "{}", reopened.body);
    assert_eq!(reopened.body["lines"].as_array().map(Vec::len), Some(2));
    let redone = send(&srv, &owner, "post", &rec_uri("complete"), None, None).await;
    assert_eq!(redone.body["status"], "completed", "{}", redone.body);

    // --- notifications: a low-stock product and today's sales, checked tomorrow morning
    let gum = send(
        &srv,
        &owner,
        "post",
        "/api/v1/products",
        Some(
            json!({ "name": "Gum", "unit_id": pcs, "selling_price": "5", "purchase_price": "3", "reorder_level": "5" }),
        ),
        None,
    )
    .await;
    assert_eq!(gum.status, StatusCode::CREATED, "{}", gum.body);
    let gum_stock = send(
        &srv,
        &owner,
        "post",
        "/api/v1/stock-adjustments",
        Some(json!({ "reason": "opening", "lines": [{ "variant_id": s(&gum.body, "variant_id"), "quantity": "2", "unit_cost": "3" }] })),
        None,
    )
    .await;
    assert_eq!(gum_stock.status, StatusCode::CREATED, "{}", gum_stock.body);
    // 10:00 in Dhaka (UTC+6, the business's zone) on the following days.
    let dhaka_today = (chrono::Utc::now() + chrono::Duration::hours(6)).date_naive();
    let morning =
        |days: i64| (dhaka_today + chrono::Duration::days(days)).and_hms_opt(4, 0, 0).expect("time").and_utc();
    let created = pos_notify::jobs::run(&pool, morning(1)).await.expect("notification checks");
    assert!(created >= 2, "created {created}");
    let again = pos_notify::jobs::run(&pool, morning(1)).await.expect("notification checks");
    assert_eq!(again, 0, "checks are idempotent");
    let inbox = get!("/api/v1/notifications").await;
    let types: Vec<&str> =
        inbox.body["data"].as_array().expect("notifications").iter().filter_map(|n| n["notif_type"].as_str()).collect();
    assert!(types.contains(&"stock.low") && types.contains(&"sales.daily_summary"), "{}", inbox.body);
    let low = find(&inbox.body["data"], "notif_type", "stock.low");
    assert_eq!(low["link"], "/inventory?status=low");
    assert!(low["title"].as_str().is_some_and(|t| t.contains("low on stock")), "{low}");
    let unread = get!("/api/v1/notifications/unread-count").await;
    assert_eq!(unread.body["unread_count"].as_i64(), Some(types.len() as i64), "{}", unread.body);
    let read = send(&srv, &owner, "post", &format!("/api/v1/notifications/{}/read", s(low, "id")), None, None).await;
    assert_eq!(read.status, StatusCode::NO_CONTENT, "{}", read.body);

    let prefs = get!("/api/v1/notification-preferences").await;
    let low_pref = find(&prefs.body, "notif_type", "stock.low");
    assert_eq!(low_pref["channels"], json!(["in_app"]), "{}", prefs.body);
    assert!(low_pref["available_channels"].as_array().is_some_and(|c| c.contains(&json!("sms"))), "{}", prefs.body);
    let bad = send(
        &srv,
        &owner,
        "put",
        "/api/v1/notification-preferences",
        Some(json!([{ "notif_type": "stock.low", "channels": ["fax"] }])),
        None,
    )
    .await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST, "{}", bad.body);
    let saved = send(
        &srv,
        &owner,
        "put",
        "/api/v1/notification-preferences",
        Some(json!([{ "notif_type": "stock.low", "channels": ["in_app", "sms"] }])),
        None,
    )
    .await;
    assert_eq!(find(&saved.body, "notif_type", "stock.low")["channels"], json!(["in_app", "sms"]), "{}", saved.body);
    pos_notify::jobs::run(&pool, morning(2)).await.expect("notification checks");
    let sent = pos_notify::delivery::dispatch(&pool, &pos_notify::delivery::Gateways::log_only(), morning(2))
        .await
        .expect("delivery");
    assert_eq!(sent.sent, 1, "{sent:?}");
    let statuses: Vec<(String, String)> = sqlx::query_as("SELECT channel, status FROM notification_deliveries")
        .fetch_all(&pool)
        .await
        .expect("deliveries");
    assert_eq!(statuses, vec![("sms".to_owned(), "sent".to_owned())]);
    let all = send(&srv, &owner, "post", "/api/v1/notifications/read-all", None, None).await;
    assert_eq!(all.status, StatusCode::NO_CONTENT);
    let unread = get!("/api/v1/notifications/unread-count").await;
    assert_eq!(unread.body["unread_count"], 0, "{}", unread.body);

    // --- once money is recorded the currency is fixed
    let locked = set_currency(&srv, &owner, "USD").await;
    assert_eq!(locked.body["errors"][0]["code"], "CURRENCY_LOCKED", "{}", locked.body);

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
