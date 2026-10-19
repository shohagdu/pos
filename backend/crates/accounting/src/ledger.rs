//! Chart of accounts (ACC-R01). System accounts are found by `system_key`;
//! payment accounts and expense categories carry their ledger account id.

use std::collections::BTreeMap;

use pos_core::{AppError, AppResult, id::new_id};
use pos_db::{TenantTx, map_db_error};
use serde::Serialize;
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
    pub status: String,
}

pub async fn list(tx: &mut TenantTx) -> AppResult<Vec<LedgerAccount>> {
    let tenant_id = tx.tenant_id();
    sqlx::query_as(
        "SELECT id, code, name, account_type, normal_side, parent_id, system_key, is_postable, status \
         FROM ledger_accounts WHERE tenant_id = ? ORDER BY code",
    )
    .bind(tenant_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)
}

pub async fn get(tx: &mut TenantTx, id: Uuid) -> AppResult<LedgerAccount> {
    let tenant_id = tx.tenant_id();
    sqlx::query_as(
        "SELECT id, code, name, account_type, normal_side, parent_id, system_key, is_postable, status \
         FROM ledger_accounts WHERE tenant_id = ? AND id = ?",
    )
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
