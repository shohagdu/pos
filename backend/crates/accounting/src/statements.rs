//! Financial statements from the journals (ACC-R06, report R-25): trial
//! balance, profit and loss, balance sheet and the general ledger. Dates are
//! business dates; there is no year-end closing yet, so the balance sheet
//! shows accumulated profit as "current earnings".

use chrono::NaiveDate;
use pos_core::{AppError, AppResult};
use pos_db::{TenantTx, map_db_error};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const BEGINNING: NaiveDate = pos_core::dates::EARLIEST;

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct PeriodQuery {
    /// First day (inclusive); blank = from the beginning.
    pub from: Option<NaiveDate>,
    /// Last day (inclusive); blank = today.
    pub to: Option<NaiveDate>,
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct AsOfQuery {
    /// Blank = today.
    pub as_of: Option<NaiveDate>,
}

#[derive(Debug, Clone, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct LedgerQuery {
    pub account_id: Uuid,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

async fn resolve_to(tx: &mut TenantTx, to: Option<NaiveDate>) -> AppResult<NaiveDate> {
    match to {
        Some(d) => Ok(d),
        None => pos_tenancy::context::today(tx).await,
    }
}

fn check_range(from: NaiveDate, to: NaiveDate) -> AppResult<()> {
    if from > to {
        return Err(AppError::field("from", "INVALID_RANGE", "the start date is after the end date"));
    }
    Ok(())
}

#[derive(sqlx::FromRow)]
struct Movement {
    id: Uuid,
    code: String,
    name: String,
    account_type: String,
    system_key: Option<String>,
    opening: Decimal,
    debit: Decimal,
    credit: Decimal,
}

/// Opening balance (debit − credit) before `from`, and debits and credits
/// from `from` to `to`, per postable account.
async fn movements(tx: &mut TenantTx, from: NaiveDate, to: NaiveDate) -> AppResult<Vec<Movement>> {
    let tenant_id = tx.tenant_id();
    sqlx::query_as(
        "SELECT a.id, a.code, a.name, a.account_type, a.system_key, \
         COALESCE(SUM(CASE WHEN l.entry_date < ? THEN l.debit - l.credit END), 0) AS opening, \
         COALESCE(SUM(CASE WHEN l.entry_date >= ? THEN l.debit END), 0) AS debit, \
         COALESCE(SUM(CASE WHEN l.entry_date >= ? THEN l.credit END), 0) AS credit \
         FROM ledger_accounts a \
         LEFT JOIN journal_lines l ON l.tenant_id = a.tenant_id AND l.account_id = a.id AND l.entry_date <= ? \
         WHERE a.tenant_id = ? AND a.is_postable \
         GROUP BY a.id, a.code, a.name, a.account_type, a.system_key ORDER BY a.code",
    )
    .bind(from)
    .bind(from)
    .bind(from)
    .bind(to)
    .bind(tenant_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)
}

// ------------------------------------------------------------- trial balance

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct TrialBalanceRow {
    pub account_id: Uuid,
    pub code: String,
    pub name: String,
    pub account_type: String,
    /// Debit − credit before `from`.
    #[schema(value_type = String)]
    pub opening: Decimal,
    #[schema(value_type = String)]
    pub debit: Decimal,
    #[schema(value_type = String)]
    pub credit: Decimal,
    #[schema(value_type = String)]
    pub closing_debit: Decimal,
    #[schema(value_type = String)]
    pub closing_credit: Decimal,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct TrialBalance {
    pub from: Option<NaiveDate>,
    pub to: NaiveDate,
    pub rows: Vec<TrialBalanceRow>,
    #[schema(value_type = String)]
    pub total_debit: Decimal,
    #[schema(value_type = String)]
    pub total_credit: Decimal,
    #[schema(value_type = String)]
    pub total_closing_debit: Decimal,
    #[schema(value_type = String)]
    pub total_closing_credit: Decimal,
    /// Closing debits equal closing credits.
    pub balanced: bool,
}

pub async fn trial_balance(tx: &mut TenantTx, query: &PeriodQuery) -> AppResult<TrialBalance> {
    let to = resolve_to(tx, query.to).await?;
    let from = query.from.unwrap_or(BEGINNING);
    check_range(from, to)?;
    let mut rows = Vec::new();
    for m in movements(tx, from, to).await? {
        let closing = m.opening + m.debit - m.credit;
        if m.opening.is_zero() && m.debit.is_zero() && m.credit.is_zero() {
            continue;
        }
        rows.push(TrialBalanceRow {
            account_id: m.id,
            code: m.code,
            name: m.name,
            account_type: m.account_type,
            opening: m.opening,
            debit: m.debit,
            credit: m.credit,
            closing_debit: closing.max(Decimal::ZERO),
            closing_credit: (-closing).max(Decimal::ZERO),
        });
    }
    let total_debit = rows.iter().map(|r| r.debit).sum();
    let total_credit = rows.iter().map(|r| r.credit).sum();
    let total_closing_debit: Decimal = rows.iter().map(|r| r.closing_debit).sum();
    let total_closing_credit: Decimal = rows.iter().map(|r| r.closing_credit).sum();
    Ok(TrialBalance {
        from: query.from,
        to,
        rows,
        total_debit,
        total_credit,
        total_closing_debit,
        total_closing_credit,
        balanced: total_closing_debit == total_closing_credit,
    })
}

// ------------------------------------------------------------ profit & loss

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct StatementLine {
    pub account_id: Uuid,
    pub code: String,
    pub name: String,
    /// In the section's natural sign (income: credit − debit; expense: debit − credit).
    #[schema(value_type = String)]
    pub amount: Decimal,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct Section {
    pub lines: Vec<StatementLine>,
    #[schema(value_type = String)]
    pub total: Decimal,
}

impl Section {
    fn new(lines: Vec<StatementLine>) -> Self {
        let total = lines.iter().map(|l| l.amount).sum();
        Self { lines, total }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ProfitAndLoss {
    pub from: Option<NaiveDate>,
    pub to: NaiveDate,
    /// Sales less returns and discounts (excluding VAT).
    pub revenue: Section,
    pub cost_of_sales: Section,
    #[schema(value_type = String)]
    pub gross_profit: Decimal,
    pub other_income: Section,
    pub expenses: Section,
    #[schema(value_type = String)]
    pub net_profit: Decimal,
}

const REVENUE_KEYS: &[&str] = &["sales", "sales_returns", "sales_discount"];

pub async fn profit_and_loss(tx: &mut TenantTx, query: &PeriodQuery) -> AppResult<ProfitAndLoss> {
    let to = resolve_to(tx, query.to).await?;
    let from = query.from.unwrap_or(BEGINNING);
    check_range(from, to)?;
    let (mut revenue, mut cost, mut other, mut expenses) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for m in movements(tx, from, to).await? {
        let key = m.system_key.as_deref();
        let line = |amount| StatementLine { account_id: m.id, code: m.code.clone(), name: m.name.clone(), amount };
        match m.account_type.as_str() {
            "income" => {
                let amount = m.credit - m.debit;
                if amount.is_zero() {
                    continue;
                }
                if key.is_some_and(|k| REVENUE_KEYS.contains(&k)) {
                    revenue.push(line(amount));
                } else {
                    other.push(line(amount));
                }
            }
            "expense" => {
                let amount = m.debit - m.credit;
                if amount.is_zero() {
                    continue;
                }
                if key == Some("cogs") {
                    cost.push(line(amount));
                } else {
                    expenses.push(line(amount));
                }
            }
            _ => {}
        }
    }
    let (revenue, cost_of_sales, other_income, expenses) =
        (Section::new(revenue), Section::new(cost), Section::new(other), Section::new(expenses));
    let gross_profit = revenue.total - cost_of_sales.total;
    let net_profit = gross_profit + other_income.total - expenses.total;
    Ok(ProfitAndLoss { from: query.from, to, revenue, cost_of_sales, gross_profit, other_income, expenses, net_profit })
}

// ------------------------------------------------------------- balance sheet

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct BalanceSheet {
    pub as_of: NaiveDate,
    pub assets: Section,
    pub liabilities: Section,
    pub equity: Section,
    /// Income less expenses up to `as_of` (not yet closed to retained earnings).
    #[schema(value_type = String)]
    pub current_earnings: Decimal,
    #[schema(value_type = String)]
    pub total_assets: Decimal,
    /// Liabilities + equity + current earnings.
    #[schema(value_type = String)]
    pub total_liabilities_and_equity: Decimal,
    pub balanced: bool,
}

pub async fn balance_sheet(tx: &mut TenantTx, query: &AsOfQuery) -> AppResult<BalanceSheet> {
    let as_of = resolve_to(tx, query.as_of).await?;
    let (mut assets, mut liabilities, mut equity) = (Vec::new(), Vec::new(), Vec::new());
    let mut earnings = Decimal::ZERO;
    for m in movements(tx, BEGINNING, as_of).await? {
        let balance = m.opening + m.debit - m.credit;
        let line = |amount| StatementLine { account_id: m.id, code: m.code.clone(), name: m.name.clone(), amount };
        match m.account_type.as_str() {
            "asset" if !balance.is_zero() => assets.push(line(balance)),
            "liability" if !balance.is_zero() => liabilities.push(line(-balance)),
            "equity" if !balance.is_zero() => equity.push(line(-balance)),
            "income" | "expense" => earnings -= balance,
            _ => {}
        }
    }
    let (assets, liabilities, equity) = (Section::new(assets), Section::new(liabilities), Section::new(equity));
    let total_liabilities_and_equity = liabilities.total + equity.total + earnings;
    Ok(BalanceSheet {
        as_of,
        total_assets: assets.total,
        balanced: assets.total == total_liabilities_and_equity,
        assets,
        liabilities,
        equity,
        current_earnings: earnings,
        total_liabilities_and_equity,
    })
}

// ------------------------------------------------------------ general ledger

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct LedgerRow {
    pub entry_id: Uuid,
    pub entry_no: String,
    pub entry_date: NaiveDate,
    pub source_type: String,
    pub source_id: Option<Uuid>,
    pub source_no: Option<String>,
    pub memo: Option<String>,
    #[schema(value_type = String)]
    pub debit: Decimal,
    #[schema(value_type = String)]
    pub credit: Decimal,
    /// Running debit − credit.
    #[sqlx(skip)]
    #[schema(value_type = String)]
    pub balance: Decimal,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct GeneralLedger {
    pub account: crate::ledger::LedgerAccount,
    pub from: Option<NaiveDate>,
    pub to: NaiveDate,
    #[schema(value_type = String)]
    pub opening: Decimal,
    pub rows: Vec<LedgerRow>,
    #[schema(value_type = String)]
    pub closing: Decimal,
    /// More than the 5,000 rows shown exist in the range.
    pub truncated: bool,
}

const LEDGER_ROW_LIMIT: usize = 5000;

pub async fn general_ledger(tx: &mut TenantTx, query: &LedgerQuery) -> AppResult<GeneralLedger> {
    let account = crate::ledger::get(tx, query.account_id).await?;
    let to = resolve_to(tx, query.to).await?;
    let from = query.from.unwrap_or(BEGINNING);
    check_range(from, to)?;
    let tenant_id = tx.tenant_id();
    let opening: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(debit - credit), 0) FROM journal_lines WHERE tenant_id = ? AND account_id = ? AND entry_date < ?",
    )
    .bind(tenant_id)
    .bind(account.id)
    .bind(from)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let mut rows: Vec<LedgerRow> = sqlx::query_as(
        "SELECT e.id AS entry_id, e.entry_no, l.entry_date, e.source_type, e.source_id, e.source_no, \
         COALESCE(l.memo, e.memo) AS memo, l.debit, l.credit \
         FROM journal_lines l JOIN journal_entries e ON e.tenant_id = l.tenant_id AND e.id = l.entry_id \
         WHERE l.tenant_id = ? AND l.account_id = ? AND l.entry_date BETWEEN ? AND ? \
         ORDER BY l.entry_date, e.created_at, e.id LIMIT ?",
    )
    .bind(tenant_id)
    .bind(account.id)
    .bind(from)
    .bind(to)
    .bind(i64::try_from(LEDGER_ROW_LIMIT + 1).unwrap_or(i64::MAX))
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let truncated = rows.len() > LEDGER_ROW_LIMIT;
    rows.truncate(LEDGER_ROW_LIMIT);
    let mut running = opening;
    for row in &mut rows {
        running += row.debit - row.credit;
        row.balance = running;
    }
    Ok(GeneralLedger { account, from: query.from, to, opening, rows, closing: running, truncated })
}
