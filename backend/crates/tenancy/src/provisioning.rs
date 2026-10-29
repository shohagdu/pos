//! Tenant provisioning (ONB-R01, DB design §10.7): one transaction creates the
//! business and all of its defaults, so a failure leaves no rows (AC-ONB-02).
//!
//! Defaults whose tables arrive with later modules (payment methods and the
//! cash account with money, the walk-in customer with parties, expense
//! categories with expenses) are added to this transaction by those modules.

use pos_audit::AuditEntry;
use pos_auth::rbac::SYSTEM_ROLES;
use pos_core::{AppError, AppResult, RequestMeta, TenantCtx, id::new_id};
use pos_db::{TenantTx, map_db_error};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::{onboarding, slug};

/// A user may own at most this many businesses (abuse guard; staff memberships are not counted).
pub const MAX_OWNED_TENANTS: i64 = 10;

pub const BUSINESS_TYPES: &[&str] =
    &["retail", "grocery", "fashion", "electronics", "wholesale", "pharmacy", "restaurant", "other"];

/// Wizard step 2: the business profile.
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct NewBusiness {
    pub name: String,
    pub business_type: String,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub district: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub bin_number: Option<String>,
    #[serde(default)]
    pub trade_license_no: Option<String>,
    /// UI language for the business; defaults to the owner's.
    #[serde(default)]
    pub locale: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ProvisionedTenant {
    pub tenant_id: Uuid,
    pub slug: String,
    pub name: String,
    pub branch_id: Uuid,
    pub membership_id: Uuid,
}

/// (doc_type, prefix, per branch?, with period?) — DB design §13.2. Document
/// numbers are per branch (D-08); journal numbers are per tenant with a period;
/// party and product codes are per tenant without a period.
pub(crate) const SEQUENCES: &[(&str, &str, bool, bool)] = &[
    ("sale", "INV", true, true),
    ("sale_return", "RET", true, true),
    ("purchase", "PUR", true, true),
    ("purchase_return", "PRT", true, true),
    ("goods_receipt", "GRN", true, true),
    ("payment_in", "RCV", true, true),
    ("payment_out", "PMT", true, true),
    ("expense", "EXP", true, true),
    ("adjustment", "ADJ", true, true),
    ("transfer", "TRF", true, true),
    ("count", "CNT", true, true),
    ("income", "INC", true, true),
    ("journal", "JV", false, true),
    ("customer", "C", false, false),
    ("supplier", "S", false, false),
    ("product", "P", false, false),
];

pub(crate) fn clean(value: Option<String>, field: &str, max: usize) -> AppResult<Option<String>> {
    let Some(value) = value.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty()) else {
        return Ok(None);
    };
    if value.chars().count() > max {
        return Err(AppError::field(field, "TOO_LONG", format!("use at most {max} characters")));
    }
    Ok(Some(value))
}

pub(crate) fn validate_name(name: &str, field: &str, max: usize) -> AppResult<String> {
    let name = name.trim();
    let len = name.chars().count();
    if len < 2 || len > max {
        return Err(AppError::field(field, "INVALID_NAME", format!("use 2–{max} characters")));
    }
    Ok(name.to_owned())
}

pub(crate) fn validate_business_type(value: &str) -> AppResult<()> {
    if BUSINESS_TYPES.contains(&value) {
        Ok(())
    } else {
        Err(AppError::field("business_type", "INVALID_BUSINESS_TYPE", "choose a business type from the list"))
    }
}

pub(crate) fn validate_contact(
    phone: Option<String>,
    email: Option<String>,
) -> AppResult<(Option<String>, Option<String>)> {
    let phone = match clean(phone, "phone", 20)? {
        Some(raw) => Some(
            pos_auth::identifier::normalize_phone(&raw)
                .ok_or_else(|| AppError::field("phone", "INVALID_PHONE", "enter a valid phone number"))?,
        ),
        None => None,
    };
    let email = match clean(email, "email", 254)? {
        Some(raw) => Some(
            pos_auth::identifier::normalize_email(&raw)
                .ok_or_else(|| AppError::field("email", "INVALID_EMAIL", "enter a valid email address"))?,
        ),
        None => None,
    };
    Ok((phone, email))
}

