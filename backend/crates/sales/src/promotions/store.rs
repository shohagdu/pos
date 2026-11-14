//! Promotions in the database: which ones are live for a cart, coupon
//! validation (§4.4), and what a completed / cancelled sale does to the
//! counters (usage, flash quantity, coupon uses).

use std::collections::BTreeMap;

use chrono::{Datelike, NaiveDateTime, NaiveTime};
use pos_core::{AppError, AppResult, id::new_id};
use pos_db::{TenantTx, map_db_error};
use rust_decimal::Decimal;
use serde::Serialize;
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

use super::engine::{self, BenefitType, Bxgy, GetTargetType, PromoKind, Target, TargetType};

pub const WEEKDAYS: [&str; 7] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];

/// A promotion row as stored (the columns the engine and the checks need).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Row {
    pub id: Uuid,
    pub name: String,
    pub name_bn: Option<String>,
    #[sqlx(rename = "type")]
    pub kind: String,
    pub discount_type: String,
    pub discount_value: Decimal,
    pub max_discount_amount: Option<Decimal>,
    pub min_purchase_amount: Option<Decimal>,
    pub starts_at: Option<NaiveDateTime>,
    pub ends_at: Option<NaiveDateTime>,
    pub active_days: Option<sqlx::types::Json<Vec<String>>>,
    pub active_time_from: Option<NaiveTime>,
    pub active_time_to: Option<NaiveTime>,
    pub priority: i32,
    pub stackable: bool,
    pub applies_to_customer: String,
    pub total_usage_limit: Option<i32>,
    pub usage_count: i32,
    pub per_customer_limit: Option<i32>,
    pub quantity_limit: Option<Decimal>,
    pub quantity_used: Decimal,
    pub bxgy_buy_quantity: Option<Decimal>,
    pub bxgy_get_quantity: Option<Decimal>,
    pub bxgy_get_target_type: Option<String>,
    pub bxgy_get_target_id: Option<Uuid>,
    pub bxgy_get_discount_type: Option<String>,
    pub bxgy_get_discount_value: Option<Decimal>,
    pub bxgy_repeat: bool,
    pub bxgy_auto_add: bool,
}

pub const COLUMNS: &str = "p.id, p.name, p.name_bn, p.type, p.discount_type, p.discount_value, p.max_discount_amount, \
     p.min_purchase_amount, p.starts_at, p.ends_at, p.active_days, p.active_time_from, p.active_time_to, p.priority, \
     p.stackable, p.applies_to_customer, p.total_usage_limit, p.usage_count, p.per_customer_limit, p.quantity_limit, \
     p.quantity_used, p.bxgy_buy_quantity, p.bxgy_get_quantity, p.bxgy_get_target_type, p.bxgy_get_target_id, \
     p.bxgy_get_discount_type, p.bxgy_get_discount_value, p.bxgy_repeat, p.bxgy_auto_add";

pub fn kind(s: &str) -> PromoKind {
    match s {
        "flash" => PromoKind::Flash,
        "bxgy" => PromoKind::Bxgy,
        "coupon" => PromoKind::Coupon,
        _ => PromoKind::ProductDiscount,
    }
}

pub fn benefit(s: &str) -> BenefitType {
    match s {
        "fixed_amount" => BenefitType::FixedAmount,
        "fixed_price" => BenefitType::FixedPrice,
        "free" => BenefitType::Free,
        _ => BenefitType::Percent,
    }
}

pub fn target_type(s: &str) -> TargetType {
    match s {
        "product" => TargetType::Product,
        "category" => TargetType::Category,
        "brand" => TargetType::Brand,
        _ => TargetType::AllProducts,
    }
}

pub fn get_target_type(s: &str) -> GetTargetType {
    match s {
        "product" => GetTargetType::Product,
        "category" => GetTargetType::Category,
        _ => GetTargetType::SameProduct,
    }
}

