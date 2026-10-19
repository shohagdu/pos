//! Database-level tenant isolation tests (Requirement Analysis AT-02…AT-04,
//! DB design §3). Each test runs against a fresh database created by
//! `#[sqlx::test]` from `DATABASE_URL` with all migrations applied.

use pos_core::{AppError, PlatformCtx, RequestMeta, TenantCtx, id::new_id};
use sqlx::{
    MySqlPool,
    mysql::{MySqlConnectOptions, MySqlPoolOptions},
};
use uuid::Uuid;

fn tenant_ctx(tenant_id: Uuid) -> TenantCtx {
    TenantCtx { tenant_id, user_id: new_id(), meta: RequestMeta::default() }
}

fn platform_ctx() -> PlatformCtx {
    PlatformCtx { actor_id: None, reason: "test setup".into(), meta: RequestMeta::default() }
}

async fn create_tenant(pool: &MySqlPool, slug: &str) -> Uuid {
    let id = new_id();
    sqlx::query("INSERT INTO tenants (id, slug, name, status) VALUES (?, ?, ?, 'active')")
        .bind(id)
        .bind(slug)
        .bind(format!("Tenant {slug}"))
        .execute(pool)
        .await
        .expect("insert tenant");
    id
}

const INSERT_BRANCH: &str = "INSERT INTO branches (tenant_id, id, code, name) VALUES (?, ?, ?, 'Branch')";

