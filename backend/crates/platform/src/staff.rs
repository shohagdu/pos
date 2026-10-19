//! Platform staff (ADM-R11): `platform_owner` does everything, `billing`
//! verifies payments and manages overrides, `support` only reads.

use chrono::{DateTime, Utc};
use pos_core::{AppError, AppResult};
use pos_db::map_db_error;
use serde::Serialize;
use sqlx::MySqlPool;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum StaffRole {
    PlatformOwner,
    Billing,
    Support,
}

impl StaffRole {
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "platform_owner" => Self::PlatformOwner,
            "billing" => Self::Billing,
            "support" => Self::Support,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::PlatformOwner => "platform_owner",
            Self::Billing => "billing",
            Self::Support => "support",
        }
    }

    /// Suspend/reactivate businesses, edit plans and settings.
    pub fn can_administer(self) -> bool {
        self == Self::PlatformOwner
    }

    /// Verify payments, manage invoices and overrides.
    pub fn can_bill(self) -> bool {
        matches!(self, Self::PlatformOwner | Self::Billing)
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct Staff {
    pub user_id: Uuid,
    pub full_name: String,
    pub role: StaffRole,
    pub created_at: DateTime<Utc>,
}

impl Staff {
    pub fn require_admin(&self) -> AppResult<()> {
        if self.role.can_administer() { Ok(()) } else { Err(AppError::Forbidden) }
    }

    pub fn require_billing(&self) -> AppResult<()> {
        if self.role.can_bill() { Ok(()) } else { Err(AppError::Forbidden) }
    }
}

/// The active staff record of a user, if any.
pub async fn load(pool: &MySqlPool, user_id: Uuid) -> AppResult<Option<Staff>> {
    let row: Option<(Uuid, String, String, DateTime<Utc>)> = sqlx::query_as(
        "SELECT s.user_id, u.full_name, s.staff_role, s.created_at FROM platform_staff s \
         JOIN users u ON u.id = s.user_id WHERE s.user_id = ? AND s.status = 'active' AND u.status = 'active'",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(map_db_error)?;
    Ok(row.and_then(|(user_id, full_name, role, created_at)| {
        StaffRole::parse(&role).map(|role| Staff { user_id, full_name, role, created_at })
    }))
}

/// Grants (or changes) a staff role for the user with this phone or email.
/// Used by the `pos-api platform-staff` command to create the first owner.
pub async fn grant(pool: &MySqlPool, identifier: &str, role: StaffRole) -> AppResult<Uuid> {
    let user: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM users WHERE (phone = ? OR phone = ? OR email = ?) AND status = 'active'")
            .bind(identifier)
            .bind(normalise_bd_phone(identifier))
            .bind(identifier.to_lowercase())
            .fetch_optional(pool)
            .await
            .map_err(map_db_error)?;
    let user_id =
        user.ok_or_else(|| AppError::field("identifier", "NOT_FOUND", "no active user with that phone or email"))?;
    sqlx::query(
        "INSERT INTO platform_staff (user_id, staff_role, status) VALUES (?, ?, 'active') \
         ON DUPLICATE KEY UPDATE staff_role = VALUES(staff_role), status = 'active'",
    )
    .bind(user_id)
    .bind(role.as_str())
    .execute(pool)
    .await
    .map_err(map_db_error)?;
    Ok(user_id)
}

/// `01712345678` → `+8801712345678` (how users store phones).
fn normalise_bd_phone(value: &str) -> String {
    let digits: String = value.chars().filter(char::is_ascii_digit).collect();
    if digits.len() == 11 && digits.starts_with('0') { format!("+88{digits}") } else { value.to_owned() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles() {
        assert!(StaffRole::PlatformOwner.can_administer() && StaffRole::PlatformOwner.can_bill());
        assert!(!StaffRole::Billing.can_administer() && StaffRole::Billing.can_bill());
        assert!(!StaffRole::Support.can_administer() && !StaffRole::Support.can_bill());
        assert_eq!(StaffRole::parse("billing"), Some(StaffRole::Billing));
        assert_eq!(normalise_bd_phone("01700000001"), "+8801700000001");
    }
}