/// Creates a business owned by `user_id`. The owner must have verified their phone.
pub async fn provision(
    pool: &MySqlPool,
    user_id: Uuid,
    input: NewBusiness,
    meta: &RequestMeta,
) -> AppResult<ProvisionedTenant> {
    let name = validate_name(&input.name, "name", 200)?;
    validate_business_type(&input.business_type)?;
    let (phone, email) = validate_contact(input.phone, input.email)?;
    let address = clean(input.address, "address", 500)?;
    let district = clean(input.district, "district", 100)?;
    let bin_number = clean(input.bin_number, "bin_number", 30)?;
    let trade_license_no = clean(input.trade_license_no, "trade_license_no", 50)?;

    let owner: Option<(String, bool, String)> = sqlx::query_as(
        "SELECT full_name, phone_verified_at IS NOT NULL, locale FROM users WHERE id = ? AND status = 'active'",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(map_db_error)?;
    let Some((owner_name, phone_verified, owner_locale)) = owner else {
        return Err(AppError::Unauthorized);
    };
    if !phone_verified {
        return Err(AppError::Denied {
            code: "PHONE_NOT_VERIFIED",
            message: "verify your mobile number before creating a business".into(),
        });
    }
    let locale = match input.locale.as_deref() {
        None => owner_locale,
        Some(l @ ("en" | "bn" | "it")) => l.to_owned(),
        Some(_) => return Err(AppError::field("locale", "INVALID_LOCALE", "language must be en, bn or it")),
    };

    let owned: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tenant_memberships m JOIN tenants t ON t.id = m.tenant_id \
         WHERE m.user_id = ? AND m.is_owner AND t.status <> 'terminated' \
         /* tenant-lint: allow the caller's own memberships across tenants (DB design §3.4) */",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(map_db_error)?;
    if owned >= MAX_OWNED_TENANTS {
        return Err(AppError::Unprocessable {
            code: "TENANT_LIMIT",
            message: format!("you can own at most {MAX_OWNED_TENANTS} businesses"),
        });
    }

    let base = slug::base(&name);
    let mut candidate = base.clone();
    for attempt in 0..5 {
        let taken: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM tenants WHERE slug = ?)")
            .bind(&candidate)
            .fetch_one(pool)
            .await
            .map_err(map_db_error)?;
        if !taken {
            let profile = Profile {
                name: &name,
                slug: &candidate,
                business_type: &input.business_type,
                locale: &locale,
                phone: phone.as_deref(),
                email: email.as_deref(),
                address: address.as_deref(),
                district: district.as_deref(),
                bin_number: bin_number.as_deref(),
                trade_license_no: trade_license_no.as_deref(),
                owner_name: &owner_name,
            };
            match provision_tx(pool, user_id, &profile, meta).await {
                // A concurrent sign-up took the slug between the check and the insert.
                Err(AppError::Conflict { code: "DUPLICATE", .. }) if attempt < 4 => {}
                other => return other,
            }
        }
        candidate = slug::with_suffix(&base)?;
    }
    Err(AppError::Conflict { code: "SLUG_UNAVAILABLE", message: "could not find a free business address".into() })
}

struct Profile<'a> {
    name: &'a str,
    slug: &'a str,
    business_type: &'a str,
    locale: &'a str,
    phone: Option<&'a str>,
    email: Option<&'a str>,
    address: Option<&'a str>,
    district: Option<&'a str>,
    bin_number: Option<&'a str>,
    trade_license_no: Option<&'a str>,
    owner_name: &'a str,
}

