//! Money (Phase 4): payment accounts and the cash/bank book, payment methods,
//! payments (list, detail, void), account transfers, expenses, other income
//! and bank reconciliation.

use actix_web::{HttpRequest, HttpResponse, get, http::StatusCode, patch, post, put, web};
use pos_core::AppError;
use pos_money::{
    accounts::{self, Book, BookQuery, PaymentAccount, PaymentAccountInput},
    expenses::{self, Expense, ExpenseCategory, ExpenseCategoryInput, ExpenseFilter, ExpenseInput, ExpensePage},
    incomes::{self, Income, IncomeCategory, IncomeCategoryInput, IncomeFilter, IncomeInput, IncomePage},
    methods::{self, PaymentMethod, PaymentMethodPatch},
    payments::{self, PaymentDetail, PaymentFilter, PaymentPage},
    reconciliation::{
        self, ClearInput, Reconciliation, ReconciliationFilter, ReconciliationInput, ReconciliationSummary,
    },
    transfers::{self, Transfer, TransferFilter, TransferInput, TransferPage},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    ApiError, AppState,
    auth::{TenantAuth, etag, if_match_version},
    posting::{self, Posting, idempotency_key},
    routes::sales::ReasonInput,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list_accounts)
        .service(create_account)
        .service(get_account)
        .service(update_account)
        .service(account_book)
        .service(list_methods)
        .service(update_method)
        .service(list_payments)
        .service(get_payment)
        .service(void_payment)
        .service(list_transfers)
        .service(create_transfer)
        .service(list_expense_categories)
        .service(create_expense_category)
        .service(update_expense_category)
        .service(list_expenses)
        .service(create_expense)
        .service(get_expense)
        .service(void_expense)
        .service(list_income_categories)
        .service(create_income_category)
        .service(update_income_category)
        .service(list_incomes)
        .service(create_income)
        .service(get_income)
        .service(void_income)
        .service(list_reconciliations)
        .service(start_reconciliation)
        .service(get_reconciliation)
        .service(clear_reconciliation_lines)
        .service(complete_reconciliation)
        .service(cancel_reconciliation)
        .service(reopen_reconciliation);
}

#[derive(Debug, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct InactiveQuery {
    /// Include inactive entries (default false).
    pub include_inactive: Option<bool>,
}

// ---------------------------------------------------------------- accounts

/// Cash, bank, mobile wallet and card accounts with their balances.
#[utoipa::path(get, path = "/api/v1/payment-accounts", tag = "money", params(InactiveQuery), security(("bearer" = [])),
    responses((status = 200, body = Vec<PaymentAccount>)))]
#[get("/payment-accounts")]
pub async fn list_accounts(
    state: web::Data<AppState>,
    auth: TenantAuth,
    query: web::Query<InactiveQuery>,
) -> Result<HttpResponse, ApiError> {
    let money_roles =
        ["accounts.view", "expenses.create", "suppliers.pay", "customers.collect_payment", "transfers.create"];
    if !money_roles.iter().any(|p| auth.access.can(p)) {
        auth.require("accounts.view")?;
    }
    let mut tx = auth.begin(&state).await?;
    let mut list = accounts::list(&mut tx, query.include_inactive.unwrap_or(false)).await?;
    tx.commit().await?;
    if !auth.access.can("accounts.view") {
        // Choosing an account does not need its balance.
        for account in &mut list {
            account.current_balance = rust_decimal::Decimal::ZERO;
            account.opening_balance = rust_decimal::Decimal::ZERO;
        }
    }
    Ok(HttpResponse::Ok().json(list))
}

#[utoipa::path(post, path = "/api/v1/payment-accounts", tag = "money", request_body = PaymentAccountInput, security(("bearer" = [])),
    responses((status = 201, body = PaymentAccount), (status = 409, body = crate::error::Problem)))]
