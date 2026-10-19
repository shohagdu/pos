//! `/api/v1/subscription`: plan, state, features and usage meters (SUB-R05).
//! Always reachable, also when read-only or suspended, so the owner can renew.

use std::io::Write;

use actix_web::{HttpResponse, get, http::header, post, web};
use pos_billing::{Feature, SubscriptionSummary};
use pos_core::AppError;
use pos_platform::billing::{self, BillingOverview, ClaimInput, Invoice, InvoiceRequest, PaymentClaim};
use pos_tenancy::entitlements::{self, UsageMeter};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{ApiError, AppState, auth::TenantAuth};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(get_subscription)
        .service(billing_overview)
        .service(request_invoice)
        .service(submit_claim)
        .service(export_data);
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SubscriptionOverview {
    pub subscription: SubscriptionSummary,
    pub features: Vec<Feature>,
    pub usage: Vec<UsageMeter>,
}

/// The business's plan, effective state, features and usage. Every member can
/// read it: staff need to know why actions are blocked.
#[utoipa::path(get, path = "/api/v1/subscription", tag = "billing", security(("bearer" = [])),
    responses((status = 200, body = SubscriptionOverview)))]
#[get("/subscription")]
pub async fn get_subscription(state: web::Data<AppState>, auth: TenantAuth) -> Result<HttpResponse, ApiError> {
    let mut tx = auth.begin(&state).await?;
    // Exact, uncached figures for the meter.
    let ents = entitlements::current(&mut tx).await?;
    let usage = entitlements::usage(&mut tx, &ents).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(SubscriptionOverview {
        subscription: ents.summary(),
        features: ents.features.values().cloned().collect(),
        usage,
    }))
}

/// Plans on sale, the business's invoices and payment claims, and how to pay
/// (SUB-R05). Owner only (`subscription.manage`).
#[utoipa::path(get, path = "/api/v1/subscription/billing", tag = "billing", security(("bearer" = [])),
    responses((status = 200, body = BillingOverview)))]
#[get("/subscription/billing")]
pub async fn billing_overview(state: web::Data<AppState>, auth: TenantAuth) -> Result<HttpResponse, ApiError> {
    auth.require("subscription.manage")?;
    let mut tx = auth.begin(&state).await?;
    let overview = billing::overview(&mut tx).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(overview))
}

/// Opens an invoice for a plan and billing cycle (or returns the open one).
#[utoipa::path(post, path = "/api/v1/subscription/invoices", tag = "billing", request_body = InvoiceRequest,
    security(("bearer" = [])), responses((status = 201, body = Invoice), (status = 400, body = crate::error::Problem)))]
#[post("/subscription/invoices")]
pub async fn request_invoice(
    state: web::Data<AppState>,
    auth: TenantAuth,
    body: web::Json<InvoiceRequest>,
) -> Result<HttpResponse, ApiError> {
    auth.require("subscription.manage")?;
    let mut tx = auth.begin(&state).await?;
    let invoice = billing::request_invoice(&mut tx, &auth.ctx.meta, &body).await?;
    tx.commit().await?;
    Ok(HttpResponse::Created().json(invoice))
}

/// Reports a manual payment (bKash/Nagad/bank) for verification by the platform (ADM-R05).
#[utoipa::path(post, path = "/api/v1/subscription/invoices/{id}/claims", tag = "billing", params(("id" = Uuid, Path)),
    request_body = ClaimInput, security(("bearer" = [])),
    responses((status = 201, body = PaymentClaim), (status = 409, body = crate::error::Problem)))]
#[post("/subscription/invoices/{id}/claims")]
pub async fn submit_claim(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
    body: web::Json<ClaimInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("subscription.manage")?;
    let mut tx = auth.begin(&state).await?;
    let claim = billing::submit_claim(&mut tx, &auth.ctx.meta, path.into_inner(), &body).await?;
    tx.commit().await?;
    Ok(HttpResponse::Created().json(claim))
}

/// The owner's full data export: a ZIP of CSV files, one per table
/// (NFR-PRIV-01). Also available while read-only or suspended.
#[utoipa::path(get, path = "/api/v1/subscription/export", tag = "billing", security(("bearer" = [])),
    responses((status = 200, content_type = "application/zip", description = "ZIP of CSV files")))]
#[get("/subscription/export")]
pub async fn export_data(state: web::Data<AppState>, auth: TenantAuth) -> Result<HttpResponse, ApiError> {
    auth.require("subscription.manage")?;
    let mut tx = auth.begin(&state).await?;
    let files = pos_platform::export::build(&mut tx, &auth.ctx.meta).await?;
    tx.commit().await?;
    let mut buffer = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buffer);
        let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for file in &files {
            zip.start_file(file.name.as_str(), options).map_err(AppError::internal)?;
            zip.write_all(file.content.as_bytes()).map_err(AppError::internal)?;
        }
        zip.finish().map_err(AppError::internal)?;
    }
    let date = chrono::Utc::now().format("%Y%m%d");
    Ok(HttpResponse::Ok()
        .content_type("application/zip")
        .insert_header((header::CONTENT_DISPOSITION, format!("attachment; filename=\"pos-export-{date}.zip\"")))
        .body(buffer.into_inner()))
}
