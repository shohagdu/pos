//! Reports (§15): dashboard, sales, purchases, expenses, payments and profit.
//! Each report needs its `reports.*` permission (REP-R05); costs and profit
//! also need `products.view_cost` or `dashboard.view_financials`.

use actix_web::{HttpResponse, get, web};
use pos_reporting::{
    DateRange,
    analysis::{self, AgingQuery, AgingReport, DiscountReport, MovementQuery, ProductMovement, VatSummary},
    dashboard::{self, Dashboard, ProfitEstimate},
    expenses::{self, ExpenseSummary},
    purchases::{self, PurchaseReport},
    sales::{self, CashierSalesRow, CategorySalesRow, PaymentSummary, ProductSales, SalesSummary},
};

use crate::{ApiError, AppState, auth::TenantAuth};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(get_dashboard)
        .service(sales_summary)
        .service(sales_by_product)
        .service(sales_by_category)
        .service(sales_by_cashier)
        .service(payment_summary)
        .service(purchase_report)
        .service(expense_summary)
        .service(profit_estimate)
        .service(aging_report)
        .service(vat_summary)
        .service(discount_report)
        .service(product_movement);
}

/// Today and this month at a glance (R-01).
#[utoipa::path(get, path = "/api/v1/dashboard", tag = "reports", security(("bearer" = [])),
    responses((status = 200, body = Dashboard)))]
#[get("/dashboard")]
pub async fn get_dashboard(state: web::Data<AppState>, auth: TenantAuth) -> Result<HttpResponse, ApiError> {
    auth.require("dashboard.view")?;
    let mut tx = auth.begin(&state).await?;
    let data = dashboard::dashboard(&mut tx, auth.access.can("dashboard.view_financials")).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(data))
}

/// Sales by day or month with payments by method (R-02, R-03).
#[utoipa::path(get, path = "/api/v1/reports/sales-summary", tag = "reports", params(DateRange), security(("bearer" = [])),
    responses((status = 200, body = SalesSummary)))]
#[get("/reports/sales-summary")]
pub async fn sales_summary(
    state: web::Data<AppState>,
    auth: TenantAuth,
    range: web::Query<DateRange>,
) -> Result<HttpResponse, ApiError> {
    auth.require("reports.sales")?;
    let mut tx = auth.begin(&state).await?;
    let report = sales::summary(&mut tx, &range, auth.access.can("products.view_cost")).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(report))
}

/// Quantity, net sales, COGS and margin per product (R-04).
#[utoipa::path(get, path = "/api/v1/reports/sales-by-product", tag = "reports", params(DateRange), security(("bearer" = [])),
    responses((status = 200, body = ProductSales)))]
#[get("/reports/sales-by-product")]
pub async fn sales_by_product(
    state: web::Data<AppState>,
    auth: TenantAuth,
    range: web::Query<DateRange>,
) -> Result<HttpResponse, ApiError> {
    auth.require("reports.sales")?;
    let mut tx = auth.begin(&state).await?;
    let report = sales::by_product(&mut tx, &range, auth.access.can("products.view_cost")).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(report))
}

#[utoipa::path(get, path = "/api/v1/reports/sales-by-category", tag = "reports", params(DateRange), security(("bearer" = [])),
    responses((status = 200, body = Vec<CategorySalesRow>)))]
#[get("/reports/sales-by-category")]
pub async fn sales_by_category(
    state: web::Data<AppState>,
    auth: TenantAuth,
    range: web::Query<DateRange>,
) -> Result<HttpResponse, ApiError> {
    auth.require("reports.sales")?;
    let mut tx = auth.begin(&state).await?;
    let report = sales::by_category(&mut tx, &range, auth.access.can("products.view_cost")).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(report))
}

/// Invoices, totals, discounts and cancellations per cashier (R-05).
#[utoipa::path(get, path = "/api/v1/reports/sales-by-cashier", tag = "reports", params(DateRange), security(("bearer" = [])),
    responses((status = 200, body = Vec<CashierSalesRow>)))]
#[get("/reports/sales-by-cashier")]
pub async fn sales_by_cashier(
    state: web::Data<AppState>,
    auth: TenantAuth,
    range: web::Query<DateRange>,
) -> Result<HttpResponse, ApiError> {
    auth.require("reports.sales")?;
    let mut tx = auth.begin(&state).await?;
    let report = sales::by_cashier(&mut tx, &range).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(report))
}

/// Money in and out by method and account (R-17).
#[utoipa::path(get, path = "/api/v1/reports/payments", tag = "reports", params(DateRange), security(("bearer" = [])),
    responses((status = 200, body = PaymentSummary)))]
