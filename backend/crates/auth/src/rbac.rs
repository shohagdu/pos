//! Permission catalogue, system role templates (Requirement Analysis §10)
//! and the per-request permission lookup.

use std::collections::BTreeMap;

use pos_core::{AppError, AppResult};
use pos_db::map_db_error;
use serde::Serialize;
use sqlx::MySqlPool;
use uuid::Uuid;

/// Every permission code. Must match the `permissions` seed in migration 0005
/// (checked by a database test).
pub const PERMISSIONS: &[&str] = &[
    "dashboard.view",
    "dashboard.view_financials",
    "pos.sell",
    "pos.price_override",
    "pos.discount_over_limit",
    "pos.override_stock",
    "pos.below_min_price",
    "sales.view",
    "sales.view_all_users",
    "sales.cancel",
    "sales.return",
    "sales.return_override",
    "sales.override_credit_limit",
    "sales.backdate",
    "products.view",
    "products.create",
    "products.update",
    "products.archive",
    "products.import",
    "products.view_cost",
    "inventory.view",
    "inventory.adjust",
    "inventory.opening_stock",
    "inventory.transfer",
    "inventory.transfer_receive",
    "inventory.count",
    "purchases.view",
    "purchases.create",
    "purchases.post",
    "purchases.approve",
    "purchases.cancel",
    "purchases.return",
    "customers.view",
    "customers.manage",
    "customers.collect_payment",
    "suppliers.view",
    "suppliers.manage",
    "suppliers.pay",
    "accounts.view",
    "accounts.manage",
    "payments.void",
    "transfers.create",
    "expenses.view",
    "expenses.create",
    "expenses.approve",
    "income.create",
    "accounting.view",
    "accounting.journal_create",
    "accounting.period_lock",
    "reports.sales",
    "reports.purchases",
    "reports.inventory",
    "reports.parties",
    "reports.financial",
    "reports.export",
    "users.view",
    "users.invite",
    "users.manage_roles",
    "users.revoke_sessions",
    "settings.view",
    "settings.manage",
    "branches.manage",
    "subscription.manage",
    "audit.view",
];

pub struct RoleTemplate {
    pub code: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    /// Discount limit for the POS; `None` = the tenant default (`cashier_max_discount_pct`).
    pub max_discount_pct: Option<u32>,
    grants: Grants,
}

enum Grants {
    All,
    AllExcept(&'static [&'static str]),
    Only(&'static [&'static str]),
}

impl RoleTemplate {
    pub fn permissions(&self) -> Vec<&'static str> {
        match self.grants {
            Grants::All => PERMISSIONS.to_vec(),
            Grants::AllExcept(denied) => PERMISSIONS.iter().copied().filter(|p| !denied.contains(p)).collect(),
            Grants::Only(granted) => granted.to_vec(),
        }
    }
}

pub const OWNER_ROLE: &str = "owner";

