//! Stock adjustments and opening stock (STK-R03, R04; DB design §10.6).
//!
//! Release 1 posts an adjustment as soon as it is saved (`draft → posted` in
//! one step). Opening stock is an adjustment with reason `opening`, which
//! writes `opening` movements and credits Opening Balance Equity; other
//! reasons post against Inventory Adjustments (§8.7).

use std::collections::BTreeSet;

use chrono::{DateTime, NaiveDate, Utc};
use pos_accounting::{NewJournal, SystemAccounts, journal};
use pos_audit::AuditEntry;
use pos_catalog::variants;
use pos_core::{
    AppError, AppResult, RequestMeta,
    id::new_id,
    money::{price, round3},
    page::{self, Keyset},
};
use pos_db::{TenantTx, map_db_error};
use pos_tenancy::{context, sequences};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

use crate::{
    posting::{MovementLine, MovementRef, balances_at, post_movements},
    stock::check_location,
};

pub const REASONS: &[&str] =
    &["opening", "damaged", "lost", "found", "expired", "correction", "internal_use", "transit_loss", "other"];
pub const MAX_LINES: usize = 500;

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AdjustmentLineInput {
    pub variant_id: Uuid,
    /// Signed base quantity: + adds stock, − removes it. Opening stock is always +.
    #[schema(value_type = String)]
    pub quantity: Decimal,
    /// Cost per unit of an increase. Blank = current average (or the product's
    /// cost when there is no stock). Needs `products.view_cost`.
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub unit_cost: Option<Decimal>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AdjustmentInput {
    /// `opening`, `damaged`, `lost`, `found`, `expired`, `correction`,
    /// `internal_use`, `transit_loss` or `other`.
    pub reason: String,
    /// Blank = the branch's default location.
    #[serde(default)]
    pub location_id: Option<Uuid>,
    #[serde(default)]
    pub note: Option<String>,
    pub lines: Vec<AdjustmentLineInput>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct AdjustmentItem {
    pub id: Uuid,
    pub variant_id: Uuid,
    pub product_name: String,
    pub sku: String,
    pub unit_short_name: String,
    #[schema(value_type = String)]
    pub quantity: Decimal,
    /// `null` without `products.view_cost`.
    #[schema(value_type = Option<String>)]
    pub unit_cost: Option<Decimal>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct AdjustmentSummary {
    pub id: Uuid,
    pub adjustment_no: Option<String>,
    pub business_date: NaiveDate,
    pub reason: String,
    pub note: Option<String>,
    pub status: String,
    pub location_id: Uuid,
    pub location_name: String,
    /// Signed value; `null` without `products.view_cost`.
    #[schema(value_type = Option<String>)]
    pub total_value: Option<Decimal>,
    pub item_count: i64,
    pub created_by_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct Adjustment {
    #[serde(flatten)]
    pub summary: AdjustmentSummary,
    pub items: Vec<AdjustmentItem>,
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct AdjustmentFilter {
    pub reason: Option<String>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct AdjustmentPage {
    pub data: Vec<AdjustmentSummary>,
    pub next_cursor: Option<String>,
}

const SUMMARY_SELECT: &str = "SELECT a.id, a.adjustment_no, a.business_date, a.reason, a.note, a.status, \
     a.location_id, l.name AS location_name, a.total_value, \
     (SELECT COUNT(*) FROM stock_adjustment_items i WHERE i.tenant_id = a.tenant_id AND i.adjustment_id = a.id) AS item_count, \
     (SELECT u.full_name FROM users u WHERE u.id = a.created_by) AS created_by_name, a.created_at \
     FROM stock_adjustments a JOIN stock_locations l ON l.tenant_id = a.tenant_id AND l.id = a.location_id \
     WHERE a.tenant_id = ";

pub async fn list(tx: &mut TenantTx, filter: &AdjustmentFilter, can_view_cost: bool) -> AppResult<AdjustmentPage> {
    let limit = page::limit(filter.limit)?;
    let after = filter.cursor.as_deref().map(Keyset::decode).transpose()?;
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(SUMMARY_SELECT);
    q.push_bind(tenant_id);
    if let Some(reason) = filter.reason.as_deref().filter(|r| !r.is_empty()) {
        q.push(" AND a.reason = ").push_bind(reason.to_owned());
    }
    if let Some(from) = filter.from {
        q.push(" AND a.business_date >= ").push_bind(from);
    }
    if let Some(to) = filter.to {
        q.push(" AND a.business_date <= ").push_bind(to);
    }
    if let Some(k) = after {
        q.push(" AND (a.created_at < ").push_bind(k.at);
        q.push(" OR (a.created_at = ").push_bind(k.at).push(" AND a.id < ").push_bind(k.id).push("))");
    }
    q.push(" ORDER BY a.created_at DESC, a.id DESC LIMIT ").push_bind(limit + 1);
    let mut rows: Vec<AdjustmentSummary> = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    let next_cursor = if rows.len() > limit as usize {
        rows.truncate(limit as usize);
        rows.last().map(|r| Keyset { at: r.created_at, id: r.id }.encode())
    } else {
        None
    };
    if !can_view_cost {
        for row in &mut rows {
            row.total_value = None;
        }
    }
    Ok(AdjustmentPage { data: rows, next_cursor })
}

pub async fn get(tx: &mut TenantTx, id: Uuid, can_view_cost: bool) -> AppResult<Adjustment> {
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(SUMMARY_SELECT);
    q.push_bind(tenant_id).push(" AND a.id = ").push_bind(id);
    let mut summary: AdjustmentSummary =
        q.build_query_as().fetch_optional(&mut **tx).await.map_err(map_db_error)?.ok_or(AppError::NotFound)?;
    let mut items: Vec<AdjustmentItem> = sqlx::query_as(
        "SELECT i.id, i.variant_id, p.name AS product_name, v.sku, u.short_name AS unit_short_name, i.quantity, \
         i.unit_cost, i.note FROM stock_adjustment_items i \
         JOIN product_variants v ON v.tenant_id = i.tenant_id AND v.id = i.variant_id \
         JOIN products p ON p.tenant_id = v.tenant_id AND p.id = v.product_id \
         JOIN units u ON u.tenant_id = p.tenant_id AND u.id = p.base_unit_id \
         WHERE i.tenant_id = ? AND i.adjustment_id = ? ORDER BY p.name",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if !can_view_cost {
        summary.total_value = None;
        for item in &mut items {
            item.unit_cost = None;
        }
    }
    Ok(Adjustment { summary, items })
}

struct Line {
    variant_id: Uuid,
    quantity: Decimal,
    unit_cost: Option<Decimal>,
    note: Option<String>,
}

fn validate(input: &AdjustmentInput, can_view_cost: bool) -> AppResult<Vec<Line>> {
    if !REASONS.contains(&input.reason.as_str()) {
        return Err(AppError::field("reason", "INVALID_REASON", "choose a reason from the list"));
    }
    if input.lines.is_empty() || input.lines.len() > MAX_LINES {
        return Err(AppError::field("lines", "INVALID_LINES", format!("add between 1 and {MAX_LINES} products")));
    }
    let opening = input.reason == "opening";
    let mut seen = BTreeSet::new();
    let mut out = Vec::with_capacity(input.lines.len());
    for (i, line) in input.lines.iter().enumerate() {
        if !seen.insert(line.variant_id) {
            return Err(AppError::field(
                &format!("lines[{i}].variant_id"),
                "DUPLICATE",
                "each product may appear once",
            ));
        }
        let quantity = round3(line.quantity);
        if quantity.is_zero() || quantity != line.quantity.normalize() || quantity.abs() > Decimal::from(1_000_000_000)
        {
            return Err(AppError::field(
                &format!("lines[{i}].quantity"),
                "INVALID_QUANTITY",
                "enter a quantity other than 0 with at most 3 decimals",
            ));
        }
        if opening && quantity < Decimal::ZERO {
            return Err(AppError::field(
                &format!("lines[{i}].quantity"),
                "POSITIVE",
                "opening stock must be more than 0",
            ));
        }
        if line.unit_cost.is_some() && !can_view_cost {
            return Err(AppError::Forbidden);
        }
        let unit_cost = line.unit_cost.map(|c| price(c, &format!("lines[{i}].unit_cost"))).transpose()?;
        let note = line.note.as_deref().map(str::trim).filter(|n| !n.is_empty()).map(|n| n.chars().take(255).collect());
        out.push(Line { variant_id: line.variant_id, quantity, unit_cost, note });
    }
    Ok(out)
}

/// Saves and posts an adjustment (or opening stock).
pub async fn create(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    input: AdjustmentInput,
    can_view_cost: bool,
) -> AppResult<Adjustment> {
    let lines = validate(&input, can_view_cost)?;
    let ctx = context::load(tx).await?;
    let location_id = input.location_id.unwrap_or(ctx.location_id);
    check_location(tx, location_id).await?;
    let ids: Vec<Uuid> = lines.iter().map(|l| l.variant_id).collect();
    let info = variants::require(tx, &ids, "lines", false).await?;
    for (i, line) in lines.iter().enumerate() {
        let v = &info[&line.variant_id];
        if !v.track_inventory {
            return Err(AppError::field(
                &format!("lines[{i}].variant_id"),
                "NOT_STOCKED",
                "this product does not track stock",
            ));
        }
        if !v.allow_fraction && !line.quantity.fract().is_zero() {
            return Err(AppError::field(
                &format!("lines[{i}].quantity"),
                "WHOLE_NUMBER",
                "this unit takes whole numbers only",
            ));
        }
    }
    let note = input
        .note
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(|n| n.chars().take(2000).collect::<String>());
    let opening = input.reason == "opening";

    // An increase without a cost comes in at the average, or at the product's
    // cost when there is no stock to average with (STK-R04).
    let current = balances_at(tx, location_id, &ids).await?;
    let costs: Vec<Option<Decimal>> = lines
        .iter()
        .map(|l| match l.unit_cost {
            Some(c) => Some(c),
            None if l.quantity > Decimal::ZERO => {
                let (qty, avg) = current.get(&l.variant_id).copied().unwrap_or_default();
                if qty <= Decimal::ZERO || avg.is_zero() { Some(info[&l.variant_id].purchase_price) } else { None }
            }
            None => None,
        })
        .collect();

    let tenant_id = tx.tenant_id();
    let user_id = tx.user_id();
    let id = new_id();
    sqlx::query(
        "INSERT INTO stock_adjustments (tenant_id, id, branch_id, location_id, business_date, reason, note, status, created_by) \
         VALUES (?, ?, ?, ?, ?, ?, ?, 'draft', ?)",
    )
    .bind(tenant_id)
    .bind(id)
    .bind(ctx.branch_id)
    .bind(location_id)
    .bind(ctx.business_date)
    .bind(&input.reason)
    .bind(&note)
    .bind(user_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let item_ids: Vec<Uuid> = lines.iter().map(|_| new_id()).collect();

    let movement_type = if opening { "opening" } else { "adjustment" };
    let movement_lines: Vec<MovementLine> = lines
        .iter()
        .zip(&costs)
        .zip(&item_ids)
        .map(|((l, cost), item_id)| MovementLine {
            location_id,
            variant_id: l.variant_id,
            movement_type,
            quantity: l.quantity,
            unit_cost: *cost,
            reference_line_id: Some(*item_id),
            reason: Some(input.reason.clone()),
        })
        .collect();

    let number = sequences::next(tx, "adjustment", Some(ctx.branch_id), ctx.business_date).await?;
    let reference = MovementRef {
        branch_id: ctx.branch_id,
        business_date: ctx.business_date,
        reference_type: "stock_adjustment",
        reference_id: id,
        reference_no: Some(number.clone()),
    };
    let posted = post_movements(tx, &reference, &movement_lines, ctx.allow_negative_stock).await?;

    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "INSERT INTO stock_adjustment_items (tenant_id, id, adjustment_id, variant_id, quantity, unit_cost, note) ",
    );
    q.push_values(lines.iter().zip(&posted).zip(&item_ids), |mut row, ((l, p), item_id)| {
        row.push_bind(tenant_id)
            .push_bind(*item_id)
            .push_bind(id)
            .push_bind(l.variant_id)
            .push_bind(l.quantity)
            .push_bind(p.unit_cost)
            .push_bind(l.note.as_deref());
    });
    q.build().execute(&mut **tx).await.map_err(map_db_error)?;

    let total_value: Decimal = posted.iter().map(|p| p.total_cost).sum();
    sqlx::query(
        "UPDATE stock_adjustments SET adjustment_no = ?, status = 'posted', total_value = ?, posted_at = UTC_TIMESTAMP(6), \
         posted_by = ? WHERE tenant_id = ? AND id = ?",
    )
    .bind(&number)
    .bind(total_value)
    .bind(user_id)
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;

    let accounts = SystemAccounts::load(tx).await?;
    let counter = if opening { accounts.opening_equity()? } else { accounts.inventory_adjustment()? };
    let mut j = NewJournal::new(ctx.business_date, "stock_adjustment", id)
        .branch(ctx.branch_id)
        .number(number.clone())
        .memo(if opening { "Opening stock".to_owned() } else { format!("Stock adjustment ({})", input.reason) });
    j.debit(accounts.inventory()?, total_value).credit(counter, total_value);
    journal::post(tx, j).await?;

    let action = if opening { "stock.opening_posted" } else { "stock.adjustment_posted" };
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new(action).entity("stock_adjustment", id).branch(ctx.branch_id).change(
            None,
            Some(serde_json::json!({ "adjustment_no": number, "reason": input.reason, "lines": lines.len(), "total_value": total_value })),
        ),
    )
    .await?;
    get(tx, id, can_view_cost).await
}
