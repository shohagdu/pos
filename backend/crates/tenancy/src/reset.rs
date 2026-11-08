//! Clear all transactions, with revert (migration 0117).
//!
//! "Clear" copies every transaction row of the business into an archive
//! (`data_archives` + `data_archive_rows`, one JSON object per row), deletes
//! them, and resets the values cached from them on master data: customer and
//! supplier dues, payment-account balances, last purchase prices and the
//! document numbers of transactions. Products, customers, suppliers, staff,
//! settings and the chart of accounts stay.
//!
//! "Restore" brings an archive back. Whatever is live at that moment (work
//! done after the clear) is archived first, so nothing is ever lost and a
//! restore can itself be restored. Owner only; one database transaction.
//!
//! Columns are read from `information_schema`, so tables that gain columns in
//! later migrations are archived whole, and an archive made before such a
//! migration restores the columns it has (new ones take their defaults).

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use pos_audit::AuditEntry;
use pos_core::{AppError, AppResult, RequestMeta, id::new_id};
use pos_db::{TenantTx, map_db_error};
use serde::Serialize;
use serde_json::{Value, json};
use sqlx::AssertSqlSafe;
use uuid::Uuid;

/// Transaction tables, parents before children: the restore order. Clearing
/// runs in reverse. `fiscal_periods` comes last because journal entries cannot
/// be inserted into a locked period.
pub const TRANSACTION_TABLES: &[&str] = &[
    "journal_entries",
    "journal_lines",
    "payments",
    "payment_allocations",
    "account_transactions",
    "account_transfers",
    "bank_reconciliations",
    "bank_reconciliation_items",
    "expenses",
    "other_incomes",
    "party_ledger_entries",
    "sales",
    "sale_items",
    "held_carts",
    "sale_returns",
    "sale_return_items",
    "product_batches",
    "purchases",
    "purchase_items",
    "goods_receipts",
    "goods_receipt_items",
    "purchase_returns",
    "purchase_return_items",
    "stock_balances",
    "stock_movements",
    "stock_adjustments",
    "stock_adjustment_items",
    "stock_transfers",
    "stock_transfer_items",
    "stock_counts",
    "stock_count_items",
    "stock_batch_balances",
    "stock_batch_movements",
    "register_shifts",
    "fiscal_periods",
];

/// Business tables that are not cleared (master data, settings, people,
/// billing, logs). With [`TRANSACTION_TABLES`] and [`ARCHIVE_TABLES`] this
/// covers every tenant table; a test fails when a new table is in neither.
pub const KEPT_TABLES: &[&str] = &[
    "tenant_settings",
    "branches",
    "stock_locations",
    "tenant_memberships",
    "invitations",
    "document_sequences",
    "subscriptions",
    "subscription_events",
    "tenant_feature_overrides",
    "subscription_invoices",
    "subscription_payment_claims",
    "roles",
    "role_permissions",
    "membership_roles",
    "audit_logs",
    "idempotency_keys",
    "outbox_events",
    "user_sessions",
    "auth_events",
    "payment_accounts",
    "payment_methods",
    "tax_rates",
    "units",
    "categories",
    "brands",
    "products",
    "product_variants",
    "product_barcodes",
    "product_price_history",
    "customers",
    "suppliers",
    "expense_categories",
    "income_categories",
    "cash_registers",
    "ledger_accounts",
    "notifications",
    "notification_preferences",
    "notification_deliveries",
    "files",
    "background_jobs",
];

/// Where the archives themselves live.
pub const ARCHIVE_TABLES: &[&str] = &["data_archives", "data_archive_rows"];

/// Values on master data that are computed from transactions: reset on a
/// clear, put back on a restore. `(table, columns, reset to, extra filter)`.
const CACHED: &[(&str, &[&str], &str, &str)] = &[
    ("customers", &["current_balance", "opening_balance"], "0", ""),
    ("suppliers", &["current_balance", "opening_balance"], "0", ""),
    ("payment_accounts", &["current_balance", "opening_balance"], "0", ""),
    (
        "product_variants",
        &[
            "last_purchase_price",
            "last_purchase_date",
            "last_purchase_qty",
            "last_supplier_id",
            "last_purchase_item_id",
        ],
        "NULL",
        "",
    ),
    (
        "document_sequences",
        &["next_value", "period_key"],
        "",
        " AND doc_type IN ('sale','sale_return','purchase','purchase_return','goods_receipt','payment_in',\
         'payment_out','expense','income','adjustment','transfer','count','journal')",
    ),
];