/// The 8 system roles created for every tenant (§10.2). Branch scoping ("B" in
/// the matrix) is applied per assignment, not per role.
pub const SYSTEM_ROLES: &[RoleTemplate] = &[
    RoleTemplate {
        code: OWNER_ROLE,
        name: "Owner",
        description: "Full access, including subscription and ownership",
        max_discount_pct: Some(100),
        grants: Grants::All,
    },
    RoleTemplate {
        code: "admin",
        name: "Admin",
        description: "Full access except subscription; cannot remove the owner",
        max_discount_pct: Some(100),
        grants: Grants::AllExcept(&["subscription.manage"]),
    },
    RoleTemplate {
        code: "branch_manager",
        name: "Branch Manager",
        description: "Runs a branch: sales, stock, purchases, parties, money and reports",
        max_discount_pct: Some(25),
        grants: Grants::Only(&[
            "dashboard.view",
            "dashboard.view_financials",
            "pos.sell",
            "pos.price_override",
            "pos.discount_over_limit",
            "sales.view",
            "sales.view_all_users",
            "sales.cancel",
            "sales.return",
            "products.view",
            "products.view_cost",
            "inventory.view",
            "inventory.adjust",
            "inventory.count",
            "inventory.transfer",
            "inventory.transfer_receive",
            "purchases.view",
            "purchases.create",
            "purchases.post",
            "customers.view",
            "customers.manage",
            "customers.collect_payment",
            "suppliers.view",
            "suppliers.manage",
            "suppliers.pay",
            "accounts.view",
            "accounts.manage",
            "transfers.create",
            "expenses.view",
            "expenses.create",
            "expenses.approve",
            "income.create",
            "reports.sales",
            "reports.purchases",
            "reports.inventory",
            "reports.parties",
            "reports.financial",
            "reports.export",
        ]),
    },
    RoleTemplate {
        code: "cashier",
        name: "Cashier",
        description: "Sells at the POS, processes returns and collects customer payments",
        max_discount_pct: None,
        grants: Grants::Only(&[
            "pos.sell",
            "sales.view",
            "sales.return",
            "products.view",
            "inventory.view",
            "customers.view",
            "customers.manage",
            "customers.collect_payment",
        ]),
    },
    RoleTemplate {
        code: "salesperson",
        name: "Salesperson",
        description: "Sells at the POS and manages customers",
        max_discount_pct: None,
        grants: Grants::Only(&[
            "pos.sell",
            "sales.view",
            "products.view",
            "inventory.view",
            "customers.view",
            "customers.manage",
        ]),
    },
    RoleTemplate {
        code: "store_manager",
        name: "Store Manager",
        description: "Manages the catalogue, stock and purchasing",
        max_discount_pct: None,
        grants: Grants::Only(&[
            "dashboard.view",
            "products.view",
            "products.create",
            "products.update",
            "products.archive",
            "products.import",
            "products.view_cost",
            "inventory.view",
            "inventory.adjust",
            "inventory.opening_stock",
            "inventory.count",
            "inventory.transfer",
            "inventory.transfer_receive",
            "purchases.view",
            "purchases.create",
            "purchases.post",
            "suppliers.view",
            "suppliers.manage",
            "reports.inventory",
            "reports.export",
        ]),
    },
    RoleTemplate {
        code: "warehouse",
        name: "Warehouse",
        description: "Receives, counts and moves stock",
        max_discount_pct: None,
        grants: Grants::Only(&[
            "products.view",
            "inventory.view",
            "inventory.adjust",
            "inventory.count",
            "inventory.transfer",
            "inventory.transfer_receive",
            "purchases.view",
            "purchases.create",
        ]),
    },
    RoleTemplate {
        code: "accountant",
        name: "Accountant",
        description: "Money, parties, accounting and financial reports",
        max_discount_pct: None,
        grants: Grants::Only(&[
            "dashboard.view",
            "dashboard.view_financials",
            "sales.view",
            "sales.view_all_users",
            "products.view",
            "products.view_cost",
            "inventory.view",
            "purchases.view",
            "customers.view",
            "customers.manage",
            "customers.collect_payment",
            "suppliers.view",
            "suppliers.manage",
            "suppliers.pay",
            "accounts.view",
            "accounts.manage",
            "payments.void",
            "transfers.create",
            "expenses.view",
            "expenses.create",
            "expenses.approve",
            "income.create",
            "accounting.view",
            "accounting.journal_create",
            "accounting.period_lock",
            "reports.sales",
            "reports.purchases",
            "reports.inventory",
            "reports.parties",
            "reports.financial",
            "reports.export",
        ]),
    },
];

/// Where a permission applies (R-TEN-04).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "branch_ids")]
pub enum BranchScope {
    All,
    Branches(Vec<Uuid>),
}

impl BranchScope {
    fn widen(&mut self, branch: Option<Uuid>) {
        match (self, branch) {
            (scope @ Self::Branches(_), None) => *scope = Self::All,
            (Self::Branches(ids), Some(id)) if !ids.contains(&id) => ids.push(id),
            _ => {}
        }
    }
}

/// What the current member may do in the active tenant, loaded per request
/// so that role changes and deactivation apply immediately (AUTH-R07/R08).
#[derive(Debug, Clone)]
pub struct TenantAccess {
    pub tenant_id: Uuid,
    pub membership_id: Uuid,
    pub user_id: Uuid,
    pub is_owner: bool,
    pub perms_version: i32,
    pub permissions: BTreeMap<String, BranchScope>,
}

impl TenantAccess {
    pub fn can(&self, permission: &str) -> bool {
        self.permissions.contains_key(permission)
    }

    pub fn require(&self, permission: &'static str) -> AppResult<()> {
        if self.can(permission) { Ok(()) } else { Err(AppError::Forbidden) }
    }
}