#[get("/reports/payments")]
pub async fn payment_summary(
    state: web::Data<AppState>,
    auth: TenantAuth,
    range: web::Query<DateRange>,
) -> Result<HttpResponse, ApiError> {
    auth.require("reports.financial")?;
    let mut tx = auth.begin(&state).await?;
    let report = sales::payment_summary(&mut tx, &range).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(report))
}

/// Purchases by supplier and by product (R-07, R-08).
#[utoipa::path(get, path = "/api/v1/reports/purchases", tag = "reports", params(DateRange), security(("bearer" = [])),
    responses((status = 200, body = PurchaseReport)))]
#[get("/reports/purchases")]
pub async fn purchase_report(
    state: web::Data<AppState>,
    auth: TenantAuth,
    range: web::Query<DateRange>,
) -> Result<HttpResponse, ApiError> {
    auth.require("reports.purchases")?;
    let mut tx = auth.begin(&state).await?;
    let report = purchases::report(&mut tx, &range).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(report))
}

/// Expenses by category and month (R-19).
#[utoipa::path(get, path = "/api/v1/reports/expenses", tag = "reports", params(DateRange), security(("bearer" = [])),
    responses((status = 200, body = ExpenseSummary)))]
#[get("/reports/expenses")]
pub async fn expense_summary(
    state: web::Data<AppState>,
    auth: TenantAuth,
    range: web::Query<DateRange>,
) -> Result<HttpResponse, ApiError> {
    if !auth.access.can("reports.financial") {
        auth.require("expenses.view")?;
    }
    let mut tx = auth.begin(&state).await?;
    let report = expenses::summary(&mut tx, &range).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(report))
}

/// Net sales − COGS − expenses (R-20).
#[utoipa::path(get, path = "/api/v1/reports/profit", tag = "reports", params(DateRange), security(("bearer" = [])),
    responses((status = 200, body = ProfitEstimate)))]
#[get("/reports/profit")]
pub async fn profit_estimate(
    state: web::Data<AppState>,
    auth: TenantAuth,
    range: web::Query<DateRange>,
) -> Result<HttpResponse, ApiError> {
    auth.require("reports.financial")?;
    let mut tx = auth.begin(&state).await?;
    let report = dashboard::profit_estimate(&mut tx, &range).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(report))
}

/// Customer or supplier dues by age: 0–30, 31–60, 61–90 and over 90 days.
#[utoipa::path(get, path = "/api/v1/reports/aging", tag = "reports", params(AgingQuery), security(("bearer" = [])),
    responses((status = 200, body = AgingReport)))]
#[get("/reports/aging")]
pub async fn aging_report(
    state: web::Data<AppState>,
    auth: TenantAuth,
    query: web::Query<AgingQuery>,
) -> Result<HttpResponse, ApiError> {
    auth.require("reports.parties")?;
    let mut tx = auth.begin(&state).await?;
    let report = analysis::aging(&mut tx, &query).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(report))
}

/// Output VAT (sales less returns) and input VAT (purchases less returns,
/// expenses) by rate, with the net payable.
#[utoipa::path(get, path = "/api/v1/reports/vat-summary", tag = "reports", params(DateRange), security(("bearer" = [])),
    responses((status = 200, body = VatSummary)))]
#[get("/reports/vat-summary")]
pub async fn vat_summary(
    state: web::Data<AppState>,
    auth: TenantAuth,
    range: web::Query<DateRange>,
) -> Result<HttpResponse, ApiError> {
    auth.require("reports.financial")?;
    let mut tx = auth.begin(&state).await?;
    let report = analysis::vat_summary(&mut tx, &range).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(report))
}

/// Discounts given, by cashier, with the largest discounted invoices.
#[utoipa::path(get, path = "/api/v1/reports/discounts", tag = "reports", params(DateRange), security(("bearer" = [])),
    responses((status = 200, body = DiscountReport)))]
#[get("/reports/discounts")]
pub async fn discount_report(
    state: web::Data<AppState>,
    auth: TenantAuth,
    range: web::Query<DateRange>,
) -> Result<HttpResponse, ApiError> {
    auth.require("reports.sales")?;
    let mut tx = auth.begin(&state).await?;
    let report = analysis::discounts(&mut tx, &range).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(report))
}

/// Fast-, slow- and non-moving products over a period (default 30 days).
#[utoipa::path(get, path = "/api/v1/reports/product-movement", tag = "reports", params(MovementQuery), security(("bearer" = [])),
    responses((status = 200, body = ProductMovement)))]
#[get("/reports/product-movement")]
pub async fn product_movement(
    state: web::Data<AppState>,
    auth: TenantAuth,
    query: web::Query<MovementQuery>,
) -> Result<HttpResponse, ApiError> {
    auth.require("reports.inventory")?;
    let mut tx = auth.begin(&state).await?;
    let report = analysis::product_movement(&mut tx, &query, auth.access.can("products.view_cost")).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(report))
}