#[post("/payment-accounts")]
pub async fn create_account(
    state: web::Data<AppState>,
    auth: TenantAuth,
    body: web::Json<PaymentAccountInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("accounts.manage")?;
    let mut tx = auth.begin(&state).await?;
    let account = accounts::create(&mut tx, &auth.ctx.meta, body.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Created().insert_header(etag(account.version)).json(account))
}

#[utoipa::path(get, path = "/api/v1/payment-accounts/{id}", tag = "money", params(("id" = Uuid, Path)), security(("bearer" = [])),
    responses((status = 200, body = PaymentAccount), (status = 404, body = crate::error::Problem)))]
#[get("/payment-accounts/{id}")]
pub async fn get_account(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("accounts.view")?;
    let mut tx = auth.begin(&state).await?;
    let account = accounts::get(&mut tx, path.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().insert_header(etag(account.version)).json(account))
}

#[utoipa::path(put, path = "/api/v1/payment-accounts/{id}", tag = "money", request_body = PaymentAccountInput,
    params(("id" = Uuid, Path), ("If-Match" = String, Header)), security(("bearer" = [])),
    responses((status = 200, body = PaymentAccount), (status = 412, body = crate::error::Problem)))]
#[put("/payment-accounts/{id}")]
pub async fn update_account(
    state: web::Data<AppState>,
    req: HttpRequest,
    auth: TenantAuth,
    path: web::Path<Uuid>,
    body: web::Json<PaymentAccountInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("accounts.manage")?;
    let version = if_match_version(&req)?;
    let mut tx = auth.begin(&state).await?;
    let account = accounts::update(&mut tx, &auth.ctx.meta, path.into_inner(), version, body.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().insert_header(etag(account.version)).json(account))
}

/// Cash book / bank book with the running balance (PAY-R05, R-18).
#[utoipa::path(get, path = "/api/v1/payment-accounts/{id}/book", tag = "money", params(("id" = Uuid, Path), BookQuery),
    security(("bearer" = [])), responses((status = 200, body = Book)))]
#[get("/payment-accounts/{id}/book")]
pub async fn account_book(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
    query: web::Query<BookQuery>,
) -> Result<HttpResponse, ApiError> {
    auth.require("accounts.view")?;
    let mut tx = auth.begin(&state).await?;
    let book = accounts::book(&mut tx, path.into_inner(), &query).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(book))
}

// ----------------------------------------------------------------- methods

/// Payment methods (every member: the POS and payment dialogs need them).
#[utoipa::path(get, path = "/api/v1/payment-methods", tag = "money", params(InactiveQuery), security(("bearer" = [])),
    responses((status = 200, body = Vec<PaymentMethod>)))]
#[get("/payment-methods")]
pub async fn list_methods(
    state: web::Data<AppState>,
    auth: TenantAuth,
    query: web::Query<InactiveQuery>,
) -> Result<HttpResponse, ApiError> {
    let mut tx = auth.begin(&state).await?;
    let list = methods::list(&mut tx, query.include_inactive.unwrap_or(false)).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(list))
}

#[utoipa::path(patch, path = "/api/v1/payment-methods/{id}", tag = "money", request_body = PaymentMethodPatch,
    params(("id" = Uuid, Path)), security(("bearer" = [])), responses((status = 200, body = PaymentMethod)))]
#[patch("/payment-methods/{id}")]
pub async fn update_method(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
    body: web::Json<PaymentMethodPatch>,
) -> Result<HttpResponse, ApiError> {
    auth.require("accounts.manage")?;
    let mut tx = auth.begin(&state).await?;
    let method = methods::update(&mut tx, &auth.ctx.meta, path.into_inner(), body.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(method))
}

// ---------------------------------------------------------------- payments

#[utoipa::path(get, path = "/api/v1/payments", tag = "money", params(PaymentFilter), security(("bearer" = [])),
    responses((status = 200, body = PaymentPage)))]
