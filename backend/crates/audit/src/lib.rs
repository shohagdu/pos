//! Audit log writer (Requirement Analysis AUD-R01…R03, DB design §9.6).
//!
//! Entries are written inside the caller's transaction, so an audit record
//! exists if and only if the business change was committed.

pub mod idempotency;
pub mod viewer;

use pos_core::{AppResult, RequestMeta, context::truncate_chars, id::new_id};
use pos_db::{PlatformTx, TenantTx, map_db_error};
use serde_json::Value;
use sqlx::MySqlConnection;
use uuid::Uuid;

/// One auditable action, e.g. `AuditEntry::new("product.updated").entity("product", id)`.
#[derive(Debug, Clone, Default)]
pub struct AuditEntry {
    pub action: &'static str,
    pub entity_type: Option<&'static str>,
    pub entity_id: Option<Uuid>,
    pub branch_id: Option<Uuid>,
    pub before: Option<Value>,
    pub after: Option<Value>,
    pub reason: Option<String>,
}

impl AuditEntry {
    pub fn new(action: &'static str) -> Self {
        Self { action, ..Self::default() }
    }

    pub fn entity(mut self, entity_type: &'static str, id: Uuid) -> Self {
        self.entity_type = Some(entity_type);
        self.entity_id = Some(id);
        self
    }

    pub fn branch(mut self, branch_id: Uuid) -> Self {
        self.branch_id = Some(branch_id);
        self
    }

    pub fn change(mut self, before: Option<Value>, after: Option<Value>) -> Self {
        self.before = before;
        self.after = after;
        self
    }

    pub fn reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }
}

/// Records an action by a tenant user.
pub async fn record(tx: &mut TenantTx, meta: &RequestMeta, entry: AuditEntry) -> AppResult<Uuid> {
    let (tenant_id, user_id) = (tx.tenant_id(), tx.user_id());
    insert(tx, Some(tenant_id), "user", Some(user_id), meta, entry).await
}

/// Records a platform-level action (`tenant_id` is the affected tenant, or None).
pub async fn record_platform(
    tx: &mut PlatformTx,
    tenant_id: Option<Uuid>,
    actor_id: Option<Uuid>,
    meta: &RequestMeta,
    entry: AuditEntry,
) -> AppResult<Uuid> {
    let actor_type = if actor_id.is_some() { "platform_staff" } else { "system" };
    insert(tx, tenant_id, actor_type, actor_id, meta, entry).await
}

async fn insert(
    conn: &mut MySqlConnection,
    tenant_id: Option<Uuid>,
    actor_type: &'static str,
    actor_id: Option<Uuid>,
    meta: &RequestMeta,
    entry: AuditEntry,
) -> AppResult<Uuid> {
    let id = new_id();
    sqlx::query(
        "INSERT INTO audit_logs (id, tenant_id, actor_type, actor_id, action, entity_type, entity_id, \
         branch_id, before_data, after_data, reason, ip_address, user_agent, request_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(tenant_id)
    .bind(actor_type)
    .bind(actor_id)
    .bind(entry.action)
    .bind(entry.entity_type)
    .bind(entry.entity_id)
    .bind(entry.branch_id)
    .bind(entry.before)
    .bind(entry.after)
    .bind(entry.reason)
    .bind(meta.ip_bytes())
    .bind(meta.user_agent_truncated())
    .bind(truncate_chars(&meta.request_id, 64))
    .execute(conn)
    .await
    .map_err(map_db_error)?;
    Ok(id)
}
