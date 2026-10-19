//! Stock counts (stock-take; DB design §9.8 Release 2).
//!
//! Starting a count snapshots the system quantity and average cost of every
//! stocked product at the location (optionally one category). Staff enter the
//! counted quantities, then posting writes the differences as
//! `count_adjustment` movements: shortages leave at the average, surpluses
//! come in at the snapshot cost. The net value is journalled like an
//! adjustment (Inventory against Inventory Adjustments). Products left blank
//! are not adjusted.

use chrono::{DateTime, NaiveDate, Utc};
use pos_accounting::{NewJournal, SystemAccounts, journal};
use pos_audit::AuditEntry;
use pos_core::{
    AppError, AppResult, RequestMeta,
    id::new_id,
    money::round3,
    page::{self, Keyset},
};
use pos_db::{TenantTx, map_db_error};
use pos_tenancy::{context, sequences};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

use crate::posting::{MovementLine, MovementRef, post_movements};

pub const MAX_ITEMS: usize = 5000;

#[derive(Debug, Clone, Default, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CountInput {
    /// Blank = the selected branch's default location.
    #[serde(default)]
    pub location_id: Option<Uuid>,
    /// Count one category (with its sub-categories) only.
    #[serde(default)]
    pub category_id: Option<Uuid>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CountedLine {
    pub item_id: Uuid,
    /// `null` clears the entry.
    #[schema(value_type = Option<String>)]
    pub counted_qty: Option<Decimal>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CountEntries {
    pub lines: Vec<CountedLine>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct CountItem {
    pub id: Uuid,
    pub variant_id: Uuid,
    pub product_name: String,
    pub sku: String,
    pub barcode: Option<String>,
    pub unit_short_name: String,
    pub allow_fraction: bool,
    #[schema(value_type = String)]
    pub system_qty: Decimal,
    #[schema(value_type = Option<String>)]
    pub counted_qty: Option<Decimal>,
    #[schema(value_type = Option<String>)]
    pub variance_qty: Option<Decimal>,
    /// `null` without `products.view_cost`.
    #[schema(value_type = Option<String>)]
    pub unit_cost: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct CountSummary {
    pub id: Uuid,
    pub count_no: Option<String>,
    pub business_date: NaiveDate,
    /// `in_progress`, `posted` or `cancelled`.
    pub status: String,
    pub branch_id: Uuid,
    pub location_id: Uuid,
    pub location_name: String,
    pub category_name: Option<String>,
    pub note: Option<String>,
    pub item_count: i64,
    pub counted_count: i64,
    /// Net value of the differences; `null` without `products.view_cost`.
    #[schema(value_type = Option<String>)]
    pub variance_value: Option<Decimal>,
    pub created_by_name: Option<String>,
    pub snapshot_at: Option<DateTime<Utc>>,
    pub posted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct StockCount {
    #[serde(flatten)]
    pub summary: CountSummary,
    pub items: Vec<CountItem>,
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct CountFilter {
    pub status: Option<String>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct CountPage {
    pub data: Vec<CountSummary>,
    pub next_cursor: Option<String>,
}

const SUMMARY_SELECT: &str = "SELECT c.id, c.count_no, c.business_date, c.status, c.branch_id, c.location_id, \
     l.name AS location_name, \
     (SELECT g.name FROM categories g WHERE g.tenant_id = c.tenant_id AND g.id = c.category_id) AS category_name, \
     c.note, \
     (SELECT COUNT(*) FROM stock_count_items i WHERE i.tenant_id = c.tenant_id AND i.count_id = c.id) AS item_count, \
     (SELECT COUNT(*) FROM stock_count_items i WHERE i.tenant_id = c.tenant_id AND i.count_id = c.id \
        AND i.counted_qty IS NOT NULL) AS counted_count, \
     c.variance_value, (SELECT u.full_name FROM users u WHERE u.id = c.created_by) AS created_by_name, \
     c.snapshot_at, c.posted_at, c.created_at \
     FROM stock_counts c JOIN stock_locations l ON l.tenant_id = c.tenant_id AND l.id = c.location_id \
     WHERE c.tenant_id = ";

pub async fn list(tx: &mut TenantTx, filter: &CountFilter, can_view_cost: bool) -> AppResult<CountPage> {
    let limit = page::limit(filter.limit)?;
    let after = filter.cursor.as_deref().map(Keyset::decode).transpose()?;
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(SUMMARY_SELECT);
    q.push_bind(tenant_id);
    if let Some(branch_id) = tx.branch_id() {
        q.push(" AND c.branch_id = ").push_bind(branch_id);
    }
    if let Some(status) = filter.status.as_deref().filter(|s| !s.is_empty()) {
        if !["in_progress", "posted", "cancelled"].contains(&status) {
            return Err(AppError::field("status", "INVALID_STATUS", "unknown count status"));
        }
        q.push(" AND c.status = ").push_bind(status.to_owned());
    }
    if let Some(k) = after {
        q.push(" AND (c.created_at < ").push_bind(k.at);
        q.push(" OR (c.created_at = ").push_bind(k.at).push(" AND c.id < ").push_bind(k.id).push("))");
    }
    q.push(" ORDER BY c.created_at DESC, c.id DESC LIMIT ").push_bind(limit + 1);
    let mut rows: Vec<CountSummary> = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    let next_cursor = if rows.len() > limit as usize {
        rows.truncate(limit as usize);
        rows.last().map(|r| Keyset { at: r.created_at, id: r.id }.encode())
    } else {
        None
    };
    if !can_view_cost {
        for row in &mut rows {
            row.variance_value = None;
        }
    }
    Ok(CountPage { data: rows, next_cursor })
}

pub async fn get(tx: &mut TenantTx, id: Uuid, can_view_cost: bool) -> AppResult<StockCount> {
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(SUMMARY_SELECT);
    q.push_bind(tenant_id).push(" AND c.id = ").push_bind(id);
    let mut summary: CountSummary =
        q.build_query_as().fetch_optional(&mut **tx).await.map_err(map_db_error)?.ok_or(AppError::NotFound)?;
    let mut items: Vec<CountItem> = sqlx::query_as(
        "SELECT i.id, i.variant_id, p.name AS product_name, v.sku, \
         (SELECT b.barcode FROM product_barcodes b WHERE b.tenant_id = v.tenant_id AND b.variant_id = v.id \
            ORDER BY b.barcode LIMIT 1) AS barcode, \
         u.short_name AS unit_short_name, u.allow_fraction, i.system_qty, i.counted_qty, i.variance_qty, i.unit_cost \
         FROM stock_count_items i \
         JOIN product_variants v ON v.tenant_id = i.tenant_id AND v.id = i.variant_id \
         JOIN products p ON p.tenant_id = v.tenant_id AND p.id = v.product_id \
         JOIN units u ON u.tenant_id = p.tenant_id AND u.id = p.base_unit_id \
         WHERE i.tenant_id = ? AND i.count_id = ? ORDER BY p.name, v.sku",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if !can_view_cost {
        summary.variance_value = None;
        for item in &mut items {
            item.unit_cost = None;
        }
    }
    Ok(StockCount { summary, items })
}

/// Starts a count: snapshots what the system thinks is on the shelf.
pub async fn create(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    input: CountInput,
    can_view_cost: bool,
) -> AppResult<StockCount> {
    let ctx = context::load(tx).await?;
    let location_id = input.location_id.unwrap_or(ctx.location_id);
    let tenant_id = tx.tenant_id();
    let branch_id: Option<(Uuid, String)> =
        sqlx::query_as("SELECT branch_id, status FROM stock_locations WHERE tenant_id = ? AND id = ?")
            .bind(tenant_id)
            .bind(location_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?;
    let branch_id = match branch_id {
        Some((b, status)) if status == "active" => b,
        _ => return Err(AppError::field("location_id", "NOT_FOUND", "choose an active stock location")),
    };
    if tx.branch_id().is_some_and(|b| b != branch_id) {
        return Err(AppError::field(
            "location_id",
            "OTHER_BRANCH",
            "count a location of the branch you are working in",
        ));
    }
    if let Some(category_id) = input.category_id {
        let found: Option<i64> = sqlx::query_scalar("SELECT 1 FROM categories WHERE tenant_id = ? AND id = ?")
            .bind(tenant_id)
            .bind(category_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?;
        if found.is_none() {
            return Err(AppError::field("category_id", "NOT_FOUND", "choose a category"));
        }
    }
    let open: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM stock_counts WHERE tenant_id = ? AND location_id = ? AND status = 'in_progress' LIMIT 1",
    )
    .bind(tenant_id)
    .bind(location_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if open.is_some() {
        return Err(AppError::Conflict {
            code: "COUNT_IN_PROGRESS",
            message: "finish or cancel the count already running at this location".into(),
        });
    }

    let id = new_id();
    let number = sequences::next(tx, "count", Some(branch_id), ctx.business_date).await?;
    let user_id = tx.user_id();
    sqlx::query(
        "INSERT INTO stock_counts (tenant_id, id, count_no, branch_id, location_id, category_id, status, note, \
         business_date, snapshot_at, created_by) VALUES (?, ?, ?, ?, ?, ?, 'in_progress', ?, ?, UTC_TIMESTAMP(6), ?)",
    )
    .bind(tenant_id)
    .bind(id)
    .bind(&number)
    .bind(branch_id)
    .bind(location_id)
    .bind(input.category_id)
    .bind(
        input.note.as_deref().map(str::trim).filter(|n| !n.is_empty()).map(|n| n.chars().take(500).collect::<String>()),
    )
    .bind(ctx.business_date)
    .bind(user_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;

    // Every active stocked product of the scope, with its balance (0 if none).
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "INSERT INTO stock_count_items (tenant_id, id, count_id, variant_id, system_qty, unit_cost) \
         SELECT v.tenant_id, UUID_TO_BIN(UUID(), 1), ",
    );
    q.push_bind(id)
        .push(
            ", v.id, COALESCE(b.quantity, 0), COALESCE(NULLIF(b.avg_cost, 0), v.purchase_price) \
             FROM product_variants v JOIN products p ON p.tenant_id = v.tenant_id AND p.id = v.product_id \
             LEFT JOIN stock_balances b ON b.tenant_id = v.tenant_id AND b.variant_id = v.id AND b.location_id = ",
        )
        .push_bind(location_id)
        .push(" WHERE v.tenant_id = ")
        .push_bind(tenant_id)
        .push(
            " AND v.deleted_at IS NULL AND p.deleted_at IS NULL AND p.track_inventory \
             AND (p.status = 'active' OR COALESCE(b.quantity, 0) <> 0)",
        );
    if let Some(category_id) = input.category_id {
        q.push(" AND p.category_id IN (SELECT g.id FROM categories g WHERE g.tenant_id = ")
            .push_bind(tenant_id)
            .push(" AND (g.id = ")
            .push_bind(category_id)
            .push(" OR g.parent_id = ")
            .push_bind(category_id)
            .push("))");
    }
    let inserted = q.build().execute(&mut **tx).await.map_err(map_db_error)?.rows_affected();
    if inserted == 0 {
        return Err(AppError::field("category_id", "NOTHING_TO_COUNT", "there are no stocked products to count here"));
    }
    if inserted as usize > MAX_ITEMS {
        return Err(AppError::field(
            "category_id",
            "TOO_MANY",
            format!("count at most {MAX_ITEMS} products at a time; pick a category"),
        ));
    }
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("stock.count_started").entity("stock_count", id).branch(branch_id).change(
            None,
            Some(serde_json::json!({ "count_no": number, "location_id": location_id, "items": inserted })),
        ),
    )
    .await?;
    get(tx, id, can_view_cost).await
}

async fn lock_open(tx: &mut TenantTx, id: Uuid) -> AppResult<(Uuid, Uuid, Option<String>)> {
    let tenant_id = tx.tenant_id();
    let row: Option<(String, Uuid, Uuid, Option<String>)> = sqlx::query_as(
        "SELECT status, branch_id, location_id, count_no FROM stock_counts WHERE tenant_id = ? AND id = ? FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let (status, branch_id, location_id, number) = row.ok_or(AppError::NotFound)?;
    if status != "in_progress" {
        return Err(AppError::Conflict { code: "INVALID_STATUS", message: format!("this count is {status}") });
    }
    if tx.branch_id().is_some_and(|b| b != branch_id) {
        return Err(AppError::Denied {
            code: "BRANCH_NOT_ALLOWED",
            message: "switch to the count's branch to work on it".into(),
        });
    }
    Ok((branch_id, location_id, number))
}

/// Saves counted quantities (any number of times before posting).
pub async fn enter(tx: &mut TenantTx, id: Uuid, input: CountEntries, can_view_cost: bool) -> AppResult<StockCount> {
    lock_open(tx, id).await?;
    if input.lines.is_empty() || input.lines.len() > MAX_ITEMS {
        return Err(AppError::field("lines", "INVALID_LINES", "send between 1 and 5000 lines"));
    }
    let tenant_id = tx.tenant_id();
    for (i, line) in input.lines.iter().enumerate() {
        let qty = line.counted_qty.map(round3);
        if let (Some(q), Some(raw)) = (qty, line.counted_qty)
            && (q < Decimal::ZERO || q != raw.normalize() || q > Decimal::from(1_000_000_000))
        {
            return Err(AppError::field(
                &format!("lines[{i}].counted_qty"),
                "INVALID_QUANTITY",
                "enter 0 or more with at most 3 decimals",
            ));
        }
        let done =
            sqlx::query("UPDATE stock_count_items SET counted_qty = ? WHERE tenant_id = ? AND count_id = ? AND id = ?")
                .bind(qty)
                .bind(tenant_id)
                .bind(id)
                .bind(line.item_id)
                .execute(&mut **tx)
                .await
                .map_err(map_db_error)?;
        if done.rows_affected() == 0 {
            let exists: Option<i64> =
                sqlx::query_scalar("SELECT 1 FROM stock_count_items WHERE tenant_id = ? AND count_id = ? AND id = ?")
                    .bind(tenant_id)
                    .bind(id)
                    .bind(line.item_id)
                    .fetch_optional(&mut **tx)
                    .await
                    .map_err(map_db_error)?;
            if exists.is_none() {
                return Err(AppError::field(&format!("lines[{i}].item_id"), "NOT_FOUND", "not a line of this count"));
            }
        }
    }
    sqlx::query("UPDATE stock_counts SET version = version + 1 WHERE tenant_id = ? AND id = ?")
        .bind(tenant_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    get(tx, id, can_view_cost).await
}

/// Posts the differences of every counted line.
pub async fn post(tx: &mut TenantTx, meta: &RequestMeta, id: Uuid, can_view_cost: bool) -> AppResult<StockCount> {
    let (branch_id, location_id, number) = lock_open(tx, id).await?;
    let ctx = context::load(tx).await?;
    let tenant_id = tx.tenant_id();
    let lines: Vec<(Uuid, Uuid, Decimal, Option<Decimal>)> = sqlx::query_as(
        "SELECT id, variant_id, variance_qty, unit_cost FROM stock_count_items \
         WHERE tenant_id = ? AND count_id = ? AND counted_qty IS NOT NULL AND variance_qty <> 0 ORDER BY variant_id",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let counted: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM stock_count_items WHERE tenant_id = ? AND count_id = ? AND counted_qty IS NOT NULL",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if counted == 0 {
        return Err(AppError::Unprocessable {
            code: "NOTHING_COUNTED",
            message: "enter at least one counted quantity".into(),
        });
    }
    let reference = MovementRef {
        branch_id,
        business_date: ctx.business_date,
        reference_type: "stock_count",
        reference_id: id,
        reference_no: number.clone(),
    };
    let movement_lines: Vec<MovementLine> = lines
        .iter()
        .map(|(item_id, variant_id, variance, cost)| MovementLine {
            location_id,
            variant_id: *variant_id,
            movement_type: "count_adjustment",
            quantity: *variance,
            unit_cost: *cost,
            reference_line_id: Some(*item_id),
            reason: Some("count".into()),
        })
        .collect();
    // A count records what is physically there, so it may correct below zero.
    let posted = post_movements(tx, &reference, &movement_lines, true).await?;
    let variance_value: Decimal = posted.iter().map(|p| p.total_cost).sum();
    let user_id = tx.user_id();
    sqlx::query(
        "UPDATE stock_counts SET status = 'posted', variance_value = ?, posted_at = UTC_TIMESTAMP(6), posted_by = ?, \
         business_date = ?, version = version + 1 WHERE tenant_id = ? AND id = ?",
    )
    .bind(variance_value)
    .bind(user_id)
    .bind(ctx.business_date)
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let accounts = SystemAccounts::load(tx).await?;
    let mut j = NewJournal::new(ctx.business_date, "stock_count", id).branch(branch_id).memo("Stock count".to_owned());
    if let Some(n) = &number {
        j = j.number(n.clone());
    }
    // Normalisation flips a net shortage to Dr Adjustments / Cr Inventory.
    j.debit(accounts.inventory()?, variance_value).credit(accounts.inventory_adjustment()?, variance_value);
    journal::post(tx, j).await?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("stock.count_posted").entity("stock_count", id).branch(branch_id).change(
            None,
            Some(serde_json::json!({ "count_no": number, "adjusted_lines": lines.len(), "variance_value": variance_value })),
        ),
    )
    .await?;
    get(tx, id, can_view_cost).await
}

pub async fn cancel(tx: &mut TenantTx, meta: &RequestMeta, id: Uuid, can_view_cost: bool) -> AppResult<StockCount> {
    let (branch_id, ..) = lock_open(tx, id).await?;
    let tenant_id = tx.tenant_id();
    sqlx::query("UPDATE stock_counts SET status = 'cancelled', version = version + 1 WHERE tenant_id = ? AND id = ?")
        .bind(tenant_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    pos_audit::record(tx, meta, AuditEntry::new("stock.count_cancelled").entity("stock_count", id).branch(branch_id))
        .await?;
    get(tx, id, can_view_cost).await
}
