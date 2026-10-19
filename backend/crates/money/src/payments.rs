//! Payments (PAY-R02, R04): one money movement in or out of an account,
//! optionally for a party and allocated to documents. Posting writes the
//! payment, its allocations and the book line; the caller owns the documents
//! it pays, the party ledger and the journal.

use chrono::{DateTime, NaiveDate, Utc};
use pos_core::{
    AppError, AppResult,
    id::new_id,
    page::{self, Keyset},
};
use pos_db::{TenantTx, map_db_error};
use pos_tenancy::sequences;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

use crate::{
    accounts::{self, AccountEntry},
    methods,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    In,
    Out,
}

impl Direction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::In => "in",
            Self::Out => "out",
        }
    }

    /// `+1` for money in, `−1` for money out.
    pub fn sign(self) -> Decimal {
        match self {
            Self::In => Decimal::ONE,
            Self::Out => Decimal::NEGATIVE_ONE,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Allocation {
    /// `sale`, `purchase`, `sale_return`, `purchase_return`, `expense`, `opening_balance`.
    pub document_type: &'static str,
    pub document_id: Uuid,
    pub amount: Decimal,
}

#[derive(Debug, Clone)]
pub struct NewPayment {
    pub direction: Direction,
    pub branch_id: Uuid,
    pub business_date: NaiveDate,
    /// `customer` or `supplier`.
    pub party: Option<(&'static str, Uuid)>,
    /// Blank = the method's default account.
    pub account_id: Option<Uuid>,
    pub method_id: Uuid,
    pub amount: Decimal,
    pub reference: Option<String>,
    pub note: Option<String>,
    /// `pos`, `collection`, `supplier_payment`, `refund`, `expense`, `income`, `manual`.
    pub source: &'static str,
    /// Book line type; blank = `payment_in` / `payment_out`.
    pub txn_type: Option<&'static str>,
    pub allocations: Vec<Allocation>,
    /// Name of the input field for errors (`payments[0]`).
    pub field: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PostedPayment {
    pub id: Uuid,
    pub payment_no: String,
    pub account_id: Uuid,
    pub ledger_account_id: Uuid,
    pub amount: Decimal,
    pub unallocated: Decimal,
}

pub async fn post(tx: &mut TenantTx, p: NewPayment) -> AppResult<PostedPayment> {
    if p.amount <= Decimal::ZERO {
        return Err(AppError::field(
            &format!("{}.amount", p.field),
            "INVALID_AMOUNT",
            "enter an amount of more than 0",
        ));
    }
    let method = methods::require_active(tx, p.method_id, &format!("{}.method_id", p.field)).await?;
    let reference = p
        .reference
        .as_deref()
        .map(str::trim)
        .filter(|r| !r.is_empty())
        .map(|r| r.chars().take(100).collect::<String>());
    if method.requires_reference && reference.is_none() {
        return Err(AppError::field(
            &format!("{}.reference", p.field),
            "REFERENCE_REQUIRED",
            format!("enter the {} transaction id", method.name),
        ));
    }
    let mut account_id = p.account_id.or(method.default_account_id).ok_or_else(|| {
        AppError::field(&format!("{}.account_id", p.field), "REQUIRED", "choose the account the money goes to")
    })?;
    if p.account_id.is_none() {
        account_id = accounts::branch_counterpart(tx, account_id, p.branch_id).await?.unwrap_or(account_id);
    }
    let account = accounts::require_active(tx, account_id, &format!("{}.account_id", p.field)).await?;
    let ledger_account_id = match account.ledger_account_id {
        Some(id) => id,
        None => pos_accounting::ledger::for_payment_account(tx, account_id).await?,
    };
    let allocated: Decimal = p.allocations.iter().map(|a| a.amount).sum();
    if allocated > p.amount || p.allocations.iter().any(|a| a.amount <= Decimal::ZERO) {
        return Err(AppError::internal("payment allocations exceed the payment"));
    }
    let unallocated = p.amount - allocated;

    let tenant_id = tx.tenant_id();
    let user_id = tx.user_id();
    let doc_type = match p.direction {
        Direction::In => "payment_in",
        Direction::Out => "payment_out",
    };
    let payment_no = sequences::next(tx, doc_type, Some(p.branch_id), p.business_date).await?;
    let id = new_id();
    sqlx::query(
        "INSERT INTO payments (tenant_id, id, branch_id, payment_no, direction, business_date, party_type, party_id, \
         account_id, payment_method_id, amount, unallocated_amount, reference, note, source, register_shift_id, created_by) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, \
           (SELECT s.id FROM register_shifts s WHERE s.tenant_id = ? AND s.branch_id = ? AND s.opened_by = ? \
              AND s.status = 'open' ORDER BY s.opened_at DESC LIMIT 1), ?)",
    )
    .bind(tenant_id)
    .bind(id)
    .bind(p.branch_id)
    .bind(&payment_no)
    .bind(p.direction.as_str())
    .bind(p.business_date)
    .bind(p.party.map(|x| x.0))
    .bind(p.party.map(|x| x.1))
    .bind(account_id)
    .bind(p.method_id)
    .bind(p.amount)
    .bind(unallocated)
    .bind(&reference)
    .bind(p.note.as_deref().map(|n| n.chars().take(500).collect::<String>()))
    .bind(p.source)
    .bind(tenant_id)
    .bind(p.branch_id)
    .bind(user_id)
    .bind(user_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if !p.allocations.is_empty() {
        let mut q: QueryBuilder<MySql> = QueryBuilder::new(
            "INSERT INTO payment_allocations (tenant_id, id, payment_id, document_type, document_id, amount) ",
        );
        q.push_values(&p.allocations, |mut row, a| {
            row.push_bind(tenant_id)
                .push_bind(new_id())
                .push_bind(id)
                .push_bind(a.document_type)
                .push_bind(a.document_id)
                .push_bind(a.amount);
        });
        q.build().execute(&mut **tx).await.map_err(map_db_error)?;
    }
    accounts::post_transaction(
        tx,
        AccountEntry {
            account_id,
            branch_id: Some(p.branch_id),
            txn_type: p.txn_type.unwrap_or(doc_type),
            amount: p.direction.sign() * p.amount,
            business_date: p.business_date,
            reference_type: "payment",
            reference_id: id,
            reference_no: Some(payment_no.clone()),
            note: p.note.clone(),
        },
    )
    .await?;
    Ok(PostedPayment { id, payment_no, account_id, ledger_account_id, amount: p.amount, unallocated })
}

/// What a void undid, for the caller to reverse documents, ledger and journal.
#[derive(Debug, Clone)]
pub struct VoidedPayment {
    pub id: Uuid,
    pub payment_no: String,
    pub direction: Direction,
    pub source: String,
    pub party: Option<(String, Uuid)>,
    pub amount: Decimal,
    pub branch_id: Uuid,
    /// Allocations that were active, now reversed: (document type, id, amount).
    pub allocations: Vec<(String, Uuid, Decimal)>,
}

#[derive(sqlx::FromRow)]
struct Locked {
    payment_no: String,
    direction: String,
    source: String,
    status: String,
    party_type: Option<String>,
    party_id: Option<Uuid>,
    amount: Decimal,
    account_id: Uuid,
    branch_id: Uuid,
}

/// Voids a posted payment (PAY-R04): marks it voided, reverses its
/// allocations and books the money back. Only `allowed_sources` may be voided
/// here (POS payments go with their sale, expense payments with the expense).
pub async fn void(
    tx: &mut TenantTx,
    id: Uuid,
    reason: &str,
    business_date: NaiveDate,
    allowed_sources: &[&str],
) -> AppResult<VoidedPayment> {
    let tenant_id = tx.tenant_id();
    let user_id = tx.user_id();
    let row: Option<Locked> = sqlx::query_as(
        "SELECT payment_no, direction, source, status, party_type, party_id, amount, account_id, branch_id \
         FROM payments WHERE tenant_id = ? AND id = ? FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let Locked { payment_no, direction, source, status, party_type, party_id, amount, account_id, branch_id } =
        row.ok_or(AppError::NotFound)?;
    if status != "posted" {
        return Err(AppError::Unprocessable {
            code: "ALREADY_VOIDED",
            message: "the payment is already voided".into(),
        });
    }
    if !allowed_sources.contains(&source.as_str()) {
        return Err(AppError::Unprocessable {
            code: "VOID_SOURCE_DOCUMENT",
            message: "this payment belongs to a sale, return or expense; cancel that document instead".into(),
        });
    }
    let direction = if direction == "in" { Direction::In } else { Direction::Out };
    let allocations: Vec<(String, Uuid, Decimal)> = sqlx::query_as(
        "SELECT document_type, document_id, amount FROM payment_allocations \
         WHERE tenant_id = ? AND payment_id = ? AND reversed_at IS NULL ORDER BY created_at, id",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    sqlx::query("UPDATE payment_allocations SET reversed_at = UTC_TIMESTAMP(6) WHERE tenant_id = ? AND payment_id = ? AND reversed_at IS NULL")
        .bind(tenant_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    sqlx::query(
        "UPDATE payments SET status = 'voided', voided_at = UTC_TIMESTAMP(6), voided_by = ?, void_reason = ? \
         WHERE tenant_id = ? AND id = ?",
    )
    .bind(user_id)
    .bind(reason.chars().take(500).collect::<String>())
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    accounts::post_transaction(
        tx,
        AccountEntry {
            account_id,
            branch_id: Some(branch_id),
            txn_type: "payment_void",
            amount: -direction.sign() * amount,
            business_date,
            reference_type: "payment",
            reference_id: id,
            reference_no: Some(payment_no.clone()),
            note: Some(format!("Void: {reason}")),
        },
    )
    .await?;
    Ok(VoidedPayment {
        id,
        payment_no,
        direction,
        source,
        party: party_type.zip(party_id),
        amount,
        branch_id,
        allocations,
    })
}

// ------------------------------------------------------------------ reading

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct Payment {
    pub id: Uuid,
    pub payment_no: String,
    /// `in` or `out`.
    pub direction: String,
    pub payment_date: DateTime<Utc>,
    pub business_date: NaiveDate,
    pub party_type: Option<String>,
    pub party_id: Option<Uuid>,
    pub party_name: Option<String>,
    pub account_id: Uuid,
    pub account_name: String,
    pub payment_method_id: Uuid,
    pub method_name: String,
    #[schema(value_type = String)]
    pub amount: Decimal,
    /// Advance not yet applied to a document.
    #[schema(value_type = String)]
    pub unallocated_amount: Decimal,
    pub reference: Option<String>,
    pub note: Option<String>,
    pub source: String,
    /// `posted` or `voided`.
    pub status: String,
    pub void_reason: Option<String>,
    pub created_by_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct PaymentAllocation {
    pub document_type: String,
    pub document_id: Uuid,
    /// Invoice / bill number.
    pub document_no: Option<String>,
    #[schema(value_type = String)]
    pub amount: Decimal,
    pub reversed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct PaymentDetail {
    #[serde(flatten)]
    pub payment: Payment,
    pub allocations: Vec<PaymentAllocation>,
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct PaymentFilter {
    /// `in` or `out`.
    pub direction: Option<String>,
    pub party_type: Option<String>,
    pub party_id: Option<Uuid>,
    pub account_id: Option<Uuid>,
    pub source: Option<String>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    /// `posted` (default), `voided` or `all`.
    pub status: Option<String>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct PaymentPage {
    pub data: Vec<Payment>,
    pub next_cursor: Option<String>,
}

const SELECT: &str = "SELECT p.id, p.payment_no, p.direction, p.payment_date, p.business_date, p.party_type, p.party_id, \
     COALESCE(c.name, s.name) AS party_name, p.account_id, a.name AS account_name, p.payment_method_id, \
     m.name AS method_name, p.amount, p.unallocated_amount, p.reference, p.note, p.source, p.status, p.void_reason, \
     (SELECT u.full_name FROM users u WHERE u.id = p.created_by) AS created_by_name, p.created_at \
     FROM payments p \
     JOIN payment_accounts a ON a.tenant_id = p.tenant_id AND a.id = p.account_id \
     JOIN payment_methods m ON m.tenant_id = p.tenant_id AND m.id = p.payment_method_id \
     LEFT JOIN customers c ON p.party_type = 'customer' AND c.tenant_id = p.tenant_id AND c.id = p.party_id \
     LEFT JOIN suppliers s ON p.party_type = 'supplier' AND s.tenant_id = p.tenant_id AND s.id = p.party_id \
     WHERE p.tenant_id = ";

pub async fn list(tx: &mut TenantTx, filter: &PaymentFilter) -> AppResult<PaymentPage> {
    let limit = page::limit(filter.limit)?;
    let after = filter.cursor.as_deref().map(Keyset::decode).transpose()?;
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(SELECT);
    q.push_bind(tenant_id);
    if let Some(branch_id) = tx.branch_id() {
        q.push(" AND p.branch_id = ").push_bind(branch_id);
    }
    match filter.direction.as_deref() {
        None | Some("") => {}
        Some(d @ ("in" | "out")) => {
            q.push(" AND p.direction = ").push_bind(d.to_owned());
        }
        Some(_) => return Err(AppError::field("direction", "INVALID_DIRECTION", "use in or out")),
    }
    match filter.status.as_deref() {
        None | Some("posted") => {
            q.push(" AND p.status = 'posted'");
        }
        Some("voided") => {
            q.push(" AND p.status = 'voided'");
        }
        Some("all") => {}
        Some(_) => return Err(AppError::field("status", "INVALID_STATUS", "use posted, voided or all")),
    }
    if let Some(party_type) = filter.party_type.as_deref().filter(|p| !p.is_empty()) {
        q.push(" AND p.party_type = ").push_bind(party_type.to_owned());
    }
    if let Some(party_id) = filter.party_id {
        q.push(" AND p.party_id = ").push_bind(party_id);
    }
    if let Some(account_id) = filter.account_id {
        q.push(" AND p.account_id = ").push_bind(account_id);
    }
    if let Some(source) = filter.source.as_deref().filter(|s| !s.is_empty()) {
        q.push(" AND p.source = ").push_bind(source.to_owned());
    }
    if let Some(from) = filter.from {
        q.push(" AND p.business_date >= ").push_bind(from);
    }
    if let Some(to) = filter.to {
        q.push(" AND p.business_date <= ").push_bind(to);
    }
    if let Some(k) = after {
        q.push(" AND (p.created_at < ").push_bind(k.at);
        q.push(" OR (p.created_at = ").push_bind(k.at).push(" AND p.id < ").push_bind(k.id).push("))");
    }
    q.push(" ORDER BY p.created_at DESC, p.id DESC LIMIT ").push_bind(limit + 1);
    let mut rows: Vec<Payment> = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    let next_cursor = if rows.len() > limit as usize {
        rows.truncate(limit as usize);
        rows.last().map(|r| Keyset { at: r.created_at, id: r.id }.encode())
    } else {
        None
    };
    Ok(PaymentPage { data: rows, next_cursor })
}

pub async fn get(tx: &mut TenantTx, id: Uuid) -> AppResult<PaymentDetail> {
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(SELECT);
    q.push_bind(tenant_id).push(" AND p.id = ").push_bind(id);
    let payment: Payment =
        q.build_query_as().fetch_optional(&mut **tx).await.map_err(map_db_error)?.ok_or(AppError::NotFound)?;
    let allocations = sqlx::query_as(
        "SELECT a.document_type, a.document_id, \
         COALESCE((SELECT invoice_no FROM sales s WHERE a.document_type = 'sale' AND s.tenant_id = a.tenant_id AND s.id = a.document_id), \
                  (SELECT purchase_no FROM purchases b WHERE a.document_type = 'purchase' AND b.tenant_id = a.tenant_id AND b.id = a.document_id), \
                  (SELECT return_no FROM sale_returns r WHERE a.document_type = 'sale_return' AND r.tenant_id = a.tenant_id AND r.id = a.document_id), \
                  (SELECT return_no FROM purchase_returns pr WHERE a.document_type = 'purchase_return' AND pr.tenant_id = a.tenant_id AND pr.id = a.document_id), \
                  (SELECT expense_no FROM expenses e WHERE a.document_type = 'expense' AND e.tenant_id = a.tenant_id AND e.id = a.document_id)) AS document_no, \
         a.amount, a.reversed_at FROM payment_allocations a WHERE a.tenant_id = ? AND a.payment_id = ? ORDER BY a.created_at, a.id",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(PaymentDetail { payment, allocations })
}

/// Active payments allocated to a document, oldest first.
pub async fn for_document(
    tx: &mut TenantTx,
    document_type: &str,
    document_id: Uuid,
) -> AppResult<Vec<(Payment, Decimal)>> {
    let tenant_id = tx.tenant_id();
    let ids: Vec<(Uuid, Decimal)> = sqlx::query_as(
        "SELECT payment_id, SUM(amount) FROM payment_allocations WHERE tenant_id = ? AND document_type = ? \
         AND document_id = ? AND reversed_at IS NULL GROUP BY payment_id",
    )
    .bind(tenant_id)
    .bind(document_type)
    .bind(document_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let mut out = Vec::with_capacity(ids.len());
    for (id, allocated) in ids {
        out.push((get(tx, id).await?.payment, allocated));
    }
    out.sort_by_key(|(p, _)| (p.created_at, p.id));
    Ok(out)
}
