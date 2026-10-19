//! Stock transfers between locations, usually of two branches (SRS-BRN-004,
//! DB design §9.8 Release 2).
//!
//! `draft → dispatched → received`, or `draft → cancelled`. Dispatch takes the
//! goods out of the source at its average cost (`transfer_out`); receiving
//! brings them in at that same cost (`transfer_in`), so value moves with the
//! goods. Goods sent but not received are written off as transit loss
//! (Dr Inventory Adjustments / Cr Inventory). While a transfer is on the road
//! its value is in neither location's stock but stays in the Inventory account.

use std::collections::BTreeSet;

use chrono::{DateTime, NaiveDate, Utc};
use pos_accounting::{NewJournal, SystemAccounts, journal};
use pos_audit::AuditEntry;
use pos_catalog::variants;
use pos_core::{
    AppError, AppResult, RequestMeta,
    id::new_id,
    money::{round2, round3},
    page::{self, Keyset},
};
use pos_db::{TenantTx, map_db_error};
use pos_tenancy::{context, sequences};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

use crate::posting::{MovementLine, MovementRef, post_movements};

pub const MAX_LINES: usize = 500;

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TransferLineInput {
    pub variant_id: Uuid,
    /// Base quantity to send, more than 0.
    #[schema(value_type = String)]
    pub quantity: Decimal,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[schema(as = StockTransferInput)]
#[serde(deny_unknown_fields)]
pub struct TransferInput {
    /// Blank = the selected branch's default location.
    #[serde(default)]
    pub from_location_id: Option<Uuid>,
    pub to_location_id: Uuid,
    #[serde(default)]
    pub note: Option<String>,
    pub lines: Vec<TransferLineInput>,
    /// Dispatch right away instead of saving a draft.
    #[serde(default)]
    pub dispatch: bool,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReceiveLineInput {
    pub item_id: Uuid,
    #[schema(value_type = String)]
    pub quantity_received: Decimal,
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReceiveInput {
    /// Lines left out arrive in full.
    #[serde(default)]
    pub lines: Vec<ReceiveLineInput>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct TransferItem {
    pub id: Uuid,
    pub variant_id: Uuid,
    pub product_name: String,
    pub sku: String,
    pub unit_short_name: String,
    #[schema(value_type = String)]
    pub quantity_sent: Decimal,
    #[schema(value_type = Option<String>)]
    pub quantity_received: Option<Decimal>,
    /// Source average at dispatch; `null` before dispatch or without `products.view_cost`.
    #[schema(value_type = Option<String>)]
    pub unit_cost: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct TransferSummary {
    pub id: Uuid,
    pub transfer_no: Option<String>,
    pub business_date: NaiveDate,
    /// `draft`, `dispatched`, `received` or `cancelled`.
    pub status: String,
    pub from_branch_id: Uuid,
    pub from_branch_name: String,
    pub from_location_id: Uuid,
    pub from_location_name: String,
    pub to_branch_id: Uuid,
    pub to_branch_name: String,
    pub to_location_id: Uuid,
    pub to_location_name: String,
    pub note: Option<String>,
    /// `null` without `products.view_cost`.
    #[schema(value_type = Option<String>)]
    pub total_value: Option<Decimal>,
    #[schema(value_type = Option<String>)]
    pub loss_value: Option<Decimal>,
    pub item_count: i64,
    pub created_by_name: Option<String>,
    pub dispatched_at: Option<DateTime<Utc>>,
    pub received_at: Option<DateTime<Utc>>,
    pub version: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[schema(as = StockTransfer)]
pub struct Transfer {
    #[serde(flatten)]
    pub summary: TransferSummary,
    pub items: Vec<TransferItem>,
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct TransferFilter {
    pub status: Option<String>,
    /// `out` (sent from the selected branch) or `in` (coming to it).
    pub direction: Option<String>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[schema(as = StockTransferPage)]
pub struct TransferPage {
    pub data: Vec<TransferSummary>,
    pub next_cursor: Option<String>,
}

const SUMMARY_SELECT: &str = "SELECT t.id, t.transfer_no, t.business_date, t.status, \
     t.from_branch_id, fb.name AS from_branch_name, t.from_location_id, fl.name AS from_location_name, \
     t.to_branch_id, tb.name AS to_branch_name, t.to_location_id, tl.name AS to_location_name, \
     t.note, t.total_value, t.loss_value, \
     (SELECT COUNT(*) FROM stock_transfer_items i WHERE i.tenant_id = t.tenant_id AND i.transfer_id = t.id) AS item_count, \
     (SELECT u.full_name FROM users u WHERE u.id = t.created_by) AS created_by_name, \
     t.dispatched_at, t.received_at, t.version, t.created_at \
     FROM stock_transfers t \
     JOIN branches fb ON fb.tenant_id = t.tenant_id AND fb.id = t.from_branch_id \
     JOIN branches tb ON tb.tenant_id = t.tenant_id AND tb.id = t.to_branch_id \
     JOIN stock_locations fl ON fl.tenant_id = t.tenant_id AND fl.id = t.from_location_id \
     JOIN stock_locations tl ON tl.tenant_id = t.tenant_id AND tl.id = t.to_location_id \
     WHERE t.tenant_id = ";

pub async fn list(tx: &mut TenantTx, filter: &TransferFilter, can_view_cost: bool) -> AppResult<TransferPage> {
    let limit = page::limit(filter.limit)?;
    let after = filter.cursor.as_deref().map(Keyset::decode).transpose()?;
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(SUMMARY_SELECT);
    q.push_bind(tenant_id);
    if let Some(branch_id) = tx.branch_id() {
        match filter.direction.as_deref() {
            Some("out") => {
                q.push(" AND t.from_branch_id = ").push_bind(branch_id);
            }
            Some("in") => {
                q.push(" AND t.to_branch_id = ").push_bind(branch_id);
            }
            _ => {
                q.push(" AND (t.from_branch_id = ").push_bind(branch_id);
                q.push(" OR t.to_branch_id = ").push_bind(branch_id).push(")");
            }
        }
    }
    if let Some(status) = filter.status.as_deref().filter(|s| !s.is_empty()) {
        if !["draft", "dispatched", "received", "cancelled"].contains(&status) {
            return Err(AppError::field("status", "INVALID_STATUS", "unknown transfer status"));
        }
        q.push(" AND t.status = ").push_bind(status.to_owned());
    }
    if let Some(from) = filter.from {
        q.push(" AND t.business_date >= ").push_bind(from);
    }
    if let Some(to) = filter.to {
        q.push(" AND t.business_date <= ").push_bind(to);
    }
    if let Some(k) = after {
        q.push(" AND (t.created_at < ").push_bind(k.at);
        q.push(" OR (t.created_at = ").push_bind(k.at).push(" AND t.id < ").push_bind(k.id).push("))");
    }
    q.push(" ORDER BY t.created_at DESC, t.id DESC LIMIT ").push_bind(limit + 1);
    let mut rows: Vec<TransferSummary> = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    let next_cursor = if rows.len() > limit as usize {
        rows.truncate(limit as usize);
        rows.last().map(|r| Keyset { at: r.created_at, id: r.id }.encode())
    } else {
        None
    };
    if !can_view_cost {
        for row in &mut rows {
            row.total_value = None;
            row.loss_value = None;
        }
    }
    Ok(TransferPage { data: rows, next_cursor })
}

pub async fn get(tx: &mut TenantTx, id: Uuid, can_view_cost: bool) -> AppResult<Transfer> {
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(SUMMARY_SELECT);
    q.push_bind(tenant_id).push(" AND t.id = ").push_bind(id);
    let mut summary: TransferSummary =
        q.build_query_as().fetch_optional(&mut **tx).await.map_err(map_db_error)?.ok_or(AppError::NotFound)?;
    let mut items: Vec<TransferItem> = sqlx::query_as(
        "SELECT i.id, i.variant_id, p.name AS product_name, v.sku, u.short_name AS unit_short_name, \
         i.quantity_sent, i.quantity_received, i.unit_cost FROM stock_transfer_items i \
         JOIN product_variants v ON v.tenant_id = i.tenant_id AND v.id = i.variant_id \
         JOIN products p ON p.tenant_id = v.tenant_id AND p.id = v.product_id \
         JOIN units u ON u.tenant_id = p.tenant_id AND u.id = p.base_unit_id \
         WHERE i.tenant_id = ? AND i.transfer_id = ? ORDER BY p.name",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if !can_view_cost {
        summary.total_value = None;
        summary.loss_value = None;
        for item in &mut items {
            item.unit_cost = None;
        }
    }
    Ok(Transfer { summary, items })
}

/// The branch a location belongs to (active locations only).
async fn location_branch(tx: &mut TenantTx, location_id: Uuid, field: &str) -> AppResult<Uuid> {
    let tenant_id = tx.tenant_id();
    let row: Option<(Uuid, String, String)> = sqlx::query_as(
        "SELECT l.branch_id, l.status, b.status FROM stock_locations l \
         JOIN branches b ON b.tenant_id = l.tenant_id AND b.id = l.branch_id WHERE l.tenant_id = ? AND l.id = ?",
    )
    .bind(tenant_id)
    .bind(location_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    match row {
        Some((branch_id, l, b)) if l == "active" && b == "active" => Ok(branch_id),
        _ => Err(AppError::field(field, "NOT_FOUND", "choose an active stock location")),
    }
}

/// Saves a draft transfer, dispatching it at once when asked.
pub async fn create(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    input: TransferInput,
    can_view_cost: bool,
) -> AppResult<Transfer> {
    if input.lines.is_empty() || input.lines.len() > MAX_LINES {
        return Err(AppError::field("lines", "INVALID_LINES", format!("add between 1 and {MAX_LINES} products")));
    }
    let ctx = context::load(tx).await?;
    let from_location_id = input.from_location_id.unwrap_or(ctx.location_id);
    if from_location_id == input.to_location_id {
        return Err(AppError::field("to_location_id", "SAME_LOCATION", "choose a different destination"));
    }
    let from_branch_id = location_branch(tx, from_location_id, "from_location_id").await?;
    let to_branch_id = location_branch(tx, input.to_location_id, "to_location_id").await?;
    if tx.branch_id().is_some_and(|b| b != from_branch_id) {
        return Err(AppError::field(
            "from_location_id",
            "OTHER_BRANCH",
            "send stock from a location of the branch you are working in",
        ));
    }

    let mut seen = BTreeSet::new();
    let ids: Vec<Uuid> = input.lines.iter().map(|l| l.variant_id).collect();
    let info = variants::require(tx, &ids, "lines", false).await?;
    let mut lines = Vec::with_capacity(input.lines.len());
    for (i, line) in input.lines.iter().enumerate() {
        if !seen.insert(line.variant_id) {
            return Err(AppError::field(
                &format!("lines[{i}].variant_id"),
                "DUPLICATE",
                "each product may appear once",
            ));
        }
        let v = &info[&line.variant_id];
        if !v.track_inventory {
            return Err(AppError::field(
                &format!("lines[{i}].variant_id"),
                "NOT_STOCKED",
                "this product does not track stock",
            ));
        }
        let quantity = round3(line.quantity);
        if quantity <= Decimal::ZERO || quantity != line.quantity.normalize() || quantity > Decimal::from(1_000_000_000)
        {
            return Err(AppError::field(
                &format!("lines[{i}].quantity"),
                "INVALID_QUANTITY",
                "enter a quantity more than 0 with at most 3 decimals",
            ));
        }
        if !v.allow_fraction && !quantity.fract().is_zero() {
            return Err(AppError::field(
                &format!("lines[{i}].quantity"),
                "WHOLE_NUMBER",
                "this unit takes whole numbers only",
            ));
        }
        lines.push((line.variant_id, quantity));
    }
    let note = clean_note(input.note.as_deref());

    let tenant_id = tx.tenant_id();
    let user_id = tx.user_id();
    let id = new_id();
    sqlx::query(
        "INSERT INTO stock_transfers (tenant_id, id, from_branch_id, from_location_id, to_branch_id, to_location_id, \
         business_date, note, created_by) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(id)
    .bind(from_branch_id)
    .bind(from_location_id)
    .bind(to_branch_id)
    .bind(input.to_location_id)
    .bind(ctx.business_date)
    .bind(&note)
    .bind(user_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let mut q: QueryBuilder<MySql> =
        QueryBuilder::new("INSERT INTO stock_transfer_items (tenant_id, id, transfer_id, variant_id, quantity_sent) ");
    q.push_values(&lines, |mut row, (variant_id, quantity)| {
        row.push_bind(tenant_id).push_bind(new_id()).push_bind(id).push_bind(*variant_id).push_bind(*quantity);
    });
    q.build().execute(&mut **tx).await.map_err(map_db_error)?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("stock.transfer_created").entity("stock_transfer", id).branch(from_branch_id).change(
            None,
            Some(serde_json::json!({ "from_location_id": from_location_id, "to_location_id": input.to_location_id, "lines": lines.len() })),
        ),
    )
    .await?;
    if input.dispatch {
        return dispatch(tx, meta, id, can_view_cost).await;
    }
    get(tx, id, can_view_cost).await
}

fn clean_note(note: Option<&str>) -> Option<String> {
    note.map(str::trim).filter(|n| !n.is_empty()).map(|n| n.chars().take(2000).collect())
}

/// Locks the transfer and checks its status.
async fn lock(tx: &mut TenantTx, id: Uuid, expected: &str) -> AppResult<(Uuid, Uuid, Uuid, Uuid)> {
    let tenant_id = tx.tenant_id();
    let row: Option<(String, Uuid, Uuid, Uuid, Uuid)> = sqlx::query_as(
        "SELECT status, from_branch_id, from_location_id, to_branch_id, to_location_id FROM stock_transfers \
         WHERE tenant_id = ? AND id = ? FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let (status, from_branch, from_location, to_branch, to_location) = row.ok_or(AppError::NotFound)?;
    if status != expected {
        return Err(AppError::Conflict {
            code: "INVALID_STATUS",
            message: format!("this transfer is {status}; it must be {expected}"),
        });
    }
    Ok((from_branch, from_location, to_branch, to_location))
}

async fn items(tx: &mut TenantTx, id: Uuid) -> AppResult<Vec<(Uuid, Uuid, Decimal, Option<Decimal>)>> {
    let tenant_id = tx.tenant_id();
    sqlx::query_as(
        "SELECT id, variant_id, quantity_sent, unit_cost FROM stock_transfer_items \
         WHERE tenant_id = ? AND transfer_id = ? ORDER BY variant_id",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)
}

/// Takes the goods out of the source location at its average cost.
pub async fn dispatch(tx: &mut TenantTx, meta: &RequestMeta, id: Uuid, can_view_cost: bool) -> AppResult<Transfer> {
    let (from_branch, from_location, _, _) = lock(tx, id, "draft").await?;
    if tx.branch_id().is_some_and(|b| b != from_branch) {
        return Err(AppError::Denied {
            code: "BRANCH_NOT_ALLOWED",
            message: "switch to the sending branch to dispatch this transfer".into(),
        });
    }
    let ctx = context::load(tx).await?;
    let lines = items(tx, id).await?;
    let number = sequences::next(tx, "transfer", Some(from_branch), ctx.business_date).await?;
    let reference = MovementRef {
        branch_id: from_branch,
        business_date: ctx.business_date,
        reference_type: "stock_transfer",
        reference_id: id,
        reference_no: Some(number.clone()),
    };
    let movement_lines: Vec<MovementLine> = lines
        .iter()
        .map(|(item_id, variant_id, qty, _)| MovementLine {
            location_id: from_location,
            variant_id: *variant_id,
            movement_type: "transfer_out",
            quantity: -*qty,
            unit_cost: None,
            reference_line_id: Some(*item_id),
            reason: None,
        })
        .collect();
    let posted = post_movements(tx, &reference, &movement_lines, ctx.allow_negative_stock).await?;
    let tenant_id = tx.tenant_id();
    for ((item_id, ..), p) in lines.iter().zip(&posted) {
        sqlx::query("UPDATE stock_transfer_items SET unit_cost = ? WHERE tenant_id = ? AND id = ?")
            .bind(p.unit_cost)
            .bind(tenant_id)
            .bind(item_id)
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
    }
    let total_value: Decimal = -posted.iter().map(|p| p.total_cost).sum::<Decimal>();
    let user_id = tx.user_id();
    sqlx::query(
        "UPDATE stock_transfers SET transfer_no = ?, status = 'dispatched', business_date = ?, total_value = ?, \
         dispatched_at = UTC_TIMESTAMP(6), dispatched_by = ?, version = version + 1 WHERE tenant_id = ? AND id = ?",
    )
    .bind(&number)
    .bind(ctx.business_date)
    .bind(total_value)
    .bind(user_id)
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("stock.transfer_dispatched")
            .entity("stock_transfer", id)
            .branch(from_branch)
            .change(None, Some(serde_json::json!({ "transfer_no": number, "total_value": total_value }))),
    )
    .await?;
    get(tx, id, can_view_cost).await
}

/// Brings the goods in at the destination; any shortfall is transit loss.
pub async fn receive(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    id: Uuid,
    input: ReceiveInput,
    can_view_cost: bool,
) -> AppResult<Transfer> {
    let (_, _, to_branch, to_location) = lock(tx, id, "dispatched").await?;
    if tx.branch_id().is_some_and(|b| b != to_branch) {
        return Err(AppError::Denied {
            code: "BRANCH_NOT_ALLOWED",
            message: "switch to the receiving branch to receive this transfer".into(),
        });
    }
    let ctx = context::load(tx).await?;
    let lines = items(tx, id).await?;
    let mut received: Vec<Decimal> = lines.iter().map(|(_, _, sent, _)| *sent).collect();
    for (i, r) in input.lines.iter().enumerate() {
        let Some(pos) = lines.iter().position(|(item_id, ..)| *item_id == r.item_id) else {
            return Err(AppError::field(&format!("lines[{i}].item_id"), "NOT_FOUND", "not a line of this transfer"));
        };
        let qty = round3(r.quantity_received);
        if qty < Decimal::ZERO || qty > lines[pos].2 || qty != r.quantity_received.normalize() {
            return Err(AppError::field(
                &format!("lines[{i}].quantity_received"),
                "INVALID_QUANTITY",
                "enter 0 up to the quantity sent",
            ));
        }
        received[pos] = qty;
    }

    let number: Option<String> = {
        let tenant_id = tx.tenant_id();
        sqlx::query_scalar("SELECT transfer_no FROM stock_transfers WHERE tenant_id = ? AND id = ?")
            .bind(tenant_id)
            .bind(id)
            .fetch_one(&mut **tx)
            .await
            .map_err(map_db_error)?
    };
    let reference = MovementRef {
        branch_id: to_branch,
        business_date: ctx.business_date,
        reference_type: "stock_transfer",
        reference_id: id,
        reference_no: number.clone(),
    };
    let movement_lines: Vec<MovementLine> = lines
        .iter()
        .zip(&received)
        .filter(|(_, qty)| !qty.is_zero())
        .map(|((item_id, variant_id, _, cost), qty)| MovementLine {
            location_id: to_location,
            variant_id: *variant_id,
            movement_type: "transfer_in",
            quantity: *qty,
            unit_cost: Some(cost.unwrap_or_default()),
            reference_line_id: Some(*item_id),
            reason: None,
        })
        .collect();
    post_movements(tx, &reference, &movement_lines, true).await?;

    let tenant_id = tx.tenant_id();
    let mut loss = Decimal::ZERO;
    for ((item_id, _, sent, cost), qty) in lines.iter().zip(&received) {
        loss += round2((*sent - *qty) * cost.unwrap_or_default());
        sqlx::query("UPDATE stock_transfer_items SET quantity_received = ? WHERE tenant_id = ? AND id = ?")
            .bind(*qty)
            .bind(tenant_id)
            .bind(item_id)
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
    }
    let user_id = tx.user_id();
    let note = clean_note(input.note.as_deref());
    sqlx::query(
        "UPDATE stock_transfers SET status = 'received', loss_value = ?, received_at = UTC_TIMESTAMP(6), received_by = ?, \
         note = IF(? IS NULL, note, CONCAT_WS('\\n', note, ?)), version = version + 1 WHERE tenant_id = ? AND id = ?",
    )
    .bind(loss)
    .bind(user_id)
    .bind(&note)
    .bind(&note)
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;

    if !loss.is_zero() {
        let accounts = SystemAccounts::load(tx).await?;
        let mut j = NewJournal::new(ctx.business_date, "stock_transfer", id)
            .branch(to_branch)
            .memo(format!("Transit loss {}", number.as_deref().unwrap_or_default()));
        if let Some(n) = &number {
            j = j.number(n.clone());
        }
        j.debit(accounts.inventory_adjustment()?, loss).credit(accounts.inventory()?, loss);
        journal::post(tx, j).await?;
    }
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("stock.transfer_received")
            .entity("stock_transfer", id)
            .branch(to_branch)
            .change(None, Some(serde_json::json!({ "transfer_no": number, "loss_value": loss }))),
    )
    .await?;
    get(tx, id, can_view_cost).await
}

/// Cancels a draft (nothing has moved yet).
pub async fn cancel(tx: &mut TenantTx, meta: &RequestMeta, id: Uuid, can_view_cost: bool) -> AppResult<Transfer> {
    let (from_branch, ..) = lock(tx, id, "draft").await?;
    let tenant_id = tx.tenant_id();
    let user_id = tx.user_id();
    sqlx::query(
        "UPDATE stock_transfers SET status = 'cancelled', cancelled_at = UTC_TIMESTAMP(6), cancelled_by = ?, \
         version = version + 1 WHERE tenant_id = ? AND id = ?",
    )
    .bind(user_id)
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("stock.transfer_cancelled").entity("stock_transfer", id).branch(from_branch),
    )
    .await?;
    get(tx, id, can_view_cost).await
}