#[get("/payments")]
pub async fn list_payments(
    state: web::Data<AppState>,
    auth: TenantAuth,
    filter: web::Query<PaymentFilter>,
) -> Result<HttpResponse, ApiError> {
    let party = filter.party_type.as_deref();
    let allowed = auth.access.can("accounts.view")
        || (party == Some("customer") && auth.access.can("customers.view"))
        || (party == Some("supplier") && auth.access.can("suppliers.view"));
    if !allowed {
        auth.require("accounts.view")?;
    }
    let mut tx = auth.begin(&state).await?;
    let page = payments::list(&mut tx, &filter).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(page))
}

#[utoipa::path(get, path = "/api/v1/payments/{id}", tag = "money", params(("id" = Uuid, Path)), security(("bearer" = [])),
    responses((status = 200, body = PaymentDetail), (status = 404, body = crate::error::Problem)))]
#[get("/payments/{id}")]
pub async fn get_payment(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let mut tx = auth.begin(&state).await?;
    let payment = payments::get(&mut tx, path.into_inner()).await?;
    tx.commit().await?;
    let allowed = auth.access.can("accounts.view")
        || (payment.payment.party_type.as_deref() == Some("customer") && auth.access.can("customers.view"))
        || (payment.payment.party_type.as_deref() == Some("supplier") && auth.access.can("suppliers.view"));
    if !allowed {
        auth.require("accounts.view")?;
    }
    Ok(HttpResponse::Ok().json(payment))
}

/// Voids a customer collection or supplier payment with reversing entries
/// (PAY-R04). POS, refund and expense payments go with their document.
#[utoipa::path(post, path = "/api/v1/payments/{id}/void", tag = "money", request_body = ReasonInput, params(("id" = Uuid, Path)),
    security(("bearer" = [])), responses((status = 200, body = PaymentDetail), (status = 422, body = crate::error::Problem)))]
#[post("/payments/{id}/void")]
pub async fn void_payment(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
    body: web::Json<ReasonInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("payments.void")?;
    let id = path.into_inner();
    let mut attempt = 0;
    loop {
        attempt += 1;
        let mut tx = auth.begin(&state).await?;
        let source: Option<String> = sqlx::query_scalar("SELECT source FROM payments WHERE tenant_id = ? AND id = ?")
            .bind(tx.tenant_id())
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(pos_db::map_db_error)?;
        let result = match source.as_deref() {
            None => Err(AppError::NotFound),
            Some("collection") => pos_sales::collections::void(&mut tx, &auth.ctx.meta, id, &body.reason).await,
            Some("supplier_payment") => pos_purchasing::payments::void(&mut tx, &auth.ctx.meta, id, &body.reason).await,
            Some(_) => Err(AppError::Unprocessable {
                code: "VOID_SOURCE_DOCUMENT",
                message: "this payment belongs to a sale, return or expense; cancel that document instead".into(),
            }),
        };
        match result {
            Ok(payment) => {
                tx.commit().await?;
                return Ok(HttpResponse::Ok().json(payment));
            }
            Err(AppError::Retryable(_)) if attempt < posting::MAX_ATTEMPTS => continue,
            Err(err) => return Err(err.into()),
        }
    }
}

// --------------------------------------------------------------- transfers

#[utoipa::path(get, path = "/api/v1/account-transfers", tag = "money", params(TransferFilter), security(("bearer" = [])),
    responses((status = 200, body = TransferPage)))]
#[get("/account-transfers")]
pub async fn list_transfers(
    state: web::Data<AppState>,
    auth: TenantAuth,
    filter: web::Query<TransferFilter>,
) -> Result<HttpResponse, ApiError> {
    auth.require("accounts.view")?;
    let mut tx = auth.begin(&state).await?;
    let page = transfers::list(&mut tx, &filter).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(page))
}

