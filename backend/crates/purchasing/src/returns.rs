//! Purchase returns (PUR-R07, §8.5). Quantity is limited to received −
//! returned under a row lock. Stock leaves at the current moving average; the
//! supplier is debited the original line price pro-rata (VAT included) and
//! the difference to cost goes to Inventory Adjustments. The credit first
//! reduces what is due on the bill; the rest reduces the payable (an advance
//! with the supplier) or is refunded in money.

use chrono::{DateTime, NaiveDate, Utc};
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
use pos_money::payments::{self, Allocation, Direction, NewPayment};
use pos_parties::{PartyEntry, PartyType, ledger};
use pos_tenancy::{context, sequences};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PurchaseReturnLineInput {
    pub purchase_item_id: Uuid,
    #[schema(value_type = String)]
    pub quantity: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PurchaseReturnInput {
    pub purchase_id: Uuid,
    pub reason: String,
    /// `reduce_payable` (default) or `refund` (the supplier pays the money back).
    #[serde(default)]
    pub settlement_mode: Option<String>,
    /// For `refund`: the method the money comes back by.
    #[serde(default)]
    pub refund_method_id: Option<Uuid>,
    #[serde(default)]
    pub refund_account_id: Option<Uuid>,
    pub lines: Vec<PurchaseReturnLineInput>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct PurchaseReturn {
    pub id: Uuid,
    pub return_no: String,
    pub purchase_id: Uuid,
    pub purchase_no: Option<String>,
    pub supplier_id: Uuid,
    pub supplier_name: String,
    pub business_date: NaiveDate,
    pub reason: String,
    pub settlement_mode: String,
    #[schema(value_type = String)]
    pub total: Decimal,
    #[schema(value_type = String)]
    pub tax_total: Decimal,
    #[schema(value_type = String)]
    pub cost_total: Decimal,
    #[schema(value_type = String)]
    pub applied_to_due: Decimal,
    #[schema(value_type = String)]
    pub refunded_amount: Decimal,
    #[schema(value_type = String)]
    pub credited_amount: Decimal,
    pub created_by_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct PurchaseReturnItem {
    pub id: Uuid,
    pub purchase_item_id: Uuid,
    pub variant_id: Uuid,
    pub product_name: String,
    pub sku: String,
    #[schema(value_type = String)]
    pub base_quantity: Decimal,
    #[schema(value_type = String)]
    pub line_total: Decimal,
    #[schema(value_type = String)]
    pub unit_cost_out: Decimal,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct PurchaseReturnDetail {
    #[serde(flatten)]
    pub purchase_return: PurchaseReturn,
    pub items: Vec<PurchaseReturnItem>,
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct PurchaseReturnFilter {
    pub purchase_id: Option<Uuid>,
    pub supplier_id: Option<Uuid>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct PurchaseReturnPage {
    pub data: Vec<PurchaseReturn>,
    pub next_cursor: Option<String>,
}

const SELECT: &str = "SELECT r.id, r.return_no, r.purchase_id, p.purchase_no, r.supplier_id, s.name AS supplier_name, \
     r.business_date, r.reason, r.settlement_mode, r.total, r.tax_total, r.cost_total, r.applied_to_due, \
     r.refunded_amount, r.credited_amount, (SELECT u.full_name FROM users u WHERE u.id = r.created_by) AS created_by_name, \
     r.created_at FROM purchase_returns r \
     JOIN purchases p ON p.tenant_id = r.tenant_id AND p.id = r.purchase_id \
     JOIN suppliers s ON s.tenant_id = r.tenant_id AND s.id = r.supplier_id WHERE r.tenant_id = ";

pub async fn list(tx: &mut TenantTx, filter: &PurchaseReturnFilter) -> AppResult<PurchaseReturnPage> {
    let limit = page::limit(filter.limit)?;
    let after = filter.cursor.as_deref().map(Keyset::decode).transpose()?;
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(SELECT);
    q.push_bind(tenant_id);
    if let Some(branch_id) = tx.branch_id() {
        q.push(" AND r.branch_id = ").push_bind(branch_id);
    }
    if let Some(id) = filter.purchase_id {
        q.push(" AND r.purchase_id = ").push_bind(id);
    }
    if let Some(id) = filter.supplier_id {
        q.push(" AND r.supplier_id = ").push_bind(id);
    }
    if let Some(from) = filter.from {
        q.push(" AND r.business_date >= ").push_bind(from);
    }
    if let Some(to) = filter.to {
        q.push(" AND r.business_date <= ").push_bind(to);
    }
    if let Some(k) = after {
        q.push(" AND (r.created_at < ").push_bind(k.at);
        q.push(" OR (r.created_at = ").push_bind(k.at).push(" AND r.id < ").push_bind(k.id).push("))");
    }
    q.push(" ORDER BY r.created_at DESC, r.id DESC LIMIT ").push_bind(limit + 1);
    let mut rows: Vec<PurchaseReturn> = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    let next_cursor = if rows.len() > limit as usize {
        rows.truncate(limit as usize);
        rows.last().map(|r| Keyset { at: r.created_at, id: r.id }.encode())
    } else {
        None
    };
    Ok(PurchaseReturnPage { data: rows, next_cursor })
}

pub async fn get(tx: &mut TenantTx, id: Uuid) -> AppResult<PurchaseReturnDetail> {
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(SELECT);
    q.push_bind(tenant_id).push(" AND r.id = ").push_bind(id);
    let purchase_return: PurchaseReturn =
        q.build_query_as().fetch_optional(&mut **tx).await.map_err(map_db_error)?.ok_or(AppError::NotFound)?;
    let items = sqlx::query_as(
        "SELECT ri.id, ri.purchase_item_id, ri.variant_id, pi.product_name, pi.sku, ri.base_quantity, ri.line_total, \
         ri.unit_cost_out FROM purchase_return_items ri \
         JOIN purchase_items pi ON pi.tenant_id = ri.tenant_id AND pi.id = ri.purchase_item_id \
         WHERE ri.tenant_id = ? AND ri.return_id = ? ORDER BY pi.line_no",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(PurchaseReturnDetail { purchase_return, items })
}

#[derive(sqlx::FromRow)]
struct LockedPurchase {
    status: String,
    branch_id: Uuid,
    location_id: Uuid,
    supplier_id: Uuid,
    purchase_no: Option<String>,
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
    base_quantity: Decimal,
    received_quantity: Decimal,
    returned_quantity: Decimal,
    tax_amount: Decimal,
    line_total: Decimal,
    allow_fraction: bool,
    returned_value: Decimal,
    returned_tax: Decimal,
}

pub async fn create(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    idempotency_key: Uuid,
    input: &PurchaseReturnInput,
) -> AppResult<PurchaseReturnDetail> {
    let reason = input.reason.trim();
    if reason.is_empty() || reason.chars().count() > 500 {
        return Err(AppError::field("reason", "REQUIRED", "enter the reason for the return"));
    }
    let mode = input.settlement_mode.as_deref().unwrap_or("reduce_payable");
    if !["reduce_payable", "refund"].contains(&mode) {
        return Err(AppError::field("settlement_mode", "INVALID_MODE", "use reduce_payable or refund"));
    }
    if input.lines.is_empty() || input.lines.len() > crate::purchases::MAX_LINES {
        return Err(AppError::field("lines", "INVALID_LINES", "choose the lines to return"));
    }
    let ctx = context::load(tx).await?;
    let tenant_id = tx.tenant_id();
    let purchase: Option<LockedPurchase> = sqlx::query_as(
        "SELECT status, branch_id, location_id, supplier_id, purchase_no, grand_total, paid_total, returned_total, due_total \
         FROM purchases WHERE tenant_id = ? AND id = ? FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(input.purchase_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let LockedPurchase {
        status,
        branch_id,
        location_id,
        supplier_id,
        purchase_no,
        grand_total: grand,
        paid_total: paid,
        returned_total: returned,
        due_total: due,
    } = purchase.ok_or_else(|| AppError::field("purchase_id", "NOT_FOUND", "the bill was not found"))?;
    if status != "received" {
        return Err(AppError::Unprocessable {
            code: "PURCHASE_NOT_RECEIVED",
            message: "only posted bills can be returned".into(),
        });
    }
    let purchase_no = purchase_no.unwrap_or_default();
    let items: Vec<ItemRow> = sqlx::query_as(
        "SELECT pi.id, pi.variant_id, pi.product_name, pi.base_quantity, pi.received_quantity, pi.returned_quantity, \
         pi.tax_amount, pi.line_total, COALESCE(u.allow_fraction, TRUE) AS allow_fraction, \
         COALESCE((SELECT SUM(ri.line_total) FROM purchase_return_items ri WHERE ri.tenant_id = pi.tenant_id AND ri.purchase_item_id = pi.id), 0) AS returned_value, \
         COALESCE((SELECT SUM(ri.tax_amount) FROM purchase_return_items ri WHERE ri.tenant_id = pi.tenant_id AND ri.purchase_item_id = pi.id), 0) AS returned_tax \
         FROM purchase_items pi LEFT JOIN units u ON u.tenant_id = pi.tenant_id AND u.id = pi.unit_id \
         WHERE pi.tenant_id = ? AND pi.purchase_id = ? ORDER BY pi.id FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(input.purchase_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;

    struct Out<'a> {
        item: &'a ItemRow,
        quantity: Decimal,
        line_total: Decimal,
        tax: Decimal,
    }
    let mut outs: Vec<Out> = Vec::new();
    for (i, line) in input.lines.iter().enumerate() {
        let item = items.iter().find(|it| it.id == line.purchase_item_id).ok_or_else(|| {
            AppError::field(&format!("lines[{i}].purchase_item_id"), "NOT_FOUND", "this line is not on the bill")
        })?;
        if outs.iter().any(|o| o.item.id == item.id) {
            return Err(AppError::field(
                &format!("lines[{i}].purchase_item_id"),
                "DUPLICATE",
                "each line may appear once",
            ));
        }
        let qty = quantity(line.quantity, &format!("lines[{i}].quantity"), item.allow_fraction)?;
        let left = item.received_quantity - item.returned_quantity;
        if qty > left {
            return Err(AppError::Unprocessable {
                code: "RETURN_QTY_EXCEEDED",
                message: format!("only {} of {} can still be returned", left.normalize(), item.product_name),
            });
        }
        let (line_total, tax) = if qty == left {
            (item.line_total - item.returned_value, item.tax_amount - item.returned_tax)
        } else {
            (round2(item.line_total * qty / item.base_quantity), round2(item.tax_amount * qty / item.base_quantity))
        };
        outs.push(Out { item, quantity: qty, line_total, tax });
    }
    let total: Decimal = outs.iter().map(|o| o.line_total).sum();
    let tax_total: Decimal = outs.iter().map(|o| o.tax).sum();
    let applied_to_due = total.min(due);
    let rest = total - applied_to_due;
    let (refunded, credited) = if mode == "refund" { (rest, Decimal::ZERO) } else { (Decimal::ZERO, rest) };

    let return_id = new_id();
    let return_no = sequences::next(tx, "purchase_return", Some(branch_id), ctx.business_date).await?;
    let item_ids: Vec<Uuid> = outs.iter().map(|_| new_id()).collect();
    let posted = post_movements(
        tx,
        &MovementRef {
            branch_id,
            business_date: ctx.business_date,
            reference_type: "purchase_return",
            reference_id: return_id,
            reference_no: Some(return_no.clone()),
        },
        &outs
            .iter()
            .zip(&item_ids)
            .map(|(o, item_id)| MovementLine {
                location_id,
                variant_id: o.item.variant_id,
                movement_type: "purchase_return",
                quantity: -o.quantity,
                unit_cost: None,
                reference_line_id: Some(*item_id),
                reason: Some(reason.chars().take(255).collect()),
            })
            .collect::<Vec<_>>(),
        ctx.allow_negative_stock,
    )
    .await?;
    let cost_total: Decimal = posted.iter().map(|p| -p.total_cost).sum();

    let user_id = tx.user_id();
    sqlx::query(
        "INSERT INTO purchase_returns (tenant_id, id, branch_id, return_no, purchase_id, supplier_id, location_id, \
         business_date, reason, total, tax_total, cost_total, settlement_mode, applied_to_due, refunded_amount, \
         credited_amount, idempotency_key, created_by) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(return_id)
    .bind(branch_id)
    .bind(&return_no)
    .bind(input.purchase_id)
    .bind(supplier_id)
    .bind(location_id)
    .bind(ctx.business_date)
    .bind(reason)
    .bind(total)
    .bind(tax_total)
    .bind(cost_total)
    .bind(mode)
    .bind(applied_to_due)
    .bind(refunded)
    .bind(credited)
    .bind(idempotency_key)
    .bind(user_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "INSERT INTO purchase_return_items (tenant_id, id, return_id, purchase_item_id, variant_id, base_quantity, \
         tax_amount, line_total, unit_cost_out) ",
    );
    q.push_values(outs.iter().zip(&item_ids).zip(&posted), |mut row, ((o, item_id), p)| {
        row.push_bind(tenant_id)
            .push_bind(*item_id)
            .push_bind(return_id)
            .push_bind(o.item.id)
            .push_bind(o.item.variant_id)
            .push_bind(o.quantity)
            .push_bind(o.tax)
            .push_bind(o.line_total)
            .push_bind(p.unit_cost);
    });
    q.build().execute(&mut **tx).await.map_err(map_db_error)?;
    for o in &outs {
        sqlx::query(
            "UPDATE purchase_items SET returned_quantity = returned_quantity + ? WHERE tenant_id = ? AND id = ?",
        )
        .bind(o.quantity)
        .bind(tenant_id)
        .bind(o.item.id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }

    let mut refund_ledger = None;
    if refunded > Decimal::ZERO {
        let method_id = match input.refund_method_id {
            Some(m) => m,
            None => {
                let account = pos_money::accounts::default_account(tx).await?;
                pos_money::methods::for_account(tx, account.id).await?.id
            }
        };
        let payment = payments::post(
            tx,
            NewPayment {
                direction: Direction::In,
                branch_id,
                business_date: ctx.business_date,
                party: Some(("supplier", supplier_id)),
                account_id: input.refund_account_id,
                method_id,
                amount: refunded,
                reference: Some(return_no.clone()),
                note: Some(format!("Refund {return_no} for {purchase_no}")),
                source: "refund",
                txn_type: None,
                allocations: vec![Allocation {
                    document_type: "purchase_return",
                    document_id: return_id,
                    amount: refunded,
                }],
                field: "refund".into(),
            },
        )
        .await?;
        refund_ledger = Some(payment.ledger_account_id);
    }

    let new_due = due - applied_to_due;
    let pay_status = if new_due <= Decimal::ZERO {
        if paid.is_zero() && returned + total >= grand { "refunded" } else { "paid" }
    } else if paid.is_zero() {
        "unpaid"
    } else {
        "partial"
    };
    sqlx::query("UPDATE purchases SET returned_total = returned_total + ?, due_total = ?, payment_status = ? WHERE tenant_id = ? AND id = ?")
        .bind(total)
        .bind(new_due)
        .bind(pay_status)
        .bind(tenant_id)
        .bind(input.purchase_id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;

    let entry = |entry_type, amount| PartyEntry {
        party_type: PartyType::Supplier,
        party_id: supplier_id,
        branch_id: Some(branch_id),
        entry_type,
        amount,
        business_date: ctx.business_date,
        reference_type: "purchase_return",
        reference_id: return_id,
        reference_no: Some(return_no.clone()),
        note: Some(format!("Return on {purchase_no}")),
    };
    ledger::post(tx, entry("purchase_return", -total)).await?;
    ledger::post(tx, entry("refund_in", refunded)).await?;

    // Dr A/P (+ money refunded) / Cr Inventory at WAC, Cr VAT receivable,
    // and the difference to Inventory Adjustments (§8.5).
    let accounts = SystemAccounts::load(tx).await?;
    let recoverable = if ctx.is_vat_registered { tax_total } else { Decimal::ZERO };
    let difference = total - recoverable - cost_total;
    let mut j = NewJournal::new(ctx.business_date, "purchase_return", return_id)
        .branch(branch_id)
        .number(return_no.clone())
        .memo(format!("Purchase return {return_no} on {purchase_no}"));
    j.line(JournalLine::debit(accounts.payable()?, applied_to_due + credited).party("supplier", supplier_id))
        .credit(accounts.inventory()?, cost_total)
        .credit(accounts.vat_receivable()?, recoverable)
        .credit(accounts.inventory_adjustment()?, difference);
    if let Some(ledger_account) = refund_ledger {
        j.debit(ledger_account, refunded);
    }
    journal::post(tx, j).await?;

    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("purchase.returned").entity("purchase_return", return_id).branch(branch_id).reason(reason).change(
            None,
            Some(serde_json::json!({ "return_no": return_no, "purchase_no": purchase_no, "total": total, "refunded": refunded })),
        ),
    )
    .await?;
    get(tx, return_id).await
}
