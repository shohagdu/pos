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

use crate::batches::Batch;

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
    /// Which batch the quantity goes to or comes from (`Auto` for most lines).
    pub batch: Batch,
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
    post_batches(tx, reference, lines).await?;

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

/// One line's share of one batch.
struct Allocation {
    line: usize,
    batch_id: Uuid,
    /// Signed like the line.
    quantity: Decimal,
    origin: Option<Uuid>,
}

type BatchBalances = BTreeMap<(Uuid, Uuid), (Uuid, Decimal)>;

fn available(balance: &BatchBalances, location_id: Uuid, batch_id: Uuid) -> Decimal {
    balance.get(&(location_id, batch_id)).map_or(Decimal::ZERO, |(_, q)| (*q).max(Decimal::ZERO))
}

/// What is left of one line to place in batches.
struct Cut {
    line: usize,
    location_id: Uuid,
    variant_id: Uuid,
    increase: bool,
    left: Decimal,
}

impl Cut {
    /// Moves up to `wanted` of the line into or out of `batch_id`; returns how much.
    fn take(
        &mut self,
        balance: &mut BatchBalances,
        allocations: &mut Vec<Allocation>,
        batch_id: Uuid,
        wanted: Decimal,
        origin: Option<Uuid>,
    ) -> Decimal {
        let amount = wanted.min(self.left);
        if amount <= Decimal::ZERO {
            return Decimal::ZERO;
        }
        let signed = if self.increase { amount } else { -amount };
        let entry = balance.entry((self.location_id, batch_id)).or_insert((self.variant_id, Decimal::ZERO));
        entry.1 += signed;
        self.left -= amount;
        allocations.push(Allocation { line: self.line, batch_id, quantity: signed, origin });
        amount
    }
}