/// Moves money between accounts (PAY-R03), with an optional fee.
#[utoipa::path(post, path = "/api/v1/account-transfers", tag = "money", request_body = TransferInput, security(("bearer" = [])),
    params(("Idempotency-Key" = Option<String>, Header)), responses((status = 201, body = Transfer)))]
#[post("/account-transfers")]
pub async fn create_transfer(
    state: web::Data<AppState>,
    req: HttpRequest,
    auth: TenantAuth,
    body: web::Json<TransferInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("transfers.create")?;
    let key = idempotency_key(&req, false)?;
    let input = body.into_inner();
    let request = serde_json::json!({
        "from": input.from_account_id, "to": input.to_account_id, "amount": input.amount.to_string(),
        "fee": input.fee_amount.map(|f| f.to_string()), "reference": input.reference, "note": input.note,
    });
    let meta = &auth.ctx.meta;
    let p = Posting {
        key,
        endpoint: "POST /account-transfers",
        resource: "account_transfer",
        status: StatusCode::CREATED,
        request: &request,
    };
    posting::run(&state, &auth, p, |mut tx| {
        let input = input.clone();
        Box::pin(async move {
            let result = transfers::create(&mut tx, meta, input).await.map(|t| (t.id, t));
            (tx, result)
        })
    })
    .await
}

// ---------------------------------------------------------------- expenses

#[utoipa::path(get, path = "/api/v1/expense-categories", tag = "money", security(("bearer" = [])),
    responses((status = 200, body = Vec<ExpenseCategory>)))]
#[get("/expense-categories")]
pub async fn list_expense_categories(state: web::Data<AppState>, auth: TenantAuth) -> Result<HttpResponse, ApiError> {
    if !auth.access.can("expenses.create") {
        auth.require("expenses.view")?;
    }
    let mut tx = auth.begin(&state).await?;
    let list = expenses::list_categories(&mut tx).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(list))
}

/// Adds a category with its own ledger account under Operating Expenses.
#[utoipa::path(post, path = "/api/v1/expense-categories", tag = "money", request_body = ExpenseCategoryInput,
    security(("bearer" = [])), responses((status = 201, body = ExpenseCategory), (status = 409, body = crate::error::Problem)))]
#[post("/expense-categories")]
pub async fn create_expense_category(
    state: web::Data<AppState>,
    auth: TenantAuth,
    body: web::Json<ExpenseCategoryInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("accounts.manage")?;
    let mut tx = auth.begin(&state).await?;
    let category = expenses::create_category(&mut tx, &auth.ctx.meta, body.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Created().json(category))
}

#[utoipa::path(put, path = "/api/v1/expense-categories/{id}", tag = "money", request_body = ExpenseCategoryInput,
    params(("id" = Uuid, Path)), security(("bearer" = [])), responses((status = 200, body = ExpenseCategory)))]
#[put("/expense-categories/{id}")]
pub async fn update_expense_category(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
    body: web::Json<ExpenseCategoryInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("accounts.manage")?;
    let mut tx = auth.begin(&state).await?;
    let category = expenses::update_category(&mut tx, &auth.ctx.meta, path.into_inner(), body.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(category))
}

#[utoipa::path(get, path = "/api/v1/expenses", tag = "money", params(ExpenseFilter), security(("bearer" = [])),
    responses((status = 200, body = ExpensePage)))]
#[get("/expenses")]
pub async fn list_expenses(
    state: web::Data<AppState>,
    auth: TenantAuth,
    filter: web::Query<ExpenseFilter>,
) -> Result<HttpResponse, ApiError> {
    auth.require("expenses.view")?;
    let mut tx = auth.begin(&state).await?;
    let page = expenses::list(&mut tx, &filter).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(page))
}

/// Records an expense paid from an account (EXP-R01).
#[utoipa::path(post, path = "/api/v1/expenses", tag = "money", request_body = ExpenseInput, security(("bearer" = [])),
    params(("Idempotency-Key" = Option<String>, Header)), responses((status = 201, body = Expense)))]
