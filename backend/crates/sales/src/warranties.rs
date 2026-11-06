//! Items sold with a warranty, one row per sale line: for checking a claim
//! (is it still covered?) and for the customer's warranty history.
//! Cancelled sales are left out; returned lines are kept with their
//! returned quantity.

use chrono::{DateTime, NaiveDate, Utc};
use pos_core::{
    AppError, AppResult,
    page::{self, Keyset},
};
use pos_db::{TenantTx, map_db_error};
use pos_tenancy::context;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

use crate::sales::Seller;

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct WarrantyFilter {
    pub customer_id: Option<Uuid>,
    /// `active` (runs out today or later) or `expired`.
    pub status: Option<String>,
    /// Invoice number, product name, SKU or customer phone containing this
    /// text, or the product's exact barcode (scan the item being claimed).
    pub q: Option<String>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct WarrantyItem {
    /// The sale line.
    pub id: Uuid,
    pub sale_id: Uuid,
    pub invoice_no: String,
    /// The day the warranty started (the sale's business date).
    pub business_date: NaiveDate,
    pub customer_id: Uuid,
    pub customer_name: String,
    pub customer_phone: Option<String>,
    pub is_walk_in: bool,
    pub variant_id: Uuid,
    pub product_name: String,
    pub sku: String,
    pub unit_short_name: String,
    #[schema(value_type = String)]
    pub quantity: Decimal,
    #[schema(value_type = String)]
    pub returned_quantity: Decimal,
    pub warranty_period: i16,
    /// `day`, `month` or `year`.
    pub warranty_unit: String,
    pub warranty_expires_on: NaiveDate,
    #[serde(skip)]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct WarrantyPage {
    pub data: Vec<WarrantyItem>,
    pub next_cursor: Option<String>,
    /// Today in the business's timezone: a warranty is active while
    /// `warranty_expires_on >= today`.
    pub today: NaiveDate,
}

pub async fn list(tx: &mut TenantTx, filter: &WarrantyFilter, seller: Seller) -> AppResult<WarrantyPage> {
    let limit = page::limit(filter.limit)?;
    let after = filter.cursor.as_deref().map(Keyset::decode).transpose()?;
    let today = context::today(tx).await?;
    let tenant_id = tx.tenant_id();

    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "SELECT i.id, s.id AS sale_id, s.invoice_no, s.business_date, s.customer_id, c.name AS customer_name, \
         c.phone AS customer_phone, c.is_walk_in, i.variant_id, i.product_name, i.sku, i.unit_short_name, i.quantity, \
         i.returned_quantity, i.warranty_period, i.warranty_unit, i.warranty_expires_on, s.created_at \
         FROM sale_items i \
         JOIN sales s ON s.tenant_id = i.tenant_id AND s.id = i.sale_id \
         JOIN customers c ON c.tenant_id = s.tenant_id AND c.id = s.customer_id \
         WHERE i.warranty_expires_on IS NOT NULL AND s.status <> 'cancelled' AND i.tenant_id = ",
    );
    q.push_bind(tenant_id);
    if let Some(branch_id) = tx.branch_id() {
        q.push(" AND s.branch_id = ").push_bind(branch_id);
    }
    if !seller.can_view_all {
        q.push(" AND s.cashier_id = ").push_bind(tx.user_id());
    }
    if let Some(customer_id) = filter.customer_id {
        q.push(" AND s.customer_id = ").push_bind(customer_id);
    }
    match filter.status.as_deref().unwrap_or_default() {
        "" => {}
        "active" => {
            q.push(" AND i.warranty_expires_on >= ").push_bind(today);
        }
        "expired" => {
            q.push(" AND i.warranty_expires_on < ").push_bind(today);
        }
        _ => return Err(AppError::field("status", "INVALID_STATUS", "use active or expired")),
    }
    if let Some(text) = filter.q.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        if text.chars().count() > 100 {
            return Err(AppError::field("q", "TOO_LONG", "search for at most 100 characters"));
        }
        let escaped = text.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
        let pattern = format!("%{escaped}%");
        q.push(" AND (s.invoice_no LIKE ")
            .push_bind(pattern.clone())
            .push(" OR i.product_name LIKE ")
            .push_bind(pattern.clone())
            .push(" OR i.sku LIKE ")
            .push_bind(pattern.clone())
            .push(" OR c.phone LIKE ")
            .push_bind(pattern)
            .push(" OR EXISTS (SELECT 1 FROM product_barcodes b WHERE b.tenant_id = i.tenant_id AND b.variant_id = i.variant_id \
                 AND b.barcode = ")
            .push_bind(text.to_owned())
            .push("))");
    }
    if let Some(k) = after {
        q.push(" AND (s.created_at < ").push_bind(k.at);
        q.push(" OR (s.created_at = ").push_bind(k.at).push(" AND i.id < ").push_bind(k.id).push("))");
    }
    q.push(" ORDER BY s.created_at DESC, i.id DESC LIMIT ").push_bind(limit + 1);
    let mut rows: Vec<WarrantyItem> = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    let next_cursor = if rows.len() > limit as usize {
        rows.truncate(limit as usize);
        rows.last().map(|r| Keyset { at: r.created_at, id: r.id }.encode())
    } else {
        None
    };
    Ok(WarrantyPage { data: rows, next_cursor, today })
}