/// The schedule part of a promotion, checked against the business's local time.
#[derive(Debug, Clone, Default, PartialEq, Serialize, utoipa::ToSchema)]
pub struct Schedule {
    #[schema(value_type = Option<String>)]
    pub starts_at: Option<NaiveDateTime>,
    #[schema(value_type = Option<String>)]
    pub ends_at: Option<NaiveDateTime>,
    /// `mon`…`sun`; `null` = every day.
    pub active_days: Option<Vec<String>>,
    #[schema(value_type = Option<String>)]
    pub active_time_from: Option<NaiveTime>,
    #[schema(value_type = Option<String>)]
    pub active_time_to: Option<NaiveTime>,
}

impl Schedule {
    pub fn of(r: &Row) -> Self {
        Self {
            starts_at: r.starts_at,
            ends_at: r.ends_at,
            active_days: r.active_days.as_ref().map(|d| d.0.clone()),
            active_time_from: r.active_time_from,
            active_time_to: r.active_time_to,
        }
    }

    /// Open at `now`: within the dates, on an active day, inside the time window
    /// (a window like 22:00–02:00 runs over midnight).
    pub fn open_at(&self, now: NaiveDateTime) -> bool {
        if self.starts_at.is_some_and(|s| now < s) || self.ends_at.is_some_and(|e| now >= e) {
            return false;
        }
        if let Some(days) = &self.active_days
            && !days.is_empty()
        {
            let today = WEEKDAYS[now.weekday().num_days_from_monday() as usize];
            if !days.iter().any(|d| d == today) {
                return false;
            }
        }
        let t = now.time();
        match (self.active_time_from, self.active_time_to) {
            (Some(from), Some(to)) if from <= to => from <= t && t < to,
            (Some(from), Some(to)) => t >= from || t < to,
            (Some(from), None) => t >= from,
            (None, Some(to)) => t < to,
            (None, None) => true,
        }
    }
}

/// The engine's view of a promotion row (targets loaded separately).
pub fn to_engine(r: &Row, targets: Vec<Target>) -> engine::Promotion {
    let kind = kind(&r.kind);
    engine::Promotion {
        id: r.id,
        name: r.name.clone(),
        name_bn: r.name_bn.clone(),
        kind,
        discount_type: benefit(&r.discount_type),
        discount_value: r.discount_value,
        max_discount_amount: r.max_discount_amount,
        min_purchase_amount: r.min_purchase_amount,
        priority: r.priority,
        stackable: r.stackable,
        targets,
        quantity_remaining: r.quantity_limit.map(|l| (l - r.quantity_used).max(Decimal::ZERO)),
        bxgy: (kind == PromoKind::Bxgy).then(|| Bxgy {
            buy_quantity: r.bxgy_buy_quantity.unwrap_or(Decimal::ONE),
            get_quantity: r.bxgy_get_quantity.unwrap_or(Decimal::ONE),
            get_target_type: get_target_type(r.bxgy_get_target_type.as_deref().unwrap_or("same_product")),
            get_target_id: r.bxgy_get_target_id,
            get_discount_type: benefit(r.bxgy_get_discount_type.as_deref().unwrap_or("free")),
            get_discount_value: r.bxgy_get_discount_value.unwrap_or_default(),
            repeat: r.bxgy_repeat,
            auto_add_free_item: r.bxgy_auto_add,
        }),
        coupon_id: None,
        coupon_code: None,
    }
}

pub async fn targets(tx: &mut TenantTx, ids: &[Uuid]) -> AppResult<BTreeMap<Uuid, Vec<Target>>> {
    let mut map: BTreeMap<Uuid, Vec<Target>> = ids.iter().map(|id| (*id, Vec::new())).collect();
    if ids.is_empty() {
        return Ok(map);
    }
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "SELECT promotion_id, target_type, target_id, is_exclusion FROM promotion_targets WHERE tenant_id = ",
    );
    q.push_bind(tx.tenant_id()).push(" AND promotion_id IN (");
    let mut sep = q.separated(", ");
    for id in ids {
        sep.push_bind(*id);
    }
    q.push(") ORDER BY id");
    let rows: Vec<(Uuid, String, Option<Uuid>, bool)> =
        q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    for (pid, tt, tid, ex) in rows {
        map.entry(pid).or_default().push(Target { target_type: target_type(&tt), target_id: tid, is_exclusion: ex });
    }
    Ok(map)
}