#[post("/expenses")]
pub async fn create_expense(
    state: web::Data<AppState>,
    req: HttpRequest,
    auth: TenantAuth,
    body: web::Json<ExpenseInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("expenses.create")?;
    let key = idempotency_key(&req, false)?;
    let input = body.into_inner();
    let request = serde_json::json!({
        "category_id": input.category_id, "amount": input.amount.to_string(),
        "tax_amount": input.tax_amount.map(|t| t.to_string()), "account_id": input.account_id,
        "business_date": input.business_date, "payee": input.payee, "reference": input.reference, "note": input.note,
    });
    let meta = &auth.ctx.meta;
    let p = Posting {
        key,
        endpoint: "POST /expenses",
        resource: "expense",
        status: StatusCode::CREATED,
        request: &request,
    };
    posting::run(&state, &auth, p, |mut tx| {
        let input = input.clone();
        Box::pin(async move {
            let result = expenses::create(&mut tx, meta, Some(key), input).await.map(|e| (e.id, e));
            (tx, result)
        })
    })
    .await
}

#[utoipa::path(get, path = "/api/v1/expenses/{id}", tag = "money", params(("id" = Uuid, Path)), security(("bearer" = [])),
    responses((status = 200, body = Expense), (status = 404, body = crate::error::Problem)))]
#[get("/expenses/{id}")]
pub async fn get_expense(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("expenses.view")?;
    let mut tx = auth.begin(&state).await?;
    let expense = expenses::get(&mut tx, path.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(expense))
}

/// Voids an expense: the money returns to the account and the journal is reversed.
#[utoipa::path(post, path = "/api/v1/expenses/{id}/void", tag = "money", request_body = ReasonInput, params(("id" = Uuid, Path)),
    security(("bearer" = [])), responses((status = 200, body = Expense), (status = 422, body = crate::error::Problem)))]
#[post("/expenses/{id}/void")]
pub async fn void_expense(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
    body: web::Json<ReasonInput>,
) -> Result<HttpResponse, ApiError> {
    if !auth.access.can("expenses.approve") {
        auth.require("payments.void")?;
    }
    let mut tx = auth.begin(&state).await?;
    let expense = expenses::void(&mut tx, &auth.ctx.meta, path.into_inner(), &body.reason).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(expense))
}

// ---------------------------------------------------------------- other income

fn require_income_view(auth: &TenantAuth) -> Result<(), ApiError> {
    if auth.access.can("income.create") { Ok(()) } else { auth.require("expenses.view") }
}

#[utoipa::path(get, path = "/api/v1/income-categories", tag = "money", security(("bearer" = [])),
    responses((status = 200, body = Vec<IncomeCategory>)))]
#[get("/income-categories")]
pub async fn list_income_categories(state: web::Data<AppState>, auth: TenantAuth) -> Result<HttpResponse, ApiError> {
    require_income_view(&auth)?;
    let mut tx = auth.begin(&state).await?;
    let list = incomes::list_categories(&mut tx).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(list))
}

#[utoipa::path(post, path = "/api/v1/income-categories", tag = "money", request_body = IncomeCategoryInput,
    security(("bearer" = [])), responses((status = 201, body = IncomeCategory), (status = 409, body = crate::error::Problem)))]
#[post("/income-categories")]
pub async fn create_income_category(
    state: web::Data<AppState>,
    auth: TenantAuth,
    body: web::Json<IncomeCategoryInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("accounts.manage")?;
    let mut tx = auth.begin(&state).await?;
    let category = incomes::create_category(&mut tx, &auth.ctx.meta, body.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Created().json(category))
}

#[utoipa::path(put, path = "/api/v1/income-categories/{id}", tag = "money", request_body = IncomeCategoryInput,
    params(("id" = Uuid, Path)), security(("bearer" = [])), responses((status = 200, body = IncomeCategory)))]
