//! Customer collections (PTY-R07, DB design §10.5): a payment from a customer
//! allocated oldest invoice first (FIFO) or to chosen invoices; whatever is
//! left becomes an advance. Voiding a collection restores the invoice dues.

use std::collections::BTreeMap;

use pos_accounting::{JournalLine, NewJournal, SystemAccounts, journal};
use pos_audit::AuditEntry;
use pos_core::{AppError, AppResult, RequestMeta, money::amount};
use pos_db::{TenantTx, map_db_error};
use pos_money::payments::{self, Allocation, Direction, NewPayment, PaymentDetail};
use pos_parties::{PartyEntry, PartyType, customers, ledger};
use pos_tenancy::context;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::sales::payment_status;

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AllocationInput {
    pub sale_id: Uuid,
    #[schema(value_type = String)]
    pub amount: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CollectionInput {
    pub customer_id: Uuid,
    #[schema(value_type = String)]
    pub amount: Decimal,
    pub method_id: Uuid,
    /// Blank = the method's default account.
    #[serde(default)]
    pub account_id: Option<Uuid>,
    #[serde(default)]
    pub reference: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    /// Blank = oldest invoices first.
    #[serde(default)]
    pub allocations: Option<Vec<AllocationInput>>,
}

/// Splits `amount` over open invoices `(id, due)` oldest first; returns the
/// allocations and what is left over (an advance).
pub fn allocate_fifo(open: &[(Uuid, Decimal)], amount: Decimal) -> (Vec<(Uuid, Decimal)>, Decimal) {
    let mut left = amount;
    let mut out = Vec::new();
    for (id, due) in open {
        if left.is_zero() {
            break;
        }
        let take = left.min(*due);
        if take > Decimal::ZERO {
            out.push((*id, take));
            left -= take;
        }
    }
    (out, left)
}

pub async fn collect(tx: &mut TenantTx, meta: &RequestMeta, input: &CollectionInput) -> AppResult<PaymentDetail> {
    let value = amount(input.amount, "amount", true)?;
    let customer = customers::get(tx, input.customer_id)
        .await
        .map_err(|_| AppError::field("customer_id", "NOT_FOUND", "choose a customer"))?;
    if customer.is_walk_in {
        return Err(AppError::field("customer_id", "WALK_IN_CUSTOMER", "choose a named customer"));
    }
    let ctx = context::load(tx).await?;
    let tenant_id = tx.tenant_id();

    // Documents first (lock order): the customer's open invoices.
    let open: Vec<(Uuid, Decimal)> = sqlx::query_as(
        "SELECT id, due_total FROM sales WHERE tenant_id = ? AND customer_id = ? AND due_total > 0 \
         AND status <> 'cancelled' ORDER BY sale_date, id FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(customer.id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let allocations: Vec<(Uuid, Decimal)> = match &input.allocations {
        None => allocate_fifo(&open, value).0,
        Some(chosen) => {
            let mut out = Vec::new();
            for (i, a) in chosen.iter().enumerate() {
                let share = amount(a.amount, &format!("allocations[{i}].amount"), true)?;
                let due = open.iter().find(|(id, _)| *id == a.sale_id).map(|(_, due)| *due).ok_or_else(|| {
                    AppError::field(&format!("allocations[{i}].sale_id"), "NOT_OPEN", "this invoice has nothing due")
                })?;
                if share > due || out.iter().any(|(id, _): &(Uuid, Decimal)| *id == a.sale_id) {
                    return Err(AppError::field(
                        &format!("allocations[{i}].amount"),
                        "TOO_LARGE",
                        format!("at most {due} is due"),
                    ));
                }
                out.push((a.sale_id, share));
            }
            if out.iter().map(|(_, a)| *a).sum::<Decimal>() > value {
                return Err(AppError::field("allocations", "TOO_LARGE", "the allocations are more than the payment"));
            }
            out
        }
    };

    let payment = payments::post(
        tx,
        NewPayment {
            direction: Direction::In,
            branch_id: ctx.branch_id,
            business_date: ctx.business_date,
            party: Some(("customer", customer.id)),
            account_id: input.account_id,
            method_id: input.method_id,
            amount: value,
            reference: input.reference.clone(),
            note: input.note.clone().or_else(|| Some(format!("Payment from {}", customer.name))),
            source: "collection",
            txn_type: None,
            allocations: allocations
                .iter()
                .map(|(id, a)| Allocation { document_type: "sale", document_id: *id, amount: *a })
                .collect(),
            field: "payment".into(),
        },
    )
    .await?;
    apply_to_sales(tx, &allocations, Decimal::ONE).await?;

    ledger::post(
        tx,
        PartyEntry {
            party_type: PartyType::Customer,
            party_id: customer.id,
            branch_id: Some(ctx.branch_id),
            entry_type: "payment_in",
            amount: -value,
            business_date: ctx.business_date,
            reference_type: "payment",
            reference_id: payment.id,
            reference_no: Some(payment.payment_no.clone()),
            note: input.note.clone(),
        },
    )
    .await?;
    let accounts = SystemAccounts::load(tx).await?;
    let mut j = NewJournal::new(ctx.business_date, "payment", payment.id)
        .branch(ctx.branch_id)
        .number(payment.payment_no.clone())
        .memo(format!("Payment from {}", customer.name));
    j.debit(payment.ledger_account_id, value)
        .line(JournalLine::credit(accounts.receivable()?, value).party("customer", customer.id));
    journal::post(tx, j).await?;

    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("customer.payment_received").entity("payment", payment.id).branch(ctx.branch_id).change(
            None,
            Some(serde_json::json!({
                "payment_no": payment.payment_no, "customer": customer.name, "amount": value, "advance": payment.unallocated,
            })),
        ),
    )
    .await?;
    payments::get(tx, payment.id).await
}

