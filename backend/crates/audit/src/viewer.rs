//! Audit log viewer (AUD-R03, E1.7): the business's own entries, newest
//! first, filtered by user, action, entity, branch and time.
//!
//! Entries recorded by platform staff or the system about this business
//! (support sessions, subscription changes) are included (R-TEN-05).

use std::net::IpAddr;

use chrono::{DateTime, Utc};
use pos_core::{
    AppError, AppResult,
    page::{self, Keyset},
};
use pos_db::{TenantTx, map_db_error};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

/// Query string of `GET /audit-logs`.
#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct AuditFilter {
    /// The user who acted (`users.id`, as in `Member.user_id`).
    pub actor_id: Option<Uuid>,
    /// Exact action, e.g. `member.suspended`, or a prefix ending in `*`, e.g. `member.*`.
    pub action: Option<String>,
    pub entity_type: Option<String>,
    pub entity_id: Option<Uuid>,
    pub branch_id: Option<Uuid>,
    /// Inclusive lower bound (RFC 3339).
    pub from: Option<DateTime<Utc>>,
    /// Exclusive upper bound (RFC 3339).
    pub to: Option<DateTime<Utc>>,
    /// Page size, 1–200 (default 50).
    pub limit: Option<u32>,
    /// `next_cursor` of the previous page.
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct AuditRecord {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    /// `user`, `platform_staff`, `system` or `api_key`.
    pub actor_type: String,
    pub actor_id: Option<Uuid>,
    /// The member's name in this business, for `user` actors.
    pub actor_name: Option<String>,
    pub action: String,
    pub entity_type: Option<String>,
    pub entity_id: Option<Uuid>,
    pub branch_id: Option<Uuid>,
    pub before: Option<Value>,
    pub after: Option<Value>,
    pub reason: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct AuditPage {
    pub data: Vec<AuditRecord>,
    /// Pass as `cursor` for the next page; `null` on the last page.
    pub next_cursor: Option<String>,
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    created_at: DateTime<Utc>,
    actor_type: String,
    actor_id: Option<Uuid>,
    actor_name: Option<String>,
    action: String,
    entity_type: Option<String>,
    entity_id: Option<Uuid>,
    branch_id: Option<Uuid>,
    before_data: Option<Value>,
    after_data: Option<Value>,
    reason: Option<String>,
    ip_address: Option<Vec<u8>>,
    user_agent: Option<String>,
    request_id: Option<String>,
}

impl From<Row> for AuditRecord {
    fn from(r: Row) -> Self {
        Self {
            id: r.id,
            created_at: r.created_at,
            actor_type: r.actor_type,
            actor_id: r.actor_id,
            actor_name: r.actor_name,
            action: r.action,
            entity_type: r.entity_type,
            entity_id: r.entity_id,
            branch_id: r.branch_id,
            before: r.before_data,
            after: r.after_data,
            reason: r.reason,
            ip_address: r.ip_address.as_deref().and_then(ip_to_string),
            user_agent: r.user_agent,
            request_id: r.request_id,
        }
    }
}

fn ip_to_string(bytes: &[u8]) -> Option<String> {
    let ip = match bytes.len() {
        4 => IpAddr::from(<[u8; 4]>::try_from(bytes).ok()?),
        16 => IpAddr::from(<[u8; 16]>::try_from(bytes).ok()?),
        _ => return None,
    };
    Some(ip.to_string())
}

/// `member.*` → prefix `member.`; anything else is an exact match.
enum ActionMatch<'a> {
    Exact(&'a str),
    Prefix(&'a str),
}

fn action_match(action: &str) -> AppResult<ActionMatch<'_>> {
    let valid = |s: &str| s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '.');
    let parsed = match action.strip_suffix('*') {
        Some(prefix) => ActionMatch::Prefix(prefix),
        None => ActionMatch::Exact(action),
    };
    let text = match parsed {
        ActionMatch::Exact(s) | ActionMatch::Prefix(s) => s,
    };
    if text.is_empty() || text.len() > 60 || !valid(text) {
        return Err(AppError::field(
            "action",
            "INVALID_ACTION",
            "use an action code such as member.suspended or member.*",
        ));
    }
    Ok(parsed)
}