/// Loads the member's access for a verified token. Errors: membership or
/// tenant gone/inactive, or the token's permission version is stale.
pub async fn load_access(
    pool: &MySqlPool,
    tenant_id: Uuid,
    membership_id: Uuid,
    user_id: Uuid,
    token_perms_version: i32,
) -> AppResult<TenantAccess> {
    let row: Option<(String, i32, bool, String)> = sqlx::query_as(
        "SELECT m.status, m.perms_version, m.is_owner, t.status FROM tenant_memberships m \
         JOIN tenants t ON t.id = m.tenant_id \
         WHERE m.tenant_id = ? AND m.id = ? AND m.user_id = ?",
    )
    .bind(tenant_id)
    .bind(membership_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(map_db_error)?;

    let Some((member_status, perms_version, is_owner, tenant_status)) = row else {
        return Err(AppError::AuthFailed { code: "MEMBERSHIP_INVALID", message: "you are no longer a member".into() });
    };
    check_tenant_status(&tenant_status)?;
    if member_status != "active" {
        return Err(AppError::AuthFailed { code: "MEMBERSHIP_INACTIVE", message: "your access was suspended".into() });
    }
    if perms_version != token_perms_version {
        return Err(AppError::AuthFailed {
            code: "TOKEN_STALE",
            message: "permissions changed, refresh the token".into(),
        });
    }

    let grants: Vec<(String, Option<Uuid>)> = sqlx::query_as(
        "SELECT rp.permission_code, mr.branch_id FROM membership_roles mr \
         JOIN role_permissions rp ON rp.tenant_id = mr.tenant_id AND rp.role_id = mr.role_id \
         WHERE mr.tenant_id = ? AND mr.membership_id = ?",
    )
    .bind(tenant_id)
    .bind(membership_id)
    .fetch_all(pool)
    .await
    .map_err(map_db_error)?;

    let mut permissions: BTreeMap<String, BranchScope> = BTreeMap::new();
    for (code, branch) in grants {
        permissions
            .entry(code)
            .and_modify(|scope| scope.widen(branch))
            .or_insert_with(|| branch.map_or(BranchScope::All, |id| BranchScope::Branches(vec![id])));
    }
    Ok(TenantAccess { tenant_id, membership_id, user_id, is_owner, perms_version, permissions })
}

/// Tenants that are not active cannot be used (Requirement Analysis §9.5).
pub fn check_tenant_status(status: &str) -> AppResult<()> {
    match status {
        "active" => Ok(()),
        "suspended" => Err(AppError::Denied {
            code: "TENANT_SUSPENDED",
            message: "this business is suspended; the owner can renew the subscription".into(),
        }),
        _ => Err(AppError::NotFound),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn templates_only_reference_catalogue_codes() {
        let catalogue: HashSet<_> = PERMISSIONS.iter().collect();
        assert_eq!(catalogue.len(), PERMISSIONS.len(), "duplicate permission code");
        for role in SYSTEM_ROLES {
            let perms = role.permissions();
            assert!(!perms.is_empty(), "{} has no permissions", role.code);
            for p in &perms {
                assert!(catalogue.contains(p), "{}: unknown permission {p}", role.code);
            }
            assert_eq!(perms.iter().collect::<HashSet<_>>().len(), perms.len(), "{}: duplicates", role.code);
        }
    }

    #[test]
    fn matrix_highlights() {
        let perms = |code: &str| {
            SYSTEM_ROLES.iter().find(|r| r.code == code).map(RoleTemplate::permissions).unwrap_or_default()
        };
        assert_eq!(perms("owner").len(), PERMISSIONS.len());
        assert!(!perms("admin").contains(&"subscription.manage"));
        assert!(perms("admin").contains(&"users.manage_roles"));
        assert!(perms("cashier").contains(&"pos.sell"));
        assert!(!perms("cashier").contains(&"sales.cancel"));
        assert!(!perms("store_manager").contains(&"pos.sell"));
        assert!(perms("accountant").contains(&"payments.void"));
        assert!(!perms("branch_manager").contains(&"payments.void"));
        assert_eq!(SYSTEM_ROLES.len(), 8);
    }

    #[test]
    fn unscoped_grant_widens_branch_scope() {
        let (a, b) = (Uuid::now_v7(), Uuid::now_v7());
        let mut scope = BranchScope::Branches(vec![a]);
        scope.widen(Some(b));
        scope.widen(Some(a));
        assert_eq!(scope, BranchScope::Branches(vec![a, b]));
        scope.widen(None);
        assert_eq!(scope, BranchScope::All);
    }
}