#[put("/income-categories/{id}")]
pub async fn update_income_category(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
    body: web::Json<IncomeCategoryInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("accounts.manage")?;
    let mut tx = auth.begin(&state).await?;
    let category = incomes::update_category(&mut tx, &auth.ctx.meta, path.into_inner(), body.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(category))
}

#[utoipa::path(get, path = "/api/v1/incomes", tag = "money", params(IncomeFilter), security(("bearer" = [])),
    responses((status = 200, body = IncomePage)))]
#[get("/incomes")]
pub async fn list_incomes(
    state: web::Data<AppState>,
    auth: TenantAuth,
    filter: web::Query<IncomeFilter>,
) -> Result<HttpResponse, ApiError> {
    require_income_view(&auth)?;
    let mut tx = auth.begin(&state).await?;
    let page = incomes::list(&mut tx, &filter).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(page))
}

/// Records other income received into an account.
#[utoipa::path(post, path = "/api/v1/incomes", tag = "money", request_body = IncomeInput, security(("bearer" = [])),
    params(("Idempotency-Key" = Option<String>, Header)), responses((status = 201, body = Income)))]
#[post("/incomes")]
pub async fn create_income(
    state: web::Data<AppState>,
    req: HttpRequest,
    auth: TenantAuth,
    body: web::Json<IncomeInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("income.create")?;
    let key = idempotency_key(&req, false)?;
    let input = body.into_inner();
    let request = serde_json::json!({
        "category_id": input.category_id, "amount": input.amount.to_string(), "account_id": input.account_id,
        "business_date": input.business_date, "source": input.source, "reference": input.reference, "note": input.note,
    });
    let meta = &auth.ctx.meta;
    let p = Posting {
        key,
        endpoint: "POST /incomes",
        resource: "other_income",
        status: StatusCode::CREATED,
        request: &request,
    };
    posting::run(&state, &auth, p, |mut tx| {
        let input = input.clone();
        Box::pin(async move {
            let result = incomes::create(&mut tx, meta, input).await.map(|i| (i.id, i));
            (tx, result)
        })
    })
    .await
}

#[utoipa::path(get, path = "/api/v1/incomes/{id}", tag = "money", params(("id" = Uuid, Path)), security(("bearer" = [])),
    responses((status = 200, body = Income), (status = 404, body = crate::error::Problem)))]
#[get("/incomes/{id}")]
pub async fn get_income(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    require_income_view(&auth)?;
    let mut tx = auth.begin(&state).await?;
    let income = incomes::get(&mut tx, path.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(income))
}

/// Voids an income: the money leaves the account again and the journal is reversed.
#[utoipa::path(post, path = "/api/v1/incomes/{id}/void", tag = "money", request_body = ReasonInput, params(("id" = Uuid, Path)),
    security(("bearer" = [])), responses((status = 200, body = Income), (status = 422, body = crate::error::Problem)))]
#[post("/incomes/{id}/void")]
pub async fn void_income(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
    body: web::Json<ReasonInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("payments.void")?;
    let mut tx = auth.begin(&state).await?;
    let income = incomes::void(&mut tx, &auth.ctx.meta, path.into_inner(), &body.reason).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(income))
}

// ------------------------------------------------------- bank reconciliation

#[utoipa::path(get, path = "/api/v1/bank-reconciliations", tag = "money", params(ReconciliationFilter),
    security(("bearer" = [])), responses((status = 200, body = Vec<ReconciliationSummary>)))]
#[get("/bank-reconciliations")]
pub async fn list_reconciliations(
    state: web::Data<AppState>,
    auth: TenantAuth,
    query: web::Query<ReconciliationFilter>,
) -> Result<HttpResponse, ApiError> {
    auth.require("accounts.view")?;
    let mut tx = auth.begin(&state).await?;
    let list = reconciliation::list(&mut tx, &query).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(list))
}

