//! Sale returns (RET-R01…R07, DB design §10.3, §8.5). Quantities are checked
//! against what is still returnable under a row lock on the sale lines. The
//! refund is the line total pro-rata (discounts and VAT included); restocked
//! goods come back at the original unit cost, damaged goods go to the
//! DAMAGED location. The refund first reduces any due on the sale, the rest
//! is paid out or credited to the customer.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use pos_accounting::{JournalLine, NewJournal, SystemAccounts, journal};
use pos_audit::AuditEntry;
use pos_core::{
    AppError, AppResult, RequestMeta,
    id::new_id,
    money::{quantity, round2},
    page::{self, Keyset},
};
use pos_db::{TenantTx, map_db_error};
use pos_inventory::{MovementLine, MovementRef, post_movements};
use pos_money::{
    accounts, methods,
    payments::{self, Allocation, Direction, NewPayment},
};
use pos_parties::{PartyEntry, PartyType, ledger};
use pos_tenancy::{context, sequences};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

use crate::sales::payment_status;

pub const REFUND_MODES: &[&str] = &["cash", "original_method", "customer_credit"];

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReturnLineInput {
    pub sale_item_id: Uuid,
    #[schema(value_type = String)]
    pub quantity: Decimal,
    /// `restock` (default) or `damaged`.
    #[serde(default)]
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReturnInput {
    pub sale_id: Uuid,
    pub reason: String,
    /// `cash`, `original_method` or `customer_credit`.
    pub refund_mode: String,
    /// Overrides the method the refund is paid with.
    #[serde(default)]
    pub refund_method_id: Option<Uuid>,
    pub lines: Vec<ReturnLineInput>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct SaleReturn {
    pub id: Uuid,
    pub return_no: String,
    pub sale_id: Uuid,
    pub invoice_no: String,
    pub customer_id: Uuid,
    pub customer_name: String,
    pub business_date: NaiveDate,
    pub reason: String,
    pub refund_mode: String,
    /// Excluding VAT.
    #[schema(value_type = String)]
    pub subtotal: Decimal,
    #[schema(value_type = String)]
    pub tax_total: Decimal,
    #[schema(value_type = String)]
    pub total: Decimal,
    #[schema(value_type = String)]
    pub applied_to_due: Decimal,
    #[schema(value_type = String)]
    pub refunded_amount: Decimal,
    #[schema(value_type = String)]
    pub credited_amount: Decimal,
    /// `null` without `products.view_cost`.
    #[schema(value_type = Option<String>)]
    pub cost_total: Option<Decimal>,
    pub created_by_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct SaleReturnItem {
    pub id: Uuid,
    pub sale_item_id: Uuid,
    pub variant_id: Uuid,
    pub product_name: String,
    pub sku: String,
    pub unit_short_name: String,
    #[schema(value_type = String)]
    pub quantity: Decimal,
    pub item_condition: String,
    #[schema(value_type = String)]
    pub tax_amount: Decimal,
    #[schema(value_type = String)]
    pub line_total: Decimal,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SaleReturnDetail {
    #[serde(flatten)]
    pub sale_return: SaleReturn,
    pub items: Vec<SaleReturnItem>,
    pub receipt: crate::sales::ReceiptInfo,
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct ReturnFilter {
    pub sale_id: Option<Uuid>,
    pub customer_id: Option<Uuid>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    /// Return or invoice number containing this text.
    pub q: Option<String>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ReturnPage {
    pub data: Vec<SaleReturn>,
    pub next_cursor: Option<String>,
    #[schema(value_type = String)]
    pub total: Decimal,
}

const SELECT: &str = "SELECT r.id, r.return_no, r.sale_id, s.invoice_no, r.customer_id, c.name AS customer_name, \
     r.business_date, r.reason, r.refund_mode, r.subtotal, r.tax_total, r.total, r.applied_to_due, r.refunded_amount, \
     r.credited_amount, r.cost_total, (SELECT u.full_name FROM users u WHERE u.id = r.created_by) AS created_by_name, \
     r.created_at FROM sale_returns r \
     JOIN sales s ON s.tenant_id = r.tenant_id AND s.id = r.sale_id \
     JOIN customers c ON c.tenant_id = r.tenant_id AND c.id = r.customer_id WHERE r.tenant_id = ";

fn filters(q: &mut QueryBuilder<MySql>, filter: &ReturnFilter) {
    if let Some(sale_id) = filter.sale_id {
        q.push(" AND r.sale_id = ").push_bind(sale_id);
    }
    if let Some(customer_id) = filter.customer_id {
        q.push(" AND r.customer_id = ").push_bind(customer_id);
    }
    if let Some(from) = filter.from {
        q.push(" AND r.business_date >= ").push_bind(from);
    }
    if let Some(to) = filter.to {
        q.push(" AND r.business_date <= ").push_bind(to);
    }
    if let Some(text) = filter.q.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        let escaped = text.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
        let pattern = format!("%{escaped}%");
        q.push(" AND (r.return_no LIKE ")
            .push_bind(pattern.clone())
            .push(" OR s.invoice_no LIKE ")
            .push_bind(pattern)
            .push(")");
    }
}

pub async fn list(tx: &mut TenantTx, filter: &ReturnFilter, can_view_cost: bool) -> AppResult<ReturnPage> {
    let limit = page::limit(filter.limit)?;
    let after = filter.cursor.as_deref().map(Keyset::decode).transpose()?;
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(SELECT);
    q.push_bind(tenant_id);
    if let Some(branch_id) = tx.branch_id() {
        q.push(" AND r.branch_id = ").push_bind(branch_id);
    }
    filters(&mut q, filter);
    if let Some(k) = after {
        q.push(" AND (r.created_at < ").push_bind(k.at);
        q.push(" OR (r.created_at = ").push_bind(k.at).push(" AND r.id < ").push_bind(k.id).push("))");
    }
    q.push(" ORDER BY r.created_at DESC, r.id DESC LIMIT ").push_bind(limit + 1);
    let mut rows: Vec<SaleReturn> = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    let next_cursor = if rows.len() > limit as usize {
        rows.truncate(limit as usize);
        rows.last().map(|r| Keyset { at: r.created_at, id: r.id }.encode())
    } else {
        None
    };
    if !can_view_cost {
        for row in &mut rows {
            row.cost_total = None;
        }
    }
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "SELECT COALESCE(SUM(r.total), 0) FROM sale_returns r JOIN sales s ON s.tenant_id = r.tenant_id AND s.id = r.sale_id \
         WHERE r.tenant_id = ",
    );
    q.push_bind(tenant_id);
    if let Some(branch_id) = tx.branch_id() {
        q.push(" AND r.branch_id = ").push_bind(branch_id);
    }
    filters(&mut q, filter);
    let total: Decimal = q.build_query_scalar().fetch_one(&mut **tx).await.map_err(map_db_error)?;
    Ok(ReturnPage { data: rows, next_cursor, total })
}

pub async fn get(tx: &mut TenantTx, id: Uuid, can_view_cost: bool) -> AppResult<SaleReturnDetail> {
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(SELECT);
    q.push_bind(tenant_id).push(" AND r.id = ").push_bind(id);
    let mut sale_return: SaleReturn =
        q.build_query_as().fetch_optional(&mut **tx).await.map_err(map_db_error)?.ok_or(AppError::NotFound)?;
    if !can_view_cost {
        sale_return.cost_total = None;
    }
    let items = sqlx::query_as(
        "SELECT ri.id, ri.sale_item_id, ri.variant_id, si.product_name, si.sku, si.unit_short_name, ri.quantity, \
         ri.item_condition, ri.tax_amount, ri.line_total FROM sale_return_items ri \
         JOIN sale_items si ON si.tenant_id = ri.tenant_id AND si.id = ri.sale_item_id \
         WHERE ri.tenant_id = ? AND ri.return_id = ? ORDER BY si.line_no",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let branch_id: Uuid = sqlx::query_scalar("SELECT branch_id FROM sale_returns WHERE tenant_id = ? AND id = ?")
        .bind(tenant_id)
        .bind(id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
    let receipt = crate::sales::receipt_info(tx, branch_id).await?;
    Ok(SaleReturnDetail { sale_return, items, receipt })
}

#[derive(sqlx::FromRow)]
struct SaleRow {
    status: String,
    business_date: NaiveDate,
    branch_id: Uuid,
    location_id: Uuid,
    invoice_no: String,
    customer_id: Uuid,
    grand_total: Decimal,
    paid_total: Decimal,
    returned_total: Decimal,
    due_total: Decimal,
}

#[derive(sqlx::FromRow)]
struct ItemRow {
    id: Uuid,
    variant_id: Uuid,
    product_name: String,
    quantity: Decimal,
    base_quantity: Decimal,
    returned_quantity: Decimal,
    tax_amount: Decimal,
    line_total: Decimal,
    unit_cost: Decimal,
    track_inventory: bool,
    allow_fraction: bool,
    returned_value: Decimal,
    returned_tax: Decimal,
}

/// The method that pays out of the default (cash) account.
async fn cash_method(tx: &mut TenantTx) -> AppResult<Uuid> {
    let account = accounts::default_account(tx).await?;
    Ok(methods::for_account(tx, account.id).await?.id)
}

/// Posts a return. `can_override_window` = `sales.return_override` (RET-R06).
pub async fn create(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    idempotency_key: Uuid,
    input: &ReturnInput,
    can_override_window: bool,
    can_view_cost: bool,
) -> AppResult<SaleReturnDetail> {
    let reason = input.reason.trim();
    if reason.is_empty() || reason.chars().count() > 500 {
        return Err(AppError::field("reason", "REQUIRED", "enter the reason for the return (RET-R07)"));
    }
    if !REFUND_MODES.contains(&input.refund_mode.as_str()) {
        return Err(AppError::field("refund_mode", "INVALID_MODE", "use cash, original_method or customer_credit"));
    }
    if input.lines.is_empty() || input.lines.len() > crate::sales::MAX_LINES {
        return Err(AppError::field("lines", "INVALID_LINES", "choose the lines to return"));
    }
    let ctx = context::load(tx).await?;
    let tenant_id = tx.tenant_id();

    let sale: SaleRow = sqlx::query_as(
        "SELECT status, business_date, branch_id, location_id, invoice_no, customer_id, grand_total, paid_total, \
         returned_total, due_total FROM sales WHERE tenant_id = ? AND id = ? FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(input.sale_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or_else(|| AppError::field("sale_id", "NOT_FOUND", "the invoice was not found"))?;
    if !matches!(sale.status.as_str(), "completed" | "partially_returned") {
        return Err(AppError::Unprocessable {
            code: "SALE_NOT_RETURNABLE",
            message: "only completed sales can be returned (RET-R01)".into(),
        });
    }
    let window_ends = sale.business_date + Duration::days(i64::from(ctx.return_window_days));
    if ctx.business_date > window_ends && !can_override_window {
        return Err(AppError::Denied {
            code: "RETURN_WINDOW_PASSED",
            message: format!("the {}-day return window has passed", ctx.return_window_days),
        });
    }
    let walk_in: bool = sqlx::query_scalar("SELECT is_walk_in FROM customers WHERE tenant_id = ? AND id = ?")
        .bind(tenant_id)
        .bind(sale.customer_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;

    // Lock the sale lines (sorted by id) and read what was already returned.
    let items: Vec<ItemRow> = sqlx::query_as(
        "SELECT si.id, si.variant_id, si.product_name, si.quantity, si.base_quantity, si.returned_quantity, si.tax_amount, \
         si.line_total, si.unit_cost, si.track_inventory, COALESCE(u.allow_fraction, TRUE) AS allow_fraction, \
         COALESCE((SELECT SUM(ri.line_total) FROM sale_return_items ri WHERE ri.tenant_id = si.tenant_id AND ri.sale_item_id = si.id), 0) AS returned_value, \
         COALESCE((SELECT SUM(ri.tax_amount) FROM sale_return_items ri WHERE ri.tenant_id = si.tenant_id AND ri.sale_item_id = si.id), 0) AS returned_tax \
         FROM sale_items si LEFT JOIN units u ON u.tenant_id = si.tenant_id AND u.id = si.unit_id \
         WHERE si.tenant_id = ? AND si.sale_id = ? ORDER BY si.id FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(input.sale_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;

    struct Out<'a> {
        item: &'a ItemRow,
        quantity: Decimal,
        base_quantity: Decimal,
        damaged: bool,
        line_total: Decimal,
        tax: Decimal,
    }
    let mut outs: Vec<Out> = Vec::new();
    for (i, line) in input.lines.iter().enumerate() {
        let item = items.iter().find(|it| it.id == line.sale_item_id).ok_or_else(|| {
            AppError::field(&format!("lines[{i}].sale_item_id"), "NOT_FOUND", "this line is not on the invoice")
        })?;
        if outs.iter().any(|o| o.item.id == item.id) {
            return Err(AppError::field(&format!("lines[{i}].sale_item_id"), "DUPLICATE", "each line may appear once"));
        }
        let qty = quantity(line.quantity, &format!("lines[{i}].quantity"), item.allow_fraction)?;
        let left = item.quantity - item.returned_quantity;
        if qty > left {
            return Err(AppError::Unprocessable {
                code: "RETURN_QTY_EXCEEDED",
                message: format!("only {} of {} can still be returned", left.normalize(), item.product_name),
            });
        }
        let damaged = match line.condition.as_deref() {
            None | Some("restock") => false,
            Some("damaged") => true,
            Some(_) => {
                return Err(AppError::field(
                    &format!("lines[{i}].condition"),
                    "INVALID_CONDITION",
                    "use restock or damaged",
                ));
            }
        };
        // The last units take whatever value is left, so rounding never drifts.
        let (line_total, tax) = if qty == left {
            (item.line_total - item.returned_value, item.tax_amount - item.returned_tax)
        } else {
            (round2(item.line_total * qty / item.quantity), round2(item.tax_amount * qty / item.quantity))
        };
        let base_quantity = item.base_quantity * qty / item.quantity;
        outs.push(Out { item, quantity: qty, base_quantity, damaged, line_total, tax });
    }

    let total: Decimal = outs.iter().map(|o| o.line_total).sum();
    let tax_total: Decimal = outs.iter().map(|o| o.tax).sum();
    let applied_to_due = total.min(sale.due_total);
    let rest = total - applied_to_due;
    let (refunded, credited) =
        if input.refund_mode == "customer_credit" { (Decimal::ZERO, rest) } else { (rest, Decimal::ZERO) };
    if credited > Decimal::ZERO && walk_in {
        return Err(AppError::Unprocessable {
            code: "DUE_REQUIRES_CUSTOMER",
            message: "the walk-in customer cannot hold credit; refund the money instead".into(),
        });
    }

    let return_id = new_id();
    let return_no = sequences::next(tx, "sale_return", Some(sale.branch_id), ctx.business_date).await?;
    let item_ids: Vec<Uuid> = outs.iter().map(|_| new_id()).collect();
    let stock: Vec<MovementLine> = outs
        .iter()
        .zip(&item_ids)
        .filter(|(o, _)| o.item.track_inventory)
        .map(|(o, item_id)| MovementLine {
            location_id: if o.damaged { ctx.damaged_location_id } else { sale.location_id },
            variant_id: o.item.variant_id,
            movement_type: "sale_return",
            quantity: o.base_quantity,
            unit_cost: Some(o.item.unit_cost),
            reference_line_id: Some(*item_id),
            reason: Some(reason.chars().take(255).collect()),
        })
        .collect();
    let posted = post_movements(
        tx,
        &MovementRef {
            branch_id: sale.branch_id,
            business_date: ctx.business_date,
            reference_type: "sale_return",
            reference_id: return_id,
            reference_no: Some(return_no.clone()),
        },
        &stock,
        true,
    )
    .await?;
    let cost_total: Decimal = posted.iter().map(|p| p.total_cost).sum();

    let user_id = tx.user_id();
    sqlx::query(
        "INSERT INTO sale_returns (tenant_id, id, branch_id, return_no, sale_id, customer_id, business_date, reason, \
         refund_mode, subtotal, tax_total, total, applied_to_due, refunded_amount, credited_amount, cost_total, \
         idempotency_key, created_by) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(return_id)
    .bind(sale.branch_id)
    .bind(&return_no)
    .bind(input.sale_id)
    .bind(sale.customer_id)
    .bind(ctx.business_date)
    .bind(reason)
    .bind(&input.refund_mode)
    .bind(total - tax_total)
    .bind(tax_total)
    .bind(total)
    .bind(applied_to_due)
    .bind(refunded)
    .bind(credited)
    .bind(cost_total)
    .bind(idempotency_key)
    .bind(user_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "INSERT INTO sale_return_items (tenant_id, id, return_id, sale_item_id, variant_id, quantity, base_quantity, \
         item_condition, location_id, taxable_amount, tax_amount, line_total, unit_cost) ",
    );
    q.push_values(outs.iter().zip(&item_ids), |mut row, (o, item_id)| {
        row.push_bind(tenant_id)
            .push_bind(*item_id)
            .push_bind(return_id)
            .push_bind(o.item.id)
            .push_bind(o.item.variant_id)
            .push_bind(o.quantity)
            .push_bind(o.base_quantity)
            .push_bind(if o.damaged { "damaged" } else { "restock" })
            .push_bind(if o.damaged { ctx.damaged_location_id } else { sale.location_id })
            .push_bind(o.line_total - o.tax)
            .push_bind(o.tax)
            .push_bind(o.line_total)
            .push_bind(o.item.unit_cost);
    });
    q.build().execute(&mut **tx).await.map_err(map_db_error)?;
    for o in &outs {
        sqlx::query("UPDATE sale_items SET returned_quantity = returned_quantity + ? WHERE tenant_id = ? AND id = ?")
            .bind(o.quantity)
            .bind(tenant_id)
            .bind(o.item.id)
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
    }

    // Refund payment (cash or original method).
    let mut refund_ledger = None;
    if refunded > Decimal::ZERO {
        let method_id = match input.refund_method_id {
            Some(id) => id,
            None if input.refund_mode == "original_method" => {
                let original: Option<Uuid> = sqlx::query_scalar(
                    "SELECT p.payment_method_id FROM payment_allocations a JOIN payments p ON p.tenant_id = a.tenant_id \
                     AND p.id = a.payment_id WHERE a.tenant_id = ? AND a.document_type = 'sale' AND a.document_id = ? \
                     AND p.status = 'posted' ORDER BY p.amount DESC LIMIT 1",
                )
                .bind(tenant_id)
                .bind(input.sale_id)
                .fetch_optional(&mut **tx)
                .await
                .map_err(map_db_error)?;
                match original {
                    Some(m) => m,
                    None => cash_method(tx).await?,
                }
            }
            None => cash_method(tx).await?,
        };
        let payment = payments::post(
            tx,
            NewPayment {
                direction: Direction::Out,
                branch_id: sale.branch_id,
                business_date: ctx.business_date,
                party: (!walk_in).then_some(("customer", sale.customer_id)),
                account_id: None,
                method_id,
                amount: refunded,
                reference: Some(return_no.clone()),
                note: Some(format!("Refund {return_no} for {}", sale.invoice_no)),
                source: "refund",
                txn_type: None,
                allocations: vec![Allocation {
                    document_type: "sale_return",
                    document_id: return_id,
                    amount: refunded,
                }],
                field: "refund".into(),
            },
        )
        .await?;
        refund_ledger = Some(payment.ledger_account_id);
    }

    // The sale itself.
    let returned_total = sale.returned_total + total;
    let due_total = sale.due_total - applied_to_due;
    let fully_returned = items.iter().all(|it| {
        let extra = outs.iter().find(|o| o.item.id == it.id).map_or(Decimal::ZERO, |o| o.quantity);
        it.returned_quantity + extra >= it.quantity
    });
    let status = if fully_returned { "returned" } else { "partially_returned" };
    let pay_status = if fully_returned && (refunded > Decimal::ZERO || credited > Decimal::ZERO) {
        "refunded"
    } else {
        payment_status(sale.grand_total, sale.paid_total, due_total)
    };
    sqlx::query(
        "UPDATE sales SET returned_total = ?, due_total = ?, status = ?, payment_status = ? WHERE tenant_id = ? AND id = ?",
    )
    .bind(returned_total)
    .bind(due_total)
    .bind(status)
    .bind(pay_status)
    .bind(tenant_id)
    .bind(input.sale_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;

    if !walk_in {
        let entry = |entry_type, amount| PartyEntry {
            party_type: PartyType::Customer,
            party_id: sale.customer_id,
            branch_id: Some(sale.branch_id),
            entry_type,
            amount,
            business_date: ctx.business_date,
            reference_type: "sale_return",
            reference_id: return_id,
            reference_no: Some(return_no.clone()),
            note: Some(format!("Return on {}", sale.invoice_no)),
        };
        ledger::post(tx, entry("sale_return", -total)).await?;
        ledger::post(tx, entry("refund_out", refunded)).await?;
    }

    // Journal (§8.7): reverse revenue and VAT; restocked cost back to inventory.
    let accounts = SystemAccounts::load(tx).await?;
    let mut j = NewJournal::new(ctx.business_date, "sale_return", return_id)
        .branch(sale.branch_id)
        .number(return_no.clone())
        .memo(format!("Return {return_no} on {}", sale.invoice_no));
    j.debit(accounts.sales_returns()?, total - tax_total)
        .debit(accounts.vat_payable()?, tax_total)
        .line(
            JournalLine::credit(accounts.receivable()?, applied_to_due + credited).party("customer", sale.customer_id),
        )
        .debit(accounts.inventory()?, cost_total)
        .credit(accounts.cogs()?, cost_total);
    if let Some(ledger_account) = refund_ledger {
        j.credit(ledger_account, refunded);
    }
    journal::post(tx, j).await?;

    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("sale.returned").entity("sale_return", return_id).branch(sale.branch_id).reason(reason).change(
            None,
            Some(serde_json::json!({
                "return_no": return_no, "invoice_no": sale.invoice_no, "total": total, "refunded": refunded,
                "credited": credited, "applied_to_due": applied_to_due,
            })),
        ),
    )
    .await?;
    get(tx, return_id, can_view_cost).await
}
