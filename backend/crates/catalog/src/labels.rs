//! Barcode label printing (Roadmap 6.4, PROD-R05): the facts a shelf label
//! shows for a set of variants, and in-store EAN-13 barcodes for variants that
//! have none, so freshly purchased goods can be labelled and scanned.
//!
//! Generated codes use the GS1 restricted-circulation prefix `20` (valid only
//! inside the shop): `20` + a 10-digit running number + the check digit. The
//! running number continues from the tenant's highest `20…` EAN-13, so codes
//! never repeat; `uq_barcodes` guards the rare race between two terminals.

use std::collections::BTreeMap;

use pos_audit::AuditEntry;
use pos_core::{AppError, AppResult, RequestMeta, id::new_id};
use pos_db::{TenantTx, map_db_error};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

use crate::masters::duplicate;

/// At most this many variants per request (a large purchase bill fits).
pub const MAX_VARIANTS: usize = 500;
const IN_STORE_PREFIX: &str = "20";

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

/// GS1 check digit for a 12-digit body.
fn ean13_check(body: &str) -> u32 {
    let sum: u32 =
        body.bytes().rev().enumerate().map(|(i, b)| u32::from(b - b'0') * if i % 2 == 0 { 3 } else { 1 }).sum();
    (10 - sum % 10) % 10
}

/// The in-store EAN-13 for running number `n`.
fn in_store_code(n: u64) -> String {
    let body = format!("{IN_STORE_PREFIX}{n:010}");
    format!("{body}{}", ean13_check(&body))
}

/// The next free in-store running number. With `lock`, the in-store range is
/// locked so two terminals do not pick the same number; without it the
/// answer is only a suggestion.
async fn next_in_store(tx: &mut TenantTx, lock: bool) -> AppResult<u64> {
    const HIGHEST: &str = "SELECT MAX(barcode) FROM product_barcodes WHERE tenant_id = ? AND barcode LIKE '20%' \
                           AND CHAR_LENGTH(barcode) = 13 AND barcode REGEXP '^[0-9]+$'";
    const HIGHEST_LOCKED: &str = "SELECT MAX(barcode) FROM product_barcodes WHERE tenant_id = ? AND barcode LIKE '20%' \
                                  AND CHAR_LENGTH(barcode) = 13 AND barcode REGEXP '^[0-9]+$' FOR UPDATE";
    let highest: Option<String> = sqlx::query_scalar(if lock { HIGHEST_LOCKED } else { HIGHEST })
        .bind(tx.tenant_id())
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(highest.and_then(|c| c.get(2..12).and_then(|n| n.parse::<u64>().ok())).map_or(1, |n| n + 1))
}

/// A new in-store EAN-13 to put on a product being saved (range locked).
pub(crate) async fn take_in_store_code(tx: &mut TenantTx) -> AppResult<String> {
    Ok(in_store_code(next_in_store(tx, true).await?))
}

/// The in-store EAN-13 a product form can offer (not reserved).
pub async fn suggest_in_store_code(tx: &mut TenantTx) -> AppResult<String> {
    Ok(in_store_code(next_in_store(tx, false).await?))
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

    let mut next = next_in_store(tx, true).await?;

    for label in missing {
        let code = in_store_code(next);
        next += 1;
        sqlx::query(
            "INSERT INTO product_barcodes (tenant_id, id, variant_id, barcode, symbology, is_primary) \
             VALUES (?, ?, ?, ?, 'ean13', TRUE)",
        )
        .bind(tenant_id)
        .bind(new_id())
        .bind(label.variant_id)
        .bind(&code)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::barcode;

    #[test]
    fn in_store_codes_are_valid_ean13() {
        for n in [1, 42, 9_999_999_999] {
            let code = in_store_code(n);
            assert_eq!(code.len(), 13);
            assert!(code.starts_with("20"));
            assert_eq!(barcode::validate(&code).ok(), Some((code.clone(), "ean13")), "{code}");
        }
        assert_eq!(in_store_code(1), "2000000000015");
    }
}
