//! Barcode label printing (Roadmap 6.4, PROD-R05): the facts a shelf label
//! shows for a set of variants, and in-store barcodes for variants that have
//! none, so freshly purchased goods can be labelled and scanned.
//!
//! Generated codes follow the business's barcode numbering (Settings →
//! Product codes): its prefix + a zero-padded running number from the
//! `barcode` document sequence, e.g. `200000001`; labels print them as
//! Code 128. Numbers somebody already typed in are skipped, and so are codes
//! that would read as a retail EAN/UPC without a valid check digit;
//! `uq_barcodes` guards the rare race between two terminals.

use std::collections::BTreeMap;

use pos_audit::AuditEntry;
use pos_core::{AppError, AppResult, RequestMeta, id::new_id};
use pos_db::{TenantTx, map_db_error};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

use pos_tenancy::sequences;

use crate::masters::duplicate;

/// At most this many variants per request (a large purchase bill fits).
pub const MAX_VARIANTS: usize = 500;
/// How many sequence numbers to try before giving up on a free code.
const ATTEMPTS: i64 = 50;

#[derive(Debug, Clone, PartialEq, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct LabelInfo {
    pub variant_id: Uuid,
    pub product_id: Uuid,
    pub product_name: String,
    pub product_name_bn: Option<String>,
    pub variant_name: Option<String>,
    pub sku: String,
    /// The primary barcode; `null` when the variant has none yet.
    pub barcode: Option<String>,
    #[schema(value_type = String)]
    pub selling_price: Decimal,
    pub unit_short_name: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct LabelRequest {
    pub variant_ids: Vec<Uuid>,
}

fn check_ids(ids: &[Uuid]) -> AppResult<()> {
    if ids.is_empty() {
        return Err(AppError::field("variant_ids", "REQUIRED", "choose at least one product"));
    }
    if ids.len() > MAX_VARIANTS {
        return Err(AppError::field("variant_ids", "TOO_MANY", format!("use at most {MAX_VARIANTS} products")));
    }
    Ok(())
}

/// Label facts for the given variants, in the order asked (unknown or deleted
/// variants are left out).
pub async fn info(tx: &mut TenantTx, ids: &[Uuid]) -> AppResult<Vec<LabelInfo>> {
    check_ids(ids)?;
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "SELECT v.id AS variant_id, p.id AS product_id, p.name AS product_name, p.name_bn AS product_name_bn, \
         v.variant_name, v.sku, \
         (SELECT b.barcode FROM product_barcodes b WHERE b.tenant_id = v.tenant_id AND b.variant_id = v.id \
          ORDER BY b.is_primary DESC, b.created_at, b.barcode LIMIT 1) AS barcode, \
         v.selling_price, u.short_name AS unit_short_name \
         FROM product_variants v \
         JOIN products p ON p.tenant_id = v.tenant_id AND p.id = v.product_id \
         JOIN units u ON u.tenant_id = p.tenant_id AND u.id = p.base_unit_id \
         WHERE v.tenant_id = ",
    );
    q.push_bind(tenant_id).push(" AND v.deleted_at IS NULL AND p.deleted_at IS NULL AND v.id IN (");
    let mut list = q.separated(", ");
    for id in ids {
        list.push_bind(*id);
    }
    q.push(")");
    let rows: Vec<LabelInfo> = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    let mut by_id: BTreeMap<Uuid, LabelInfo> = rows.into_iter().map(|r| (r.variant_id, r)).collect();
    let mut seen = std::collections::BTreeSet::new();
    Ok(ids.iter().filter(|id| seen.insert(**id)).filter_map(|id| by_id.remove(id)).collect())
}

async fn barcode_taken(tx: &mut TenantTx, code: &str) -> AppResult<bool> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM product_barcodes WHERE tenant_id = ? AND barcode = ?)")
        .bind(tx.tenant_id())
        .bind(code)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)
}

/// A generated code must also pass the barcode check a product save runs.
fn usable(code: &str) -> bool {
    crate::barcode::validate(code).is_ok()
}

fn today() -> chrono::NaiveDate {
    // Codes have no period, so the date does not matter.
    chrono::Utc::now().date_naive()
}

/// Takes the next free barcode from the business's numbering (row lock on the
/// sequence, so two terminals never get the same number).
pub(crate) async fn take_in_store_code(tx: &mut TenantTx) -> AppResult<String> {
    for _ in 0..ATTEMPTS {
        let code = sequences::next(tx, "barcode", None, today()).await?;
        if usable(&code) && !barcode_taken(tx, &code).await? {
            return Ok(code);
        }
    }
    Err(AppError::Conflict { code: "DUPLICATE_BARCODE", message: "could not find a free barcode; enter one".into() })
}

/// The barcode a product form can offer (not reserved).
pub async fn suggest_in_store_code(tx: &mut TenantTx) -> AppResult<String> {
    for code in sequences::preview(tx, "barcode", None, today(), ATTEMPTS).await? {
        if usable(&code) && !barcode_taken(tx, &code).await? {
            return Ok(code);
        }
    }
    Err(AppError::Conflict { code: "DUPLICATE_BARCODE", message: "could not find a free barcode; enter one".into() })
}

/// Gives every listed variant without a barcode a new in-store EAN-13 and
/// returns the label facts for all of them. Variants that already have a
/// barcode keep it.
pub async fn generate_barcodes(tx: &mut TenantTx, meta: &RequestMeta, ids: &[Uuid]) -> AppResult<Vec<LabelInfo>> {
    check_ids(ids)?;
    let tenant_id = tx.tenant_id();
    let current = info(tx, ids).await?;
    let missing: Vec<&LabelInfo> = current.iter().filter(|l| l.barcode.is_none()).collect();
    if missing.is_empty() {
        return Ok(current);
    }

    for label in missing {
        let code = take_in_store_code(tx).await?;
        sqlx::query(
            "INSERT INTO product_barcodes (tenant_id, id, variant_id, barcode, symbology, is_primary) \
             VALUES (?, ?, ?, ?, ?, TRUE)",
        )
        .bind(tenant_id)
        .bind(new_id())
        .bind(label.variant_id)
        .bind(&code)
        .bind(symbology_for(&code))
        .execute(&mut **tx)
        .await
        .map_err(duplicate("DUPLICATE_BARCODE", "a barcode belongs to another product; try again"))?;
        // Open edit forms must reload to see the new barcode.
        sqlx::query("UPDATE products SET version = version + 1 WHERE tenant_id = ? AND id = ?")
            .bind(tenant_id)
            .bind(label.product_id)
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
        pos_audit::record(
            tx,
            meta,
            AuditEntry::new("product.barcode_generated")
                .entity("product", label.product_id)
                .change(None, Some(serde_json::json!({ "sku": label.sku, "barcode": code }))),
        )
        .await?;
    }
    info(tx, ids).await
}

/// Stored symbology of a generated code: retail formats keep their name,
/// anything else is printed as Code 128.
fn symbology_for(code: &str) -> &'static str {
    match crate::barcode::symbology(code) {
        "other" => "code128",
        kind => kind,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_codes_are_checked_like_typed_ones() {
        // Business numbering such as `20` + 7 digits is free format (Code 128).
        assert!(usable("200000001"));
        assert_eq!(symbology_for("200000001"), "code128");
        assert_eq!(symbology_for("MIR0001"), "code128");
        // Eight digits read as EAN-8: only usable with a valid check digit.
        assert!(!usable("20000001"));
        assert!(usable("96385074"));
        assert_eq!(symbology_for("96385074"), "ean8");
    }
}