/// Adds (`sign` = 1) or removes (−1) paid amounts on sales.
async fn apply_to_sales(tx: &mut TenantTx, allocations: &[(Uuid, Decimal)], sign: Decimal) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    for (sale_id, value) in allocations {
        let (grand, paid, due, status): (Decimal, Decimal, Decimal, String) = sqlx::query_as(
            "SELECT grand_total, paid_total, due_total, status FROM sales WHERE tenant_id = ? AND id = ? FOR UPDATE",
        )
        .bind(tenant_id)
        .bind(sale_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
        let paid = paid + sign * value;
        let due = due - sign * value;
        let pay_status = if status == "cancelled" { "refunded" } else { payment_status(grand, paid, due) };
        sqlx::query(
            "UPDATE sales SET paid_total = ?, due_total = ?, payment_status = ? WHERE tenant_id = ? AND id = ?",
        )
        .bind(paid)
        .bind(due)
        .bind(pay_status)
        .bind(tenant_id)
        .bind(sale_id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }
    Ok(())
}

/// Voids a customer collection (PAY-R04): dues come back on the invoices,
/// the money leaves the account, ledger and journal are reversed.
pub async fn void(tx: &mut TenantTx, meta: &RequestMeta, payment_id: Uuid, reason: &str) -> AppResult<PaymentDetail> {
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(AppError::field("reason", "REQUIRED", "enter a reason"));
    }
    let tenant_id = tx.tenant_id();
    // Lock the invoices before the payment's account (lock order).
    let mut sales: Vec<Uuid> = sqlx::query_scalar(
        "SELECT DISTINCT document_id FROM payment_allocations WHERE tenant_id = ? AND payment_id = ? \
         AND document_type = 'sale' AND reversed_at IS NULL",
    )
    .bind(tenant_id)
    .bind(payment_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    sales.sort();
    for sale_id in &sales {
        sqlx::query("SELECT id FROM sales WHERE tenant_id = ? AND id = ? FOR UPDATE")
            .bind(tenant_id)
            .bind(sale_id)
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
    }
    let today = context::today(tx).await?;
    let voided = payments::void(tx, payment_id, reason, today, &["collection"]).await?;
    let mut per_sale: BTreeMap<Uuid, Decimal> = BTreeMap::new();
    for (kind, id, value) in &voided.allocations {
        if kind == "sale" {
            *per_sale.entry(*id).or_default() += *value;
        }
    }
    let allocations: Vec<(Uuid, Decimal)> = per_sale.into_iter().collect();
    apply_to_sales(tx, &allocations, Decimal::NEGATIVE_ONE).await?;
    if let Some((_, customer_id)) = voided.party.as_ref().filter(|(t, _)| t == "customer") {
        ledger::post(
            tx,
            PartyEntry {
                party_type: PartyType::Customer,
                party_id: *customer_id,
                branch_id: Some(voided.branch_id),
                entry_type: "payment_void",
                amount: voided.amount,
                business_date: today,
                reference_type: "payment",
                reference_id: payment_id,
                reference_no: Some(voided.payment_no.clone()),
                note: Some(format!("Void: {reason}")),
            },
        )
        .await?;
    }
    journal::reverse_source(tx, "payment", payment_id, today, &format!("Void: {reason}")).await?;
    pos_audit::record(tx, meta, AuditEntry::new("payment.voided").entity("payment", payment_id).reason(reason)).await?;
    payments::get(tx, payment_id).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fifo_pays_oldest_first_and_keeps_the_rest() {
        let (a, b) = (Uuid::from_u128(1), Uuid::from_u128(2));
        let d = |s: &str| s.parse::<Decimal>().expect("decimal");
        let (alloc, left) = allocate_fifo(&[(a, d("300")), (b, d("500"))], d("600"));
        assert_eq!(alloc, vec![(a, d("300")), (b, d("300"))]);
        assert_eq!(left, Decimal::ZERO);
        let (alloc, left) = allocate_fifo(&[(a, d("300"))], d("450"));
        assert_eq!((alloc, left), (vec![(a, d("300"))], d("150")));
    }
}
