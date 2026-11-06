//! Batches and expiry dates (STK-R11). A batch is a variant's lot with a
//! number and an expiry date; [`crate::posting`] keeps the per-location batch
//! balances. Stock of a variant is its batches plus a "no batch" remainder,
//! so products without batches work exactly as before.
//!
//! Batch-only rules:
//! * receipts (opening stock, purchases, "found" adjustments) name a batch;
//! * outflows take the earliest expiry first (FEFO), then the remainder;
//! * reversals put stock back into the batches the original line moved.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use pos_core::{
    AppError, AppResult,
    id::new_id,
    page::{self, OffsetCursor},
};
use pos_db::{TenantTx, map_db_error};
use pos_tenancy::context;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

use crate::stock::like_contains;

pub const MAX_BATCH_NO: usize = 40;

/// Where a stock line's quantity goes to or comes from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Batch {
    /// Increases: no batch. Decreases: earliest expiry first, then no batch.
    #[default]
    Auto,
    /// Into (increase) or out of (decrease) this batch; a decrease takes
    /// what the batch lacks earliest expiry first.
    Exact(Uuid),
    /// Undoes document line `origin`: an increase puts back what that line
    /// took (a sale line being returned), a decrease takes back what it put
    /// in (a purchase line returned to the supplier). The rest as `Auto`.
    Reverse(Uuid),
}