fn is_guard(err: &AppError, guard: &str) -> bool {
    matches!(err, AppError::Internal(source) if source.to_string().contains(guard))
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn schema_guards_pass(pool: MySqlPool) {
    let report = pos_db::guards::check_schema(&pool).await.expect("guard queries");
    assert!(report.is_clean(), "schema guard violations: {report:?}");
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn seeds_are_present(pool: MySqlPool) {
    let plans: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM plans").fetch_one(&pool).await.expect("plans");
    let permissions: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM permissions").fetch_one(&pool).await.expect("perms");
    let plan_features: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM plan_features").fetch_one(&pool).await.expect("plan features");
    let features: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM features").fetch_one(&pool).await.expect("features");
    assert_eq!(plans, 4);
    assert_eq!(permissions, 64);
    assert_eq!(plan_features, plans * features, "every plan must define every feature");
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn tenant_can_write_its_own_rows(pool: MySqlPool) {
    let a = create_tenant(&pool, "shop-a").await;
    let mut tx = pos_db::begin_tenant(&pool, &tenant_ctx(a)).await.expect("begin");
    sqlx::query(INSERT_BRANCH)
        .bind(tx.tenant_id())
        .bind(new_id())
        .bind("MAIN")
        .execute(&mut *tx)
        .await
        .expect("insert");
    tx.commit().await.expect("commit");
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn guard_blocks_cross_tenant_insert_update_delete(pool: MySqlPool) {
    let a = create_tenant(&pool, "shop-a").await;
    let b = create_tenant(&pool, "shop-b").await;

    let mut tx = pos_db::begin_tenant(&pool, &tenant_ctx(a)).await.expect("begin a");
    sqlx::query(INSERT_BRANCH).bind(a).bind(new_id()).bind("MAIN").execute(&mut *tx).await.expect("insert a");
    tx.commit().await.expect("commit a");

    // Tenant B's context tries to touch tenant A's rows.
    let mut tx = pos_db::begin_tenant(&pool, &tenant_ctx(b)).await.expect("begin b");
    let insert = sqlx::query(INSERT_BRANCH).bind(a).bind(new_id()).bind("EVIL").execute(&mut *tx).await;
    assert!(is_guard(&pos_db::map_db_error(insert.expect_err("insert must fail")), "TENANT_GUARD"));

    let update = sqlx::query("UPDATE branches SET name = 'hacked' WHERE tenant_id = ?").bind(a).execute(&mut *tx).await;
    assert!(is_guard(&pos_db::map_db_error(update.expect_err("update must fail")), "TENANT_GUARD"));

    let delete = sqlx::query("DELETE FROM branches WHERE tenant_id = ?").bind(a).execute(&mut *tx).await;
    assert!(is_guard(&pos_db::map_db_error(delete.expect_err("delete must fail")), "TENANT_GUARD"));
    drop(tx);

    let name: String = sqlx::query_scalar("SELECT name FROM branches WHERE tenant_id = ?")
        .bind(a)
        .fetch_one(&pool)
        .await
        .expect("tenant A branch intact");
    assert_eq!(name, "Branch");
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn writes_without_tenant_context_are_rejected(pool: MySqlPool) {
    let a = create_tenant(&pool, "shop-a").await;
    let err =
        sqlx::query(INSERT_BRANCH).bind(a).bind(new_id()).bind("MAIN").execute(&pool).await.expect_err("no context");
    assert!(is_guard(&pos_db::map_db_error(err), "TENANT_GUARD"));
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn composite_foreign_keys_reject_cross_tenant_links(pool: MySqlPool) {
    let a = create_tenant(&pool, "shop-a").await;
    let b = create_tenant(&pool, "shop-b").await;
    let branch_a = new_id();

    let mut tx = pos_db::begin_tenant(&pool, &tenant_ctx(a)).await.expect("begin a");
    sqlx::query(INSERT_BRANCH).bind(a).bind(branch_a).bind("MAIN").execute(&mut *tx).await.expect("branch a");
    tx.commit().await.expect("commit a");

    // A stock location of tenant B pointing at tenant A's branch.
    let mut tx = pos_db::begin_tenant(&pool, &tenant_ctx(b)).await.expect("begin b");
    let err = sqlx::query(
        "INSERT INTO stock_locations (tenant_id, id, branch_id, code, name) VALUES (?, ?, ?, 'MAIN', 'Main')",
    )
    .bind(b)
    .bind(new_id())
    .bind(branch_a)
    .execute(&mut *tx)
    .await
    .expect_err("cross-tenant FK must fail");
    assert!(matches!(pos_db::map_db_error(err), AppError::Unprocessable { code: "INVALID_REFERENCE", .. }));
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn platform_transaction_may_write_any_tenant(pool: MySqlPool) {
    let a = create_tenant(&pool, "shop-a").await;
    let mut tx = pos_db::begin_platform(&pool, &platform_ctx()).await.expect("begin platform");
    sqlx::query(INSERT_BRANCH).bind(a).bind(new_id()).bind("MAIN").execute(&mut *tx).await.expect("platform insert");
    tx.commit().await.expect("commit");
}

/// The pool's after_release hook must clear @app_tenant_id so a pooled
/// connection never carries a previous request's tenant (DB design §3.3).
#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn tenant_context_is_cleared_when_connection_returns_to_pool(_: MySqlPoolOptions, opts: MySqlConnectOptions) {
    let pool = pos_db::pool_options().max_connections(1).connect_with(opts).await.expect("pool");
    let a = create_tenant(&pool, "shop-a").await;

    // committed transaction
    let tx = pos_db::begin_tenant(&pool, &tenant_ctx(a)).await.expect("begin");
    tx.commit().await.expect("commit");
    let leaked: Option<Vec<u8>> = sqlx::query_scalar("SELECT @app_tenant_id").fetch_one(&pool).await.expect("read var");
    assert_eq!(leaked, None, "context leaked after commit");

    // dropped (rolled back) transaction
    let tx = pos_db::begin_platform(&pool, &platform_ctx()).await.expect("begin platform");
    drop(tx);
    let platform: Option<i64> = sqlx::query_scalar("SELECT @app_platform").fetch_one(&pool).await.expect("read var");
    assert_eq!(platform, None, "platform flag leaked after rollback");
}

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn partial_unique_keys_allow_one_default_branch(pool: MySqlPool) {
    let a = create_tenant(&pool, "shop-a").await;
    let mut tx = pos_db::begin_tenant(&pool, &tenant_ctx(a)).await.expect("begin");
    let insert = "INSERT INTO branches (tenant_id, id, code, name, is_default) VALUES (?, ?, ?, 'B', ?)";
    sqlx::query(insert).bind(a).bind(new_id()).bind("B1").bind(true).execute(&mut *tx).await.expect("first default");
    sqlx::query(insert).bind(a).bind(new_id()).bind("B2").bind(false).execute(&mut *tx).await.expect("non-default");
    sqlx::query(insert).bind(a).bind(new_id()).bind("B3").bind(false).execute(&mut *tx).await.expect("non-default");
    let err = sqlx::query(insert)
        .bind(a)
        .bind(new_id())
        .bind("B4")
        .bind(true)
        .execute(&mut *tx)
        .await
        .expect_err("second default");
    assert!(matches!(pos_db::map_db_error(err), AppError::Conflict { code: "DUPLICATE", .. }));
}