/// Who is buying (for "registered customers only" and per-customer limits).
#[derive(Debug, Clone, Copy)]
pub struct Buyer {
    pub customer_id: Uuid,
    pub is_walk_in: bool,
}

async fn customer_uses(tx: &mut TenantTx, promotion_id: Uuid, customer_id: Uuid) -> AppResult<i64> {
    sqlx::query_scalar(
        "SELECT COUNT(DISTINCT d.sale_id) FROM sale_item_discounts d \
         JOIN sales s ON s.tenant_id = d.tenant_id AND s.id = d.sale_id \
         WHERE d.tenant_id = ? AND d.promotion_id = ? AND s.customer_id = ? AND s.status <> 'cancelled'",
    )
    .bind(tx.tenant_id())
    .bind(promotion_id)
    .bind(customer_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)
}

/// A live automatic promotion, as the POS screen gets it: the engine's fields
/// plus the schedule, so the screen can stop applying it when the window closes.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct LivePromotion {
    #[serde(flatten)]
    pub promotion: engine::Promotion,
    #[serde(flatten)]
    pub schedule: Schedule,
}

/// Automatic promotions (not coupons) live now in `branch_id` for `buyer`,
/// in engine order (priority, then oldest first). With `lock`, the rows stay
/// locked until the transaction ends, so two registers cannot both sell the
/// last flash units or the last allowed use.
pub async fn live(
    tx: &mut TenantTx,
    branch_id: Uuid,
    now: NaiveDateTime,
    buyer: Buyer,
    lock: bool,
) -> AppResult<Vec<LivePromotion>> {
    let sql = format!(
        "SELECT {COLUMNS} FROM promotions p \
         WHERE p.tenant_id = ? AND p.deleted_at IS NULL AND p.status = 'active' AND p.type <> 'coupon' \
           AND (p.starts_at IS NULL OR p.starts_at <= ?) AND (p.ends_at IS NULL OR p.ends_at > ?) \
           AND (p.total_usage_limit IS NULL OR p.usage_count < p.total_usage_limit) \
           AND (p.quantity_limit IS NULL OR p.quantity_used < p.quantity_limit) \
           AND (NOT EXISTS (SELECT 1 FROM promotion_branches b WHERE b.tenant_id = p.tenant_id AND b.promotion_id = p.id) \
                OR EXISTS (SELECT 1 FROM promotion_branches b WHERE b.tenant_id = p.tenant_id AND b.promotion_id = p.id \
                           AND b.branch_id = ?)) \
         ORDER BY p.priority DESC, p.created_at, p.id{}",
        if lock { " FOR UPDATE" } else { "" }
    );
    let rows: Vec<Row> = sqlx::query_as(sqlx::AssertSqlSafe(sql))
        .bind(tx.tenant_id())
        .bind(now)
        .bind(now)
        .bind(branch_id)
        .fetch_all(&mut **tx)
        .await
        .map_err(map_db_error)?;
    let mut keep = Vec::new();
    for r in rows {
        if !Schedule::of(&r).open_at(now) {
            continue;
        }
        if r.applies_to_customer == "registered_only" && buyer.is_walk_in {
            continue;
        }
        if let Some(limit) = r.per_customer_limit {
            // Uses cannot be counted for the walk-in customer.
            if buyer.is_walk_in || customer_uses(tx, r.id, buyer.customer_id).await? >= i64::from(limit) {
                continue;
            }
        }
        keep.push(r);
    }
    let ids: Vec<Uuid> = keep.iter().map(|r| r.id).collect();
    let mut all_targets = targets(tx, &ids).await?;
    Ok(keep
        .iter()
        .map(|r| LivePromotion {
            promotion: to_engine(r, all_targets.remove(&r.id).unwrap_or_default()),
            schedule: Schedule::of(r),
        })
        .collect())
}

fn coupon_error(code: &'static str, message: &str) -> AppError {
    AppError::field("coupon_code", code, message)
}

