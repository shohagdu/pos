//! Supplier payments (PTY-R07 for suppliers, DB design §10.4 step 7): paid
//! when a bill is posted or later, allocated oldest bill first (FIFO) or to
//! chosen bills; any rest is an advance with the supplier. Each payment
//! journals Dr Accounts Payable / Cr cash or bank, and can be voided.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use pos_accounting::{JournalLine, NewJournal, SystemAccounts, journal};
use pos_audit::AuditEntry;
use pos_core::{AppError, AppResult, RequestMeta, money::amount};
use pos_db::{TenantTx, map_db_error};
use pos_money::payments::{self, Allocation, Direction, NewPayment, PaymentDetail, PostedPayment};
use pos_parties::{PartyEntry, PartyType, ledger, suppliers};
use pos_tenancy::context;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PayInput {
    pub method_id: Uuid,
    #[schema(value_type = String)]
    pub amount: Decimal,
    /// Blank = the method's default account.
    #[serde(default)]
    pub account_id: Option<Uuid>,
    #[serde(default)]
    pub reference: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct BillAllocationInput {
    pub purchase_id: Uuid,
    #[schema(value_type = String)]
    pub amount: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SupplierPaymentInput {
    pub supplier_id: Uuid,
    #[schema(value_type = String)]
    pub amount: Decimal,
    pub method_id: Uuid,
    #[serde(default)]
    pub account_id: Option<Uuid>,
    #[serde(default)]
    pub reference: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    /// Blank = oldest bills first.
    #[serde(default)]
    pub allocations: Option<Vec<BillAllocationInput>>,
}

/// `received` = value of the goods received (what was owed before payments).
fn payment_status(received: Decimal, paid: Decimal, due: Decimal) -> &'static str {
    if due <= Decimal::ZERO || received.is_zero() {
        "paid"
    } else if paid.is_zero() && due >= received {
        "unpaid"
    } else {
        "partial"
    }
}

/// Adds (`sign` = 1) or removes (−1) paid amounts on bills.
async fn apply_to_purchases(tx: &mut TenantTx, allocations: &[(Uuid, Decimal)], sign: Decimal) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    for (purchase_id, value) in allocations {
        let (received, paid, due): (Decimal, Decimal, Decimal) = sqlx::query_as(
            "SELECT received_total, paid_total, due_total FROM purchases WHERE tenant_id = ? AND id = ? FOR UPDATE",
        )
        .bind(tenant_id)
        .bind(purchase_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
        let paid = paid + sign * value;
        let due = due - sign * value;
        sqlx::query(
            "UPDATE purchases SET paid_total = ?, due_total = ?, payment_status = ? WHERE tenant_id = ? AND id = ?",
        )
        .bind(paid)
        .bind(due)
        .bind(payment_status(received, paid, due))
        .bind(tenant_id)
        .bind(purchase_id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }
    Ok(())
}

/// Posts the payment, the supplier ledger entry and the journal.
#[allow(clippy::too_many_arguments)]
async fn post_supplier_payment(
    tx: &mut TenantTx,
    supplier_id: Uuid,
    supplier_name: &str,
    input: &PayInput,
    value: Decimal,
    allocations: &[(Uuid, Decimal)],
    branch_id: Uuid,
    business_date: NaiveDate,
) -> AppResult<PostedPayment> {
    let payment = payments::post(
        tx,
        NewPayment {
            direction: Direction::Out,
            branch_id,
            business_date,
            party: Some(("supplier", supplier_id)),
            account_id: input.account_id,
            method_id: input.method_id,
            amount: value,
            reference: input.reference.clone(),
            note: input.note.clone().or_else(|| Some(format!("Payment to {supplier_name}"))),
            source: "supplier_payment",
            txn_type: None,
            allocations: allocations
                .iter()
                .map(|(id, a)| Allocation { document_type: "purchase", document_id: *id, amount: *a })
                .collect(),
            field: "payment".into(),
        },
    )
    .await?;
    apply_to_purchases(tx, allocations, Decimal::ONE).await?;
    ledger::post(
        tx,
        PartyEntry {
            party_type: PartyType::Supplier,
            party_id: supplier_id,
            branch_id: Some(branch_id),
            entry_type: "payment_out",
            amount: -value,
            business_date,
            reference_type: "payment",
            reference_id: payment.id,
            reference_no: Some(payment.payment_no.clone()),
            note: input.note.clone(),
        },
    )
    .await?;
    let accounts = SystemAccounts::load(tx).await?;
    let mut j = NewJournal::new(business_date, "payment", payment.id)
        .branch(branch_id)
        .number(payment.payment_no.clone())
        .memo(format!("Payment to {supplier_name}"));
    j.line(JournalLine::debit(accounts.payable()?, value).party("supplier", supplier_id))
        .credit(payment.ledger_account_id, value);
    journal::post(tx, j).await?;
    Ok(payment)
}

/// Pays (part of) one bill while posting it.
pub(crate) async fn pay_document(
    tx: &mut TenantTx,
    supplier_id: Uuid,
    purchase_id: Uuid,
    purchase_no: &str,
    input: &PayInput,
    branch_id: Uuid,
    business_date: NaiveDate,
) -> AppResult<PostedPayment> {
    let value = amount(input.amount, "payment.amount", true)?;
    let tenant_id = tx.tenant_id();
    let due: Decimal = sqlx::query_scalar("SELECT due_total FROM purchases WHERE tenant_id = ? AND id = ? FOR UPDATE")
        .bind(tenant_id)
        .bind(purchase_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
    if value > due {
        return Err(AppError::field("payment.amount", "TOO_LARGE", format!("the bill total is {due}")));
    }
    let supplier = suppliers::get(tx, supplier_id).await?;
    let mut input = input.clone();
    if input.note.is_none() {
        input.note = Some(format!("Bill {purchase_no}"));
    }
    post_supplier_payment(
        tx,
        supplier_id,
        &supplier.name,
        &input,
        value,
        &[(purchase_id, value)],
        branch_id,
        business_date,
    )
    .await
}

/// Pays a supplier against their open bills (FIFO or chosen).
pub async fn pay(tx: &mut TenantTx, meta: &RequestMeta, input: &SupplierPaymentInput) -> AppResult<PaymentDetail> {
    let value = amount(input.amount, "amount", true)?;
    let supplier = suppliers::get(tx, input.supplier_id)
        .await
        .map_err(|_| AppError::field("supplier_id", "NOT_FOUND", "choose a supplier"))?;
    let ctx = context::load(tx).await?;
    let tenant_id = tx.tenant_id();
    let open: Vec<(Uuid, Decimal)> = sqlx::query_as(
        "SELECT id, due_total FROM purchases WHERE tenant_id = ? AND supplier_id = ? AND status IN ('partially_received', 'received') \
         AND due_total > 0 ORDER BY purchase_date, created_at, id FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(supplier.id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let allocations: Vec<(Uuid, Decimal)> = match &input.allocations {
        None => {
            let mut left = value;
            let mut out = Vec::new();
            for (id, due) in &open {
                if left.is_zero() {
                    break;
                }
                let take = left.min(*due);
                out.push((*id, take));
                left -= take;
            }
            out
        }
        Some(chosen) => {
            let mut out: Vec<(Uuid, Decimal)> = Vec::new();
            for (i, a) in chosen.iter().enumerate() {
                let share = amount(a.amount, &format!("allocations[{i}].amount"), true)?;
                let due = open.iter().find(|(id, _)| *id == a.purchase_id).map(|(_, d)| *d).ok_or_else(|| {
                    AppError::field(&format!("allocations[{i}].purchase_id"), "NOT_OPEN", "this bill has nothing due")
                })?;
                if share > due || out.iter().any(|(id, _)| *id == a.purchase_id) {
                    return Err(AppError::field(
                        &format!("allocations[{i}].amount"),
                        "TOO_LARGE",
                        format!("at most {due} is due"),
                    ));
                }
                out.push((a.purchase_id, share));
            }
            if out.iter().map(|(_, a)| *a).sum::<Decimal>() > value {
                return Err(AppError::field("allocations", "TOO_LARGE", "the allocations are more than the payment"));
            }
            out
        }
    };
    let pay = PayInput {
        method_id: input.method_id,
        amount: value,
        account_id: input.account_id,
        reference: input.reference.clone(),
        note: input.note.clone(),
    };
    let payment = post_supplier_payment(
        tx,
        supplier.id,
        &supplier.name,
        &pay,
        value,
        &allocations,
        ctx.branch_id,
        ctx.business_date,
    )
    .await?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("supplier.payment_made").entity("payment", payment.id).branch(ctx.branch_id).change(
            None,
            Some(serde_json::json!({ "payment_no": payment.payment_no, "supplier": supplier.name, "amount": value })),
        ),
    )
    .await?;
    payments::get(tx, payment.id).await
}

/// Voids a supplier payment: dues come back on the bills, the money returns to
/// the account, ledger and journal are reversed.
pub async fn void(tx: &mut TenantTx, meta: &RequestMeta, payment_id: Uuid, reason: &str) -> AppResult<PaymentDetail> {
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(AppError::field("reason", "REQUIRED", "enter a reason"));
    }
    let tenant_id = tx.tenant_id();
    let mut bills: Vec<Uuid> = sqlx::query_scalar(
        "SELECT DISTINCT document_id FROM payment_allocations WHERE tenant_id = ? AND payment_id = ? \
         AND document_type = 'purchase' AND reversed_at IS NULL",
    )
    .bind(tenant_id)
    .bind(payment_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    bills.sort();
    for id in &bills {
        sqlx::query("SELECT id FROM purchases WHERE tenant_id = ? AND id = ? FOR UPDATE")
            .bind(tenant_id)
            .bind(id)
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
    }
    let today = context::today(tx).await?;
    let voided = payments::void(tx, payment_id, reason, today, &["supplier_payment"]).await?;
    let mut per_bill: BTreeMap<Uuid, Decimal> = BTreeMap::new();
    for (kind, id, value) in &voided.allocations {
        if kind == "purchase" {
            *per_bill.entry(*id).or_default() += *value;
        }
    }
    let allocations: Vec<(Uuid, Decimal)> = per_bill.into_iter().collect();
    apply_to_purchases(tx, &allocations, Decimal::NEGATIVE_ONE).await?;
    if let Some((_, supplier_id)) = voided.party.as_ref().filter(|(t, _)| t == "supplier") {
        ledger::post(
            tx,
            PartyEntry {
                party_type: PartyType::Supplier,
                party_id: *supplier_id,
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
