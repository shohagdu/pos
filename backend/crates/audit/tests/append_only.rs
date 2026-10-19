use pos_audit::{AuditEntry, record};
use pos_core::{AppError, RequestMeta, TenantCtx, id::new_id};
use sqlx::MySqlPool;

#[sqlx::test(migrator = "pos_db::MIGRATOR")]
async fn audit_entries_are_written_and_cannot_be_changed(pool: MySqlPool) {
    let tenant_id = new_id();
    sqlx::query("INSERT INTO tenants (id, slug, name, status) VALUES (?, 'shop-a', 'Shop A', 'active')")
        .bind(tenant_id)
        .execute(&pool)
        .await
        .expect("tenant");

    let meta =
        RequestMeta { request_id: "req-1".into(), ip: "203.0.113.7".parse().ok(), user_agent: Some("test".into()) };
    let ctx = TenantCtx { tenant_id, user_id: new_id(), meta: meta.clone() };
    let mut tx = pos_db::begin_tenant(&pool, &ctx).await.expect("begin");
    let entity = new_id();
    let audit_id = record(&mut tx, &meta, AuditEntry::new("branch.created").entity("branch", entity).reason("test"))
        .await
        .expect("record");
    tx.commit().await.expect("commit");

    let (action, ip): (String, Option<String>) =
        sqlx::query_as("SELECT action, INET6_NTOA(ip_address) FROM audit_logs WHERE id = ? AND tenant_id = ?")
            .bind(audit_id)
            .bind(tenant_id)
            .fetch_one(&pool)
            .await
            .expect("read back");
    assert_eq!(action, "branch.created");
    assert_eq!(ip.as_deref(), Some("203.0.113.7"));

    for sql in ["UPDATE audit_logs SET action = 'tampered' WHERE id = ?", "DELETE FROM audit_logs WHERE id = ?"] {
        let err = sqlx::query(sql).bind(audit_id).execute(&pool).await.expect_err("append-only");
        let mapped = pos_db::map_db_error(err);
        assert!(
            matches!(&mapped, AppError::Internal(src) if src.to_string().contains("APPEND_ONLY")),
            "{sql}: {mapped:?}"
        );
    }
}
