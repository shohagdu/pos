//! Creating notifications: one row per recipient, written in the recipient's
//! language, plus a queued delivery for each extra channel they chose.

use pos_core::{AppResult, id::new_id};
use pos_db::{TenantTx, map_db_error};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    catalog::{self, Audience, EMAIL, IN_APP, SMS},
    prefs,
};

/// A notification's text in one language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Text {
    pub title: String,
    pub body: Option<String>,
}

/// What to send; `text` is called with each recipient's locale (`en`, `bn`).
pub struct Draft<'a> {
    pub notif_type: &'static str,
    pub link: Option<String>,
    pub data: Value,
    /// One notification per user and key (scheduled checks run repeatedly).
    pub dedupe_key: Option<String>,
    pub text: &'a (dyn Fn(&str) -> Text + Sync),
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Recipient {
    pub user_id: Uuid,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub locale: String,
}

const MEMBER_SELECT: &str = "SELECT DISTINCT u.id AS user_id, u.email, u.phone, u.locale FROM tenant_memberships m \
     JOIN users u ON u.id = m.user_id \
     WHERE m.tenant_id = ? AND m.status = 'active' AND u.status = 'active'";

/// Active members of the business who should receive `audience`
/// (`subject` is the user an event is about).
pub async fn recipients(tx: &mut TenantTx, audience: Audience, subject: Option<Uuid>) -> AppResult<Vec<Recipient>> {
    let tenant_id = tx.tenant_id();
    let rows = match audience {
        Audience::Owners => {
            sqlx::query_as(sqlx::AssertSqlSafe(format!("{MEMBER_SELECT} AND m.is_owner")))
                .bind(tenant_id)
                .fetch_all(&mut **tx)
                .await
        }
        Audience::Subject => {
            sqlx::query_as(sqlx::AssertSqlSafe(format!("{MEMBER_SELECT} AND m.user_id = ?")))
                .bind(tenant_id)
                .bind(subject)
                .fetch_all(&mut **tx)
                .await
        }
        Audience::Permission(permission) => {
            sqlx::query_as(sqlx::AssertSqlSafe(format!(
                "{MEMBER_SELECT} AND (m.is_owner OR EXISTS (SELECT 1 FROM membership_roles mr \
               JOIN role_permissions rp ON rp.tenant_id = mr.tenant_id AND rp.role_id = mr.role_id \
               WHERE mr.tenant_id = m.tenant_id AND mr.membership_id = m.id AND rp.permission_code = ?))"
            )))
            .bind(tenant_id)
            .bind(permission)
            .fetch_all(&mut **tx)
            .await
        }
    };
    rows.map_err(map_db_error)
}

/// Whether a notification with this key was already created in the business.
pub async fn already_sent(tx: &mut TenantTx, dedupe_key: &str) -> AppResult<bool> {
    let tenant_id = tx.tenant_id();
    let found: Option<i32> =
        sqlx::query_scalar("SELECT 1 FROM notifications WHERE tenant_id = ? AND dedupe_key = ? LIMIT 1")
            .bind(tenant_id)
            .bind(dedupe_key)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?;
    Ok(found.is_some())
}

/// Creates the notification for each recipient and queues its deliveries.
/// `sms_allowed` is the plan feature `notifications.sms`. Returns how many
/// notifications were created (duplicates by `dedupe_key` are skipped).
pub async fn publish(tx: &mut TenantTx, draft: &Draft<'_>, to: &[Recipient], sms_allowed: bool) -> AppResult<usize> {
    let tenant_id = tx.tenant_id();
    let defaults = catalog::find(draft.notif_type).map(|t| t.default_channels).unwrap_or(&[IN_APP]);
    let mut created = 0;
    for r in to {
        let channels = prefs::channels_for(tx, r.user_id, draft.notif_type, defaults).await?;
        if channels.is_empty() {
            continue;
        }
        if let Some(key) = &draft.dedupe_key {
            let seen: Option<i32> = sqlx::query_scalar(
                "SELECT 1 FROM notifications WHERE tenant_id = ? AND user_id = ? AND dedupe_key = ? LIMIT 1",
            )
            .bind(tenant_id)
            .bind(r.user_id)
            .bind(key)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?;
            if seen.is_some() {
                continue;
            }
        }
        let text = (draft.text)(&r.locale);
        let id = new_id();
        // Not wanted in the app: kept (deliveries point at it) but already read.
        let read = !channels.iter().any(|c| c == IN_APP);
        let inserted = sqlx::query(
            "INSERT INTO notifications (tenant_id, id, user_id, notif_type, title, body, link, data, dedupe_key, read_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, IF(?, UTC_TIMESTAMP(6), NULL)) \
             ON DUPLICATE KEY UPDATE id = id",
        )
        .bind(tenant_id)
        .bind(id)
        .bind(r.user_id)
        .bind(draft.notif_type)
        .bind(text.title.chars().take(200).collect::<String>())
        .bind(text.body.as_ref().map(|b| b.chars().take(1000).collect::<String>()))
        .bind(&draft.link)
        .bind(sqlx::types::Json(&draft.data))
        .bind(&draft.dedupe_key)
        .bind(read)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
        if inserted.rows_affected() == 0 {
            continue;
        }
        created += 1;
        let mut deliveries: Vec<(&str, &str)> = Vec::new();
        if channels.iter().any(|c| c == EMAIL)
            && let Some(email) = r.email.as_deref().filter(|e| !e.is_empty())
        {
            deliveries.push((EMAIL, email));
        }
        if sms_allowed
            && channels.iter().any(|c| c == SMS)
            && let Some(phone) = r.phone.as_deref().filter(|p| !p.is_empty())
        {
            deliveries.push((SMS, phone));
        }
        for (channel, destination) in deliveries {
            sqlx::query(
                "INSERT INTO notification_deliveries (tenant_id, id, notification_id, channel, destination) VALUES (?, ?, ?, ?, ?)",
            )
            .bind(tenant_id)
            .bind(new_id())
            .bind(id)
            .bind(channel)
            .bind(destination)
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
        }
    }
    Ok(created)
}
