//! Catalogue over HTTP (E2.1, E2.2): seeded masters, products with automatic
//! SKUs, barcodes (AC-PROD-01), price history (PROD-R02), search, archive
//! (AC-PROD-02), cost visibility and the `products.max` limit.

mod common;

use actix_web::{http::StatusCode, test};
use common::{Req, Session, call, create_business, join, sign_up};
use pos_api::{AppState, app};
use serde_json::{Value, json};
use sqlx::MySqlPool;
use uuid::Uuid;

fn find<'a>(list: &'a Value, key: &str, value: &str) -> &'a Value {
    list.as_array().into_iter().flatten().find(|v| v[key] == value).unwrap_or_else(|| panic!("{value} in {list}"))
}

fn id(value: &Value) -> String {
    value["id"].as_str().unwrap_or_else(|| panic!("id in {value}")).to_owned()
}

fn names(page: &Value) -> Vec<String> {
    page["data"].as_array().into_iter().flatten().map(|p| p["name"].as_str().unwrap_or_default().to_owned()).collect()
}

async fn pcs<S, B>(srv: &S, owner: &Session) -> String
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let units = call(srv, Req::get("/api/v1/units").bearer(&owner.token).build()).await;
    id(find(&units.body, "short_name", "pcs"))
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn new_business_has_tax_rates_and_units(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool.clone(), common::config()))).await;
    let mut owner = sign_up(&srv, "Owner", "01711000501").await;
    let tenant = create_business(&srv, &mut owner, "Rahim Store").await;

    let taxes = call(&srv, Req::get("/api/v1/tax-rates").bearer(&owner.token).build()).await;
    assert_eq!(taxes.body.as_array().map(Vec::len), Some(5), "{}", taxes.body);
    let no_vat = find(&taxes.body, "name", "No VAT");
    assert_eq!(no_vat["is_default"], true);
    assert_eq!(find(&taxes.body, "name", "VAT 15%")["rate_pct"], "15.0000");
    let setting: Option<Uuid> =
        sqlx::query_scalar("SELECT default_tax_rate_id FROM tenant_settings WHERE tenant_id = UUID_TO_BIN(?)")
            .bind(&tenant)
            .fetch_one(&pool)
            .await
            .expect("settings");
    assert_eq!(setting.map(|u| u.to_string()).as_deref(), no_vat["id"].as_str());

    let units = call(&srv, Req::get("/api/v1/units").bearer(&owner.token).build()).await;
    assert_eq!(units.body.as_array().map(Vec::len), Some(8));
    assert_eq!(find(&units.body, "short_name", "kg")["allow_fraction"], true);
    assert_eq!(find(&units.body, "short_name", "pcs")["allow_fraction"], false);

    // A new default moves the setting with it.
    let vat15 = id(find(&taxes.body, "name", "VAT 15%"));
    let made_default = call(
        &srv,
        Req::put(
            &format!("/api/v1/tax-rates/{vat15}"),
            json!({ "name": "VAT 15%", "rate_pct": "15", "is_default": true }),
        )
        .bearer(&owner.token)
        .build(),
    )
    .await;
    assert_eq!(made_default.status, StatusCode::OK, "{}", made_default.body);
    let taxes = call(&srv, Req::get("/api/v1/tax-rates").bearer(&owner.token).build()).await;
    let defaults: Vec<&Value> =
        taxes.body.as_array().into_iter().flatten().filter(|t| t["is_default"] == true).collect();
    assert_eq!(defaults.len(), 1);
    assert_eq!(defaults[0]["name"], "VAT 15%");
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn product_lifecycle(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool, common::config()))).await;
    let mut owner = sign_up(&srv, "Owner", "01711000511").await;
    let tenant = create_business(&srv, &mut owner, "Rahim Store").await;
    let pcs = pcs(&srv, &owner).await;
    let post = |uri: &str, body: Value| Req::post(uri, body).bearer(&owner.token).build();

    let rice = call(
        &srv,
        post(
            "/api/v1/products",
            json!({ "name": "Miniket Rice 5kg", "name_bn": "মিনিকেট চাল", "unit_id": pcs, "selling_price": "450.50",
                    "purchase_price": "400", "barcodes": ["4006381333931", "RICE-5"] }),
        ),
    )
    .await;
    assert_eq!(rice.status, StatusCode::CREATED, "{}", rice.body);
    assert_eq!(rice.body["sku"], "P000001");
    assert_eq!(rice.body["selling_price"], "450.5000");
    assert_eq!(rice.body["purchase_price"], "400.0000");
    assert_eq!(rice.body["barcodes"], json!(["4006381333931", "RICE-5"]));
    assert_eq!(rice.body["tax_rate_pct"], "0.0000", "default tax rate applied");
    assert_eq!(rice.body["track_inventory"], true);
    assert_eq!(rice.headers.get("etag").and_then(|v| v.to_str().ok()), Some("\"1\""));
    let rice_id = id(&rice.body);

    let oil =
        call(&srv, post("/api/v1/products", json!({ "name": "Soybean Oil", "unit_id": pcs, "selling_price": "190" })))
            .await;
    assert_eq!(oil.body["sku"], "P000002");
    let service = call(
        &srv,
        post(
            "/api/v1/products",
            json!({ "name": "Home delivery", "product_type": "service", "unit_id": pcs, "selling_price": "50",
                    "sku": "SVC-DELIVERY" }),
        ),
    )
    .await;
    assert_eq!(service.body["track_inventory"], false, "services are never stocked");

    // AC-PROD-01 and SKU uniqueness.
    let dup_barcode = call(
        &srv,
        post(
            "/api/v1/products",
            json!({ "name": "Copy", "unit_id": pcs, "selling_price": "1", "barcodes": ["RICE-5"] }),
        ),
    )
    .await;
    assert_eq!((dup_barcode.status, dup_barcode.code()), (StatusCode::CONFLICT, "DUPLICATE_BARCODE"));
    let dup_sku = call(
        &srv,
        post("/api/v1/products", json!({ "name": "Copy", "unit_id": pcs, "selling_price": "1", "sku": "p000001" })),
    )
    .await;
    assert_eq!((dup_sku.status, dup_sku.code()), (StatusCode::CONFLICT, "DUPLICATE_SKU"), "SKUs are case-insensitive");
    for bad in [
        json!({ "name": "Bad", "unit_id": pcs, "selling_price": "1", "barcodes": ["4006381333932"] }),
        json!({ "name": "Bad", "unit_id": pcs, "selling_price": "-1" }),
        json!({ "name": "Bad", "unit_id": pcs, "selling_price": "1.12345" }),
        json!({ "name": "Bad", "unit_id": pcs, "selling_price": "1", "sku": "has space" }),
        json!({ "name": "Bad", "unit_id": pcs, "selling_price": "1", "reorder_level": "1.5" }),
        json!({ "name": "Bad", "unit_id": Uuid::now_v7(), "selling_price": "1" }),
        json!({ "name": "", "unit_id": pcs, "selling_price": "1" }),
        json!({ "name": "Bad", "unit_id": pcs, "selling_price": "1", "product_type": "combo" }),
    ] {
        let res = call(&srv, post("/api/v1/products", bad.clone())).await;
        assert_eq!(res.status, StatusCode::BAD_REQUEST, "{bad}: {}", res.body);
    }

    // Update with If-Match; price changes are recorded.
    let update = |version: i64, body: Value| {
        Req::put(&format!("/api/v1/products/{rice_id}"), body).bearer(&owner.token).if_match(version).build()
    };
    let body = json!({ "name": "Miniket Rice 5kg", "name_bn": "মিনিকেট চাল", "unit_id": pcs, "selling_price": "460",
                       "barcodes": ["RICE-5"], "reorder_level": "10" });
    let stale = call(&srv, update(7, body.clone())).await;
    assert_eq!(stale.status, StatusCode::PRECONDITION_FAILED);
    let updated = call(&srv, update(1, body)).await;
    assert_eq!(updated.status, StatusCode::OK, "{}", updated.body);
    assert_eq!(updated.body["version"], 2);
    assert_eq!(updated.body["sku"], "P000001", "a blank SKU keeps the current one");
    assert_eq!(updated.body["purchase_price"], "400.0000", "a blank cost keeps the current one");
    assert_eq!(updated.body["barcodes"], json!(["RICE-5"]));
    let history =
        call(&srv, Req::get(&format!("/api/v1/products/{rice_id}/price-history")).bearer(&owner.token).build()).await;
    assert_eq!(history.body.as_array().map(Vec::len), Some(1), "{}", history.body);
    assert_eq!(history.body[0]["price_field"], "selling_price");
    assert_eq!(history.body[0]["old_value"], "450.5000");
    assert_eq!(history.body[0]["new_value"], "460.0000");
    // The removed barcode is free again.
    let reuse = call(
        &srv,
        post(
            "/api/v1/products",
            json!({ "name": "Other", "unit_id": pcs, "selling_price": "1", "barcodes": ["4006381333931"] }),
        ),
    )
    .await;
    assert_eq!(reuse.status, StatusCode::CREATED, "{}", reuse.body);

    // Search: Bengali name, exact barcode, part of a SKU.
    let search = |q: &str| Req::get(&format!("/api/v1/products?q={q}")).bearer(&owner.token).build();
    assert_eq!(names(&call(&srv, search("%E0%A6%9A%E0%A6%BE%E0%A6%B2")).await.body), ["Miniket Rice 5kg"]); // চাল
    assert_eq!(names(&call(&srv, search("RICE-5")).await.body), ["Miniket Rice 5kg"]);
    assert_eq!(names(&call(&srv, search("delivery")).await.body), ["Home delivery"]);
    assert_eq!(names(&call(&srv, search("100_%25")).await.body), Vec::<String>::new(), "wildcards are literal");

    // Archive hides it from the default list; restore brings it back (AC-PROD-02).
    let archived =
        call(&srv, Req::post(&format!("/api/v1/products/{rice_id}/archive"), json!({})).bearer(&owner.token).build())
            .await;
    assert_eq!(archived.body["status"], "archived");
    let active = call(&srv, Req::get("/api/v1/products").bearer(&owner.token).build()).await;
    assert!(!names(&active.body).contains(&"Miniket Rice 5kg".to_owned()));
    let only_archived = call(&srv, Req::get("/api/v1/products?status=archived").bearer(&owner.token).build()).await;
    assert_eq!(names(&only_archived.body), ["Miniket Rice 5kg"]);
    let again =
        call(&srv, Req::post(&format!("/api/v1/products/{rice_id}/archive"), json!({})).bearer(&owner.token).build())
            .await;
    assert_eq!(again.code(), "INVALID_STATUS");
    let restored =
        call(&srv, Req::post(&format!("/api/v1/products/{rice_id}/restore"), json!({})).bearer(&owner.token).build())
            .await;
    assert_eq!(restored.body["status"], "active");

    // Delete frees the SKU.
    let deleted =
        call(&srv, Req::delete(&format!("/api/v1/products/{}", id(&oil.body))).bearer(&owner.token).build()).await;
    assert_eq!(deleted.status, StatusCode::NO_CONTENT);
    let gone = call(&srv, Req::get(&format!("/api/v1/products/{}", id(&oil.body))).bearer(&owner.token).build()).await;
    assert_eq!(gone.status, StatusCode::NOT_FOUND);
    let same_sku = call(
        &srv,
        post(
            "/api/v1/products",
            json!({ "name": "Oil again", "unit_id": pcs, "selling_price": "1", "sku": "P000002" }),
        ),
    )
    .await;
    assert_eq!(same_sku.status, StatusCode::CREATED, "{}", same_sku.body);

    // Pagination walks every product once.
    let mut seen = Vec::new();
    let mut uri = "/api/v1/products?status=all&limit=2".to_owned();
    loop {
        let page = call(&srv, Req::get(&uri).bearer(&owner.token).build()).await;
        seen.extend(names(&page.body));
        match page.body["next_cursor"].as_str() {
            Some(c) => uri = format!("/api/v1/products?status=all&limit=2&cursor={c}"),
            None => break,
        }
    }
    assert_eq!(seen.len(), 4, "{seen:?}");

    // Photo: the type comes from the bytes; a new photo replaces the old one.
    let soap =
        call(&srv, post("/api/v1/products", json!({ "name": "Soap", "unit_id": pcs, "selling_price": "40" }))).await;
    let image_uri = format!("/api/v1/products/{}/image", id(&soap.body));
    let png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR-pretend-image-data";
    // A photo for a product that is not there is refused before anything is stored.
    let photos = std::path::Path::new(&common::config().storage_dir).join(format!("tenants/{tenant}/product_image"));
    let stored = || std::fs::read_dir(&photos).map_or(0, Iterator::count);
    let missing = format!("/api/v1/products/{}/image", Uuid::now_v7());
    let nowhere = call(&srv, Req::put_bytes(&missing, png, "image/png").bearer(&owner.token).build()).await;
    assert_eq!(nowhere.status, StatusCode::NOT_FOUND, "{}", nowhere.body);
    assert_eq!(stored(), 0, "no bytes kept for a missing product");
    let not_image =
        call(&srv, Req::put_bytes(&image_uri, b"GIF89a....", "image/gif").bearer(&owner.token).build()).await;
    assert_eq!(
        (not_image.status, not_image.body["errors"][0]["code"].as_str()),
        (StatusCode::BAD_REQUEST, Some("UNSUPPORTED_TYPE"))
    );
    let too_big = vec![0xFF_u8; 2 * 1024 * 1024 + 1];
    let refused = call(&srv, Req::put_bytes(&image_uri, &too_big, "image/jpeg").bearer(&owner.token).build()).await;
    assert_eq!(refused.status, StatusCode::BAD_REQUEST, "{}", refused.body);
    let first = call(
        &srv,
        Req::put_bytes(&image_uri, png, "image/png")
            .header("x-file-name", "soap%20front.png")
            .bearer(&owner.token)
            .build(),
    )
    .await;
    assert_eq!(first.status, StatusCode::OK, "{}", first.body);
    assert_eq!(stored(), 1);
    let first_file = first.body["image_file_id"].as_str().expect("image id").to_owned();
    let served =
        test::call_service(&srv, Req::get(&format!("/api/v1/files/{first_file}")).bearer(&owner.token).build()).await;
    assert_eq!(served.status(), StatusCode::OK);
    assert_eq!(served.headers().get("content-type").and_then(|v| v.to_str().ok()), Some("image/png"));
    assert_eq!(test::read_body(served).await.as_ref(), png);
    let jpg = [0xFF, 0xD8, 0xFF, 0xE0, 1, 2, 3];
    let second = call(&srv, Req::put_bytes(&image_uri, &jpg, "image/jpeg").bearer(&owner.token).build()).await;
    assert_eq!(second.status, StatusCode::OK, "{}", second.body);
    assert_ne!(second.body["image_file_id"], json!(first_file));
    let gone = call(&srv, Req::get(&format!("/api/v1/files/{first_file}")).bearer(&owner.token).build()).await;
    assert_eq!(gone.status, StatusCode::NOT_FOUND, "the replaced photo is removed");
    let usage = call(&srv, Req::get("/api/v1/subscription").bearer(&owner.token).build()).await;
    let storage = usage.body["usage"].as_array().into_iter().flatten().find(|m| m["feature"] == "storage.mb").cloned();
    assert_eq!(storage.map(|m| m["used"].clone()), Some(json!(1)), "{}", usage.body);
    let cleared = call(&srv, Req::delete(&image_uri).bearer(&owner.token).build()).await;
    assert_eq!((cleared.status, cleared.body["image_file_id"].clone()), (StatusCode::OK, Value::Null));

    let audit = call(&srv, Req::get("/api/v1/audit-logs?action=product.*").bearer(&owner.token).build()).await;
    let actions: Vec<&str> =
        audit.body["data"].as_array().into_iter().flatten().filter_map(|e| e["action"].as_str()).collect();
    for action in [
        "product.created",
        "product.updated",
        "product.archived",
        "product.restored",
        "product.deleted",
        "product.image_set",
        "product.image_removed",
    ] {
        assert!(actions.contains(&action), "{action} in {actions:?}");
    }
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn categories_and_brands(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool, common::config()))).await;
    let mut owner = sign_up(&srv, "Owner", "01711000521").await;
    create_business(&srv, &mut owner, "Rahim Store").await;
    let pcs = pcs(&srv, &owner).await;
    let post = |uri: &str, body: Value| Req::post(uri, body).bearer(&owner.token).build();

    let food = call(&srv, post("/api/v1/categories", json!({ "name": "Food" }))).await;
    assert_eq!(food.status, StatusCode::CREATED, "{}", food.body);
    let grains = call(&srv, post("/api/v1/categories", json!({ "name": "Grains", "parent_id": id(&food.body) }))).await;
    let rice = call(&srv, post("/api/v1/categories", json!({ "name": "Rice", "parent_id": id(&grains.body) }))).await;
    assert_eq!(rice.body["depth"], 2);
    let too_deep =
        call(&srv, post("/api/v1/categories", json!({ "name": "Basmati", "parent_id": id(&rice.body) }))).await;
    assert_eq!(too_deep.status, StatusCode::BAD_REQUEST, "{}", too_deep.body);
    let dup = call(&srv, post("/api/v1/categories", json!({ "name": "food" }))).await;
    assert_eq!(dup.code(), "DUPLICATE_NAME", "names are case-insensitive per level");
    let same_name_elsewhere =
        call(&srv, post("/api/v1/categories", json!({ "name": "Rice", "parent_id": id(&food.body) }))).await;
    assert_eq!(same_name_elsewhere.status, StatusCode::CREATED);
    let moved = call(
        &srv,
        Req::put(&format!("/api/v1/categories/{}", id(&grains.body)), json!({ "name": "Grains" }))
            .bearer(&owner.token)
            .build(),
    )
    .await;
    assert_eq!(moved.status, StatusCode::BAD_REQUEST, "a category with children cannot move: {}", moved.body);

    let brand = call(&srv, post("/api/v1/brands", json!({ "name": "ACI" }))).await;
    let product = call(
        &srv,
        post(
            "/api/v1/products",
            json!({ "name": "Chinigura", "unit_id": pcs, "selling_price": "150",
                    "category_id": id(&rice.body), "brand_id": id(&brand.body) }),
        ),
    )
    .await;
    assert_eq!(product.body["category_name"], "Rice");
    assert_eq!(product.body["brand_name"], "ACI");

    // A filter on a top category includes its sub-categories.
    let in_food =
        call(&srv, Req::get(&format!("/api/v1/products?category_id={}", id(&food.body))).bearer(&owner.token).build())
            .await;
    assert_eq!(names(&in_food.body), ["Chinigura"]);
    let by_brand =
        call(&srv, Req::get(&format!("/api/v1/products?brand_id={}", id(&brand.body))).bearer(&owner.token).build())
            .await;
    assert_eq!(names(&by_brand.body), ["Chinigura"]);

    // Categories and brands in use cannot be deleted.
    for uri in [format!("/api/v1/categories/{}", id(&rice.body)), format!("/api/v1/brands/{}", id(&brand.body))] {
        let res = call(&srv, Req::delete(&uri).bearer(&owner.token).build()).await;
        assert_eq!((res.status, res.code()), (StatusCode::CONFLICT, "IN_USE"), "{uri}");
    }
    let empty = call(
        &srv,
        Req::delete(&format!("/api/v1/categories/{}", id(&same_name_elsewhere.body))).bearer(&owner.token).build(),
    )
    .await;
    assert_eq!(empty.status, StatusCode::NO_CONTENT);
    let list = call(&srv, Req::get("/api/v1/categories").bearer(&owner.token).build()).await;
    assert_eq!(list.body.as_array().map(Vec::len), Some(3));
    assert_eq!(find(&list.body, "name", "Rice")["product_count"], 1);
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn cost_is_hidden_and_writes_need_permission(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool, common::config()))).await;
    let mut owner = sign_up(&srv, "Owner", "01711000531").await;
    let tenant = create_business(&srv, &mut owner, "Rahim Store").await;
    let pcs = pcs(&srv, &owner).await;
    let created = call(
        &srv,
        Req::post(
            "/api/v1/products",
            json!({ "name": "Tea", "unit_id": pcs, "selling_price": "20", "purchase_price": "15" }),
        )
        .bearer(&owner.token)
        .build(),
    )
    .await;
    let cashier = join(&srv, &owner, &tenant, "Cashier", "01711000532", &["cashier"]).await;

    let seen =
        call(&srv, Req::get(&format!("/api/v1/products/{}", id(&created.body))).bearer(&cashier.token).build()).await;
    assert_eq!(seen.status, StatusCode::OK);
    assert_eq!(seen.body["selling_price"], "20.0000");
    assert!(seen.body["purchase_price"].is_null(), "cashiers do not see cost");
    let create = call(
        &srv,
        Req::post("/api/v1/products", json!({ "name": "Coffee", "unit_id": pcs, "selling_price": "30" }))
            .bearer(&cashier.token)
            .build(),
    )
    .await;
    assert_eq!(create.status, StatusCode::FORBIDDEN);
    let brand = call(&srv, Req::post("/api/v1/brands", json!({ "name": "X" })).bearer(&cashier.token).build()).await;
    assert_eq!(brand.status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn plan_product_limit(pool: MySqlPool) {
    let srv = test::init_service(app(AppState::new(pool.clone(), common::config()))).await;
    let mut owner = sign_up(&srv, "Owner", "01711000541").await;
    let tenant = Uuid::parse_str(&create_business(&srv, &mut owner, "Rahim Store").await).expect("uuid");
    let pcs = pcs(&srv, &owner).await;
    let owner_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE phone = '+8801711000541'")
        .fetch_one(&pool)
        .await
        .expect("owner");
    let ctx = pos_core::PlatformCtx { actor_id: Some(owner_id), reason: "test".into(), meta: Default::default() };
    let mut tx = pos_db::begin_platform(&pool, &ctx).await.expect("tx");
    sqlx::query(
        "INSERT INTO tenant_feature_overrides (tenant_id, id, feature_code, limit_value, reason, created_by) \
         VALUES (?, ?, 'products.max', 2, 'test', ?)",
    )
    .bind(tenant)
    .bind(pos_core::id::new_id())
    .bind(owner_id)
    .execute(&mut *tx)
    .await
    .expect("override");
    tx.commit().await.expect("commit");

    let make = |name: &str| {
        Req::post("/api/v1/products", json!({ "name": name, "unit_id": pcs, "selling_price": "1" }))
            .bearer(&owner.token)
            .build()
    };
    let first = call(&srv, make("One")).await;
    call(&srv, make("Two")).await;
    let third = call(&srv, make("Three")).await;
    assert_eq!((third.status, third.code()), (StatusCode::PAYMENT_REQUIRED, "PLAN_LIMIT_REACHED"), "{}", third.body);

    // Archived products do not count; restoring one counts again.
    let archive_uri = format!("/api/v1/products/{}/archive", id(&first.body));
    call(&srv, Req::post(&archive_uri, json!({})).bearer(&owner.token).build()).await;
    assert_eq!(call(&srv, make("Three")).await.status, StatusCode::CREATED);
    let restore_uri = format!("/api/v1/products/{}/restore", id(&first.body));
    let restore = call(&srv, Req::post(&restore_uri, json!({})).bearer(&owner.token).build()).await;
    assert_eq!(restore.code(), "PLAN_LIMIT_REACHED");

    let usage = call(&srv, Req::get("/api/v1/subscription").bearer(&owner.token).build()).await;
    let meter = find(&usage.body["usage"], "feature", "products.max");
    assert_eq!((meter["used"].as_i64(), meter["limit"].as_i64()), (Some(2), Some(2)));
}