/// The batch named by a document line: an existing one of the variant with
/// this number, or a new one. A blank number with an expiry date uses the
/// date as the number. `field` prefixes error fields (`lines[2]`). Turns on
/// batch tracking for the product.
pub async fn resolve(
    tx: &mut TenantTx,
    variant_id: Uuid,
    batch_no: Option<&str>,
    expiry_date: Option<NaiveDate>,
    field: &str,
) -> AppResult<Option<Uuid>> {
    let batch_no = batch_no.map(str::trim).filter(|b| !b.is_empty());
    let number = match (batch_no, expiry_date) {
        (None, None) => return Ok(None),
        (Some(n), _) => n.to_owned(),
        (None, Some(date)) => date.format("%Y-%m-%d").to_string(),
    };
    if number.chars().count() > MAX_BATCH_NO {
        return Err(AppError::field(
            &format!("{field}.batch_no"),
            "TOO_LONG",
            format!("a batch number has at most {MAX_BATCH_NO} characters"),
        ));
    }
    let tenant_id = tx.tenant_id();
    let existing: Option<(Uuid, Option<NaiveDate>)> = sqlx::query_as(
        "SELECT id, expiry_date FROM product_batches WHERE tenant_id = ? AND variant_id = ? AND batch_no = ?",
    )
    .bind(tenant_id)
    .bind(variant_id)
    .bind(&number)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let id = match existing {
        Some((_, Some(known))) if expiry_date.is_some_and(|d| d != known) => {
            return Err(AppError::field(
                &format!("{field}.expiry_date"),
                "BATCH_EXPIRY_MISMATCH",
                format!("batch {number} expires on {known}"),
            ));
        }
        Some((id, None)) if expiry_date.is_some() => {
            sqlx::query("UPDATE product_batches SET expiry_date = ? WHERE tenant_id = ? AND id = ?")
                .bind(expiry_date)
                .bind(tenant_id)
                .bind(id)
                .execute(&mut **tx)
                .await
                .map_err(map_db_error)?;
            id
        }
        Some((id, _)) => id,
        None => {
            let id = new_id();
            sqlx::query(
                "INSERT INTO product_batches (tenant_id, id, variant_id, batch_no, expiry_date, created_by) \
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(tenant_id)
            .bind(id)
            .bind(variant_id)
            .bind(&number)
            .bind(expiry_date)
            .bind(tx.user_id())
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
            id
        }
    };
    sqlx::query(
        "UPDATE products p JOIN product_variants v ON v.tenant_id = p.tenant_id AND v.product_id = p.id \
         SET p.track_batches = TRUE WHERE v.tenant_id = ? AND v.id = ? AND NOT p.track_batches",
    )
    .bind(tenant_id)
    .bind(variant_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(Some(id))
}

/// A batch of a variant at a location, with what is left of it.
#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct BatchStock {
    pub batch_id: Uuid,
    pub batch_no: String,
    pub expiry_date: Option<NaiveDate>,
    /// Days until expiry (negative = expired); `null` without a date.
    pub days_left: Option<i32>,
    pub variant_id: Uuid,
    pub product_id: Uuid,
    pub name: String,
    pub sku: String,
    pub unit_short_name: String,
    pub location_id: Uuid,
    pub location_name: String,
    #[schema(value_type = String)]
    pub quantity: Decimal,
    /// Quantity × the location's average cost; `null` without `products.view_cost`.
    #[schema(value_type = Option<String>)]
    pub value: Option<Decimal>,
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct BatchFilter {
    /// Blank = every location of the selected branch.
    pub location_id: Option<Uuid>,
    pub variant_id: Option<Uuid>,
    /// Product name or SKU containing this text, or the batch number.
    pub q: Option<String>,
    /// `all` (default), `expired`, `expiring` (within `days`) or `ok`.
    pub status: Option<String>,
    /// Window for `expiring`; default 30, at most 3650.
    pub days: Option<u32>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct BatchSummary {
    /// Batches in stock that are past their expiry date.
    pub expired: i64,
    /// Batches in stock expiring within the window.
    pub expiring: i64,
    /// Value of the expired stock; `null` without `products.view_cost`.
    #[schema(value_type = Option<String>)]
    pub expired_value: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct BatchPage {
    pub data: Vec<BatchStock>,
    pub next_cursor: Option<String>,
    pub summary: BatchSummary,
}

const DEFAULT_DAYS: u32 = 30;

fn batch_base(
    q: &mut QueryBuilder<MySql>,
    tenant_id: Uuid,
    branch_id: Option<Uuid>,
    filter: &BatchFilter,
    today: NaiveDate,
) -> AppResult<()> {
    q.push(
        " FROM stock_batch_balances bb \
         JOIN product_batches pb ON pb.tenant_id = bb.tenant_id AND pb.id = bb.batch_id \
         JOIN product_variants v ON v.tenant_id = bb.tenant_id AND v.id = bb.variant_id \
         JOIN products p ON p.tenant_id = v.tenant_id AND p.id = v.product_id \
         JOIN units u ON u.tenant_id = p.tenant_id AND u.id = p.base_unit_id \
         JOIN stock_locations l ON l.tenant_id = bb.tenant_id AND l.id = bb.location_id \
         LEFT JOIN stock_balances sb ON sb.tenant_id = bb.tenant_id AND sb.location_id = bb.location_id \
           AND sb.variant_id = bb.variant_id \
         WHERE bb.tenant_id = ",
    );
    q.push_bind(tenant_id).push(" AND bb.quantity > 0 AND p.deleted_at IS NULL");
    if let Some(location_id) = filter.location_id {
        q.push(" AND bb.location_id = ").push_bind(location_id);
    }
    if let Some(branch_id) = branch_id {
        q.push(" AND l.branch_id = ").push_bind(branch_id);
    }
    if let Some(variant_id) = filter.variant_id {
        q.push(" AND bb.variant_id = ").push_bind(variant_id);
    }
    if let Some(text) = filter.q.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        if text.chars().count() > 100 {
            return Err(AppError::field("q", "TOO_LONG", "search for at most 100 characters"));
        }
        q.push(" AND (v.search_text LIKE ").push_bind(like_contains(text));
        q.push(" OR pb.batch_no = ").push_bind(text.to_owned()).push(")");
    }
    let horizon = horizon(today, filter);
    match filter.status.as_deref() {
        None | Some("all") => {}
        Some("expired") => {
            q.push(" AND pb.expiry_date < ").push_bind(today);
        }
        Some("expiring") => {
            q.push(" AND pb.expiry_date >= ").push_bind(today).push(" AND pb.expiry_date <= ").push_bind(horizon);
        }
        Some("ok") => {
            q.push(" AND (pb.expiry_date IS NULL OR pb.expiry_date > ").push_bind(horizon).push(")");
        }
        Some(_) => return Err(AppError::field("status", "INVALID_STATUS", "use all, expired, expiring or ok")),
    }
    Ok(())
}

/// The last day of the "expiring" window.
fn horizon(today: NaiveDate, filter: &BatchFilter) -> NaiveDate {
    today + chrono::Days::new(u64::from(filter.days.unwrap_or(DEFAULT_DAYS)))
}

/// Batches in stock, earliest expiry first (the expiry report).
pub async fn list(tx: &mut TenantTx, filter: &BatchFilter, can_view_cost: bool) -> AppResult<BatchPage> {
    if filter.days.is_some_and(|d| d > 3650) {
        return Err(AppError::field("days", "OUT_OF_RANGE", "look ahead at most 3650 days"));
    }
    let limit = page::limit(filter.limit)?;
    let start = OffsetCursor::start(filter.cursor.as_deref())?;
    let tenant_id = tx.tenant_id();
    let branch_id = tx.branch_id();
    let today = context::today(tx).await?;

    let mut q: QueryBuilder<MySql> =
        QueryBuilder::new("SELECT pb.id AS batch_id, pb.batch_no, pb.expiry_date, CAST(DATEDIFF(pb.expiry_date, ");
    q.push_bind(today).push(
        ") AS SIGNED) AS days_left, v.id AS variant_id, \
         p.id AS product_id, p.name, v.sku, u.short_name AS unit_short_name, l.id AS location_id, \
         l.name AS location_name, bb.quantity, ROUND(bb.quantity * COALESCE(sb.avg_cost, 0), 2) AS value",
    );
    batch_base(&mut q, tenant_id, branch_id, filter, today)?;
    q.push(" ORDER BY pb.expiry_date IS NULL, pb.expiry_date, p.name, pb.batch_no, l.name LIMIT ")
        .push_bind(limit + 1)
        .push(" OFFSET ")
        .push_bind(start);
    let mut rows: Vec<BatchStock> = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    let next_cursor = OffsetCursor::next(start, limit, rows.len());
    rows.truncate(limit as usize);
    if !can_view_cost {
        for row in &mut rows {
            row.value = None;
        }
    }

    // The summary ignores the status filter: it is what the report's tabs count.
    let all = BatchFilter { status: None, limit: None, cursor: None, ..filter.clone() };
    let mut q: QueryBuilder<MySql> = QueryBuilder::new("SELECT CAST(COALESCE(SUM(pb.expiry_date < ");
    q.push_bind(today)
        .push("), 0) AS SIGNED), CAST(COALESCE(SUM(pb.expiry_date >= ")
        .push_bind(today)
        .push(" AND pb.expiry_date <= ")
        .push_bind(horizon(today, filter))
        .push("), 0) AS SIGNED), COALESCE(SUM(CASE WHEN pb.expiry_date < ")
        .push_bind(today)
        .push(" THEN ROUND(bb.quantity * COALESCE(sb.avg_cost, 0), 2) END), 0)");
    batch_base(&mut q, tenant_id, branch_id, &all, today)?;
    let (expired, expiring, expired_value): (i64, i64, Decimal) =
        q.build_query_as().fetch_one(&mut **tx).await.map_err(map_db_error)?;
    Ok(BatchPage {
        data: rows,
        next_cursor,
        summary: BatchSummary { expired, expiring, expired_value: can_view_cost.then_some(expired_value) },
    })
}

/// What one document line moved, batch by batch.
#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct LineBatch {
    /// The document line (`reference_line_id` of the movement).
    pub line_id: Uuid,
    pub batch_id: Uuid,
    pub batch_no: String,
    pub expiry_date: Option<NaiveDate>,
    /// Signed: + in, − out.
    #[schema(value_type = String)]
    pub quantity: Decimal,
}

/// The batches each line of a posted document moved, by line id.
pub async fn of_document(
    tx: &mut TenantTx,
    reference_type: &str,
    reference_id: Uuid,
) -> AppResult<BTreeMap<Uuid, Vec<LineBatch>>> {
    let rows: Vec<LineBatch> = sqlx::query_as(
        "SELECT m.reference_line_id AS line_id, pb.id AS batch_id, pb.batch_no, pb.expiry_date, SUM(m.quantity) AS quantity \
         FROM stock_batch_movements m JOIN product_batches pb ON pb.tenant_id = m.tenant_id AND pb.id = m.batch_id \
         WHERE m.tenant_id = ? AND m.reference_type = ? AND m.reference_id = ? AND m.reference_line_id IS NOT NULL \
         GROUP BY m.reference_line_id, pb.id, pb.batch_no, pb.expiry_date \
         ORDER BY pb.expiry_date IS NULL, pb.expiry_date, pb.batch_no",
    )
    .bind(tx.tenant_id())
    .bind(reference_type)
    .bind(reference_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let mut out: BTreeMap<Uuid, Vec<LineBatch>> = BTreeMap::new();
    for row in rows {
        out.entry(row.line_id).or_default().push(row);
    }
    Ok(out)
}

/// A batch that can be picked for a write-off or transfer.
#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct BatchOption {
    pub batch_id: Uuid,
    pub variant_id: Uuid,
    pub batch_no: String,
    pub expiry_date: Option<NaiveDate>,
    #[schema(value_type = String)]
    pub quantity: Decimal,
}

/// Batches in stock at a location for some variants, earliest expiry first.
pub async fn at_location(tx: &mut TenantTx, location_id: Uuid, variant_ids: &[Uuid]) -> AppResult<Vec<BatchOption>> {
    if variant_ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "SELECT pb.id AS batch_id, bb.variant_id, pb.batch_no, pb.expiry_date, bb.quantity \
         FROM stock_batch_balances bb JOIN product_batches pb ON pb.tenant_id = bb.tenant_id AND pb.id = bb.batch_id \
         WHERE bb.tenant_id = ",
    );
    q.push_bind(tx.tenant_id())
        .push(" AND bb.location_id = ")
        .push_bind(location_id)
        .push(" AND bb.quantity > 0 AND bb.variant_id IN (");
    let mut list = q.separated(", ");
    for id in variant_ids {
        list.push_bind(*id);
    }
    q.push(") ORDER BY pb.expiry_date IS NULL, pb.expiry_date, pb.batch_no");
    q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)
}

/// Checks that `batch_id` is a batch of `variant_id`.
pub async fn require_of_variant(tx: &mut TenantTx, batch_id: Uuid, variant_id: Uuid, field: &str) -> AppResult<()> {
    let ok: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM product_batches WHERE tenant_id = ? AND id = ? AND variant_id = ?)",
    )
    .bind(tx.tenant_id())
    .bind(batch_id)
    .bind(variant_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if ok { Ok(()) } else { Err(AppError::field(field, "NOT_FOUND", "this batch is not of this product")) }
}