/// Prefix of `data_archive_rows.table_name` for [`CACHED`] values.
const CACHED_PREFIX: &str = "cached:";

/// The documents counted in an archive's summary, in display order.
const SUMMARY_DOCUMENTS: &[&str] = &[
    "sales",
    "sale_returns",
    "purchases",
    "purchase_returns",
    "payments",
    "expenses",
    "other_incomes",
    "stock_adjustments",
    "stock_transfers",
    "stock_counts",
    "journal_entries",
    "register_shifts",
];

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct Archive {
    pub id: Uuid,
    /// `clear` (made by "Clear all transactions") or `restore` (the live data
    /// set aside when another archive was restored).
    pub kind: String,
    /// `held` (can be restored) or `restored`.
    pub status: String,
    /// Documents per type (`sales`, `purchases`, …), zero counts left out.
    #[schema(value_type = Object)]
    pub documents: Value,
    /// All archived rows, lines and ledger entries included.
    pub total_rows: i64,
    pub created_by_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub restored_by_name: Option<String>,
    pub restored_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ResetOverview {
    /// Live documents per type, zero counts left out.
    #[schema(value_type = Object)]
    pub live: Value,
    /// Newest first.
    pub archives: Vec<Archive>,
}

/// Live document counts and the archives.
pub async fn overview(tx: &mut TenantTx) -> AppResult<ResetOverview> {
    let tenant_id = tx.tenant_id();
    let mut live = serde_json::Map::new();
    for table in SUMMARY_DOCUMENTS {
        let sql = format!("SELECT COUNT(*) FROM {table} WHERE tenant_id = ?");
        let n: i64 =
            sqlx::query_scalar(AssertSqlSafe(sql)).bind(tenant_id).fetch_one(&mut **tx).await.map_err(map_db_error)?;
        if n > 0 {
            live.insert((*table).to_owned(), json!(n));
        }
    }
    Ok(ResetOverview { live: Value::Object(live), archives: list(tx).await? })
}

async fn list(tx: &mut TenantTx) -> AppResult<Vec<Archive>> {
    type Row = (Uuid, String, String, Value, Option<String>, DateTime<Utc>, Option<String>, Option<DateTime<Utc>>);
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT a.id, a.kind, a.status, a.summary, cu.full_name, a.created_at, ru.full_name, a.restored_at \
         FROM data_archives a LEFT JOIN users cu ON cu.id = a.created_by LEFT JOIN users ru ON ru.id = a.restored_by \
         WHERE a.tenant_id = ? ORDER BY a.created_at DESC",
    )
    .bind(tx.tenant_id())
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(rows
        .into_iter()
        .map(|(id, kind, status, summary, created_by_name, created_at, restored_by_name, restored_at)| {
            let (documents, total_rows) = documents(&summary);
            Archive {
                id,
                kind,
                status,
                documents,
                total_rows,
                created_by_name,
                created_at,
                restored_by_name,
                restored_at,
            }
        })
        .collect())
}

fn documents(summary: &Value) -> (Value, i64) {
    let mut docs = serde_json::Map::new();
    for table in SUMMARY_DOCUMENTS {
        if let Some(n) = summary.get(*table).and_then(Value::as_i64).filter(|n| *n > 0) {
            docs.insert((*table).to_owned(), json!(n));
        }
    }
    let total = TRANSACTION_TABLES.iter().filter_map(|t| summary.get(*t).and_then(Value::as_i64)).sum();
    (Value::Object(docs), total)
}

/// Archives every transaction and clears it (owner only, checked by the route).
pub async fn clear(tx: &mut TenantTx, meta: &RequestMeta) -> AppResult<Archive> {
    let archive_id = archive_live(tx, "clear").await?;
    wipe_live(tx).await?;
    let archive = get(tx, archive_id).await?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("data.cleared")
            .entity("data_archive", archive_id)
            .change(None, Some(json!({ "documents": archive.documents, "rows": archive.total_rows }))),
    )
    .await?;
    Ok(archive)
}

