//! Chart of accounts (ACC-R01). System accounts are found by `system_key`;
//! payment accounts and expense categories carry their ledger account id.
//! A business adds its own accounts and groups and edits them; system
//! accounts keep their code, type and place and only their name can change.

use std::collections::BTreeMap;

use pos_audit::AuditEntry;
use pos_core::{AppError, AppResult, FieldError, RequestMeta, id::new_id};
use pos_db::{TenantTx, map_db_error};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct LedgerAccount {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    /// `asset`, `liability`, `equity`, `income` or `expense`.
    pub account_type: String,
    /// `debit` or `credit`.
    pub normal_side: String,
    pub parent_id: Option<Uuid>,
    pub system_key: Option<String>,
    /// False for group headers.
    pub is_postable: bool,
    /// `active` or `inactive` (no new postings).
    pub status: String,
    pub version: i32,
    /// Journal lines exist: type, side and "postable" can no longer change.
    pub has_entries: bool,
}

const SELECT: &str = "SELECT a.id, a.code, a.name, a.account_type, a.normal_side, a.parent_id, a.system_key, a.is_postable, \
     a.status, a.version, EXISTS (SELECT 1 FROM journal_lines jl WHERE jl.tenant_id = a.tenant_id AND jl.account_id = a.id) \
     AS has_entries FROM ledger_accounts a";

pub async fn list(tx: &mut TenantTx) -> AppResult<Vec<LedgerAccount>> {
    let tenant_id = tx.tenant_id();
    sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} WHERE a.tenant_id = ? ORDER BY a.code")))
        .bind(tenant_id)
        .fetch_all(&mut **tx)
        .await
        .map_err(map_db_error)
}

pub async fn get(tx: &mut TenantTx, id: Uuid) -> AppResult<LedgerAccount> {
    let tenant_id = tx.tenant_id();
    sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} WHERE a.tenant_id = ? AND a.id = ?")))
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_db_error)?
        .ok_or(AppError::NotFound)
}

/// The system accounts of one business, loaded once per posting.
#[derive(Debug, Clone)]
pub struct SystemAccounts(BTreeMap<String, Uuid>);

impl SystemAccounts {
    pub async fn load(tx: &mut TenantTx) -> AppResult<Self> {
        let tenant_id = tx.tenant_id();
        let rows: Vec<(String, Uuid)> =
            sqlx::query_as("SELECT system_key, id FROM ledger_accounts WHERE tenant_id = ? AND system_key IS NOT NULL")
                .bind(tenant_id)
                .fetch_all(&mut **tx)
                .await
                .map_err(map_db_error)?;
        Ok(Self(rows.into_iter().collect()))
    }

    pub fn get(&self, key: &str) -> AppResult<Uuid> {
        self.0.get(key).copied().ok_or_else(|| AppError::internal(format!("ledger account {key} is missing")))
    }

    pub fn cash(&self) -> AppResult<Uuid> {
        self.get("cash")
    }
    pub fn receivable(&self) -> AppResult<Uuid> {
        self.get("ar")
    }
    pub fn payable(&self) -> AppResult<Uuid> {
        self.get("ap")
    }
    pub fn inventory(&self) -> AppResult<Uuid> {
        self.get("inventory")
    }
    pub fn sales(&self) -> AppResult<Uuid> {
        self.get("sales")
    }
    pub fn sales_returns(&self) -> AppResult<Uuid> {
        self.get("sales_returns")
    }
    pub fn vat_payable(&self) -> AppResult<Uuid> {
        self.get("vat_payable")
    }
    pub fn vat_receivable(&self) -> AppResult<Uuid> {
        self.get("vat_receivable")
    }
    pub fn cogs(&self) -> AppResult<Uuid> {
        self.get("cogs")
    }
    pub fn inventory_adjustment(&self) -> AppResult<Uuid> {
        self.get("inventory_adjustment")
    }
    pub fn opening_equity(&self) -> AppResult<Uuid> {
        self.get("opening_equity")
    }
    pub fn other_income(&self) -> AppResult<Uuid> {
        self.get("other_income")
    }
    pub fn rounding(&self) -> AppResult<Uuid> {
        self.get("rounding")
    }
    pub fn bank_charges(&self) -> AppResult<Uuid> {
        self.get("bank_charges")
    }
}

/// The ledger account behind a payment account (cash, bank, wallet).
pub async fn for_payment_account(tx: &mut TenantTx, payment_account_id: Uuid) -> AppResult<Uuid> {
    let tenant_id = tx.tenant_id();
    let row: Option<(Option<Uuid>, String)> =
        sqlx::query_as("SELECT ledger_account_id, account_type FROM payment_accounts WHERE tenant_id = ? AND id = ?")
            .bind(tenant_id)
            .bind(payment_account_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?;
    let (linked, account_type) = row.ok_or(AppError::NotFound)?;
    if let Some(id) = linked {
        return Ok(id);
    }
    let accounts = SystemAccounts::load(tx).await?;
    accounts.get(system_key_for_payment_account(&account_type))
}

/// Which control account a payment account of this type posts to.
pub fn system_key_for_payment_account(account_type: &str) -> &'static str {
    match account_type {
        "cash" => "cash",
        "mobile_wallet" => "mobile_wallet",
        _ => "bank",
    }
}