#[derive(sqlx::FromRow)]
struct CouponRow {
    coupon_id: Uuid,
    code: String,
    status: String,
    usage_limit: Option<i32>,
    used_count: i32,
    assigned_customer_id: Option<Uuid>,
    expires_at: Option<NaiveDateTime>,
    promotion_status: String,
    deleted_at: Option<NaiveDateTime>,
    #[sqlx(flatten)]
    p: Row,
}

/// Checks a coupon code for this branch, time and buyer (§4.4 steps 1–5; the
/// minimum purchase and qualifying items are the engine's). Returns the
/// coupon as an engine promotion.
pub async fn validate_coupon(
    tx: &mut TenantTx,
    code: &str,
    branch_id: Uuid,
    now: NaiveDateTime,
    buyer: Buyer,
    lock: bool,
) -> AppResult<engine::Promotion> {
    let code = code.trim();
    if code.is_empty() || code.chars().count() > 40 {
        return Err(coupon_error("COUPON_NOT_FOUND", "no coupon has this code"));
    }
    let sql = format!(
        "SELECT c.id AS coupon_id, c.code, c.status, c.usage_limit, c.used_count, c.assigned_customer_id, c.expires_at, \
         p.status AS promotion_status, p.deleted_at, {COLUMNS} \
         FROM coupons c JOIN promotions p ON p.tenant_id = c.tenant_id AND p.id = c.promotion_id \
         WHERE c.tenant_id = ? AND c.code = ?{}",
        if lock { " FOR UPDATE" } else { "" }
    );
    let row: Option<CouponRow> = sqlx::query_as(sqlx::AssertSqlSafe(sql))
        .bind(tx.tenant_id())
        .bind(code)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_db_error)?;
    // 1. Exists and is active.
    let Some(c) = row else { return Err(coupon_error("COUPON_NOT_FOUND", "no coupon has this code")) };
    if c.p.kind != "coupon" || c.deleted_at.is_some() || c.status == "disabled" || c.promotion_status != "active" {
        return Err(coupon_error("COUPON_INACTIVE", "this coupon is not active"));
    }
    // 2. Date, time and branch.
    let schedule = Schedule::of(&c.p);
    if c.expires_at.is_some_and(|e| now >= e) || schedule.ends_at.is_some_and(|e| now >= e) {
        return Err(coupon_error("COUPON_EXPIRED", "this coupon has expired"));
    }
    if !schedule.open_at(now) {
        return Err(coupon_error("COUPON_NOT_NOW", "this coupon cannot be used at this time"));
    }
    let branches: Vec<Uuid> =
        sqlx::query_scalar("SELECT branch_id FROM promotion_branches WHERE tenant_id = ? AND promotion_id = ?")
            .bind(tx.tenant_id())
            .bind(c.p.id)
            .fetch_all(&mut **tx)
            .await
            .map_err(map_db_error)?;
    if !branches.is_empty() && !branches.contains(&branch_id) {
        return Err(coupon_error("COUPON_WRONG_BRANCH", "this coupon is not valid in this branch"));
    }
    // 3. Usage limits.
    if c.status == "used"
        || c.usage_limit.is_some_and(|l| c.used_count >= l)
        || c.p.total_usage_limit.is_some_and(|l| c.p.usage_count >= l)
    {
        return Err(coupon_error("COUPON_USED_UP", "this coupon has been used up"));
    }
    // 4. Customer.
    if (c.p.applies_to_customer == "registered_only" || c.assigned_customer_id.is_some()) && buyer.is_walk_in {
        return Err(coupon_error(
            "COUPON_CUSTOMER_REQUIRED",
            "choose the customer first: this coupon is for registered customers",
        ));
    }
    if c.assigned_customer_id.is_some_and(|id| id != buyer.customer_id) {
        return Err(coupon_error("COUPON_NOT_FOR_CUSTOMER", "this coupon belongs to another customer"));
    }
    // 5. Per-customer limit.
    if let Some(limit) = c.p.per_customer_limit {
        if buyer.is_walk_in {
            return Err(coupon_error(
                "COUPON_CUSTOMER_REQUIRED",
                "choose the customer first: this coupon is limited per customer",
            ));
        }
        let used: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM coupon_redemptions WHERE tenant_id = ? AND promotion_id = ? AND customer_id = ? \
             AND status = 'applied'",
        )
        .bind(tx.tenant_id())
        .bind(c.p.id)
        .bind(buyer.customer_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
        if used >= i64::from(limit) {
            return Err(coupon_error("COUPON_CUSTOMER_LIMIT", "this customer has already used this coupon"));
        }
    }
    let targets = targets(tx, &[c.p.id]).await?.remove(&c.p.id).unwrap_or_default();
    let mut promotion = to_engine(&c.p, targets);
    promotion.coupon_id = Some(c.coupon_id);
    promotion.coupon_code = Some(c.code);
    Ok(promotion)
}

