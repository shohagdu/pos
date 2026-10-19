//! The owner's full data export (NFR-PRIV-01, E5.5): every business table of
//! the tenant as CSV (one file per table), zipped by the API. Ids are written
//! as UUIDs, generated helper columns are left out, and only the tenant's own
//! rows are read (`WHERE tenant_id = ?` in a `TenantTx`).

use pos_core::{AppResult, RequestMeta};
use pos_db::{TenantTx, map_db_error};
use sqlx::Row;

/// Tables in the export, in a readable order. Credentials, sessions,
/// idempotency keys and the outbox are deliberately absent.
pub const TABLES: &[&str] = &[
    "tenant_settings",
    "branches",
    "stock_locations",
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
    "party_ledger_entries",
    "sales",
    "sale_items",
    "sale_returns",
    "sale_return_items",
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
    "cash_registers",
    "register_shifts",
    "payment_accounts",
    "payment_methods",
    "payments",
    "payment_allocations",
    "account_transactions",
    "account_transfers",
    "expense_categories",
    "expenses",
    "income_categories",
    "other_incomes",
    "ledger_accounts",
    "journal_entries",
    "journal_lines",
    "fiscal_periods",
    "subscription_invoices",
];

/// Rows per table; larger tables are cut and the README says so.
pub const MAX_ROWS: i64 = 500_000;

pub struct ExportFile {
    pub name: String,
    pub content: String,
}

fn csv_cell(value: Option<&str>) -> String {
    let text = value.unwrap_or_default();
    if text.contains([',', '"', '\n', '\r']) { format!("\"{}\"", text.replace('"', "\"\"")) } else { text.to_owned() }
}

/// The columns to export and how to render each as text.
async fn columns(tx: &mut TenantTx, table: &str) -> AppResult<Vec<(String, String)>> {
    let rows: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT column_name, data_type, CAST(COALESCE(character_octet_length, 0) AS SIGNED) FROM information_schema.columns \
         WHERE table_schema = DATABASE() AND table_name = ? AND extra NOT LIKE '%GENERATED%' ORDER BY ordinal_position",
    )
    .bind(table)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(rows
        .into_iter()
        .map(|(name, data_type, octets)| {
            let expr = match data_type.as_str() {
                "binary" if octets == 16 => format!("BIN_TO_UUID(`{name}`)"),
                "binary" | "varbinary" | "blob" => format!("HEX(`{name}`)"),
                _ => format!("CAST(`{name}` AS CHAR)"),
            };
            (name, expr)
        })
        .collect())
}

pub async fn build(tx: &mut TenantTx, meta: &RequestMeta) -> AppResult<Vec<ExportFile>> {
    let tenant_id = tx.tenant_id();
    let mut files = Vec::with_capacity(TABLES.len() + 1);
    let mut truncated = Vec::new();
    for &table in TABLES {
        let cols = columns(tx, table).await?;
        if cols.is_empty() {
            continue;
        }
        let select = cols.iter().map(|(_, expr)| expr.as_str()).collect::<Vec<_>>().join(", ");
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {select} FROM {table} WHERE tenant_id = ? LIMIT {}",
            MAX_ROWS + 1
        )))
        .bind(tenant_id)
        .fetch_all(&mut **tx)
        .await
        .map_err(map_db_error)?;
        if i64::try_from(rows.len()).unwrap_or(i64::MAX) > MAX_ROWS {
            truncated.push(table);
        }
        let mut out = String::new();
        out.push_str(&cols.iter().map(|(n, _)| csv_cell(Some(n))).collect::<Vec<_>>().join(","));
        out.push_str("\r\n");
        for row in rows.iter().take(usize::try_from(MAX_ROWS).unwrap_or(usize::MAX)) {
            let cells: Vec<String> = (0..cols.len())
                .map(|i| csv_cell(row.try_get::<Option<String>, _>(i).ok().flatten().as_deref()))
                .collect();
            out.push_str(&cells.join(","));
            out.push_str("\r\n");
        }
        files.push(ExportFile { name: format!("{table}.csv"), content: out });
    }
    let mut readme = format!(
        "Full data export\r\nBusiness id: {tenant_id}\r\nCreated: {}\r\n\r\nOne CSV file per table (UTF-8). Ids are UUIDs; \
         amounts are decimals with a dot. Dates and times are UTC unless the column is a business_date.\r\n",
        chrono::Utc::now().to_rfc3339()
    );
    if !truncated.is_empty() {
        readme.push_str(&format!("\r\nThese tables were cut at {MAX_ROWS} rows: {}\r\n", truncated.join(", ")));
    }
    files.insert(0, ExportFile { name: "README.txt".into(), content: readme });
    pos_audit::record(
        tx,
        meta,
        pos_audit::AuditEntry::new("tenant.data_exported")
            .change(None, Some(serde_json::json!({ "tables": files.len() - 1 }))),
    )
    .await?;
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_quotes() {
        assert_eq!(csv_cell(Some("a,b")), "\"a,b\"");
        assert_eq!(csv_cell(Some("say \"hi\"")), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_cell(None), "");
    }
}
