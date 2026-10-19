use chrono::NaiveDate;
use pos_worker::{cleanup, partitions};
use sqlx::MySqlPool;

fn names(v: Vec<String>) -> Vec<String> {
    let mut v = v;
    v.sort();
    v
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn ensure_adds_future_partitions_and_is_idempotent(pool: MySqlPool) {
    let today = NaiveDate::from_ymd_opt(2027, 1, 10).expect("date");
    let added = partitions::ensure(&pool, "audit_logs", today, 3).await.expect("ensure");
    assert_eq!(added.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["p2027_02", "p2027_03", "p2027_04"]);

    let again = partitions::ensure(&pool, "audit_logs", today, 3).await.expect("ensure again");
    assert!(again.is_empty());

    let existing: Vec<String> = sqlx::query_scalar(
        "SELECT partition_name FROM information_schema.partitions WHERE table_schema = DATABASE() AND table_name = 'audit_logs'",
    )
    .fetch_all(&pool)
    .await
    .expect("partitions");
    assert_eq!(
        names(existing),
        ["p2026_10", "p2026_11", "p2026_12", "p2027_01", "p2027_02", "p2027_03", "p2027_04", "pmax"]
    );
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn missing_tables_are_skipped(pool: MySqlPool) {
    // stock_movements arrives in Phase 2
    let today = NaiveDate::from_ymd_opt(2026, 10, 17).expect("date");
    assert!(partitions::ensure(&pool, "stock_movements", today, 3).await.expect("ensure").is_empty());
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn purge_removes_only_expired_keys(pool: MySqlPool) {
    let tenant = pos_core::id::new_id();
    sqlx::query("INSERT INTO tenants (id, slug, name, status) VALUES (?, 'shop-a', 'Shop A', 'active')")
        .bind(tenant)
        .execute(&pool)
        .await
        .expect("tenant");
    let ctx = pos_core::PlatformCtx { actor_id: None, reason: "seed".into(), meta: Default::default() };
    let mut tx = pos_db::begin_platform(&pool, &ctx).await.expect("begin");
    for (key, expires) in [(pos_core::id::new_id(), "2000-01-01"), (pos_core::id::new_id(), "2999-01-01")] {
        sqlx::query(
            "INSERT INTO idempotency_keys (tenant_id, idem_key, user_id, endpoint, request_hash, expires_at) \
             VALUES (?, ?, ?, 'POST /v1/test', UNHEX(SHA2('x', 256)), ?)",
        )
        .bind(tenant)
        .bind(key)
        .bind(pos_core::id::new_id())
        .bind(expires)
        .execute(&mut *tx)
        .await
        .expect("insert key");
    }
    tx.commit().await.expect("commit");

    assert_eq!(cleanup::purge_expired_idempotency_keys(&pool).await.expect("purge"), 1);
    let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM idempotency_keys").fetch_one(&pool).await.expect("count");
    assert_eq!(left, 1);
}