/// Starts reconciling a bank, wallet or card account against one statement (PAY-R06).
#[utoipa::path(post, path = "/api/v1/bank-reconciliations", tag = "money", request_body = ReconciliationInput,
    security(("bearer" = [])), responses((status = 201, body = Reconciliation), (status = 409, body = crate::error::Problem)))]
#[post("/bank-reconciliations")]
pub async fn start_reconciliation(
    state: web::Data<AppState>,
    auth: TenantAuth,
    body: web::Json<ReconciliationInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("accounts.manage")?;
    let mut tx = auth.begin(&state).await?;
    let rec = reconciliation::start(&mut tx, &auth.ctx.meta, &body).await?;
    tx.commit().await?;
    Ok(HttpResponse::Created().json(rec))
}

#[utoipa::path(get, path = "/api/v1/bank-reconciliations/{id}", tag = "money", params(("id" = Uuid, Path)),
    security(("bearer" = [])), responses((status = 200, body = Reconciliation), (status = 404, body = crate::error::Problem)))]
#[get("/bank-reconciliations/{id}")]
pub async fn get_reconciliation(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("accounts.view")?;
    let mut tx = auth.begin(&state).await?;
    let rec = reconciliation::get(&mut tx, path.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(rec))
}

/// Ticks book lines as on the statement, or unticks them.
#[utoipa::path(put, path = "/api/v1/bank-reconciliations/{id}/lines", tag = "money", request_body = ClearInput,
    params(("id" = Uuid, Path)), security(("bearer" = [])),
    responses((status = 200, body = Reconciliation), (status = 422, body = crate::error::Problem)))]
#[put("/bank-reconciliations/{id}/lines")]
pub async fn clear_reconciliation_lines(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
    body: web::Json<ClearInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("accounts.manage")?;
    let mut tx = auth.begin(&state).await?;
    let rec = reconciliation::set_cleared(&mut tx, path.into_inner(), &body).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(rec))
}

/// Completes a reconciliation whose cleared balance equals the statement balance.
#[utoipa::path(post, path = "/api/v1/bank-reconciliations/{id}/complete", tag = "money", params(("id" = Uuid, Path)),
    security(("bearer" = [])), responses((status = 200, body = Reconciliation), (status = 422, body = crate::error::Problem)))]
#[post("/bank-reconciliations/{id}/complete")]
pub async fn complete_reconciliation(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("accounts.manage")?;
    let mut tx = auth.begin(&state).await?;
    let rec = reconciliation::complete(&mut tx, &auth.ctx.meta, path.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(rec))
}

/// Cancels a reconciliation in progress.
#[utoipa::path(post, path = "/api/v1/bank-reconciliations/{id}/cancel", tag = "money", params(("id" = Uuid, Path)),
    security(("bearer" = [])), responses((status = 200, body = Reconciliation), (status = 422, body = crate::error::Problem)))]
#[post("/bank-reconciliations/{id}/cancel")]
pub async fn cancel_reconciliation(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("accounts.manage")?;
    let mut tx = auth.begin(&state).await?;
    let rec = reconciliation::cancel(&mut tx, &auth.ctx.meta, path.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(rec))
}

/// Reopens the account's latest completed reconciliation to correct it.
#[utoipa::path(post, path = "/api/v1/bank-reconciliations/{id}/reopen", tag = "money", request_body = ReasonInput,
    params(("id" = Uuid, Path)), security(("bearer" = [])),
    responses((status = 200, body = Reconciliation), (status = 422, body = crate::error::Problem)))]
#[post("/bank-reconciliations/{id}/reopen")]
pub async fn reopen_reconciliation(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
    body: web::Json<ReasonInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("accounts.manage")?;
    let mut tx = auth.begin(&state).await?;
    let rec = reconciliation::reopen(&mut tx, &auth.ctx.meta, path.into_inner(), &body.reason).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(rec))
}