/// Product, brand and category chain of each variant, for matching targets.
pub async fn line_targets(
    tx: &mut TenantTx,
    variant_ids: &[Uuid],
) -> AppResult<BTreeMap<Uuid, (Option<Uuid>, Vec<Uuid>)>> {
    let mut out = BTreeMap::new();
    if variant_ids.is_empty() {
        return Ok(out);
    }
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "SELECT v.id, p.brand_id, p.category_id, pc.parent_id, gc.parent_id FROM product_variants v \
         JOIN products p ON p.tenant_id = v.tenant_id AND p.id = v.product_id \
         LEFT JOIN categories pc ON pc.tenant_id = p.tenant_id AND pc.id = p.category_id \
         LEFT JOIN categories gc ON gc.tenant_id = pc.tenant_id AND gc.id = pc.parent_id \
         WHERE v.tenant_id = ",
    );
    q.push_bind(tx.tenant_id()).push(" AND v.id IN (");
    let mut sep = q.separated(", ");
    for id in variant_ids {
        sep.push_bind(*id);
    }
    q.push(")");
    #[allow(clippy::type_complexity)]
    let rows: Vec<(Uuid, Option<Uuid>, Option<Uuid>, Option<Uuid>, Option<Uuid>)> =
        q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    for (id, brand, c0, c1, c2) in rows {
        out.insert(id, (brand, [c0, c1, c2].into_iter().flatten().collect()));
    }
    Ok(out)
}

/// What a completed sale used: the per-line snapshots, the counters and the coupon redemption.
pub struct SaleUse<'a> {
    pub sale_id: Uuid,
    pub branch_id: Uuid,
    pub customer_id: Uuid,
    /// `(sale_item_id, line result)` in cart order.
    pub lines: Vec<(Uuid, &'a engine::LineOut)>,
    /// The engine promotions by id (for Buy X Get Y snapshots).
    pub promotions: BTreeMap<Uuid, &'a engine::Promotion>,
    pub coupon_total: Decimal,
}