async fn provision_tx(
    pool: &MySqlPool,
    user_id: Uuid,
    p: &Profile<'_>,
    meta: &RequestMeta,
) -> AppResult<ProvisionedTenant> {
    let tenant_id = new_id();
    let ctx = TenantCtx { tenant_id, user_id, meta: meta.clone() };
    let mut tx = pos_db::begin_tenant(pool, &ctx).await?;

    sqlx::query(
        "INSERT INTO tenants (id, slug, name, business_type, status, locale, phone, email, address, district, \
         bin_number, trade_license_no) VALUES (?, ?, ?, ?, 'provisioning', ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(p.slug)
    .bind(p.name)
    .bind(p.business_type)
    .bind(p.locale)
    .bind(p.phone)
    .bind(p.email)
    .bind(p.address)
    .bind(p.district)
    .bind(p.bin_number)
    .bind(p.trade_license_no)
    .execute(&mut *tx)
    .await
    .map_err(map_db_error)?;

    let default_tax_rate_id = create_catalog_defaults(&mut tx).await?;
    sqlx::query("INSERT INTO tenant_settings (tenant_id, default_tax_rate_id, extra, updated_by) VALUES (?, ?, ?, ?)")
        .bind(tenant_id)
        .bind(default_tax_rate_id)
        .bind(onboarding::initial_extra())
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

    let branch_id = create_default_branch(&mut tx, p, user_id).await?;
    let owner_role_id = create_system_roles(&mut tx).await?;

    let membership_id = new_id();
    sqlx::query(
        "INSERT INTO tenant_memberships (tenant_id, id, user_id, status, is_owner, display_name, joined_at, created_by) \
         VALUES (?, ?, ?, 'active', TRUE, ?, NOW(6), ?)",
    )
    .bind(tenant_id)
    .bind(membership_id)
    .bind(user_id)
    .bind(p.owner_name)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(map_db_error)?;
    sqlx::query(
        "INSERT INTO membership_roles (tenant_id, id, membership_id, role_id, branch_id, created_by) \
         VALUES (?, ?, ?, ?, NULL, ?)",
    )
    .bind(tenant_id)
    .bind(new_id())
    .bind(membership_id)
    .bind(owner_role_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(map_db_error)?;

    for (doc_type, prefix, per_branch, with_period) in SEQUENCES {
        sqlx::query(
            "INSERT INTO document_sequences (tenant_id, id, branch_id, doc_type, prefix, include_period, padding) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(tenant_id)
        .bind(new_id())
        .bind(per_branch.then_some(branch_id))
        .bind(doc_type)
        .bind(prefix)
        .bind(with_period)
        // Documents `INV-2610-00042`; codes `P000042` (Requirement Analysis §7.5).
        .bind(if *with_period { 5 } else { 6 })
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
    }

    let ledger = create_chart_of_accounts(&mut tx).await?;
    create_money_defaults(&mut tx, &ledger).await?;
    create_party_and_expense_defaults(&mut tx, &ledger).await?;

    start_trial(&mut tx, user_id).await?;

    sqlx::query("UPDATE tenants SET status = 'active' WHERE id = ?")
        .bind(tenant_id)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
    let after = json!({ "slug": p.slug, "name": p.name, "business_type": p.business_type });
    pos_audit::record(
        &mut tx,
        meta,
        AuditEntry::new("tenant.provisioned").entity("tenant", tenant_id).change(None, Some(after)),
    )
    .await?;
    tx.commit().await?;

    tracing::info!(%tenant_id, slug = p.slug, "tenant provisioned");
    Ok(ProvisionedTenant { tenant_id, slug: p.slug.to_owned(), name: p.name.to_owned(), branch_id, membership_id })
}

/// (name, rate %, default?) — DB design §13.2. The default is 0% until the
/// business says it is VAT-registered.
const TAX_RATES: &[(&str, i64, u32, bool)] = &[
    ("No VAT", 0, 0, true),
    ("VAT 5%", 5, 0, false),
    ("VAT 7.5%", 75, 1, false),
    ("VAT 10%", 10, 0, false),
    ("VAT 15%", 15, 0, false),
];

/// (name, short name, fractional quantities?) — DB design §13.2.
const UNITS: &[(&str, &str, bool)] = &[
    ("Piece", "pcs", false),
    ("Kilogram", "kg", true),
    ("Gram", "g", true),
    ("Litre", "L", true),
    ("Millilitre", "ml", true),
    ("Box", "box", false),
    ("Dozen", "dz", false),
    ("Pack", "pack", false),
];

/// Tax rates and units; returns the default tax rate. Migration 0009 adds the
/// same rows for businesses created before the catalogue existed.
async fn create_catalog_defaults(tx: &mut TenantTx) -> AppResult<Uuid> {
    let tenant_id = tx.tenant_id();
    let mut default_id = None;
    for &(name, mantissa, scale, is_default) in TAX_RATES {
        let id = new_id();
        sqlx::query("INSERT INTO tax_rates (tenant_id, id, name, rate_pct, is_default) VALUES (?, ?, ?, ?, ?)")
            .bind(tenant_id)
            .bind(id)
            .bind(name)
            .bind(Decimal::new(mantissa, scale))
            .bind(is_default)
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
        if is_default {
            default_id = Some(id);
        }
    }
    for &(name, short_name, allow_fraction) in UNITS {
        sqlx::query("INSERT INTO units (tenant_id, id, name, short_name, allow_fraction) VALUES (?, ?, ?, ?, ?)")
            .bind(tenant_id)
            .bind(new_id())
            .bind(name)
            .bind(short_name)
            .bind(allow_fraction)
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
    }
    default_id.ok_or_else(|| AppError::internal("no default tax rate in TAX_RATES"))
}

/// Branch `MAIN` with locations `MAIN` (sellable) and `DAMAGED`. The branch is
/// inserted first and pointed at its location afterwards (circular FK).
async fn create_default_branch(tx: &mut TenantTx, p: &Profile<'_>, user_id: Uuid) -> AppResult<Uuid> {
    let tenant_id = tx.tenant_id();
    let branch_id = new_id();
    sqlx::query(
        "INSERT INTO branches (tenant_id, id, code, name, address, phone, is_default, created_by) \
         VALUES (?, ?, 'MAIN', 'Main Branch', ?, ?, TRUE, ?)",
    )
    .bind(tenant_id)
    .bind(branch_id)
    .bind(p.address)
    .bind(p.phone)
    .bind(user_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;

    let main_location = new_id();
    for (id, code, name, kind, sellable) in
        [(main_location, "MAIN", "Shop Floor", "store", true), (new_id(), "DAMAGED", "Damaged", "damaged", false)]
    {
        sqlx::query(
            "INSERT INTO stock_locations (tenant_id, id, branch_id, code, name, location_type, is_sellable) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(tenant_id)
        .bind(id)
        .bind(branch_id)
        .bind(code)
        .bind(name)
        .bind(kind)
        .bind(sellable)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }

    sqlx::query("UPDATE branches SET default_location_id = ? WHERE tenant_id = ? AND id = ?")
        .bind(main_location)
        .bind(tenant_id)
        .bind(branch_id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(branch_id)
}

/// One row of the default chart of accounts (DB design §9.16): code, name,
/// type, normal side, parent code, system key, postable.
pub type CoaRow =
    (&'static str, &'static str, &'static str, &'static str, Option<&'static str>, Option<&'static str>, bool);

/// Default chart of accounts. Migration 0105 seeds the same rows for businesses
/// created before accounting existed; keep the two in step.
pub const CHART_OF_ACCOUNTS: &[CoaRow] = &[
    ("1000", "Assets", "asset", "debit", None, None, false),
    ("1010", "Cash in Hand", "asset", "debit", Some("1000"), Some("cash"), true),
    ("1020", "Bank Accounts", "asset", "debit", Some("1000"), Some("bank"), true),
    ("1030", "Mobile Wallets", "asset", "debit", Some("1000"), Some("mobile_wallet"), true),
    ("1100", "Accounts Receivable", "asset", "debit", Some("1000"), Some("ar"), true),
    ("1200", "Inventory", "asset", "debit", Some("1000"), Some("inventory"), true),
    ("1300", "VAT Receivable (Input)", "asset", "debit", Some("1000"), Some("vat_receivable"), true),
    ("2000", "Liabilities", "liability", "credit", None, None, false),
    ("2100", "Accounts Payable", "liability", "credit", Some("2000"), Some("ap"), true),
    ("2200", "VAT Payable (Output)", "liability", "credit", Some("2000"), Some("vat_payable"), true),
    ("2300", "Customer Advances", "liability", "credit", Some("2000"), Some("customer_advances"), true),
    ("3000", "Equity", "equity", "credit", None, None, false),
    ("3100", "Owner's Capital", "equity", "credit", Some("3000"), Some("owner_capital"), true),
    ("3200", "Opening Balance Equity", "equity", "credit", Some("3000"), Some("opening_equity"), true),
    ("3300", "Retained Earnings", "equity", "credit", Some("3000"), Some("retained_earnings"), true),
    ("4000", "Income", "income", "credit", None, None, false),
    ("4100", "Sales", "income", "credit", Some("4000"), Some("sales"), true),
    ("4110", "Sales Returns", "income", "debit", Some("4000"), Some("sales_returns"), true),
    ("4120", "Sales Discounts", "income", "debit", Some("4000"), Some("sales_discount"), true),
    ("4200", "Other Income", "income", "credit", Some("4000"), Some("other_income"), true),
    ("4300", "Rounding Gain/Loss", "income", "credit", Some("4000"), Some("rounding"), true),
    ("5000", "Cost of Goods Sold", "expense", "debit", None, Some("cogs"), true),
    ("5100", "Inventory Adjustments", "expense", "debit", None, Some("inventory_adjustment"), true),
    ("6000", "Operating Expenses", "expense", "debit", None, None, false),
    ("6010", "Rent", "expense", "debit", Some("6000"), None, true),
    ("6020", "Utilities", "expense", "debit", Some("6000"), None, true),
    ("6030", "Salaries", "expense", "debit", Some("6000"), None, true),
    ("6040", "Transport", "expense", "debit", Some("6000"), None, true),
    ("6050", "Marketing", "expense", "debit", Some("6000"), None, true),
    ("6060", "Maintenance", "expense", "debit", Some("6000"), None, true),
    ("6070", "Bank & MFS Charges", "expense", "debit", Some("6000"), Some("bank_charges"), true),
    ("6080", "Miscellaneous", "expense", "debit", Some("6000"), Some("misc_expense"), true),
];

/// Ledger account ids by code, for linking the other defaults.
struct Ledger(Vec<(&'static str, Uuid)>);

impl Ledger {
    fn id(&self, code: &str) -> AppResult<Uuid> {
        self.0
            .iter()
            .find(|(c, _)| *c == code)
            .map(|(_, id)| *id)
            .ok_or_else(|| AppError::internal(format!("no ledger account {code} in CHART_OF_ACCOUNTS")))
    }

    fn by_name(&self, name: &str) -> AppResult<Uuid> {
        let code = CHART_OF_ACCOUNTS
            .iter()
            .find(|row| row.1 == name && row.0.starts_with("60"))
            .map(|row| row.0)
            .ok_or_else(|| AppError::internal(format!("no expense account named {name}")))?;
        self.id(code)
    }
}

async fn create_chart_of_accounts(tx: &mut TenantTx) -> AppResult<Ledger> {
    let tenant_id = tx.tenant_id();
    let mut ids: Vec<(&'static str, Uuid)> = Vec::new();
    // Parents come before their children in the list.
    for &(code, name, account_type, normal_side, parent, system_key, postable) in CHART_OF_ACCOUNTS {
        let id = new_id();
        let parent_id = parent.and_then(|p| ids.iter().find(|(c, _)| *c == p).map(|(_, id)| *id));
        sqlx::query(
            "INSERT INTO ledger_accounts (tenant_id, id, code, name, account_type, normal_side, parent_id, \
             system_key, is_postable) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(tenant_id)
        .bind(id)
        .bind(code)
        .bind(name)
        .bind(account_type)
        .bind(normal_side)
        .bind(parent_id)
        .bind(system_key)
        .bind(postable)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
        ids.push((code, id));
    }
    Ok(Ledger(ids))
}

/// (name, type, ledger code, default?) and (method, type, account, needs reference?).
const PAYMENT_ACCOUNTS: &[(&str, &str, &str, bool)] = &[
    ("Cash", "cash", "1010", true),
    ("Bank Account", "bank", "1020", false),
    ("Mobile Wallet", "mobile_wallet", "1030", false),
];
const PAYMENT_METHODS: &[(&str, &str, &str, bool)] = &[
    ("Cash", "cash", "Cash", false),
    ("Card", "card", "Bank Account", false),
    ("bKash", "mobile_wallet", "Mobile Wallet", true),
    ("Nagad", "mobile_wallet", "Mobile Wallet", true),
    ("Rocket", "mobile_wallet", "Mobile Wallet", true),
    ("Bank Transfer", "bank_transfer", "Bank Account", true),
];

/// Payment accounts and methods (migration 0012 seeds the same for older businesses).
async fn create_money_defaults(tx: &mut TenantTx, ledger: &Ledger) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    let mut accounts = Vec::new();
    for &(name, account_type, code, is_default) in PAYMENT_ACCOUNTS {
        let id = new_id();
        sqlx::query(
            "INSERT INTO payment_accounts (tenant_id, id, account_type, name, ledger_account_id, is_default) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(tenant_id)
        .bind(id)
        .bind(account_type)
        .bind(name)
        .bind(ledger.id(code)?)
        .bind(is_default)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
        accounts.push((name, id));
    }
    for (sort_order, &(name, method_type, account, requires_reference)) in PAYMENT_METHODS.iter().enumerate() {
        let account_id = accounts.iter().find(|(n, _)| *n == account).map(|(_, id)| *id);
        sqlx::query(
            "INSERT INTO payment_methods (tenant_id, id, name, method_type, default_account_id, requires_reference, \
             sort_order) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(tenant_id)
        .bind(new_id())
        .bind(name)
        .bind(method_type)
        .bind(account_id)
        .bind(requires_reference)
        .bind(i32::try_from(sort_order).unwrap_or(0) + 1)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }
    Ok(())
}

pub const INCOME_CATEGORIES: &[&str] =
    &["Interest", "Rent Received", "Commission", "Scrap Sales", "Miscellaneous Income"];

pub const EXPENSE_CATEGORIES: &[&str] =
    &["Rent", "Utilities", "Salaries", "Transport", "Marketing", "Maintenance", "Bank & MFS Charges", "Miscellaneous"];

/// The walk-in customer (PTY-R03), expense and income categories (migrations 0013, 0017, 0106).
async fn create_party_and_expense_defaults(tx: &mut TenantTx, ledger: &Ledger) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    sqlx::query(
        "INSERT INTO customers (tenant_id, id, code, name, is_walk_in) VALUES (?, ?, 'WALKIN', 'Walk-in Customer', TRUE)",
    )
    .bind(tenant_id)
    .bind(new_id())
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    for name in EXPENSE_CATEGORIES {
        sqlx::query(
            "INSERT INTO expense_categories (tenant_id, id, name, ledger_account_id, is_system) VALUES (?, ?, ?, ?, TRUE)",
        )
        .bind(tenant_id)
        .bind(new_id())
        .bind(name)
        .bind(ledger.by_name(name)?)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }
    for name in INCOME_CATEGORIES {
        sqlx::query(
            "INSERT INTO income_categories (tenant_id, id, name, ledger_account_id, is_system) VALUES (?, ?, ?, ?, TRUE)",
        )
        .bind(tenant_id)
        .bind(new_id())
        .bind(name)
        .bind(ledger.id("4200")?)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }
    Ok(())
}

/// The 8 system roles with their permissions; returns the owner role id.
async fn create_system_roles(tx: &mut TenantTx) -> AppResult<Uuid> {
    let tenant_id = tx.tenant_id();
    let mut owner_role_id = None;
    for role in SYSTEM_ROLES {
        let role_id = new_id();
        sqlx::query(
            "INSERT INTO roles (tenant_id, id, code, name, description, is_system, max_discount_pct) \
             VALUES (?, ?, ?, ?, ?, TRUE, ?)",
        )
        .bind(tenant_id)
        .bind(role_id)
        .bind(role.code)
        .bind(role.name)
        .bind(role.description)
        .bind(role.max_discount_pct)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;

        let codes = serde_json::to_string(&role.permissions()).map_err(AppError::internal)?;
        sqlx::query(
            "INSERT INTO role_permissions (tenant_id, role_id, permission_code) \
             SELECT ?, ?, j.code FROM JSON_TABLE(?, '$[*]' COLUMNS (code VARCHAR(60) PATH '$')) AS j",
        )
        .bind(tenant_id)
        .bind(role_id)
        .bind(codes)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;

        if role.code == pos_auth::rbac::OWNER_ROLE {
            owner_role_id = Some(role_id);
        }
    }
    owner_role_id.ok_or_else(|| AppError::internal("system roles have no owner role"))
}

/// 14-day trial on the `trial` plan (D-18) and its first lifecycle event.
async fn start_trial(tx: &mut TenantTx, user_id: Uuid) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    let plan: Option<(Uuid, Option<i32>)> =
        sqlx::query_as("SELECT id, trial_days FROM plans WHERE code = 'trial' AND status = 'active'")
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?;
    let (plan_id, trial_days) = plan.ok_or_else(|| AppError::internal("the trial plan is missing"))?;
    let trial_days = trial_days.unwrap_or(14);

    let subscription_id = new_id();
    sqlx::query(
        "INSERT INTO subscriptions (tenant_id, id, plan_id, status, trial_ends_at, current_period_start, \
         current_period_end) VALUES (?, ?, ?, 'trialing', NOW(6) + INTERVAL ? DAY, NOW(6), NOW(6) + INTERVAL ? DAY)",
    )
    .bind(tenant_id)
    .bind(subscription_id)
    .bind(plan_id)
    .bind(trial_days)
    .bind(trial_days)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    sqlx::query(
        "INSERT INTO subscription_events (tenant_id, id, subscription_id, event_type, to_status, to_plan_id, \
         actor_type, actor_id) VALUES (?, ?, ?, 'trial_started', 'trialing', ?, 'tenant_user', ?)",
    )
    .bind(tenant_id)
    .bind(new_id())
    .bind(subscription_id)
    .bind(plan_id)
    .bind(user_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}
