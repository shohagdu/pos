//! Expense summary by category and by month (R-19).

use chrono::NaiveDate;
use pos_core::AppResult;
use pos_db::{TenantTx, map_db_error};
use rust_decimal::Decimal;
use serde::Serialize;
use uuid::Uuid;

use crate::period::{DateRange, resolve};

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct CategoryExpenseRow {
    pub category_id: Uuid,
    pub category_name: String,
    pub count: i64,
    #[schema(value_type = String)]
    pub amount: Decimal,
    #[schema(value_type = String)]
    pub tax_amount: Decimal,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct MonthExpenseRow {
    /// `2026-10`.
    pub period: String,
    pub count: i64,
    #[schema(value_type = String)]
    pub amount: Decimal,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ExpenseSummary {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub by_category: Vec<CategoryExpenseRow>,
    pub by_month: Vec<MonthExpenseRow>,
    pub count: i64,
    #[schema(value_type = String)]
    pub total: Decimal,
}

pub async fn summary(tx: &mut TenantTx, range: &DateRange) -> AppResult<ExpenseSummary> {
    let (from, to) = resolve(tx, range).await?;
    let tenant_id = tx.tenant_id();
    let branch = tx.branch_id();
    let by_category: Vec<CategoryExpenseRow> = sqlx::query_as(
        "SELECT c.id AS category_id, c.name AS category_name, COUNT(*) AS count, COALESCE(SUM(e.amount), 0) AS amount, \
         COALESCE(SUM(e.tax_amount), 0) AS tax_amount FROM expenses e \
         JOIN expense_categories c ON c.tenant_id = e.tenant_id AND c.id = e.category_id \
         WHERE e.tenant_id = ? AND (? IS NULL OR e.branch_id = ?) AND e.status = 'posted' AND e.business_date BETWEEN ? AND ? \
         GROUP BY c.id, c.name ORDER BY amount DESC",
    )
    .bind(tenant_id)
    .bind(branch)
    .bind(branch)
    .bind(from)
    .bind(to)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let by_month = sqlx::query_as(
        "SELECT DATE_FORMAT(business_date, '%Y-%m') AS period, COUNT(*) AS count, COALESCE(SUM(amount), 0) AS amount \
         FROM expenses WHERE tenant_id = ? AND (? IS NULL OR branch_id = ?) AND status = 'posted' AND business_date BETWEEN ? AND ? \
         GROUP BY period ORDER BY period",
    )
    .bind(tenant_id)
    .bind(branch)
    .bind(branch)
    .bind(from)
    .bind(to)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(ExpenseSummary {
        from,
        to,
        count: by_category.iter().map(|r| r.count).sum(),
        total: by_category.iter().map(|r| r.amount).sum(),
        by_category,
        by_month,
    })
}