/// One page of the current business's audit log.
pub async fn list(tx: &mut TenantTx, filter: &AuditFilter) -> AppResult<AuditPage> {
    let limit = page::limit(filter.limit)?;
    let after = filter.cursor.as_deref().map(Keyset::decode).transpose()?;
    let action = filter.action.as_deref().map(action_match).transpose()?;
    if let (Some(from), Some(to)) = (filter.from, filter.to)
        && from >= to
    {
        return Err(AppError::field("to", "INVALID_RANGE", "to must be later than from"));
    }

    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "SELECT a.id, a.created_at, a.actor_type, a.actor_id, \
         CASE WHEN a.actor_type = 'user' THEN COALESCE(m.display_name, u.full_name) END AS actor_name, \
         a.action, a.entity_type, a.entity_id, a.branch_id, a.before_data, a.after_data, a.reason, \
         a.ip_address, a.user_agent, a.request_id FROM audit_logs a \
         LEFT JOIN tenant_memberships m ON m.tenant_id = a.tenant_id AND m.user_id = a.actor_id \
         LEFT JOIN users u ON u.id = a.actor_id \
         WHERE a.tenant_id = ",
    );
    q.push_bind(tenant_id);
    if let Some(actor_id) = filter.actor_id {
        q.push(" AND a.actor_id = ").push_bind(actor_id);
    }
    match action {
        Some(ActionMatch::Exact(a)) => {
            q.push(" AND a.action = ").push_bind(a.to_owned());
        }
        // Codes are [a-z0-9_.]; `_` is a LIKE wildcard, so escape it.
        Some(ActionMatch::Prefix(p)) => {
            q.push(" AND a.action LIKE ").push_bind(format!("{}%", p.replace('_', "\\_")));
        }
        None => {}
    }
    if let Some(entity_type) = &filter.entity_type {
        q.push(" AND a.entity_type = ").push_bind(entity_type.clone());
    }
    if let Some(entity_id) = filter.entity_id {
        q.push(" AND a.entity_id = ").push_bind(entity_id);
    }
    if let Some(branch_id) = filter.branch_id {
        q.push(" AND a.branch_id = ").push_bind(branch_id);
    }
    if let Some(from) = filter.from {
        q.push(" AND a.created_at >= ").push_bind(from);
    }
    if let Some(to) = filter.to {
        q.push(" AND a.created_at < ").push_bind(to);
    }
    if let Some(k) = after {
        q.push(" AND (a.created_at < ").push_bind(k.at);
        q.push(" OR (a.created_at = ").push_bind(k.at).push(" AND a.id < ").push_bind(k.id).push("))");
    }
    // One extra row tells whether another page exists.
    q.push(" ORDER BY a.created_at DESC, a.id DESC LIMIT ").push_bind(limit + 1);

    let mut rows: Vec<Row> = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    let next_cursor = if rows.len() > limit as usize {
        rows.truncate(limit as usize);
        rows.last().map(|r| Keyset { at: r.created_at, id: r.id }.encode())
    } else {
        None
    };
    Ok(AuditPage { data: rows.into_iter().map(AuditRecord::from).collect(), next_cursor })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_filter_forms() {
        assert!(matches!(action_match("member.suspended"), Ok(ActionMatch::Exact("member.suspended"))));
        assert!(matches!(action_match("member.*"), Ok(ActionMatch::Prefix("member."))));
        for bad in ["", "*", "member%", "Member.x", "a'b"] {
            assert!(action_match(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn ip_addresses_render() {
        assert_eq!(ip_to_string(&[203, 0, 113, 7]).as_deref(), Some("203.0.113.7"));
        let mut v6 = [0u8; 16];
        v6[15] = 1;
        assert_eq!(ip_to_string(&v6).as_deref(), Some("::1"));
        assert_eq!(ip_to_string(&[1, 2]), None);
    }
}