pub async fn record_sale(tx: &mut TenantTx, u: &SaleUse<'_>) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    let mut per_promotion: BTreeMap<Uuid, (PromoKind, Decimal)> = BTreeMap::new();
    let mut coupon: Option<(Uuid, Uuid)> = None;
    let mut rows = Vec::new();
    for (item_id, line) in &u.lines {
        for d in &line.discounts {
            let entry = per_promotion.entry(d.promotion_id).or_insert((d.kind, Decimal::ZERO));
            entry.1 += d.units;
            if let Some(cid) = d.coupon_id {
                coupon = Some((cid, d.promotion_id));
            }
            rows.push((*item_id, d));
        }
    }
    if rows.is_empty() {
        return Ok(());
    }
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "INSERT INTO sale_item_discounts (tenant_id, id, sale_id, sale_item_id, promotion_id, coupon_id, coupon_code, \
         promotion_type, promotion_name_snapshot, promotion_name_bn, discount_type, discount_value, discount_amount, \
         units, free_quantity, bxgy_buy_quantity, bxgy_get_quantity, bxgy_get_target_type) ",
    );
    q.push_values(&rows, |mut b, (item_id, d)| {
        let rule = u.promotions.get(&d.promotion_id).and_then(|p| p.bxgy.as_ref());
        b.push_bind(tenant_id)
            .push_bind(new_id())
            .push_bind(u.sale_id)
            .push_bind(*item_id)
            .push_bind(d.promotion_id)
            .push_bind(d.coupon_id)
            .push_bind(d.coupon_code.clone())
            .push_bind(d.kind.as_str())
            .push_bind(d.name.chars().take(120).collect::<String>())
            .push_bind(d.name_bn.clone())
            .push_bind(d.discount_type.as_str())
            .push_bind(d.discount_value)
            .push_bind(d.amount)
            .push_bind(d.units)
            .push_bind(d.free_quantity)
            .push_bind(rule.map(|r| r.buy_quantity))
            .push_bind(rule.map(|r| r.get_quantity))
            .push_bind(rule.map(|r| match r.get_target_type {
                GetTargetType::SameProduct => "same_product",
                GetTargetType::Product => "product",
                GetTargetType::Category => "category",
            }));
    });
    q.build().execute(&mut **tx).await.map_err(map_db_error)?;

    for (pid, (kind, units)) in &per_promotion {
        let flash_units = if *kind == PromoKind::Flash { *units } else { Decimal::ZERO };
        sqlx::query(
            "UPDATE promotions SET usage_count = usage_count + 1, quantity_used = quantity_used + ?, \
             first_used_at = COALESCE(first_used_at, NOW()) WHERE tenant_id = ? AND id = ?",
        )
        .bind(flash_units)
        .bind(tenant_id)
        .bind(pid)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }
    if let Some((coupon_id, promotion_id)) = coupon {
        sqlx::query(
            "UPDATE coupons SET used_count = used_count + 1, \
             status = IF(usage_limit IS NOT NULL AND used_count >= usage_limit, 'used', status) \
             WHERE tenant_id = ? AND id = ?",
        )
        .bind(tenant_id)
        .bind(coupon_id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
        sqlx::query(
            "INSERT INTO coupon_redemptions (tenant_id, id, coupon_id, promotion_id, sale_id, customer_id, branch_id, \
             user_id, discount_amount) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(tenant_id)
        .bind(new_id())
        .bind(coupon_id)
        .bind(promotion_id)
        .bind(u.sale_id)
        .bind(u.customer_id)
        .bind(u.branch_id)
        .bind(tx.user_id())
        .bind(u.coupon_total)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }
    Ok(())
}

/// Gives back a coupon use (cancelled sale, or a full return with "Restore
/// coupon on full return" on).
pub async fn reverse_coupon(tx: &mut TenantTx, sale_id: Uuid) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    let redemptions: Vec<Uuid> = sqlx::query_scalar(
        "SELECT coupon_id FROM coupon_redemptions WHERE tenant_id = ? AND sale_id = ? AND status = 'applied' FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(sale_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    for coupon_id in redemptions {
        sqlx::query(
            "UPDATE coupons SET used_count = GREATEST(used_count - 1, 0), status = IF(status = 'used', 'active', status) \
             WHERE tenant_id = ? AND id = ?",
        )
        .bind(tenant_id)
        .bind(coupon_id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }
    sqlx::query(
        "UPDATE coupon_redemptions SET status = 'reversed', reversed_at = NOW() \
         WHERE tenant_id = ? AND sale_id = ? AND status = 'applied'",
    )
    .bind(tenant_id)
    .bind(sale_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}

/// A cancelled sale gives back everything it used: promotion uses, flash
/// units and the coupon.
pub async fn reverse_sale(tx: &mut TenantTx, sale_id: Uuid) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    let used: Vec<(Uuid, String, Decimal)> = sqlx::query_as(
        "SELECT promotion_id, MAX(promotion_type), SUM(units) FROM sale_item_discounts \
         WHERE tenant_id = ? AND sale_id = ? GROUP BY promotion_id",
    )
    .bind(tenant_id)
    .bind(sale_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    for (pid, kind, units) in used {
        let flash = if kind == "flash" { units } else { Decimal::ZERO };
        sqlx::query(
            "UPDATE promotions SET usage_count = GREATEST(usage_count - 1, 0), \
             quantity_used = GREATEST(quantity_used - ?, 0) WHERE tenant_id = ? AND id = ?",
        )
        .bind(flash)
        .bind(tenant_id)
        .bind(pid)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }
    reverse_coupon(tx, sale_id).await
}