/// Creates a postable expense account under `6000` for a new expense
/// category, with the next free `60xx` code.
pub async fn create_expense_account(tx: &mut TenantTx, name: &str) -> AppResult<Uuid> {
    let tenant_id = tx.tenant_id();
    let parent: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM ledger_accounts WHERE tenant_id = ? AND code = '6000'")
            .bind(tenant_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?;
    let codes: Vec<String> =
        sqlx::query_scalar("SELECT code FROM ledger_accounts WHERE tenant_id = ? AND code LIKE '6%' FOR UPDATE")
            .bind(tenant_id)
            .fetch_all(&mut **tx)
            .await
            .map_err(map_db_error)?;
    let next =
        codes.iter().filter_map(|c| c.parse::<u32>().ok()).filter(|c| (6000..7000).contains(c)).max().unwrap_or(6000);
    let code = (next / 10 + 1) * 10;
    if code >= 7000 {
        return Err(AppError::Unprocessable { code: "LEDGER_FULL", message: "no free expense account code".into() });
    }
    let id = new_id();
    sqlx::query(
        "INSERT INTO ledger_accounts (tenant_id, id, code, name, account_type, normal_side, parent_id, is_postable) \
         VALUES (?, ?, ?, ?, 'expense', 'debit', ?, TRUE)",
    )
    .bind(tenant_id)
    .bind(id)
    .bind(code.to_string())
    .bind(name)
    .bind(parent)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(id)
}

/// Renames the ledger account of an expense category (system accounts keep their name).
pub async fn rename(tx: &mut TenantTx, id: Uuid, name: &str) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    sqlx::query("UPDATE ledger_accounts SET name = ? WHERE tenant_id = ? AND id = ? AND system_key IS NULL")
        .bind(name)
        .bind(tenant_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(())
}

const TYPES: [&str; 5] = ["asset", "liability", "equity", "income", "expense"];

/// The usual side of a type; a contra account (e.g. Sales Returns) takes the other.
fn default_side(account_type: &str) -> &'static str {
    if matches!(account_type, "asset" | "expense") { "debit" } else { "credit" }
}

/// Add or edit a ledger account. Under a group, the type comes from the group.
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct LedgerAccountInput {
    pub code: String,
    pub name: String,
    /// Required at the top level; under a group it must match the group's.
    pub account_type: Option<String>,
    /// The group; `null` = top level. Left out on update, it stays where it is.
    #[serde(default, deserialize_with = "present")]
    #[schema(value_type = Option<Uuid>)]
    pub parent_id: Option<Option<Uuid>>,
    /// Defaults to the type's usual side.
    pub normal_side: Option<String>,
    /// False makes it a group header. Defaults to true.
    pub is_postable: Option<bool>,
    /// `active` or `inactive` (update only).
    pub status: Option<String>,
}

/// Tells a field sent as `null` (`Some(None)`) from one left out (`None`).
fn present<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Option<Uuid>>, D::Error> {
    Option::<Uuid>::deserialize(d).map(Some)
}

struct Checked {
    code: String,
    name: String,
    account_type: String,
    normal_side: String,
    parent_id: Option<Uuid>,
    is_postable: bool,
    status: String,
}