/// Moves batch balances for `lines` (the stock balances are already locked,
/// so the batch rows of those variants cannot change under us): increases go
/// into their batch, decreases take the earliest expiry first, reversals
/// follow the batches of the line they undo. Stock not in a batch needs no row.
async fn post_batches(tx: &mut TenantTx, reference: &MovementRef, lines: &[MovementLine]) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    let mut keys: Vec<(Uuid, Uuid)> = lines.iter().map(|l| (l.location_id, l.variant_id)).collect();
    keys.sort();
    keys.dedup();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "SELECT location_id, batch_id, variant_id, quantity FROM stock_batch_balances WHERE tenant_id = ",
    );
    q.push_bind(tenant_id).push(" AND (location_id, variant_id) IN (");
    let mut list = q.separated(", ");
    for (location_id, variant_id) in &keys {
        list.push("(")
            .push_bind_unseparated(*location_id)
            .push_unseparated(", ")
            .push_bind_unseparated(*variant_id)
            .push_unseparated(")");
    }
    q.push(") ORDER BY location_id, batch_id FOR UPDATE");
    let rows: Vec<(Uuid, Uuid, Uuid, Decimal)> = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    let explicit = lines.iter().any(|l| l.batch != Batch::Auto);
    if rows.is_empty() && !explicit {
        return Ok(());
    }
    // (location, batch) → (variant, quantity)
    let mut balance: BatchBalances = rows.into_iter().map(|(l, b, v, q)| ((l, b), (v, q))).collect();

    // What the lines being undone moved, and what earlier reversals already undid.
    let origins: Vec<Uuid> =
        lines.iter().filter_map(|l| if let Batch::Reverse(o) = l.batch { Some(o) } else { None }).collect();
    // origin → batch → (taken out, put in), net of earlier reversals.
    let mut history: BTreeMap<Uuid, BTreeMap<Uuid, (Decimal, Decimal)>> = BTreeMap::new();
    if !origins.is_empty() {
        let mut q: QueryBuilder<MySql> = QueryBuilder::new(
            "SELECT batch_id, quantity, reference_line_id, origin_line_id FROM stock_batch_movements WHERE tenant_id = ",
        );
        q.push_bind(tenant_id).push(" AND (reference_line_id IN (");
        let mut list = q.separated(", ");
        for o in &origins {
            list.push_bind(*o);
        }
        q.push(") OR origin_line_id IN (");
        let mut list = q.separated(", ");
        for o in &origins {
            list.push_bind(*o);
        }
        q.push("))");
        let moved: Vec<(Uuid, Decimal, Option<Uuid>, Option<Uuid>)> =
            q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
        for (batch_id, quantity, line_id, origin_id) in moved {
            match origin_id {
                // A reversal: putting back reduces what can still be put back, and so on.
                Some(o) if origins.contains(&o) => {
                    let entry = history.entry(o).or_default().entry(batch_id).or_default();
                    if quantity > Decimal::ZERO { entry.0 -= quantity } else { entry.1 += quantity }
                }
                Some(_) => {}
                None => {
                    if let Some(o) = line_id.filter(|o| origins.contains(o)) {
                        let entry = history.entry(o).or_default().entry(batch_id).or_default();
                        if quantity < Decimal::ZERO { entry.0 -= quantity } else { entry.1 += quantity }
                    }
                }
            }
        }
    }

    // Expiry of every batch involved, for earliest-expiry-first order.
    let mut batch_ids: Vec<Uuid> = balance.keys().map(|(_, b)| *b).collect();
    batch_ids.extend(lines.iter().filter_map(|l| if let Batch::Exact(b) = l.batch { Some(b) } else { None }));
    batch_ids.extend(history.values().flat_map(|m| m.keys().copied()));
    batch_ids.sort();
    batch_ids.dedup();
    let mut order: BTreeMap<Uuid, (bool, Option<NaiveDate>, String)> = BTreeMap::new();
    if !batch_ids.is_empty() {
        let mut q: QueryBuilder<MySql> =
            QueryBuilder::new("SELECT id, expiry_date, batch_no FROM product_batches WHERE tenant_id = ");
        q.push_bind(tenant_id).push(" AND id IN (");
        let mut list = q.separated(", ");
        for id in &batch_ids {
            list.push_bind(*id);
        }
        q.push(")");
        let rows: Vec<(Uuid, Option<NaiveDate>, String)> =
            q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
        order = rows.into_iter().map(|(id, e, n)| (id, (e.is_none(), e, n))).collect();
    }
    let fefo = |ids: &mut Vec<Uuid>| ids.sort_by(|a, b| order.get(a).cmp(&order.get(b)).then(a.cmp(b)));

    let mut allocations: Vec<Allocation> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let quantity = round3(line.quantity);
        let increase = quantity > Decimal::ZERO;
        let mut cut =
            Cut { line: i, location_id: line.location_id, variant_id: line.variant_id, increase, left: quantity.abs() };
        match line.batch {
            Batch::Auto => {}
            Batch::Exact(b) => {
                let can = if increase { cut.left } else { available(&balance, line.location_id, b) };
                cut.take(&mut balance, &mut allocations, b, can, None);
            }
            Batch::Reverse(o) => {
                let mut ids: Vec<Uuid> = history.get(&o).map(|m| m.keys().copied().collect()).unwrap_or_default();
                fefo(&mut ids);
                if increase {
                    // Back in the reverse order of taking: latest expiry first.
                    ids.reverse();
                }
                for b in ids {
                    let Some(entry) = history.get_mut(&o).and_then(|m| m.get_mut(&b)) else { continue };
                    let can = if increase { entry.0 } else { entry.1.min(available(&balance, line.location_id, b)) };
                    let done = cut.take(&mut balance, &mut allocations, b, can, Some(o));
                    if increase { entry.0 -= done } else { entry.1 -= done }
                }
            }
        }
        if !increase && cut.left > Decimal::ZERO {
            // Earliest expiry first; what the batches lack leaves "no batch".
            let mut ids: Vec<Uuid> = balance
                .iter()
                .filter(|((l, _), (v, q))| *l == line.location_id && *v == line.variant_id && *q > Decimal::ZERO)
                .map(|((_, b), _)| *b)
                .collect();
            fefo(&mut ids);
            for b in ids {
                let can = available(&balance, line.location_id, b);
                cut.take(&mut balance, &mut allocations, b, can, None);
            }
        }
    }
    if allocations.is_empty() {
        return Ok(());
    }

    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "INSERT INTO stock_batch_movements (tenant_id, id, batch_id, variant_id, location_id, quantity, reference_type, \
         reference_id, reference_line_id, origin_line_id) ",
    );
    q.push_values(&allocations, |mut row, a| {
        let line = &lines[a.line];
        row.push_bind(tenant_id)
            .push_bind(new_id())
            .push_bind(a.batch_id)
            .push_bind(line.variant_id)
            .push_bind(line.location_id)
            .push_bind(a.quantity)
            .push_bind(reference.reference_type)
            .push_bind(reference.reference_id)
            .push_bind(line.reference_line_id)
            .push_bind(a.origin);
    });
    q.build().execute(&mut **tx).await.map_err(map_db_error)?;

    let mut touched: Vec<(Uuid, Uuid)> = allocations.iter().map(|a| (lines[a.line].location_id, a.batch_id)).collect();
    touched.sort();
    touched.dedup();
    for key in touched {
        let Some((variant_id, quantity)) = balance.get(&key).copied() else { continue };
        sqlx::query(
            "INSERT INTO stock_batch_balances (tenant_id, location_id, batch_id, variant_id, quantity) VALUES (?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE quantity = ?",
        )
        .bind(tenant_id)
        .bind(key.0)
        .bind(key.1)
        .bind(variant_id)
        .bind(quantity)
        .bind(quantity)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }
    Ok(())
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