/// Brings archive `id` back; what is live now is archived first.
pub async fn restore(tx: &mut TenantTx, meta: &RequestMeta, id: Uuid) -> AppResult<Archive> {
    let tenant_id = tx.tenant_id();
    let target: Option<(String, Value)> =
        sqlx::query_as("SELECT status, columns_json FROM data_archives WHERE tenant_id = ? AND id = ? FOR UPDATE")
            .bind(tenant_id)
            .bind(id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?;
    let (status, columns) = target.ok_or(AppError::NotFound)?;
    if status != "held" {
        return Err(AppError::Conflict {
            code: "ALREADY_RESTORED",
            message: "this archive is already restored".into(),
        });
    }

    let set_aside = archive_live(tx, "restore").await?;
    wipe_live(tx).await?;
    put_back(tx, id, &columns).await?;

    sqlx::query("UPDATE data_archives SET status = 'restored', restored_by = ?, restored_at = NOW(6) WHERE tenant_id = ? AND id = ?")
        .bind(tx.user_id())
        .bind(tenant_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    // The rows are live again; the archive keeps its summary for the history.
    sqlx::query("DELETE FROM data_archive_rows WHERE tenant_id = ? AND archive_id = ?")
        .bind(tenant_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;

    let archive = get(tx, id).await?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("data.restored")
            .entity("data_archive", id)
            .change(None, Some(json!({ "documents": archive.documents, "set_aside_archive_id": set_aside }))),
    )
    .await?;
    Ok(archive)
}

async fn get(tx: &mut TenantTx, id: Uuid) -> AppResult<Archive> {
    list(tx).await?.into_iter().find(|a| a.id == id).ok_or(AppError::NotFound)
}

#[derive(Debug, Clone)]
struct Column {
    name: String,
    kind: ColumnKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColumnKind {
    Binary,
    Json,
    Plain,
}

/// The stored (not generated) columns of `table`, in table order.
async fn columns(tx: &mut TenantTx, table: &str) -> AppResult<Vec<Column>> {
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT CAST(COLUMN_NAME AS CHAR), CAST(DATA_TYPE AS CHAR), CAST(EXTRA AS CHAR) FROM information_schema.COLUMNS \
         WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ? ORDER BY ORDINAL_POSITION",
    )
    .bind(table)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if rows.is_empty() {
        return Err(AppError::internal(format!("table {table} not found")));
    }
    Ok(rows
        .into_iter()
        // Computed columns cannot be written. `DEFAULT_GENERATED` (a
        // `DEFAULT CURRENT_TIMESTAMP` column such as `created_at`) is stored.
        .filter(|(_, _, extra)| {
            let extra = extra.to_ascii_uppercase();
            !(extra.contains("VIRTUAL GENERATED") || extra.contains("STORED GENERATED"))
        })
        .map(|(name, data_type, _)| {
            let kind = match data_type.to_ascii_lowercase().as_str() {
                "binary" | "varbinary" | "blob" | "tinyblob" | "mediumblob" | "longblob" => ColumnKind::Binary,
                "json" => ColumnKind::Json,
                _ => ColumnKind::Plain,
            };
            Column { name, kind }
        })
        .collect())
}

/// `JSON_OBJECT('a', a, 'b', HEX(b), …)` for archiving a row.
fn json_object(cols: &[Column], alias: &str) -> String {
    let parts: Vec<String> = cols
        .iter()
        .map(|c| {
            let col = format!("{alias}.`{}`", c.name);
            let value = if c.kind == ColumnKind::Binary { format!("HEX({col})") } else { col };
            format!("'{}', {value}", c.name)
        })
        .collect();
    format!("JSON_OBJECT({})", parts.join(", "))
}

/// The SQL value of column `c` read back from `r.row_data`.
fn json_value(c: &Column) -> String {
    let v = format!("JSON_EXTRACT(r.row_data, '$.\"{}\"')", c.name);
    let value = match c.kind {
        ColumnKind::Binary => format!("UNHEX(JSON_UNQUOTE({v}))"),
        ColumnKind::Json => v.clone(),
        ColumnKind::Plain => format!("JSON_UNQUOTE({v})"),
    };
    format!("IF(JSON_TYPE({v}) = 'NULL', NULL, {value})")
}

/// Copies every live transaction row and cached value into a new archive.
async fn archive_live(tx: &mut TenantTx, kind: &str) -> AppResult<Uuid> {
    let tenant_id = tx.tenant_id();
    let archive_id = new_id();
    sqlx::query(
        "INSERT INTO data_archives (tenant_id, id, kind, summary, columns_json, created_by) VALUES (?, ?, ?, '{}', '{}', ?)",
    )
    .bind(tenant_id)
    .bind(archive_id)
    .bind(kind)
    .bind(tx.user_id())
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;

    let mut summary = BTreeMap::new();
    let mut column_lists = BTreeMap::new();
    for table in TRANSACTION_TABLES {
        let cols = columns(tx, table).await?;
        // Reversals after the entries they reverse.
        let order = if *table == "journal_entries" { "ORDER BY t.created_at, t.id" } else { "" };
        let sql = format!(
            "INSERT INTO data_archive_rows (tenant_id, archive_id, table_name, row_no, row_data) \
             SELECT t.tenant_id, ?, '{table}', ROW_NUMBER() OVER ({order}), {} FROM {table} t WHERE t.tenant_id = ?",
            json_object(&cols, "t")
        );
        let done = sqlx::query(AssertSqlSafe(sql))
            .bind(archive_id)
            .bind(tenant_id)
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
        summary.insert((*table).to_owned(), json!(done.rows_affected()));
        column_lists.insert((*table).to_owned(), json!(cols.iter().map(|c| c.name.clone()).collect::<Vec<_>>()));
    }
    for (table, cached, _, filter) in CACHED {
        let all = columns(tx, table).await?;
        let cols: Vec<Column> =
            // `updated_at` too: resetting moves it, and putting it back explicitly
            // also stops `ON UPDATE CURRENT_TIMESTAMP` on the restore.
            all.into_iter()
                .filter(|c| c.name == "id" || c.name == "updated_at" || cached.contains(&c.name.as_str()))
                .collect();
        let sql = format!(
            "INSERT INTO data_archive_rows (tenant_id, archive_id, table_name, row_no, row_data) \
             SELECT t.tenant_id, ?, '{CACHED_PREFIX}{table}', ROW_NUMBER() OVER (), {} FROM {table} t \
             WHERE t.tenant_id = ?{filter}",
            json_object(&cols, "t")
        );
        sqlx::query(AssertSqlSafe(sql))
            .bind(archive_id)
            .bind(tenant_id)
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
        column_lists
            .insert(format!("{CACHED_PREFIX}{table}"), json!(cols.iter().map(|c| c.name.clone()).collect::<Vec<_>>()));
    }

    sqlx::query("UPDATE data_archives SET summary = ?, columns_json = ? WHERE tenant_id = ? AND id = ?")
        .bind(json!(summary))
        .bind(json!(column_lists))
        .bind(tenant_id)
        .bind(archive_id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(archive_id)
}

/// Deletes every live transaction row and resets the cached values.
async fn wipe_live(tx: &mut TenantTx) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    // Ledgers are append-only; deleting them needs the maintenance flag. The
    // pool resets session variables when the connection is released.
    sqlx::query("SET @app_maintenance = 1").execute(&mut **tx).await.map_err(map_db_error)?;
    for table in TRANSACTION_TABLES.iter().rev() {
        // Reversals before the entries they reverse.
        let order = if *table == "journal_entries" { " ORDER BY created_at DESC, id DESC" } else { "" };
        let sql = format!("DELETE FROM {table} WHERE tenant_id = ?{order}");
        sqlx::query(AssertSqlSafe(sql)).bind(tenant_id).execute(&mut **tx).await.map_err(map_db_error)?;
    }
    sqlx::query("SET @app_maintenance = NULL").execute(&mut **tx).await.map_err(map_db_error)?;
    for (table, cached, reset_to, filter) in CACHED {
        let assignments: Vec<String> = cached
            .iter()
            .map(|c| match (*table, *c) {
                ("document_sequences", "next_value") => "next_value = 1".to_owned(),
                ("document_sequences", _) => format!("`{c}` = NULL"),
                _ => format!("`{c}` = {reset_to}"),
            })
            .collect();
        let sql = format!("UPDATE {table} SET {} WHERE tenant_id = ?{filter}", assignments.join(", "));
        sqlx::query(AssertSqlSafe(sql)).bind(tenant_id).execute(&mut **tx).await.map_err(map_db_error)?;
    }
    Ok(())
}

/// Inserts archive `id`'s rows and puts its cached values back.
async fn put_back(tx: &mut TenantTx, id: Uuid, archived_columns: &Value) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    let archived = |key: &str| -> Vec<String> {
        archived_columns
            .get(key)
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect())
            .unwrap_or_default()
    };
    for table in TRANSACTION_TABLES {
        let had = archived(table);
        // Columns in both the archive and today's table.
        let cols: Vec<Column> = columns(tx, table).await?.into_iter().filter(|c| had.contains(&c.name)).collect();
        if cols.is_empty() {
            continue;
        }
        let names: Vec<String> = cols.iter().map(|c| format!("`{}`", c.name)).collect();
        let values: Vec<String> = cols.iter().map(json_value).collect();
        let sql = format!(
            "INSERT INTO {table} ({}) SELECT {} FROM data_archive_rows r \
             WHERE r.tenant_id = ? AND r.archive_id = ? AND r.table_name = '{table}' ORDER BY r.row_no",
            names.join(", "),
            values.join(", ")
        );
        sqlx::query(AssertSqlSafe(sql))
            .bind(tenant_id)
            .bind(id)
            .execute(&mut **tx)
            .await
            .map_err(|e| restore_error(map_db_error(e)))?;
    }
    for (table, _, _, _) in CACHED {
        let key = format!("{CACHED_PREFIX}{table}");
        let had = archived(&key);
        let cols: Vec<Column> =
            columns(tx, table).await?.into_iter().filter(|c| c.name != "id" && had.contains(&c.name)).collect();
        if cols.is_empty() {
            continue;
        }
        let id_col = Column { name: "id".into(), kind: ColumnKind::Binary };
        let sets: Vec<String> = cols.iter().map(|c| format!("t.`{}` = {}", c.name, json_value(c))).collect();
        let sql = format!(
            "UPDATE {table} t JOIN data_archive_rows r \
               ON r.tenant_id = t.tenant_id AND r.archive_id = ? AND r.table_name = '{key}' AND t.id = {} \
             SET {} WHERE t.tenant_id = ?",
            json_value(&id_col),
            sets.join(", ")
        );
        sqlx::query(AssertSqlSafe(sql)).bind(id).bind(tenant_id).execute(&mut **tx).await.map_err(map_db_error)?;
    }
    Ok(())
}