/// Field checks shared by create and update; `current` is the account being edited.
async fn check(tx: &mut TenantTx, input: &LedgerAccountInput, current: Option<&LedgerAccount>) -> AppResult<Checked> {
    let mut errors = Vec::new();
    let mut err = |field: &str, code: &str, message: &str| {
        errors.push(FieldError { field: field.to_owned(), code: code.to_owned(), message: message.to_owned() });
    };
    let code = input.code.trim().to_owned();
    if code.is_empty() {
        err("code", "REQUIRED", "this field is required");
    } else if code.len() > 20 || !code.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.') {
        err("code", "INVALID_CODE", "use up to 20 letters, digits, '-' or '.'");
    }
    let name = input.name.trim().to_owned();
    if name.is_empty() {
        err("name", "REQUIRED", "this field is required");
    } else if name.chars().count() > 150 {
        err("name", "TOO_LONG", "use at most 150 characters");
    }

    let parent_id = input.parent_id.unwrap_or_else(|| current.and_then(|c| c.parent_id));
    let parent = match parent_id {
        Some(pid) => match get(tx, pid).await {
            Ok(p) => Some(p),
            Err(AppError::NotFound) => {
                err("parent_id", "NOT_FOUND", "the group was not found");
                None
            }
            Err(e) => return Err(e),
        },
        None => None,
    };
    if let Some(p) = &parent {
        if p.is_postable {
            err("parent_id", "PARENT_NOT_GROUP", "choose a group header as the parent");
        } else if p.status != "active" {
            err("parent_id", "PARENT_INACTIVE", "the group is inactive");
        } else if let Some(cur) = current
            && (p.id == cur.id || is_descendant(tx, p.id, cur.id).await?)
        {
            err("parent_id", "PARENT_CYCLE", "an account cannot sit under itself");
        }
    }

    let requested = input.account_type.as_deref().map(str::trim).filter(|t| !t.is_empty());
    let account_type = match (&parent, requested) {
        (Some(p), Some(t)) if t != p.account_type => {
            err("account_type", "TYPE_MISMATCH", "an account has the same type as its group");
            p.account_type.clone()
        }
        (Some(p), _) => p.account_type.clone(),
        (None, Some(t)) if TYPES.contains(&t) => t.to_owned(),
        (None, Some(_)) => {
            err("account_type", "INVALID_TYPE", "choose asset, liability, equity, income or expense");
            String::new()
        }
        (None, None) => match current {
            Some(cur) => cur.account_type.clone(),
            None => {
                err("account_type", "REQUIRED", "this field is required");
                String::new()
            }
        },
    };
    let normal_side = match input.normal_side.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(side @ ("debit" | "credit")) => side.to_owned(),
        Some(_) => {
            err("normal_side", "INVALID_SIDE", "choose debit or credit");
            String::new()
        }
        None => match current {
            Some(cur) if cur.account_type == account_type => cur.normal_side.clone(),
            _ => default_side(&account_type).to_owned(),
        },
    };
    let status = match input.status.as_deref() {
        None => current.map_or_else(|| "active".to_owned(), |c| c.status.clone()),
        Some(st @ ("active" | "inactive")) => st.to_owned(),
        Some(_) => {
            err("status", "INVALID_STATUS", "choose active or inactive");
            String::new()
        }
    };
    let is_postable = input.is_postable.unwrap_or_else(|| current.is_none_or(|c| c.is_postable));

    if !code.is_empty() && current.is_none_or(|c| c.code != code) {
        let tenant_id = tx.tenant_id();
        let taken: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM ledger_accounts WHERE tenant_id = ? AND code = ?)")
                .bind(tenant_id)
                .bind(&code)
                .fetch_one(&mut **tx)
                .await
                .map_err(map_db_error)?;
        if taken {
            err("code", "DUPLICATE_CODE", "another account already uses this code");
        }
    }

    if !errors.is_empty() {
        return Err(AppError::validation("check the highlighted fields", errors));
    }
    Ok(Checked { code, name, account_type, normal_side, parent_id, is_postable, status })
}

/// Whether `id` sits somewhere under `ancestor`.
async fn is_descendant(tx: &mut TenantTx, id: Uuid, ancestor: Uuid) -> AppResult<bool> {
    let tenant_id = tx.tenant_id();
    let mut cursor = Some(id);
    // The chart is shallow; the bound only guards against bad data.
    for _ in 0..32 {
        let Some(at) = cursor else { return Ok(false) };
        if at == ancestor {
            return Ok(true);
        }
        cursor = sqlx::query_scalar("SELECT parent_id FROM ledger_accounts WHERE tenant_id = ? AND id = ?")
            .bind(tenant_id)
            .bind(at)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?
            .flatten();
    }
    Ok(false)
}

fn to_json(a: &LedgerAccount) -> AppResult<serde_json::Value> {
    serde_json::to_value(a).map_err(AppError::internal)
}

pub async fn create(tx: &mut TenantTx, meta: &RequestMeta, input: LedgerAccountInput) -> AppResult<LedgerAccount> {
    let c = check(tx, &input, None).await?;
    if c.status != "active" {
        return Err(AppError::field("status", "INVALID_STATUS", "a new account starts active"));
    }
    let tenant_id = tx.tenant_id();
    let id = new_id();
    sqlx::query(
        "INSERT INTO ledger_accounts (tenant_id, id, code, name, account_type, normal_side, parent_id, is_postable) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(id)
    .bind(&c.code)
    .bind(&c.name)
    .bind(&c.account_type)
    .bind(&c.normal_side)
    .bind(c.parent_id)
    .bind(c.is_postable)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let account = get(tx, id).await?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("ledger_account.created").entity("ledger_account", id).change(None, Some(to_json(&account)?)),
    )
    .await?;
    Ok(account)
}

fn read_only(field: &str, message: &str) -> AppError {
    AppError::field(field, "READ_ONLY", message)
}

