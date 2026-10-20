//! Notification types (NOT-R01).

/// Who receives a type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Audience {
    /// Active members holding this permission (owners always do).
    Permission(&'static str),
    /// Business owners only.
    Owners,
    /// Only the user the event is about (security alerts).
    Subject,
}

#[derive(Debug, Clone, Copy)]
pub struct NotifType {
    pub code: &'static str,
    pub audience: Audience,
    /// Channels used until the user chooses.
    pub default_channels: &'static [&'static str],
}

pub const IN_APP: &str = "in_app";
pub const EMAIL: &str = "email";
pub const SMS: &str = "sms";
pub const CHANNELS: [&str; 3] = [IN_APP, EMAIL, SMS];

pub const STOCK_LOW: &str = "stock.low";
pub const CUSTOMER_DUE: &str = "customer.due";
pub const SUPPLIER_DUE: &str = "supplier.due";
pub const SUBSCRIPTION_EXPIRING: &str = "subscription.expiring";
pub const SALES_DAILY: &str = "sales.daily_summary";
pub const SECURITY: &str = "security.alert";
pub const APPROVAL: &str = "approval.request";

pub const TYPES: [NotifType; 7] = [
    NotifType { code: STOCK_LOW, audience: Audience::Permission("inventory.view"), default_channels: &[IN_APP] },
    NotifType { code: CUSTOMER_DUE, audience: Audience::Permission("customers.view"), default_channels: &[IN_APP] },
    NotifType { code: SUPPLIER_DUE, audience: Audience::Permission("suppliers.view"), default_channels: &[IN_APP] },
    NotifType { code: SUBSCRIPTION_EXPIRING, audience: Audience::Owners, default_channels: &[IN_APP, EMAIL] },
    NotifType { code: SALES_DAILY, audience: Audience::Permission("reports.sales"), default_channels: &[IN_APP] },
    NotifType { code: SECURITY, audience: Audience::Subject, default_channels: &[IN_APP, EMAIL] },
    // Approvals carry the permission per document (purchases.approve, expenses.approve).
    NotifType { code: APPROVAL, audience: Audience::Permission("purchases.approve"), default_channels: &[IN_APP] },
];

pub fn find(code: &str) -> Option<&'static NotifType> {
    TYPES.iter().find(|t| t.code == code)
}
