//! Branches (wizard step 4; multi-branch SRS-BRN-001…006). A new branch gets
//! its own sellable and damaged stock locations, its own document numbering
//! and a cash account (BRN-R01). Branches count against `branches.max`.

use pos_audit::AuditEntry;
use pos_core::{AppError, AppResult, RequestMeta, id::new_id};
use pos_db::{TenantTx, map_db_error};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    entitlements,
    provisioning::{SEQUENCES, clean, validate_contact, validate_name},
};

#[derive(Debug, Clone, Serialize, PartialEq, Eq, utoipa::ToSchema)]
pub struct Branch {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub receipt_header: Option<String>,
    pub is_default: bool,
    pub status: String,
    pub version: i32,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NewBranch {
    /// 2–10 letters or digits, e.g. `MIRPUR`; shown on documents.
    pub code: String,
    pub name: String,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub receipt_header: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct BranchPatch {
    pub name: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub receipt_header: Option<String>,
}

type Row = (Uuid, String, String, Option<String>, Option<String>, Option<String>, bool, String, i32);

fn from_row((id, code, name, address, phone, receipt_header, is_default, status, version): Row) -> Branch {
    Branch { id, code, name, address, phone, receipt_header, is_default, status, version }
}

pub async fn list(tx: &mut TenantTx) -> AppResult<Vec<Branch>> {
    let tenant_id = tx.tenant_id();
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT id, code, name, address, phone, receipt_header, is_default, status, version FROM branches \
         WHERE tenant_id = ? ORDER BY is_default DESC, name",
    )
    .bind(tenant_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(rows.into_iter().map(from_row).collect())
}

pub async fn update(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    branch_id: Uuid,
    expected_version: i32,
    patch: BranchPatch,
) -> AppResult<Branch> {
    let tenant_id = tx.tenant_id();
    let row: Option<Row> = sqlx::query_as(
        "SELECT id, code, name, address, phone, receipt_header, is_default, status, version FROM branches \
         WHERE tenant_id = ? AND id = ? FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(branch_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let current = row.map(from_row).ok_or(AppError::NotFound)?;
    if current.version != expected_version {
        return Err(AppError::PreconditionFailed {
            code: "VERSION_MISMATCH",
            message: "the branch was changed by someone else; reload and try again".into(),
        });
    }

    let mut next = current.clone();
    if let Some(name) = patch.name {
        next.name = validate_name(&name, "name", 150)?;
    }
    if let Some(v) = patch.address {
        next.address = clean(Some(v), "address", 500)?;
    }
    if patch.phone.is_some() {
        next.phone = validate_contact(patch.phone, None)?.0;
    }
    if let Some(v) = patch.receipt_header {
        next.receipt_header = clean(Some(v), "receipt_header", 500)?;
    }
    if next == current {
        return Ok(current);
    }
    next.version += 1;

    sqlx::query(
        "UPDATE branches SET name = ?, address = ?, phone = ?, receipt_header = ?, version = version + 1 \
         WHERE tenant_id = ? AND id = ?",
    )
    .bind(&next.name)
    .bind(&next.address)
    .bind(&next.phone)
    .bind(&next.receipt_header)
    .bind(tenant_id)
    .bind(branch_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;

    let to_json = |b: &Branch| serde_json::to_value(b).map_err(AppError::internal);
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("branch.updated")
            .entity("branch", branch_id)
            .branch(branch_id)
            .change(Some(to_json(&current)?), Some(to_json(&next)?)),
    )
    .await?;
    Ok(next)
}

async fn get(tx: &mut TenantTx, branch_id: Uuid) -> AppResult<Branch> {
    let tenant_id = tx.tenant_id();
    let row: Option<Row> = sqlx::query_as(
        "SELECT id, code, name, address, phone, receipt_header, is_default, status, version FROM branches \
         WHERE tenant_id = ? AND id = ?",
    )
    .bind(tenant_id)
    .bind(branch_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    row.map(from_row).ok_or(AppError::NotFound)
}

/// Active branches (for `branches.max`).
pub async fn active_count(tx: &mut TenantTx) -> AppResult<i64> {
    let tenant_id = tx.tenant_id();
    sqlx::query_scalar("SELECT COUNT(*) FROM branches WHERE tenant_id = ? AND status = 'active'")
        .bind(tenant_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)
}

/// Adds a branch with its locations, numbering and cash account (BRN-R01).
pub async fn create(tx: &mut TenantTx, meta: &RequestMeta, input: NewBranch) -> AppResult<Branch> {
    let code = input.code.trim().to_uppercase();
    if !(2..=10).contains(&code.len()) || !code.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return Err(AppError::field("code", "INVALID_CODE", "use 2–10 letters or digits"));
    }
    let name = validate_name(&input.name, "name", 150)?;
    let address = clean(input.address, "address", 500)?;
    let phone = validate_contact(input.phone, None)?.0;
    let receipt_header = clean(input.receipt_header, "receipt_header", 500)?;
    let used = active_count(tx).await?;
    entitlements::ensure_within(tx, "branches.max", used, 1).await?;

    let tenant_id = tx.tenant_id();
    let user_id = tx.user_id();
    let branch_id = new_id();
    sqlx::query(
        "INSERT INTO branches (tenant_id, id, code, name, address, phone, receipt_header, is_default, created_by) \
         VALUES (?, ?, ?, ?, ?, ?, ?, FALSE, ?)",
    )
    .bind(tenant_id)
    .bind(branch_id)
    .bind(&code)
    .bind(&name)
    .bind(&address)
    .bind(&phone)
    .bind(&receipt_header)
    .bind(user_id)
    .execute(&mut **tx)
    .await
    .map_err(|e| match map_db_error(e) {
        AppError::Conflict { code: "DUPLICATE", .. } => {
            AppError::Conflict { code: "DUPLICATE_CODE", message: "another branch has this code".into() }
        }
        other => other,
    })?;

    let main_location = new_id();
    for (id, location_code, location_name, kind, sellable) in [
        (main_location, format!("{code}-MAIN"), format!("{name} — Shop Floor"), "store", true),
        (new_id(), format!("{code}-DMG"), format!("{name} — Damaged"), "damaged", false),
    ] {
        sqlx::query(
            "INSERT INTO stock_locations (tenant_id, id, branch_id, code, name, location_type, is_sellable) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(tenant_id)
        .bind(id)
        .bind(branch_id)
        .bind(location_code)
        .bind(location_name.chars().take(100).collect::<String>())
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

    for (doc_type, prefix, per_branch, with_period) in SEQUENCES {
        if !per_branch {
            continue;
        }
        sqlx::query(
            "INSERT INTO document_sequences (tenant_id, id, branch_id, doc_type, prefix, include_period, padding) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(tenant_id)
        .bind(new_id())
        .bind(branch_id)
        .bind(doc_type)
        // `INV-MIR-2610-00042` when it fits; numbers are unique per branch anyway.
        .bind(Some(format!("{prefix}-{code}")).filter(|p| p.len() <= 10).unwrap_or_else(|| (*prefix).to_owned()))
        .bind(with_period)
        .bind(if *with_period { 5 } else { 6 })
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }

    // The branch's own cash drawer (BRN-R01), posting to Cash in Hand.
    let cash_ledger: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM ledger_accounts WHERE tenant_id = ? AND system_key = 'cash'")
            .bind(tenant_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?;
    sqlx::query(
        "INSERT INTO payment_accounts (tenant_id, id, branch_id, account_type, name, ledger_account_id) \
         VALUES (?, ?, ?, 'cash', ?, ?)",
    )
    .bind(tenant_id)
    .bind(new_id())
    .bind(branch_id)
    .bind(format!("Cash - {name}").chars().take(100).collect::<String>())
    .bind(cash_ledger)
    .execute(&mut **tx)
    .await
    .map_err(|e| match map_db_error(e) {
        AppError::Conflict { code: "DUPLICATE", .. } => AppError::Conflict {
            code: "DUPLICATE_NAME",
            message: "a payment account with this branch name exists".into(),
        },
        other => other,
    })?;

    let created = get(tx, branch_id).await?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("branch.created")
            .entity("branch", branch_id)
            .branch(branch_id)
            .change(None, Some(serde_json::to_value(&created).map_err(AppError::internal)?)),
    )
    .await?;
    Ok(created)
}

/// Activates or deactivates a branch. The default branch stays active;
/// reactivating counts against `branches.max`.
pub async fn set_status(tx: &mut TenantTx, meta: &RequestMeta, branch_id: Uuid, active: bool) -> AppResult<Branch> {
    let current = get(tx, branch_id).await?;
    let to = if active { "active" } else { "inactive" };
    if current.status == to {
        return Ok(current);
    }
    if !active && current.is_default {
        return Err(AppError::Unprocessable {
            code: "DEFAULT_BRANCH",
            message: "the main branch cannot be closed".into(),
        });
    }
    if active {
        let used = active_count(tx).await?;
        entitlements::ensure_within(tx, "branches.max", used, 1).await?;
    }
    let tenant_id = tx.tenant_id();
    sqlx::query("UPDATE branches SET status = ?, version = version + 1 WHERE tenant_id = ? AND id = ?")
        .bind(to)
        .bind(tenant_id)
        .bind(branch_id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new(if active { "branch.activated" } else { "branch.deactivated" })
            .entity("branch", branch_id)
            .branch(branch_id)
            .change(Some(serde_json::json!({ "status": current.status })), Some(serde_json::json!({ "status": to }))),
    )
    .await?;
    get(tx, branch_id).await
}
