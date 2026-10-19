//! Accounting (ACC-R01…R06): chart of accounts, journals, manual journals,
//! fiscal-period locks and the financial statements (R-25).

use actix_web::{HttpResponse, get, post, web};
use pos_accounting::{
    journal::{self, JournalDetail, JournalFilter, JournalPage, ManualJournalInput},
    ledger::{self, LedgerAccount},
    periods::{self, FiscalPeriod, PeriodInput},
    statements::{self, AsOfQuery, BalanceSheet, GeneralLedger, LedgerQuery, PeriodQuery, ProfitAndLoss, TrialBalance},
};
use uuid::Uuid;

use crate::{ApiError, AppState, auth::TenantAuth};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list_ledger_accounts)
        .service(list_journals)
        .service(create_journal)
        .service(get_journal)
        .service(reverse_journal)
        .service(list_periods)
        .service(lock_period)
        .service(reopen_period)
        .service(trial_balance)
        .service(profit_and_loss)
        .service(balance_sheet)
        .service(general_ledger);
}

/// Statements are for `accounting.view` or `reports.financial`.
fn require_statements(auth: &TenantAuth) -> Result<(), ApiError> {
    if auth.access.can("reports.financial") {
        return Ok(());
    }
    auth.require("accounting.view")
}

#[utoipa::path(get, path = "/api/v1/ledger-accounts", tag = "accounting", security(("bearer" = [])),
    responses((status = 200, body = Vec<LedgerAccount>)))]
#[get("/ledger-accounts")]
pub async fn list_ledger_accounts(state: web::Data<AppState>, auth: TenantAuth) -> Result<HttpResponse, ApiError> {
    require_statements(&auth)?;
    let mut tx = auth.begin(&state).await?;
    let list = ledger::list(&mut tx).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(list))
}

#[utoipa::path(get, path = "/api/v1/journals", tag = "accounting", params(JournalFilter), security(("bearer" = [])),
    responses((status = 200, body = JournalPage)))]
#[get("/journals")]
pub async fn list_journals(
    state: web::Data<AppState>,
    auth: TenantAuth,
    filter: web::Query<JournalFilter>,
) -> Result<HttpResponse, ApiError> {
    auth.require("accounting.view")?;
    let mut tx = auth.begin(&state).await?;
    let page = journal::list(&mut tx, &filter).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(page))
}

/// A balanced manual journal (ACC-R04).
#[utoipa::path(post, path = "/api/v1/journals", tag = "accounting", request_body = ManualJournalInput, security(("bearer" = [])),
    responses((status = 201, body = JournalDetail), (status = 422, body = crate::error::Problem),
              (status = 423, body = crate::error::Problem)))]
#[post("/journals")]
pub async fn create_journal(
    state: web::Data<AppState>,
    auth: TenantAuth,
    body: web::Json<ManualJournalInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("accounting.journal_create")?;
    let mut tx = auth.begin(&state).await?;
    let detail = journal::create_manual(&mut tx, &auth.ctx.meta, body.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Created().json(detail))
}

#[utoipa::path(get, path = "/api/v1/journals/{id}", tag = "accounting", params(("id" = Uuid, Path)), security(("bearer" = [])),
    responses((status = 200, body = JournalDetail), (status = 404, body = crate::error::Problem)))]
#[get("/journals/{id}")]
pub async fn get_journal(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("accounting.view")?;
    let mut tx = auth.begin(&state).await?;
    let detail = journal::get(&mut tx, path.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(detail))
}

/// Reverses a manual journal; returns the reversal.
#[utoipa::path(post, path = "/api/v1/journals/{id}/reverse", tag = "accounting", params(("id" = Uuid, Path)), security(("bearer" = [])),
    responses((status = 200, body = JournalDetail), (status = 422, body = crate::error::Problem)))]
#[post("/journals/{id}/reverse")]
pub async fn reverse_journal(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("accounting.journal_create")?;
    let mut tx = auth.begin(&state).await?;
    let detail = journal::reverse_manual(&mut tx, &auth.ctx.meta, path.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(detail))
}

#[utoipa::path(get, path = "/api/v1/fiscal-periods", tag = "accounting", security(("bearer" = [])),
    responses((status = 200, body = Vec<FiscalPeriod>)))]
