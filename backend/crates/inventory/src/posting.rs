//! The stock posting service (STK-R01, R02, R05; DB design §10.1 steps 5 and 8).
//!
//! Balance rows are created if missing, then locked in `(location, variant)`
//! order so concurrent postings cannot deadlock on them. Each line updates the
//! moving weighted average (§8.4): an increase with a known cost re-averages,
//! a decrease leaves the average and costs out at it. Every change writes an
//! immutable `stock_movements` row.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use pos_core::{
    AppError, AppResult,
    id::new_id,
    money::{round2, round3, round4},
};
use pos_db::{TenantTx, map_db_error};
use rust_decimal::Decimal;
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

/// The document a group of movements belongs to.
#[derive(Debug, Clone)]
pub struct MovementRef {
    pub branch_id: Uuid,
    pub business_date: NaiveDate,
    /// `sale`, `sale_cancel`, `sale_return`, `goods_receipt`, `purchase_return`, `stock_adjustment`.
    pub reference_type: &'static str,
    pub reference_id: Uuid,
    pub reference_no: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MovementLine {
    pub location_id: Uuid,
    pub variant_id: Uuid,
    /// One of the `ck_movements_type` values.
    pub movement_type: &'static str,
    /// Signed quantity in the base unit: + in, − out.
    pub quantity: Decimal,
    /// Cost per base unit of an increase; `None` = the current average.
    /// Ignored for decreases, which always leave at the average.
    pub unit_cost: Option<Decimal>,
    pub reference_line_id: Option<Uuid>,
    pub reason: Option<String>,
}

/// What a line did, in input order.
#[derive(Debug, Clone, PartialEq)]
pub struct Posted {
    pub variant_id: Uuid,
    pub location_id: Uuid,
    pub unit_cost: Decimal,
    /// Signed `round2(quantity × unit_cost)`.
    pub total_cost: Decimal,
    pub balance_after: Decimal,
    pub avg_cost_after: Decimal,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Balance {
    quantity: Decimal,
    avg_cost: Decimal,
}

/// Applies one movement to a balance (§8.4). Returns the new balance and the
/// unit cost the movement is valued at.
fn apply(balance: Balance, quantity: Decimal, in_cost: Option<Decimal>) -> (Balance, Decimal) {
    if quantity > Decimal::ZERO {
        let cost = round4(in_cost.unwrap_or(balance.avg_cost).max(Decimal::ZERO));
        let new_qty = balance.quantity + quantity;
        let avg = if balance.quantity <= Decimal::ZERO {
            cost
        } else {
            round4((balance.quantity * balance.avg_cost + quantity * cost) / new_qty)
        };
        (Balance { quantity: new_qty, avg_cost: avg }, cost)
    } else {
        // Out at the average; the average stays, also below zero (STK-R05).
        (Balance { quantity: balance.quantity + quantity, avg_cost: balance.avg_cost }, balance.avg_cost)
    }
}

pub async fn post_movements(
    tx: &mut TenantTx,
    reference: &MovementRef,
    lines: &[MovementLine],
    allow_negative: bool,
) -> AppResult<Vec<Posted>> {
    if lines.is_empty() {
        return Ok(Vec::new());
    }
    for line in lines {
        if line.quantity.is_zero() {
            return Err(AppError::internal("stock movement with zero quantity"));
        }
    }
    let tenant_id = tx.tenant_id();
    let user_id = tx.user_id();
    let mut keys: Vec<(Uuid, Uuid)> = lines.iter().map(|l| (l.location_id, l.variant_id)).collect();
    keys.sort();
    keys.dedup();

    let mut q: QueryBuilder<MySql> =
        QueryBuilder::new("INSERT INTO stock_balances (tenant_id, location_id, variant_id) ");
    q.push_values(&keys, |mut row, (location_id, variant_id)| {
        row.push_bind(tenant_id).push_bind(*location_id).push_bind(*variant_id);
    });
    q.push(" ON DUPLICATE KEY UPDATE variant_id = variant_id");
    q.build().execute(&mut **tx).await.map_err(map_db_error)?;

    let mut q: QueryBuilder<MySql> =
        QueryBuilder::new("SELECT location_id, variant_id, quantity, avg_cost FROM stock_balances WHERE tenant_id = ");
    q.push_bind(tenant_id).push(" AND (location_id, variant_id) IN (");
    let mut list = q.separated(", ");
    for (location_id, variant_id) in &keys {
        list.push("(")
            .push_bind_unseparated(*location_id)
            .push_unseparated(", ")
            .push_bind_unseparated(*variant_id)
            .push_unseparated(")");
    }
    q.push(") ORDER BY location_id, variant_id FOR UPDATE");
    let rows: Vec<(Uuid, Uuid, Decimal, Decimal)> =
        q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    let mut balances: BTreeMap<(Uuid, Uuid), Balance> =
        rows.into_iter().map(|(l, v, quantity, avg_cost)| ((l, v), Balance { quantity, avg_cost })).collect();

    let mut posted = Vec::with_capacity(lines.len());
    for line in lines {
        let key = (line.location_id, line.variant_id);
        let before = *balances.get(&key).ok_or_else(|| AppError::internal("stock balance row missing after insert"))?;
        let quantity = round3(line.quantity);
        let (after, unit_cost) = apply(before, quantity, line.unit_cost);
        if quantity < Decimal::ZERO && after.quantity < Decimal::ZERO && !allow_negative {
            return Err(insufficient(tx, line.variant_id, before.quantity).await);
        }
        balances.insert(key, after);
        posted.push(Posted {
            variant_id: line.variant_id,
            location_id: line.location_id,
            unit_cost,
            total_cost: round2(quantity * unit_cost),
            balance_after: after.quantity,
            avg_cost_after: after.avg_cost,
        });
    }

    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "INSERT INTO stock_movements (tenant_id, variant_id, location_id, id, branch_id, movement_type, quantity, \
         unit_cost, total_cost, balance_qty_after, avg_cost_after, reference_type, reference_id, reference_line_id, \
         reference_no, reason, business_date, created_by) ",
    );
    q.push_values(lines.iter().zip(&posted), |mut row, (line, done)| {
        row.push_bind(tenant_id)
            .push_bind(line.variant_id)
            .push_bind(line.location_id)
            .push_bind(new_id())
            .push_bind(reference.branch_id)
            .push_bind(line.movement_type)
            .push_bind(round3(line.quantity))
            .push_bind(done.unit_cost)
            .push_bind(done.total_cost)
            .push_bind(done.balance_after)
            .push_bind(done.avg_cost_after)
            .push_bind(reference.reference_type)
            .push_bind(reference.reference_id)
            .push_bind(line.reference_line_id)
            .push_bind(reference.reference_no.as_deref())
            .push_bind(line.reason.as_deref())
            .push_bind(reference.business_date)
            .push_bind(user_id);
    });
    q.build().execute(&mut **tx).await.map_err(map_db_error)?;