/// A restored row names a product, customer, … that no longer exists.
fn restore_error(err: AppError) -> AppError {
    match err {
        AppError::Unprocessable { code: "INVALID_REFERENCE", .. } => AppError::Conflict {
            code: "RESTORE_BLOCKED",
            message: "the archive refers to a product, customer, supplier or account that was deleted after it \
                      was made; it cannot be restored"
                .into(),
        },
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every table with a `tenant_id` must be either cleared or kept, so a new
    /// transaction table is never silently left behind by "Clear".
    #[test]
    fn every_tenant_table_is_classified() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../migrations");
        let mut missing = Vec::new();
        for entry in std::fs::read_dir(dir).expect("migrations dir") {
            let path = entry.expect("entry").path();
            if path.extension().and_then(|e| e.to_str()) != Some("sql") {
                continue;
            }
            let sql = std::fs::read_to_string(&path).expect("read migration");
            for part in sql.split("CREATE TABLE ").skip(1) {
                let name: String = part.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
                let body = part.split(");").next().unwrap_or_default();
                let tenant_scoped = body.lines().any(|l| l.trim_start().starts_with("tenant_id "));
                let known = TRANSACTION_TABLES.contains(&name.as_str())
                    || KEPT_TABLES.contains(&name.as_str())
                    || ARCHIVE_TABLES.contains(&name.as_str());
                if tenant_scoped && !known && !name.is_empty() && name != "IF" {
                    missing.push(name);
                }
            }
        }
        assert!(missing.is_empty(), "add these tables to TRANSACTION_TABLES or KEPT_TABLES in reset.rs: {missing:?}");
    }

    #[test]
    fn transaction_and_kept_tables_do_not_overlap() {
        for t in TRANSACTION_TABLES {
            assert!(!KEPT_TABLES.contains(t), "{t} is both cleared and kept");
        }
    }
}
