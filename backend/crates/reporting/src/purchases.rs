//! Purchase reports: by supplier (R-07 totals) and by product (R-08), over
//! posted bills in the period, net of purchase returns.

use chrono::NaiveDate;
use pos_core::AppResult;
use pos_db::{TenantTx, map_db_error};
use rust_decimal::Decimal;
use serde::Serialize;
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

use crate::period::{DateRange, resolve};

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct SupplierPurchaseRow {
    pub supplier_id: Uuid,
    pub supplier_name: String,
    pub bills: i64,
    #[schema(value_type = String)]
    pub grand_total: Decimal,
    #[schema(value_type = String)]
    pub returned_total: Decimal,
    #[schema(value_type = String)]
    pub paid_total: Decimal,
    #[schema(value_type = String)]
    pub due_total: Decimal,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct ProductPurchaseRow {
    pub variant_id: Uuid,
    pub product_name: String,
    pub sku: String,
    pub unit_short_name: String,
    /// Received less returned.
    #[schema(value_type = String)]
    pub quantity: Decimal,
    /// Net of discounts, excluding VAT.
    #[schema(value_type = String)]
    pub net_cost: Decimal,
    #[schema(value_type = String)]
    pub avg_unit_cost: Decimal,
    #[schema(value_type = Option<String>)]
    pub last_unit_cost: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct PurchaseReport {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub by_supplier: Vec<SupplierPurchaseRow>,
    pub by_product: Vec<ProductPurchaseRow>,
    pub bills: i64,
    #[schema(value_type = String)]
    pub grand_total: Decimal,
    #[schema(value_type = String)]
    pub returned_total: Decimal,
    #[schema(value_type = String)]
    pub paid_total: Decimal,
    #[schema(value_type = String)]
    pub due_total: Decimal,
    #[schema(value_type = String)]
    pub tax_total: Decimal,
}

pub async fn report(tx: &mut TenantTx, range: &DateRange) -> AppResult<PurchaseReport> {
    let (from, to) = resolve(tx, range).await?;
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "SELECT p.supplier_id, s.name AS supplier_name, COUNT(*) AS bills, COALESCE(SUM(p.received_total), 0) AS grand_total, \
         COALESCE(SUM(p.returned_total), 0) AS returned_total, COALESCE(SUM(p.paid_total), 0) AS paid_total, \
         COALESCE(SUM(p.due_total), 0) AS due_total FROM purchases p \
         JOIN suppliers s ON s.tenant_id = p.tenant_id AND s.id = p.supplier_id \
         WHERE p.status IN ('partially_received', 'received') AND p.tenant_id = ",
    );
    q.push_bind(tenant_id);
    if let Some(branch_id) = tx.branch_id() {
        q.push(" AND p.branch_id = ").push_bind(branch_id);
    }
    q.push(" AND p.purchase_date BETWEEN ").push_bind(from).push(" AND ").push_bind(to);
    if let Some(supplier_id) = range.supplier_id {
        q.push(" AND p.supplier_id = ").push_bind(supplier_id);
    }
    q.push(" GROUP BY p.supplier_id, s.name ORDER BY grand_total DESC");
    let by_supplier: Vec<SupplierPurchaseRow> = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;

    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "SELECT i.variant_id, MAX(i.product_name) AS product_name, MAX(i.sku) AS sku, MAX(i.unit_short_name) AS unit_short_name, \
         SUM(i.received_quantity - i.returned_quantity) AS quantity, \
         COALESCE(SUM((i.line_total - i.tax_amount) * (i.received_quantity - i.returned_quantity) / i.base_quantity), 0) AS net_cost, \
         COALESCE(ROUND(SUM(i.line_total - i.tax_amount) / NULLIF(SUM(i.base_quantity), 0), 4), 0) AS avg_unit_cost, \
         (SELECT v.last_purchase_price FROM product_variants v WHERE v.tenant_id = i.tenant_id AND v.id = i.variant_id) AS last_unit_cost \
         FROM purchase_items i JOIN purchases p ON p.tenant_id = i.tenant_id AND p.id = i.purchase_id \
         WHERE p.status IN ('partially_received', 'received') AND i.tenant_id = ",
    );
    q.push_bind(tenant_id);
    if let Some(branch_id) = tx.branch_id() {
        q.push(" AND p.branch_id = ").push_bind(branch_id);
    }
    q.push(" AND p.purchase_date BETWEEN ").push_bind(from).push(" AND ").push_bind(to);
    if let Some(supplier_id) = range.supplier_id {
        q.push(" AND p.supplier_id = ").push_bind(supplier_id);
    }
    q.push(" GROUP BY i.tenant_id, i.variant_id ORDER BY net_cost DESC LIMIT 2000");
    let mut by_product: Vec<ProductPurchaseRow> =
        q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    for row in &mut by_product {
        row.net_cost = row.net_cost.round_dp(2);
    }

    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "SELECT COALESCE(SUM(gi.tax_amount), 0) FROM goods_receipt_items gi \
         JOIN goods_receipts g ON g.tenant_id = gi.tenant_id AND g.id = gi.receipt_id \
         JOIN purchases p ON p.tenant_id = g.tenant_id AND p.id = g.purchase_id \
         WHERE p.status IN ('partially_received', 'received') AND gi.tenant_id = ",
    );
    q.push_bind(tenant_id);
    if let Some(branch_id) = tx.branch_id() {
        q.push(" AND p.branch_id = ").push_bind(branch_id);
    }
    q.push(" AND p.purchase_date BETWEEN ").push_bind(from).push(" AND ").push_bind(to);
    if let Some(supplier_id) = range.supplier_id {
        q.push(" AND p.supplier_id = ").push_bind(supplier_id);
    }
    let tax_total: Decimal = q.build_query_scalar().fetch_one(&mut **tx).await.map_err(map_db_error)?;
    Ok(PurchaseReport {
        from,
        to,
        bills: by_supplier.iter().map(|r| r.bills).sum(),
        grand_total: by_supplier.iter().map(|r| r.grand_total).sum(),
        returned_total: by_supplier.iter().map(|r| r.returned_total).sum(),
        paid_total: by_supplier.iter().map(|r| r.paid_total).sum(),
        due_total: by_supplier.iter().map(|r| r.due_total).sum(),
        tax_total,
        by_supplier,
        by_product,
    })
}