    for ((location_id, variant_id), balance) in &balances {
        sqlx::query(
            "UPDATE stock_balances SET quantity = ?, avg_cost = ?, last_movement_at = UTC_TIMESTAMP(6) \
             WHERE tenant_id = ? AND location_id = ? AND variant_id = ?",
        )
        .bind(balance.quantity)
        .bind(balance.avg_cost)
        .bind(tenant_id)
        .bind(location_id)
        .bind(variant_id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }
    Ok(posted)
}

/// `409 INSUFFICIENT_STOCK` naming the product and what is available (POS-R04).
async fn insufficient(tx: &mut TenantTx, variant_id: Uuid, available: Decimal) -> AppError {
    let tenant_id = tx.tenant_id();
    let name: Option<(String, String)> = sqlx::query_as(
        "SELECT p.name, v.sku FROM product_variants v JOIN products p ON p.tenant_id = v.tenant_id AND p.id = v.product_id \
         WHERE v.tenant_id = ? AND v.id = ?",
    )
    .bind(tenant_id)
    .bind(variant_id)
    .fetch_optional(&mut **tx)
    .await
    .ok()
    .flatten();
    let label = name.map_or_else(|| variant_id.to_string(), |(n, sku)| format!("{n} ({sku})"));
    AppError::Conflict {
        code: "INSUFFICIENT_STOCK",
        message: format!("not enough stock of {label}: {} available", available.max(Decimal::ZERO).normalize()),
    }
}

/// Current quantity and average cost of variants at a location (no lock).
pub async fn balances_at(
    tx: &mut TenantTx,
    location_id: Uuid,
    variant_ids: &[Uuid],
) -> AppResult<BTreeMap<Uuid, (Decimal, Decimal)>> {
    if variant_ids.is_empty() {
        return Ok(BTreeMap::new());
    }
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> =
        QueryBuilder::new("SELECT variant_id, quantity, avg_cost FROM stock_balances WHERE tenant_id = ");
    q.push_bind(tenant_id).push(" AND location_id = ").push_bind(location_id).push(" AND variant_id IN (");
    let mut list = q.separated(", ");
    for id in variant_ids {
        list.push_bind(*id);
    }
    q.push(")");
    let rows: Vec<(Uuid, Decimal, Decimal)> = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    Ok(rows.into_iter().map(|(v, q, c)| (v, (q, c))).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Decimal {
        s.parse().expect("decimal")
    }

    fn bal(q: &str, c: &str) -> Balance {
        Balance { quantity: d(q), avg_cost: d(c) }
    }

    #[test]
    fn purchase_re_averages_ac_pur_01() {
        // 10 pcs @ 90 in stock; receive 10 pcs landed at 105 (100 + 50 shipping / 10).
        let (after, cost) = apply(bal("10", "90"), d("10"), Some(d("105")));
        assert_eq!(cost, d("105"));
        assert_eq!(after, bal("20", "97.5"));
    }

    #[test]
    fn outflow_keeps_average_and_costs_at_it() {
        let (after, cost) = apply(bal("20", "97.5"), d("-3"), Some(d("1")));
        assert_eq!((after, cost), (bal("17", "97.5"), d("97.5")));
    }

    #[test]
    fn inflow_after_negative_stock_takes_the_new_cost() {
        let (after, cost) = apply(bal("-2", "50"), d("5"), Some(d("60")));
        assert_eq!((after, cost), (bal("3", "60"), d("60")));
    }

    #[test]
    fn inflow_without_cost_uses_the_average() {
        let (after, cost) = apply(bal("4", "12.5"), d("1"), None);
        assert_eq!((after, cost), (bal("5", "12.5"), d("12.5")));
    }

    #[test]
    fn quantity_is_the_sum_of_movements() {
        // A random-ish sequence keeps qty = Σ movements and avg ≥ 0.
        let moves = ["5", "-2", "3", "-10", "4", "1.5", "-0.5", "7"];
        let costs = ["10", "0", "13", "0", "8", "9.25", "0", "11"];
        let mut b = bal("0", "0");
        let mut sum = Decimal::ZERO;
        for (m, c) in moves.iter().zip(costs) {
            let q = d(m);
            sum += q;
            b = apply(b, q, Some(d(c))).0;
            assert!(b.avg_cost >= Decimal::ZERO);
        }
        assert_eq!(b.quantity, sum);
    }
}