pub async fn update(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    id: Uuid,
    expected_version: i32,
    input: LedgerAccountInput,
) -> AppResult<LedgerAccount> {
    let tenant_id = tx.tenant_id();
    let version: i32 =
        sqlx::query_scalar("SELECT version FROM ledger_accounts WHERE tenant_id = ? AND id = ? FOR UPDATE")
            .bind(tenant_id)
            .bind(id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?
            .ok_or(AppError::NotFound)?;
    if version != expected_version {
        return Err(AppError::PreconditionFailed {
            code: "VERSION_MISMATCH",
            message: "the account was changed by someone else; reload and try again".into(),
        });
    }
    let before = get(tx, id).await?;
    let c = check(tx, &input, Some(&before)).await?;

    if before.system_key.is_some() {
        let system = "system accounts keep their code, type and place; only the name can change";
        if c.code != before.code {
            return Err(read_only("code", system));
        }
        if c.parent_id != before.parent_id {
            return Err(read_only("parent_id", system));
        }
        if c.account_type != before.account_type || c.normal_side != before.normal_side {
            return Err(read_only("account_type", system));
        }
        if c.is_postable != before.is_postable {
            return Err(read_only("is_postable", system));
        }
        if c.status != before.status {
            return Err(read_only("status", system));
        }
    }

    let children: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM ledger_accounts WHERE tenant_id = ? AND parent_id = ?")
            .bind(tenant_id)
            .bind(id)
            .fetch_one(&mut **tx)
            .await
            .map_err(map_db_error)?;
    if c.account_type != before.account_type && (before.has_entries || children > 0) {
        return Err(AppError::field(
            "account_type",
            "IN_USE",
            "the type cannot change once the account has entries or sub-accounts",
        ));
    }
    if c.normal_side != before.normal_side && before.has_entries {
        return Err(AppError::field("normal_side", "IN_USE", "the side cannot change once the account has entries"));
    }
    if !c.is_postable && before.is_postable && before.has_entries {
        return Err(AppError::field("is_postable", "IN_USE", "an account with entries cannot become a group"));
    }
    if c.is_postable && !before.is_postable && children > 0 {
        return Err(AppError::field("is_postable", "HAS_CHILDREN", "move its sub-accounts first"));
    }
    if c.status == "inactive" && before.status == "active" {
        deactivation_check(tx, id).await?;
    }

    sqlx::query(
        "UPDATE ledger_accounts SET code = ?, name = ?, account_type = ?, normal_side = ?, parent_id = ?, is_postable = ?, \
         status = ?, version = version + 1 WHERE tenant_id = ? AND id = ?",
    )
    .bind(&c.code)
    .bind(&c.name)
    .bind(&c.account_type)
    .bind(&c.normal_side)
    .bind(c.parent_id)
    .bind(c.is_postable)
    .bind(&c.status)
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let after = get(tx, id).await?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("ledger_account.updated")
            .entity("ledger_account", id)
            .change(Some(to_json(&before)?), Some(to_json(&after)?)),
    )
    .await?;
    Ok(after)
}

/// An account can be made inactive when nothing still posts to it and it is
/// empty: no active sub-accounts, no active payment account or category
/// linked to it, and a zero balance.
async fn deactivation_check(tx: &mut TenantTx, id: Uuid) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    let blocked = |code: &'static str, message: &str| AppError::Unprocessable { code, message: message.into() };
    let count = |sql: &'static str| sqlx::query_scalar::<_, i64>(sql).bind(tenant_id).bind(id);
    if count("SELECT COUNT(*) FROM ledger_accounts WHERE tenant_id = ? AND parent_id = ? AND status = 'active'")
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?
        > 0
    {
        return Err(blocked("ACCOUNT_HAS_CHILDREN", "make its sub-accounts inactive first"));
    }
    for sql in [
        "SELECT COUNT(*) FROM payment_accounts WHERE tenant_id = ? AND ledger_account_id = ? AND status = 'active'",
        "SELECT COUNT(*) FROM expense_categories WHERE tenant_id = ? AND ledger_account_id = ? AND status = 'active'",
        "SELECT COUNT(*) FROM income_categories WHERE tenant_id = ? AND ledger_account_id = ? AND status = 'active'",
    ] {
        if count(sql).fetch_one(&mut **tx).await.map_err(map_db_error)? > 0 {
            return Err(blocked(
                "ACCOUNT_LINKED",
                "a cash/bank account or an expense/income category still uses this account",
            ));
        }
    }
    let balance: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(debit - credit), 0) FROM journal_lines WHERE tenant_id = ? AND account_id = ?",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if !balance.is_zero() {
        return Err(blocked("ACCOUNT_HAS_BALANCE", "only an account with a zero balance can be made inactive"));
    }
    Ok(())
}
