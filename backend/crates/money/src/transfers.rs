//! Account transfers (PAY-R03): one transfer record and two book lines, e.g.
//! a bKash cash-out to the bank or a cash deposit. A fee leaves the source
//! account as a bank/MFS charge expense.

use chrono::{DateTime, NaiveDate, Utc};
use pos_accounting::{NewJournal, SystemAccounts, journal, ledger};
use pos_audit::AuditEntry;
use pos_core::{
    AppError, AppResult, RequestMeta,
    id::new_id,
    money::amount,
    page::{self, Keyset},
};
use pos_db::{TenantTx, map_db_error};
use pos_tenancy::{context, sequences};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

use crate::accounts::{self, AccountEntry};

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TransferInput {
    pub from_account_id: Uuid,
    pub to_account_id: Uuid,
    #[schema(value_type = String)]
    pub amount: Decimal,
    /// Charged by the bank or MFS, paid from the source account.
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub fee_amount: Option<Decimal>,
    #[serde(default)]
    pub reference: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct Transfer {
    pub id: Uuid,
    pub transfer_no: String,
    pub from_account_id: Uuid,
    pub from_account_name: String,
    pub to_account_id: Uuid,
    pub to_account_name: String,
    #[schema(value_type = String)]
    pub amount: Decimal,
    #[schema(value_type = String)]
    pub fee_amount: Decimal,
    pub business_date: NaiveDate,
    pub reference: Option<String>,
    pub note: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct TransferFilter {
    pub account_id: Option<Uuid>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct TransferPage {
    pub data: Vec<Transfer>,
    pub next_cursor: Option<String>,
}

const SELECT: &str = "SELECT t.id, t.transfer_no, t.from_account_id, f.name AS from_account_name, t.to_account_id, \
     d.name AS to_account_name, t.amount, t.fee_amount, t.business_date, t.reference, t.note, t.status, t.created_at \
     FROM account_transfers t \
     JOIN payment_accounts f ON f.tenant_id = t.tenant_id AND f.id = t.from_account_id \
     JOIN payment_accounts d ON d.tenant_id = t.tenant_id AND d.id = t.to_account_id \
     WHERE t.tenant_id = ";

pub async fn get(tx: &mut TenantTx, id: Uuid) -> AppResult<Transfer> {
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(SELECT);
    q.push_bind(tenant_id).push(" AND t.id = ").push_bind(id);
    q.build_query_as().fetch_optional(&mut **tx).await.map_err(map_db_error)?.ok_or(AppError::NotFound)
}

pub async fn list(tx: &mut TenantTx, filter: &TransferFilter) -> AppResult<TransferPage> {
    let limit = page::limit(filter.limit)?;
    let after = filter.cursor.as_deref().map(Keyset::decode).transpose()?;
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(SELECT);
    q.push_bind(tenant_id);
    if let Some(account_id) = filter.account_id {
        q.push(" AND (t.from_account_id = ")
            .push_bind(account_id)
            .push(" OR t.to_account_id = ")
            .push_bind(account_id)
            .push(")");
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
    let mut rows: Vec<Transfer> = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    let next_cursor = if rows.len() > limit as usize {
        rows.truncate(limit as usize);
        rows.last().map(|r| Keyset { at: r.created_at, id: r.id }.encode())
    } else {
        None
    };
    Ok(TransferPage { data: rows, next_cursor })
}

pub async fn create(tx: &mut TenantTx, meta: &RequestMeta, input: TransferInput) -> AppResult<Transfer> {
    if input.from_account_id == input.to_account_id {
        return Err(AppError::field("to_account_id", "SAME_ACCOUNT", "choose two different accounts"));
    }
    let value = amount(input.amount, "amount", true)?;
    let fee = amount(input.fee_amount.unwrap_or_default(), "fee_amount", false)?;
    let reference = input
        .reference
        .as_deref()
        .map(str::trim)
        .filter(|r| !r.is_empty())
        .map(|r| r.chars().take(100).collect::<String>());
    let note =
        input.note.as_deref().map(str::trim).filter(|r| !r.is_empty()).map(|r| r.chars().take(500).collect::<String>());
    let ctx = context::load(tx).await?;
    // Lock both accounts in id order (DB design §10 lock order).
    let (first, second) = if input.from_account_id < input.to_account_id {
        (input.from_account_id, input.to_account_id)
    } else {
        (input.to_account_id, input.from_account_id)
    };
    let tenant_id = tx.tenant_id();
    for id in [first, second] {
        sqlx::query("SELECT id FROM payment_accounts WHERE tenant_id = ? AND id = ? FOR UPDATE")
            .bind(tenant_id)
            .bind(id)
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
    }
    let from = accounts::require_active(tx, input.from_account_id, "from_account_id").await?;
    let to = accounts::require_active(tx, input.to_account_id, "to_account_id").await?;

    let number = sequences::next(tx, "transfer", Some(ctx.branch_id), ctx.business_date).await?;
    let id = new_id();
    let user_id = tx.user_id();
    sqlx::query(
        "INSERT INTO account_transfers (tenant_id, id, branch_id, transfer_no, from_account_id, to_account_id, amount, \
         fee_amount, business_date, reference, note, created_by) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(id)
    .bind(ctx.branch_id)
    .bind(&number)
    .bind(from.id)
    .bind(to.id)
    .bind(value)
    .bind(fee)
    .bind(ctx.business_date)
    .bind(&reference)
    .bind(&note)
    .bind(user_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;

    let book = |account_id, txn_type, amount: Decimal, note: Option<String>| AccountEntry {
        account_id,
        branch_id: Some(ctx.branch_id),
        txn_type,
        amount,
        business_date: ctx.business_date,
        reference_type: "account_transfer",
        reference_id: id,
        reference_no: Some(number.clone()),
        note,
    };
    accounts::post_transaction(tx, book(from.id, "transfer_out", -value, Some(format!("To {}", to.name)))).await?;
    accounts::post_transaction(tx, book(to.id, "transfer_in", value, Some(format!("From {}", from.name)))).await?;
    if !fee.is_zero() {
        accounts::post_transaction(tx, book(from.id, "expense", -fee, Some("Transfer fee".into()))).await?;
    }

    let from_ledger = ledger::for_payment_account(tx, from.id).await?;
    let to_ledger = ledger::for_payment_account(tx, to.id).await?;
    let system = SystemAccounts::load(tx).await?;
    let mut j = NewJournal::new(ctx.business_date, "transfer", id)
        .branch(ctx.branch_id)
        .number(number.clone())
        .memo(format!("Transfer {} → {}", from.name, to.name));
    j.debit(to_ledger, value).credit(from_ledger, value);
    j.debit(system.bank_charges()?, fee).credit(from_ledger, fee);
    journal::post(tx, j).await?;

    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("account_transfer.created").entity("account_transfer", id).branch(ctx.branch_id).change(
            None,
            Some(serde_json::json!({ "transfer_no": number, "from": from.name, "to": to.name, "amount": value, "fee": fee })),
        ),
    )
    .await?;
    get(tx, id).await
}