#[get("/fiscal-periods")]
pub async fn list_periods(state: web::Data<AppState>, auth: TenantAuth) -> Result<HttpResponse, ApiError> {
    auth.require("accounting.view")?;
    let mut tx = auth.begin(&state).await?;
    let list = periods::list(&mut tx).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(list))
}

/// Locks a month that has ended (ACC-R05).
#[utoipa::path(post, path = "/api/v1/fiscal-periods/lock", tag = "accounting", request_body = PeriodInput, security(("bearer" = [])),
    responses((status = 200, body = FiscalPeriod), (status = 422, body = crate::error::Problem)))]
#[post("/fiscal-periods/lock")]
pub async fn lock_period(
    state: web::Data<AppState>,
    auth: TenantAuth,
    body: web::Json<PeriodInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("accounting.period_lock")?;
    let mut tx = auth.begin(&state).await?;
    let period = periods::set_locked(&mut tx, &auth.ctx.meta, body.into_inner(), true).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(period))
}

#[utoipa::path(post, path = "/api/v1/fiscal-periods/reopen", tag = "accounting", request_body = PeriodInput, security(("bearer" = [])),
    responses((status = 200, body = FiscalPeriod)))]
#[post("/fiscal-periods/reopen")]
pub async fn reopen_period(
    state: web::Data<AppState>,
    auth: TenantAuth,
    body: web::Json<PeriodInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("accounting.period_lock")?;
    let mut tx = auth.begin(&state).await?;
    let period = periods::set_locked(&mut tx, &auth.ctx.meta, body.into_inner(), false).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(period))
}

/// Trial balance for a period: opening, movements and closing per account.
#[utoipa::path(get, path = "/api/v1/reports/trial-balance", tag = "accounting", params(PeriodQuery), security(("bearer" = [])),
    responses((status = 200, body = TrialBalance)))]
#[get("/reports/trial-balance")]
pub async fn trial_balance(
    state: web::Data<AppState>,
    auth: TenantAuth,
    query: web::Query<PeriodQuery>,
) -> Result<HttpResponse, ApiError> {
    require_statements(&auth)?;
    let mut tx = auth.begin(&state).await?;
    let report = statements::trial_balance(&mut tx, &query).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(report))
}

/// Profit and loss account for a period.
#[utoipa::path(get, path = "/api/v1/reports/profit-and-loss", tag = "accounting", params(PeriodQuery), security(("bearer" = [])),
    responses((status = 200, body = ProfitAndLoss)))]
#[get("/reports/profit-and-loss")]
pub async fn profit_and_loss(
    state: web::Data<AppState>,
    auth: TenantAuth,
    query: web::Query<PeriodQuery>,
) -> Result<HttpResponse, ApiError> {
    require_statements(&auth)?;
    let mut tx = auth.begin(&state).await?;
    let report = statements::profit_and_loss(&mut tx, &query).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(report))
}

/// Balance sheet as of a date.
#[utoipa::path(get, path = "/api/v1/reports/balance-sheet", tag = "accounting", params(AsOfQuery), security(("bearer" = [])),
    responses((status = 200, body = BalanceSheet)))]
#[get("/reports/balance-sheet")]
pub async fn balance_sheet(
    state: web::Data<AppState>,
    auth: TenantAuth,
    query: web::Query<AsOfQuery>,
) -> Result<HttpResponse, ApiError> {
    require_statements(&auth)?;
    let mut tx = auth.begin(&state).await?;
    let report = statements::balance_sheet(&mut tx, &query).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(report))
}

/// Ledger of one account with the running balance.
#[utoipa::path(get, path = "/api/v1/reports/general-ledger", tag = "accounting", params(LedgerQuery), security(("bearer" = [])),
    responses((status = 200, body = GeneralLedger)))]
#[get("/reports/general-ledger")]
pub async fn general_ledger(
    state: web::Data<AppState>,
    auth: TenantAuth,
    query: web::Query<LedgerQuery>,
) -> Result<HttpResponse, ApiError> {
    require_statements(&auth)?;
    let mut tx = auth.begin(&state).await?;
    let report = statements::general_ledger(&mut tx, &query).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(report))
}
