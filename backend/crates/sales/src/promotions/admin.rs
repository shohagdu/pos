//! Promotions and coupons for the owner (§7): list, create, edit (benefit
//! and products locked after first use), pause / resume / end / duplicate,
//! coupon codes and redemptions. Every change is in the audit log.

use chrono::{NaiveDateTime, NaiveTime};
use pos_audit::AuditEntry;
use pos_core::{AppError, AppResult, RequestMeta, id::new_id, money};
use pos_db::{TenantTx, map_db_error};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

use super::store::{self, Row, Schedule, WEEKDAYS};

// --------------------------------------------------------------------- input

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TargetInput {
    /// `product`, `category` (with its sub-categories), `brand` or `all_products`.
    pub target_type: String,
    #[serde(default)]
    pub target_id: Option<Uuid>,
    #[serde(default)]
    pub is_exclusion: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct BxgyInput {
    #[schema(value_type = String)]
    pub buy_quantity: Decimal,
    #[schema(value_type = String)]
    pub get_quantity: Decimal,
    /// `same_product`, `product` or `category`.
    pub get_target_type: String,
    #[serde(default)]
    pub get_target_id: Option<Uuid>,
    /// `free` or `percent`.
    pub get_discount_type: String,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub get_discount_value: Option<Decimal>,
    #[serde(default = "yes")]
    pub repeat: bool,
    #[serde(default)]
    pub auto_add_free_item: bool,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PromotionInput {
    pub name: String,
    #[serde(default)]
    pub name_bn: Option<String>,
    /// `product_discount`, `flash`, `bxgy` or `coupon`.
    #[serde(rename = "type")]
    pub kind: String,
    /// `percent`, `fixed_amount` or `fixed_price` (coupons: percent or fixed amount;
    /// ignored for Buy X Get Y).
    #[serde(default)]
    pub discount_type: Option<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub discount_value: Option<Decimal>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub max_discount_amount: Option<Decimal>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub min_purchase_amount: Option<Decimal>,
    /// Business local time, `YYYY-MM-DDTHH:MM[:SS]`.
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub starts_at: Option<NaiveDateTime>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub ends_at: Option<NaiveDateTime>,
    /// `mon`…`sun`; empty or absent = every day.
    #[serde(default)]
    pub active_days: Option<Vec<String>>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub active_time_from: Option<NaiveTime>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub active_time_to: Option<NaiveTime>,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub stackable: bool,
    /// `all` or `registered_only`.
    #[serde(default)]
    pub applies_to_customer: Option<String>,
    #[serde(default)]
    pub total_usage_limit: Option<i32>,
    #[serde(default)]
    pub per_customer_limit: Option<i32>,
    /// Flash: discounted units allowed.
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub quantity_limit: Option<Decimal>,
    #[serde(default)]
    pub bxgy: Option<BxgyInput>,
    pub targets: Vec<TargetInput>,
    /// Empty = every branch.
    #[serde(default)]
    pub branch_ids: Vec<Uuid>,
    /// On create: `draft` (default) or `active`.
    #[serde(default)]
    pub status: Option<String>,
}

// -------------------------------------------------------------------- output

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct TargetOut {
    pub target_type: String,
    pub target_id: Option<Uuid>,
    pub is_exclusion: bool,
    /// Product, category or brand name.
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct BxgyOut {
    #[schema(value_type = String)]
    pub buy_quantity: Decimal,
    #[schema(value_type = String)]
    pub get_quantity: Decimal,
    pub get_target_type: String,
    pub get_target_id: Option<Uuid>,
    pub get_target_name: Option<String>,
    pub get_discount_type: String,
    #[schema(value_type = String)]
    pub get_discount_value: Decimal,
    pub repeat: bool,
    pub auto_add_free_item: bool,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct PromotionSummary {
    pub id: Uuid,
    pub name: String,
    pub name_bn: Option<String>,
    #[serde(rename = "type")]
    #[sqlx(rename = "type")]
    pub kind: String,
    pub discount_type: String,
    #[schema(value_type = String)]
    pub discount_value: Decimal,
    /// What the owner set: `draft`, `active`, `paused` or `ended`.
    pub status: String,
    /// What it is now: `draft`, `scheduled`, `running`, `paused`, `expired`, `sold_out` or `ended`.
    #[sqlx(skip)]
    pub state: String,
    #[schema(value_type = Option<String>)]
    pub starts_at: Option<NaiveDateTime>,
    #[schema(value_type = Option<String>)]
    pub ends_at: Option<NaiveDateTime>,
    pub priority: i32,
    pub usage_count: i32,
    pub total_usage_limit: Option<i32>,
    #[schema(value_type = String)]
    pub quantity_used: Decimal,
    #[schema(value_type = Option<String>)]
    pub quantity_limit: Option<Decimal>,
    pub coupon_count: i64,
    /// Branch names; empty = every branch.
    #[sqlx(skip)]
    pub branches: Vec<String>,
    #[schema(value_type = Option<String>)]
    pub first_used_at: Option<NaiveDateTime>,
    #[schema(value_type = String)]
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct PromotionDetail {
    pub id: Uuid,
    pub name: String,
    pub name_bn: Option<String>,
    #[serde(rename = "type")]
    pub kind: String,
    pub discount_type: String,
    #[schema(value_type = String)]
    pub discount_value: Decimal,
    #[schema(value_type = Option<String>)]
    pub max_discount_amount: Option<Decimal>,
    #[schema(value_type = Option<String>)]
    pub min_purchase_amount: Option<Decimal>,
    #[serde(flatten)]
    pub schedule: Schedule,
    pub priority: i32,
    pub stackable: bool,
    pub applies_to_customer: String,
    pub total_usage_limit: Option<i32>,
    pub usage_count: i32,
    pub per_customer_limit: Option<i32>,
    #[schema(value_type = Option<String>)]
    pub quantity_limit: Option<Decimal>,
    #[schema(value_type = String)]
    pub quantity_used: Decimal,
    pub bxgy: Option<BxgyOut>,
    pub targets: Vec<TargetOut>,
    pub branch_ids: Vec<Uuid>,
    pub status: String,
    pub state: String,
    /// Used by a sale: the type, benefit, Buy X Get Y rule and products can no longer change.
    pub locked: bool,
    pub coupon_count: i64,
    /// Active products the targets match now.
    pub matched_products: i64,
    pub version: i32,
    #[schema(value_type = String)]
    pub created_at: NaiveDateTime,
    #[schema(value_type = String)]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct PromotionPage {
    pub data: Vec<PromotionSummary>,
    pub total: i64,
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct PromotionFilter {
    #[serde(rename = "type")]
    #[param(rename = "type")]
    pub kind: Option<String>,
    /// `draft`, `scheduled`, `running`, `paused`, `expired`, `ended` (state) or `all`.
    pub state: Option<String>,
    pub branch_id: Option<Uuid>,
    pub q: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

// --------------------------------------------------------------- validation

const TYPES: &[&str] = &["product_discount", "flash", "bxgy", "coupon"];

fn trimmed(v: &Option<String>) -> Option<String> {
    v.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned)
}

/// The validated row values, ready to write.
struct Clean {
    name: String,
    name_bn: Option<String>,
    kind: String,
    discount_type: String,
    discount_value: Decimal,
    max_discount_amount: Option<Decimal>,
    min_purchase_amount: Option<Decimal>,
    starts_at: Option<NaiveDateTime>,
    ends_at: Option<NaiveDateTime>,
    active_days: Option<Vec<String>>,
    active_time_from: Option<NaiveTime>,
    active_time_to: Option<NaiveTime>,
    priority: i32,
    stackable: bool,
    applies_to_customer: String,
    total_usage_limit: Option<i32>,
    per_customer_limit: Option<i32>,
    quantity_limit: Option<Decimal>,
    bxgy: Option<BxgyInput>,
    targets: Vec<TargetInput>,
    branch_ids: Vec<Uuid>,
}

async fn exists(tx: &mut TenantTx, table: &str, id: Uuid) -> AppResult<bool> {
    let sql = format!("SELECT COUNT(*) FROM {table} WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL");
    let n: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
        .bind(tx.tenant_id())
        .bind(id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(n > 0)
}

async fn validate(tx: &mut TenantTx, input: &PromotionInput) -> AppResult<Clean> {
    let name = input.name.trim().to_owned();
    if name.is_empty() || name.chars().count() > 120 {
        return Err(AppError::field("name", "REQUIRED", "enter a name of up to 120 characters"));
    }
    let name_bn = trimmed(&input.name_bn);
    if name_bn.as_ref().is_some_and(|n| n.chars().count() > 120) {
        return Err(AppError::field("name_bn", "TOO_LONG", "up to 120 characters"));
    }
    let kind = input.kind.trim().to_owned();
    if !TYPES.contains(&kind.as_str()) {
        return Err(AppError::field("type", "INVALID_TYPE", "choose a promotion type"));
    }

    // Benefit.
    let (discount_type, discount_value, bxgy) = if kind == "bxgy" {
        let Some(b) = input.bxgy.clone() else {
            return Err(AppError::field("bxgy", "REQUIRED", "set the Buy X Get Y rule"));
        };
        money::quantity(b.buy_quantity, "bxgy.buy_quantity", true)?;
        money::quantity(b.get_quantity, "bxgy.get_quantity", true)?;
        if !["same_product", "product", "category"].contains(&b.get_target_type.as_str()) {
            return Err(AppError::field("bxgy.get_target_type", "INVALID", "choose what the customer gets"));
        }
        if b.get_target_type != "same_product" {
            let Some(id) = b.get_target_id else {
                return Err(AppError::field("bxgy.get_target_id", "REQUIRED", "choose the free product or category"));
            };
            let table = if b.get_target_type == "product" { "products" } else { "categories" };
            if !exists(tx, table, id).await? {
                return Err(AppError::field(
                    "bxgy.get_target_id",
                    "NOT_FOUND",
                    "the free product or category was not found",
                ));
            }
        }
        let value = match b.get_discount_type.as_str() {
            "free" => Decimal::ZERO,
            "percent" => {
                let v = money::percent(b.get_discount_value.unwrap_or_default(), "bxgy.get_discount_value")?;
                if v.is_zero() {
                    return Err(AppError::field("bxgy.get_discount_value", "INVALID_PERCENT", "enter more than 0"));
                }
                v
            }
            _ => return Err(AppError::field("bxgy.get_discount_type", "INVALID", "free or a percentage off")),
        };
        (b.get_discount_type.clone(), value, Some(BxgyInput { get_discount_value: Some(value), ..b }))
    } else {
        let dt = input.discount_type.as_deref().unwrap_or("percent").to_owned();
        let allowed: &[&str] =
            if kind == "coupon" { &["percent", "fixed_amount"] } else { &["percent", "fixed_amount", "fixed_price"] };
        if !allowed.contains(&dt.as_str()) {
            return Err(AppError::field("discount_type", "INVALID", "choose how much off"));
        }
        let raw = input.discount_value.unwrap_or_default();
        let v = match dt.as_str() {
            "percent" => money::percent(raw, "discount_value")?,
            "fixed_price" => money::amount(raw, "discount_value", false)?,
            _ => money::amount(raw, "discount_value", true)?,
        };
        if dt == "percent" && v.is_zero() {
            return Err(AppError::field("discount_value", "INVALID_PERCENT", "enter more than 0"));
        }
        (dt, v, None)
    };
    let max_discount_amount =
        input.max_discount_amount.map(|v| money::amount(v, "max_discount_amount", true)).transpose()?;
    let min_purchase_amount =
        input.min_purchase_amount.map(|v| money::amount(v, "min_purchase_amount", true)).transpose()?;

    // Schedule.
    if kind == "flash" && (input.starts_at.is_none() || input.ends_at.is_none()) {
        return Err(AppError::field("ends_at", "REQUIRED", "a flash sale needs a start and an end"));
    }
    if let (Some(s), Some(e)) = (input.starts_at, input.ends_at)
        && e <= s
    {
        return Err(AppError::field("ends_at", "BEFORE_START", "the end must be after the start"));
    }
    let active_days = input.active_days.clone().filter(|d| !d.is_empty()).map(|days| {
        let mut out: Vec<String> = days.iter().map(|d| d.trim().to_lowercase()).collect();
        out.sort_by_key(|d| WEEKDAYS.iter().position(|w| w == d));
        out.dedup();
        out
    });
    if active_days.as_ref().is_some_and(|d| d.iter().any(|x| !WEEKDAYS.contains(&x.as_str()))) {
        return Err(AppError::field("active_days", "INVALID_DAY", "use mon … sun"));
    }
    if input.active_time_from.is_some() != input.active_time_to.is_some() {
        return Err(AppError::field("active_time_to", "REQUIRED", "set both the start and the end time"));
    }
    if input.active_time_from.is_some() && input.active_time_from == input.active_time_to {
        return Err(AppError::field("active_time_to", "BEFORE_START", "the end time must differ from the start"));
    }

    // Limits and eligibility.
    for (field, v) in [("total_usage_limit", input.total_usage_limit), ("per_customer_limit", input.per_customer_limit)]
    {
        if v.is_some_and(|n| n <= 0) {
            return Err(AppError::field(field, "INVALID", "enter 1 or more, or leave blank"));
        }
    }
    let quantity_limit = input.quantity_limit.map(|q| money::quantity(q, "quantity_limit", true)).transpose()?;
    let applies_to_customer = input.applies_to_customer.clone().unwrap_or_else(|| "all".into());
    if !["all", "registered_only"].contains(&applies_to_customer.as_str()) {
        return Err(AppError::field("applies_to_customer", "INVALID", "all or registered customers only"));
    }
    if !(-1000..=1000).contains(&input.priority) {
        return Err(AppError::field("priority", "OUT_OF_RANGE", "between -1000 and 1000"));
    }

    // Products.
    if input.targets.is_empty() || input.targets.len() > 200 {
        return Err(AppError::field("targets", "REQUIRED", "choose the products (up to 200 entries)"));
    }
    if !input.targets.iter().any(|t| !t.is_exclusion) {
        return Err(AppError::field(
            "targets",
            "REQUIRED",
            "choose at least one product, category or brand to include",
        ));
    }
    for (i, t) in input.targets.iter().enumerate() {
        let table = match t.target_type.as_str() {
            "product" => "products",
            "category" => "categories",
            "brand" => "brands",
            "all_products" if !t.is_exclusion => {
                if t.target_id.is_some() {
                    return Err(AppError::field(
                        &format!("targets[{i}].target_id"),
                        "INVALID",
                        "all products takes no id",
                    ));
                }
                continue;
            }
            _ => {
                return Err(AppError::field(
                    &format!("targets[{i}].target_type"),
                    "INVALID",
                    "product, category, brand or all products",
                ));
            }
        };
        let Some(id) = t.target_id else {
            return Err(AppError::field(&format!("targets[{i}].target_id"), "REQUIRED", "choose one"));
        };
        if !exists(tx, table, id).await? {
            return Err(AppError::field(&format!("targets[{i}].target_id"), "NOT_FOUND", "not found"));
        }
    }
    let mut branch_ids = input.branch_ids.clone();
    branch_ids.sort();
    branch_ids.dedup();
    for (i, id) in branch_ids.iter().enumerate() {
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM branches WHERE tenant_id = ? AND id = ?")
            .bind(tx.tenant_id())
            .bind(id)
            .fetch_one(&mut **tx)
            .await
            .map_err(map_db_error)?;
        if n == 0 {
            return Err(AppError::field(&format!("branch_ids[{i}]"), "NOT_FOUND", "branch not found"));
        }
    }
    Ok(Clean {
        name,
        name_bn,
        kind,
        discount_type,
        discount_value,
        max_discount_amount,
        min_purchase_amount,
        starts_at: input.starts_at,
        ends_at: input.ends_at,
        active_days,
        active_time_from: input.active_time_from,
        active_time_to: input.active_time_to,
        priority: input.priority,
        stackable: input.stackable,
        applies_to_customer,
        total_usage_limit: input.total_usage_limit,
        per_customer_limit: input.per_customer_limit,
        quantity_limit,
        bxgy,
        targets: input.targets.clone(),
        branch_ids,
    })
}

// ------------------------------------------------------------------- state

/// What a promotion is right now (for the list and the detail).
pub fn state(
    status: &str,
    schedule: &Schedule,
    now: NaiveDateTime,
    quantity_limit: Option<Decimal>,
    quantity_used: Decimal,
    usage: (i32, Option<i32>),
) -> &'static str {
    match status {
        "draft" => "draft",
        "paused" => "paused",
        "ended" => "ended",
        _ if schedule.starts_at.is_some_and(|s| now < s) => "scheduled",
        _ if schedule.ends_at.is_some_and(|e| now >= e) => "expired",
        _ if quantity_limit.is_some_and(|l| quantity_used >= l) || usage.1.is_some_and(|l| usage.0 >= l) => "sold_out",
        _ => "running",
    }
}

async fn local_now(tx: &mut TenantTx) -> AppResult<NaiveDateTime> {
    let ctx = pos_tenancy::context::load(tx).await?;
    Ok(ctx.now.with_timezone(&ctx.timezone).naive_local())
}

// -------------------------------------------------------------------- reads

const SUMMARY: &str = "SELECT p.id, p.name, p.name_bn, p.type, p.discount_type, p.discount_value, p.status, p.starts_at, \
     p.ends_at, p.priority, p.usage_count, p.total_usage_limit, p.quantity_used, p.quantity_limit, \
     (SELECT COUNT(*) FROM coupons c WHERE c.tenant_id = p.tenant_id AND c.promotion_id = p.id) AS coupon_count, \
     p.first_used_at, p.created_at FROM promotions p WHERE p.tenant_id = ";

pub async fn list(tx: &mut TenantTx, filter: &PromotionFilter) -> AppResult<PromotionPage> {
    let now = local_now(tx).await?;
    let limit = i64::from(filter.limit.unwrap_or(50).clamp(1, 200));
    let offset = i64::from(filter.offset.unwrap_or(0));
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(SUMMARY);
    q.push_bind(tx.tenant_id()).push(" AND p.deleted_at IS NULL");
    if let Some(k) = filter.kind.as_deref().filter(|k| !k.is_empty()) {
        q.push(" AND p.type = ").push_bind(k.to_owned());
    }
    if let Some(b) = filter.branch_id {
        q.push(
            " AND (NOT EXISTS (SELECT 1 FROM promotion_branches x WHERE x.tenant_id = p.tenant_id AND x.promotion_id = p.id) \
             OR EXISTS (SELECT 1 FROM promotion_branches x WHERE x.tenant_id = p.tenant_id AND x.promotion_id = p.id AND x.branch_id = ",
        )
        .push_bind(b)
        .push("))");
    }
    if let Some(text) = filter.q.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        let like = format!("%{text}%");
        q.push(" AND (p.name LIKE ").push_bind(like.clone()).push(" OR p.name_bn LIKE ").push_bind(like.clone());
        q.push(" OR EXISTS (SELECT 1 FROM coupons c WHERE c.tenant_id = p.tenant_id AND c.promotion_id = p.id AND c.code = ")
            .push_bind(text.to_owned())
            .push("))");
    }
    q.push(" ORDER BY p.created_at DESC, p.id DESC");
    let mut rows: Vec<PromotionSummary> = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    for r in &mut rows {
        let schedule = Schedule { starts_at: r.starts_at, ends_at: r.ends_at, ..Schedule::default() };
        r.state =
            state(&r.status, &schedule, now, r.quantity_limit, r.quantity_used, (r.usage_count, r.total_usage_limit))
                .to_owned();
    }
    if let Some(s) = filter.state.as_deref().filter(|s| !s.is_empty() && *s != "all") {
        rows.retain(|r| r.state == s || (s == "expired" && r.state == "sold_out"));
    }
    let total = i64::try_from(rows.len()).unwrap_or(i64::MAX);
    let mut page: Vec<PromotionSummary> = rows
        .into_iter()
        .skip(usize::try_from(offset).unwrap_or(0))
        .take(usize::try_from(limit).unwrap_or(50))
        .collect();
    for r in &mut page {
        r.branches = sqlx::query_scalar(
            "SELECT b.name FROM promotion_branches pb JOIN branches b ON b.tenant_id = pb.tenant_id AND b.id = pb.branch_id \
             WHERE pb.tenant_id = ? AND pb.promotion_id = ? ORDER BY b.name",
        )
        .bind(tx.tenant_id())
        .bind(r.id)
        .fetch_all(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }
    Ok(PromotionPage { data: page, total })
}

#[derive(sqlx::FromRow)]
struct DetailRow {
    #[sqlx(flatten)]
    row: Row,
    status: String,
    first_used_at: Option<NaiveDateTime>,
    version: i32,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

async fn load_row(tx: &mut TenantTx, id: Uuid, lock: bool) -> AppResult<DetailRow> {
    let sql = format!(
        "SELECT {}, p.status, p.first_used_at, p.version, p.created_at, p.updated_at FROM promotions p \
         WHERE p.tenant_id = ? AND p.id = ? AND p.deleted_at IS NULL{}",
        store::COLUMNS,
        if lock { " FOR UPDATE" } else { "" }
    );
    sqlx::query_as(sqlx::AssertSqlSafe(sql))
        .bind(tx.tenant_id())
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_db_error)?
        .ok_or(AppError::NotFound)
}

async fn name_of(tx: &mut TenantTx, kind: &str, id: Uuid) -> AppResult<Option<String>> {
    let table = match kind {
        "product" => "products",
        "category" => "categories",
        "brand" => "brands",
        _ => return Ok(None),
    };
    let sql = format!("SELECT name FROM {table} WHERE tenant_id = ? AND id = ?");
    sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
        .bind(tx.tenant_id())
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_db_error)
}

/// Active products the targets match (included and not excluded).
pub async fn count_matches(tx: &mut TenantTx, targets: &[TargetInput]) -> AppResult<i64> {
    fn cond(q: &mut QueryBuilder<MySql>, t: &TargetInput) {
        match (t.target_type.as_str(), t.target_id) {
            ("product", Some(id)) => {
                q.push("p.id = ").push_bind(id);
            }
            ("brand", Some(id)) => {
                q.push("p.brand_id = ").push_bind(id);
            }
            ("category", Some(id)) => {
                q.push("(p.category_id = ").push_bind(id);
                q.push(" OR pc.parent_id = ").push_bind(id);
                q.push(" OR gc.parent_id = ").push_bind(id).push(")");
            }
            ("all_products", _) => {
                q.push("TRUE");
            }
            _ => {
                q.push("FALSE");
            }
        }
    }
    if !targets.iter().any(|t| !t.is_exclusion) {
        return Ok(0);
    }
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "SELECT COUNT(*) FROM products p \
         LEFT JOIN categories pc ON pc.tenant_id = p.tenant_id AND pc.id = p.category_id \
         LEFT JOIN categories gc ON gc.tenant_id = pc.tenant_id AND gc.id = pc.parent_id \
         WHERE p.tenant_id = ",
    );
    q.push_bind(tx.tenant_id()).push(" AND p.deleted_at IS NULL AND p.status = 'active' AND (");
    for (i, t) in targets.iter().filter(|t| !t.is_exclusion).enumerate() {
        if i > 0 {
            q.push(" OR ");
        }
        cond(&mut q, t);
    }
    q.push(")");
    for t in targets.iter().filter(|t| t.is_exclusion) {
        q.push(" AND NOT (");
        cond(&mut q, t);
        q.push(")");
    }
    q.build_query_scalar().fetch_one(&mut **tx).await.map_err(map_db_error)
}

pub async fn get(tx: &mut TenantTx, id: Uuid) -> AppResult<PromotionDetail> {
    let d = load_row(tx, id, false).await?;
    let now = local_now(tx).await?;
    let r = &d.row;
    let raw: Vec<(String, Option<Uuid>, bool)> = sqlx::query_as(
        "SELECT target_type, target_id, is_exclusion FROM promotion_targets WHERE tenant_id = ? AND promotion_id = ? ORDER BY id",
    )
    .bind(tx.tenant_id())
    .bind(id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let mut targets = Vec::with_capacity(raw.len());
    for (tt, tid, ex) in raw {
        let name = match tid {
            Some(t) => name_of(tx, &tt, t).await?,
            None => None,
        };
        targets.push(TargetOut { target_type: tt, target_id: tid, is_exclusion: ex, name });
    }
    let inputs: Vec<TargetInput> = targets
        .iter()
        .map(|t| TargetInput {
            target_type: t.target_type.clone(),
            target_id: t.target_id,
            is_exclusion: t.is_exclusion,
        })
        .collect();
    let matched_products = count_matches(tx, &inputs).await?;
    let branch_ids: Vec<Uuid> =
        sqlx::query_scalar("SELECT branch_id FROM promotion_branches WHERE tenant_id = ? AND promotion_id = ?")
            .bind(tx.tenant_id())
            .bind(id)
            .fetch_all(&mut **tx)
            .await
            .map_err(map_db_error)?;
    let coupon_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM coupons WHERE tenant_id = ? AND promotion_id = ?")
        .bind(tx.tenant_id())
        .bind(id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
    let bxgy = match (r.kind.as_str(), r.bxgy_get_target_type.as_deref()) {
        ("bxgy", Some(gt)) => {
            let get_target_name = match (gt, r.bxgy_get_target_id) {
                ("product" | "category", Some(t)) => name_of(tx, gt, t).await?,
                _ => None,
            };
            Some(BxgyOut {
                buy_quantity: r.bxgy_buy_quantity.unwrap_or(Decimal::ONE),
                get_quantity: r.bxgy_get_quantity.unwrap_or(Decimal::ONE),
                get_target_type: gt.to_owned(),
                get_target_id: r.bxgy_get_target_id,
                get_target_name,
                get_discount_type: r.bxgy_get_discount_type.clone().unwrap_or_else(|| "free".into()),
                get_discount_value: r.bxgy_get_discount_value.unwrap_or_default(),
                repeat: r.bxgy_repeat,
                auto_add_free_item: r.bxgy_auto_add,
            })
        }
        _ => None,
    };
    let schedule = Schedule::of(r);
    Ok(PromotionDetail {
        id: r.id,
        name: r.name.clone(),
        name_bn: r.name_bn.clone(),
        kind: r.kind.clone(),
        discount_type: r.discount_type.clone(),
        discount_value: r.discount_value,
        max_discount_amount: r.max_discount_amount,
        min_purchase_amount: r.min_purchase_amount,
        state: state(
            &d.status,
            &schedule,
            now,
            r.quantity_limit,
            r.quantity_used,
            (r.usage_count, r.total_usage_limit),
        )
        .to_owned(),
        schedule,
        priority: r.priority,
        stackable: r.stackable,
        applies_to_customer: r.applies_to_customer.clone(),
        total_usage_limit: r.total_usage_limit,
        usage_count: r.usage_count,
        per_customer_limit: r.per_customer_limit,
        quantity_limit: r.quantity_limit,
        quantity_used: r.quantity_used,
        bxgy,
        targets,
        branch_ids,
        status: d.status.clone(),
        locked: d.first_used_at.is_some(),
        coupon_count,
        matched_products,
        version: d.version,
        created_at: d.created_at,
        updated_at: d.updated_at,
    })
}

// ------------------------------------------------------------------- writes

async fn write_children(tx: &mut TenantTx, id: Uuid, c: &Clean, targets: bool) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    if targets {
        sqlx::query("DELETE FROM promotion_targets WHERE tenant_id = ? AND promotion_id = ?")
            .bind(tenant_id)
            .bind(id)
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
        let mut q: QueryBuilder<MySql> = QueryBuilder::new(
            "INSERT INTO promotion_targets (tenant_id, id, promotion_id, target_type, target_id, is_exclusion) ",
        );
        q.push_values(&c.targets, |mut b, t| {
            let all = t.target_type == "all_products";
            b.push_bind(tenant_id)
                .push_bind(new_id())
                .push_bind(id)
                .push_bind(t.target_type.clone())
                .push_bind(if all { None } else { t.target_id })
                .push_bind(t.is_exclusion);
        });
        q.build().execute(&mut **tx).await.map_err(map_db_error)?;
    }
    sqlx::query("DELETE FROM promotion_branches WHERE tenant_id = ? AND promotion_id = ?")
        .bind(tenant_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    if !c.branch_ids.is_empty() {
        let mut q: QueryBuilder<MySql> =
            QueryBuilder::new("INSERT INTO promotion_branches (tenant_id, promotion_id, branch_id) ");
        q.push_values(&c.branch_ids, |mut b, bid| {
            b.push_bind(tenant_id).push_bind(id).push_bind(*bid);
        });
        q.build().execute(&mut **tx).await.map_err(map_db_error)?;
    }
    Ok(())
}

pub async fn create(tx: &mut TenantTx, meta: &RequestMeta, input: &PromotionInput) -> AppResult<PromotionDetail> {
    let c = validate(tx, input).await?;
    let status = input.status.clone().unwrap_or_else(|| "draft".into());
    if !["draft", "active"].contains(&status.as_str()) {
        return Err(AppError::field("status", "INVALID", "draft or active"));
    }
    let id = new_id();
    let b = c.bxgy.as_ref();
    sqlx::query(
        "INSERT INTO promotions (tenant_id, id, name, name_bn, type, discount_type, discount_value, max_discount_amount, \
         min_purchase_amount, starts_at, ends_at, active_days, active_time_from, active_time_to, priority, stackable, \
         applies_to_customer, total_usage_limit, per_customer_limit, quantity_limit, bxgy_buy_quantity, bxgy_get_quantity, \
         bxgy_get_target_type, bxgy_get_target_id, bxgy_get_discount_type, bxgy_get_discount_value, bxgy_repeat, bxgy_auto_add, \
         status, created_by, updated_by) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(tx.tenant_id())
    .bind(id)
    .bind(&c.name)
    .bind(&c.name_bn)
    .bind(&c.kind)
    .bind(&c.discount_type)
    .bind(c.discount_value)
    .bind(c.max_discount_amount)
    .bind(c.min_purchase_amount)
    .bind(c.starts_at)
    .bind(c.ends_at)
    .bind(c.active_days.as_ref().map(sqlx::types::Json))
    .bind(c.active_time_from)
    .bind(c.active_time_to)
    .bind(c.priority)
    .bind(c.stackable)
    .bind(&c.applies_to_customer)
    .bind(c.total_usage_limit)
    .bind(c.per_customer_limit)
    .bind(c.quantity_limit)
    .bind(b.map(|b| b.buy_quantity))
    .bind(b.map(|b| b.get_quantity))
    .bind(b.map(|b| b.get_target_type.clone()))
    .bind(b.and_then(|b| (b.get_target_type != "same_product").then_some(b.get_target_id).flatten()))
    .bind(b.map(|b| b.get_discount_type.clone()))
    .bind(b.and_then(|b| b.get_discount_value))
    .bind(b.is_none_or(|b| b.repeat))
    .bind(b.is_some_and(|b| b.auto_add_free_item))
    .bind(&status)
    .bind(tx.user_id())
    .bind(tx.user_id())
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    write_children(tx, id, &c, true).await?;
    let detail = get(tx, id).await?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("promotion.created").entity("promotion", id).change(None, Some(json!(detail))),
    )
    .await?;
    Ok(detail)
}

pub async fn update(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    id: Uuid,
    expected_version: Option<i32>,
    input: &PromotionInput,
) -> AppResult<PromotionDetail> {
    let current = load_row(tx, id, true).await?;
    if expected_version.is_some_and(|v| v != current.version) {
        return Err(AppError::PreconditionFailed {
            code: "VERSION_MISMATCH",
            message: "the promotion was changed by someone else; reload it".into(),
        });
    }
    if current.status == "ended" {
        return Err(AppError::Conflict {
            code: "PROMOTION_ENDED",
            message: "an ended promotion cannot be edited; duplicate it".into(),
        });
    }
    let before = get(tx, id).await?;
    let c = validate(tx, input).await?;
    let locked = current.first_used_at.is_some();
    if locked {
        // §7: once used, the benefit and products stay as they were (reports).
        let r = &current.row;
        let b = c.bxgy.as_ref();
        let same_targets = {
            let mut a: Vec<(String, Option<Uuid>, bool)> =
                before.targets.iter().map(|t| (t.target_type.clone(), t.target_id, t.is_exclusion)).collect();
            let mut z: Vec<(String, Option<Uuid>, bool)> = c
                .targets
                .iter()
                .map(|t| {
                    (
                        t.target_type.clone(),
                        if t.target_type == "all_products" { None } else { t.target_id },
                        t.is_exclusion,
                    )
                })
                .collect();
            a.sort();
            z.sort();
            a == z
        };
        let same_benefit = c.kind == r.kind
            && c.discount_type == r.discount_type
            && c.discount_value == r.discount_value
            && c.max_discount_amount == r.max_discount_amount
            && b.map(|b| b.buy_quantity) == r.bxgy_buy_quantity
            && b.map(|b| b.get_quantity) == r.bxgy_get_quantity
            && b.map(|b| b.get_target_type.clone()) == r.bxgy_get_target_type
            && b.and_then(|b| b.get_discount_value).unwrap_or_default()
                == r.bxgy_get_discount_value.unwrap_or_default();
        if !same_benefit || !same_targets {
            return Err(AppError::Conflict {
                code: "PROMOTION_LOCKED",
                message:
                    "this promotion has been used: its benefit and products cannot change; end it and duplicate it"
                        .into(),
            });
        }
    }
    let b = c.bxgy.as_ref();
    sqlx::query(
        "UPDATE promotions SET name = ?, name_bn = ?, type = ?, discount_type = ?, discount_value = ?, max_discount_amount = ?, \
         min_purchase_amount = ?, starts_at = ?, ends_at = ?, active_days = ?, active_time_from = ?, active_time_to = ?, \
         priority = ?, stackable = ?, applies_to_customer = ?, total_usage_limit = ?, per_customer_limit = ?, quantity_limit = ?, \
         bxgy_buy_quantity = ?, bxgy_get_quantity = ?, bxgy_get_target_type = ?, bxgy_get_target_id = ?, \
         bxgy_get_discount_type = ?, bxgy_get_discount_value = ?, bxgy_repeat = ?, bxgy_auto_add = ?, \
         version = version + 1, updated_by = ? WHERE tenant_id = ? AND id = ?",
    )
    .bind(&c.name)
    .bind(&c.name_bn)
    .bind(&c.kind)
    .bind(&c.discount_type)
    .bind(c.discount_value)
    .bind(c.max_discount_amount)
    .bind(c.min_purchase_amount)
    .bind(c.starts_at)
    .bind(c.ends_at)
    .bind(c.active_days.as_ref().map(sqlx::types::Json))
    .bind(c.active_time_from)
    .bind(c.active_time_to)
    .bind(c.priority)
    .bind(c.stackable)
    .bind(&c.applies_to_customer)
    .bind(c.total_usage_limit)
    .bind(c.per_customer_limit)
    .bind(c.quantity_limit)
    .bind(b.map(|b| b.buy_quantity))
    .bind(b.map(|b| b.get_quantity))
    .bind(b.map(|b| b.get_target_type.clone()))
    .bind(b.and_then(|b| (b.get_target_type != "same_product").then_some(b.get_target_id).flatten()))
    .bind(b.map(|b| b.get_discount_type.clone()))
    .bind(b.and_then(|b| b.get_discount_value))
    .bind(b.is_none_or(|b| b.repeat))
    .bind(b.is_some_and(|b| b.auto_add_free_item))
    .bind(tx.user_id())
    .bind(tx.tenant_id())
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    write_children(tx, id, &c, !locked).await?;
    let after = get(tx, id).await?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("promotion.updated").entity("promotion", id).change(Some(json!(before)), Some(json!(after))),
    )
    .await?;
    Ok(after)
}

/// `activate` / `resume` (→ active), `pause`, `end`.
pub async fn set_status(tx: &mut TenantTx, meta: &RequestMeta, id: Uuid, action: &str) -> AppResult<PromotionDetail> {
    let current = load_row(tx, id, true).await?;
    let next = match (action, current.status.as_str()) {
        (_, "ended") => {
            return Err(AppError::Conflict {
                code: "PROMOTION_ENDED",
                message: "the promotion has ended; duplicate it".into(),
            });
        }
        ("activate" | "resume", "draft" | "paused" | "active") => "active",
        ("pause", "active" | "paused") => "paused",
        ("end", _) => "ended",
        ("pause", _) => {
            return Err(AppError::Conflict {
                code: "INVALID_STATUS",
                message: "only an active promotion can be paused".into(),
            });
        }
        _ => return Err(AppError::NotFound),
    };
    sqlx::query(
        "UPDATE promotions SET status = ?, version = version + 1, updated_by = ? WHERE tenant_id = ? AND id = ?",
    )
    .bind(next)
    .bind(tx.user_id())
    .bind(tx.tenant_id())
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new(match next {
            "active" => "promotion.activated",
            "paused" => "promotion.paused",
            _ => "promotion.ended",
        })
        .entity("promotion", id)
        .change(Some(json!({ "status": current.status })), Some(json!({ "status": next }))),
    )
    .await?;
    get(tx, id).await
}

/// A draft copy with the same benefit, products, schedule and limits (no coupons, counters at zero).
pub async fn duplicate(tx: &mut TenantTx, meta: &RequestMeta, id: Uuid) -> AppResult<PromotionDetail> {
    let d = get(tx, id).await?;
    let input = PromotionInput {
        name: format!("{} (copy)", d.name).chars().take(120).collect(),
        name_bn: d.name_bn.clone(),
        kind: d.kind.clone(),
        discount_type: Some(d.discount_type.clone()),
        discount_value: Some(d.discount_value),
        max_discount_amount: d.max_discount_amount,
        min_purchase_amount: d.min_purchase_amount,
        starts_at: d.schedule.starts_at,
        ends_at: d.schedule.ends_at,
        active_days: d.schedule.active_days.clone(),
        active_time_from: d.schedule.active_time_from,
        active_time_to: d.schedule.active_time_to,
        priority: d.priority,
        stackable: d.stackable,
        applies_to_customer: Some(d.applies_to_customer.clone()),
        total_usage_limit: d.total_usage_limit,
        per_customer_limit: d.per_customer_limit,
        quantity_limit: d.quantity_limit,
        bxgy: d.bxgy.as_ref().map(|b| BxgyInput {
            buy_quantity: b.buy_quantity,
            get_quantity: b.get_quantity,
            get_target_type: b.get_target_type.clone(),
            get_target_id: b.get_target_id,
            get_discount_type: b.get_discount_type.clone(),
            get_discount_value: Some(b.get_discount_value),
            repeat: b.repeat,
            auto_add_free_item: b.auto_add_free_item,
        }),
        targets: d
            .targets
            .iter()
            .map(|t| TargetInput {
                target_type: t.target_type.clone(),
                target_id: t.target_id,
                is_exclusion: t.is_exclusion,
            })
            .collect(),
        branch_ids: d.branch_ids.clone(),
        status: Some("draft".into()),
    };
    let copy = create(tx, meta, &input).await?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("promotion.duplicated").entity("promotion", copy.id).change(Some(json!({ "from": id })), None),
    )
    .await?;
    Ok(copy)
}

/// Removes a promotion that was never used (used ones are ended instead).
pub async fn delete(tx: &mut TenantTx, meta: &RequestMeta, id: Uuid) -> AppResult<()> {
    let current = load_row(tx, id, true).await?;
    if current.first_used_at.is_some() {
        return Err(AppError::Conflict {
            code: "PROMOTION_USED",
            message: "a used promotion cannot be deleted; end it instead".into(),
        });
    }
    sqlx::query(
        "UPDATE promotions SET deleted_at = NOW(), status = 'ended', updated_by = ? WHERE tenant_id = ? AND id = ?",
    )
    .bind(tx.user_id())
    .bind(tx.tenant_id())
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    sqlx::query("UPDATE coupons SET status = 'disabled' WHERE tenant_id = ? AND promotion_id = ?")
        .bind(tx.tenant_id())
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("promotion.deleted")
            .entity("promotion", id)
            .change(Some(json!({ "name": current.row.name })), None),
    )
    .await?;
    Ok(())
}

// ------------------------------------------------------------------ coupons

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct Coupon {
    pub id: Uuid,
    pub promotion_id: Uuid,
    pub code: String,
    /// `public` (one shared code) or `unique` (single-use generated).
    #[serde(rename = "type")]
    #[sqlx(rename = "type")]
    pub kind: String,
    pub usage_limit: Option<i32>,
    pub used_count: i32,
    pub assigned_customer_id: Option<Uuid>,
    pub customer_name: Option<String>,
    #[schema(value_type = Option<String>)]
    pub expires_at: Option<NaiveDateTime>,
    /// `active`, `used` or `disabled`.
    pub status: String,
    #[schema(value_type = String)]
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct CouponPage {
    pub data: Vec<Coupon>,
    pub total: i64,
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct CouponFilter {
    pub q: Option<String>,
    /// `active`, `used` or `disabled`.
    pub status: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CouponInput {
    pub code: String,
    #[serde(default)]
    pub usage_limit: Option<i32>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub expires_at: Option<NaiveDateTime>,
    /// A personal coupon for this customer.
    #[serde(default)]
    pub assigned_customer_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct GenerateInput {
    /// 1–1000 codes.
    pub count: u32,
    #[serde(default)]
    pub prefix: Option<String>,
    /// Uses per code (default 1).
    #[serde(default)]
    pub usage_limit: Option<i32>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub expires_at: Option<NaiveDateTime>,
}

const COUPON_SELECT: &str = "SELECT c.id, c.promotion_id, c.code, c.type, c.usage_limit, c.used_count, c.assigned_customer_id, \
     cu.name AS customer_name, c.expires_at, c.status, c.created_at FROM coupons c \
     LEFT JOIN customers cu ON cu.tenant_id = c.tenant_id AND cu.id = c.assigned_customer_id WHERE c.tenant_id = ";

async fn coupon_promotion(tx: &mut TenantTx, promotion_id: Uuid) -> AppResult<DetailRow> {
    let p = load_row(tx, promotion_id, true).await?;
    if p.row.kind != "coupon" {
        return Err(AppError::Conflict { code: "NOT_A_COUPON", message: "codes belong to coupon promotions".into() });
    }
    if p.status == "ended" {
        return Err(AppError::Conflict { code: "PROMOTION_ENDED", message: "the promotion has ended".into() });
    }
    Ok(p)
}

fn clean_code(code: &str) -> AppResult<String> {
    let code = code.trim().to_uppercase();
    if !(3..=40).contains(&code.chars().count())
        || !code.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(AppError::field("code", "INVALID_CODE", "3–40 letters, digits, - or _"));
    }
    Ok(code)
}

pub async fn coupons(tx: &mut TenantTx, promotion_id: Uuid, filter: &CouponFilter) -> AppResult<CouponPage> {
    load_row(tx, promotion_id, false).await?;
    let limit = i64::from(filter.limit.unwrap_or(50).clamp(1, 1000));
    let offset = i64::from(filter.offset.unwrap_or(0));
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(COUPON_SELECT);
    q.push_bind(tx.tenant_id()).push(" AND c.promotion_id = ").push_bind(promotion_id);
    if let Some(s) = filter.status.as_deref().filter(|s| !s.is_empty()) {
        q.push(" AND c.status = ").push_bind(s.to_owned());
    }
    if let Some(t) = filter.q.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        q.push(" AND c.code LIKE ").push_bind(format!("%{t}%"));
    }
    let mut cq: QueryBuilder<MySql> = QueryBuilder::new("SELECT COUNT(*) FROM coupons c WHERE c.tenant_id = ");
    cq.push_bind(tx.tenant_id()).push(" AND c.promotion_id = ").push_bind(promotion_id);
    if let Some(s) = filter.status.as_deref().filter(|s| !s.is_empty()) {
        cq.push(" AND c.status = ").push_bind(s.to_owned());
    }
    if let Some(t) = filter.q.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        cq.push(" AND c.code LIKE ").push_bind(format!("%{t}%"));
    }
    let total: i64 = cq.build_query_scalar().fetch_one(&mut **tx).await.map_err(map_db_error)?;
    q.push(" ORDER BY c.created_at DESC, c.code LIMIT ").push_bind(limit).push(" OFFSET ").push_bind(offset);
    let data = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    Ok(CouponPage { data, total })
}

async fn get_coupon(tx: &mut TenantTx, id: Uuid) -> AppResult<Coupon> {
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(COUPON_SELECT);
    q.push_bind(tx.tenant_id()).push(" AND c.id = ").push_bind(id);
    q.build_query_as().fetch_optional(&mut **tx).await.map_err(map_db_error)?.ok_or(AppError::NotFound)
}

pub async fn add_coupon(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    promotion_id: Uuid,
    input: &CouponInput,
) -> AppResult<Coupon> {
    coupon_promotion(tx, promotion_id).await?;
    let code = clean_code(&input.code)?;
    if input.usage_limit.is_some_and(|n| n <= 0) {
        return Err(AppError::field("usage_limit", "INVALID", "enter 1 or more, or leave blank"));
    }
    if let Some(cid) = input.assigned_customer_id {
        let n: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM customers WHERE tenant_id = ? AND id = ? AND NOT is_walk_in")
                .bind(tx.tenant_id())
                .bind(cid)
                .fetch_one(&mut **tx)
                .await
                .map_err(map_db_error)?;
        if n == 0 {
            return Err(AppError::field("assigned_customer_id", "NOT_FOUND", "choose a registered customer"));
        }
    }
    let id = new_id();
    let done = sqlx::query(
        "INSERT IGNORE INTO coupons (tenant_id, id, promotion_id, code, type, usage_limit, assigned_customer_id, expires_at, created_by) \
         VALUES (?, ?, ?, ?, 'public', ?, ?, ?, ?)",
    )
    .bind(tx.tenant_id())
    .bind(id)
    .bind(promotion_id)
    .bind(&code)
    .bind(input.usage_limit)
    .bind(input.assigned_customer_id)
    .bind(input.expires_at)
    .bind(tx.user_id())
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if done.rows_affected() == 0 {
        return Err(AppError::field("code", "CODE_TAKEN", "another coupon already has this code"));
    }
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("coupon.created").entity("promotion", promotion_id).change(None, Some(json!({ "code": code }))),
    )
    .await?;
    get_coupon(tx, id).await
}

/// Unambiguous characters (no 0/O, 1/I/L).
const ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";

fn random_code(prefix: &str, len: usize) -> AppResult<String> {
    let mut buf = vec![0u8; len];
    getrandom::fill(&mut buf).map_err(|e| AppError::internal(e.to_string()))?;
    let body: String = buf.iter().map(|b| ALPHABET[usize::from(*b) % ALPHABET.len()] as char).collect();
    Ok(format!("{prefix}{body}"))
}

pub async fn generate(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    promotion_id: Uuid,
    input: &GenerateInput,
) -> AppResult<Vec<Coupon>> {
    coupon_promotion(tx, promotion_id).await?;
    if !(1..=1000).contains(&input.count) {
        return Err(AppError::field("count", "OUT_OF_RANGE", "between 1 and 1000 codes"));
    }
    let prefix = input.prefix.as_deref().map(str::trim).unwrap_or("").to_uppercase();
    if prefix.chars().count() > 20 || !prefix.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err(AppError::field("prefix", "INVALID_CODE", "up to 20 letters, digits, - or _"));
    }
    let usage_limit = input.usage_limit.unwrap_or(1);
    if usage_limit <= 0 {
        return Err(AppError::field("usage_limit", "INVALID", "enter 1 or more"));
    }
    let batch_start: NaiveDateTime =
        sqlx::query_scalar("SELECT NOW()").fetch_one(&mut **tx).await.map_err(map_db_error)?;
    let mut made = 0u32;
    // Collisions are rare; a code that is taken is simply drawn again.
    for _ in 0..5 {
        let want = input.count - made;
        if want == 0 {
            break;
        }
        let codes: Vec<String> = (0..want).map(|_| random_code(&prefix, 8)).collect::<AppResult<_>>()?;
        let mut q: QueryBuilder<MySql> = QueryBuilder::new(
            "INSERT IGNORE INTO coupons (tenant_id, id, promotion_id, code, type, usage_limit, expires_at, created_by) ",
        );
        q.push_values(&codes, |mut b, code| {
            b.push_bind(tx.tenant_id())
                .push_bind(new_id())
                .push_bind(promotion_id)
                .push_bind(code.clone())
                .push_bind("unique")
                .push_bind(usage_limit)
                .push_bind(input.expires_at)
                .push_bind(tx.user_id());
        });
        let done = q.build().execute(&mut **tx).await.map_err(map_db_error)?;
        made += u32::try_from(done.rows_affected()).unwrap_or(0);
    }
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("coupon.generated")
            .entity("promotion", promotion_id)
            .change(None, Some(json!({ "count": made, "prefix": prefix }))),
    )
    .await?;
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(COUPON_SELECT);
    q.push_bind(tx.tenant_id())
        .push(" AND c.promotion_id = ")
        .push_bind(promotion_id)
        .push(" AND c.type = 'unique' AND c.created_by = ")
        .push_bind(tx.user_id())
        .push(" AND c.created_at >= ")
        .push_bind(batch_start)
        .push(" ORDER BY c.code");
    q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)
}

/// `disable` or `enable` one code.
pub async fn set_coupon_status(tx: &mut TenantTx, meta: &RequestMeta, id: Uuid, enable: bool) -> AppResult<Coupon> {
    let c = get_coupon(tx, id).await?;
    let next = if enable {
        if c.usage_limit.is_some_and(|l| c.used_count >= l) { "used" } else { "active" }
    } else {
        "disabled"
    };
    sqlx::query("UPDATE coupons SET status = ? WHERE tenant_id = ? AND id = ?")
        .bind(next)
        .bind(tx.tenant_id())
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new(if enable { "coupon.enabled" } else { "coupon.disabled" })
            .entity("promotion", c.promotion_id)
            .change(Some(json!({ "code": c.code, "status": c.status })), Some(json!({ "status": next }))),
    )
    .await?;
    get_coupon(tx, id).await
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct Redemption {
    pub id: Uuid,
    pub code: String,
    pub sale_id: Uuid,
    pub invoice_no: String,
    pub customer_name: String,
    pub cashier_name: Option<String>,
    pub branch_name: String,
    #[schema(value_type = String)]
    pub discount_amount: Decimal,
    /// `applied` or `reversed`.
    pub status: String,
    #[schema(value_type = String)]
    pub created_at: NaiveDateTime,
}

pub async fn redemptions(tx: &mut TenantTx, promotion_id: Uuid) -> AppResult<Vec<Redemption>> {
    load_row(tx, promotion_id, false).await?;
    sqlx::query_as(
        "SELECT r.id, c.code, r.sale_id, s.invoice_no, cu.name AS customer_name, \
         (SELECT u.full_name FROM users u WHERE u.id = r.user_id) AS cashier_name, b.name AS branch_name, \
         r.discount_amount, r.status, r.created_at FROM coupon_redemptions r \
         JOIN coupons c ON c.tenant_id = r.tenant_id AND c.id = r.coupon_id \
         JOIN sales s ON s.tenant_id = r.tenant_id AND s.id = r.sale_id \
         JOIN customers cu ON cu.tenant_id = r.tenant_id AND cu.id = r.customer_id \
         JOIN branches b ON b.tenant_id = r.tenant_id AND b.id = r.branch_id \
         WHERE r.tenant_id = ? AND r.promotion_id = ? ORDER BY r.created_at DESC LIMIT 500",
    )
    .bind(tx.tenant_id())
    .bind(promotion_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;

    fn at(h: u32, m: u32) -> NaiveDateTime {
        // 2026-11-13 is a Friday.
        NaiveDate::from_ymd_opt(2026, 11, 13).and_then(|d| d.and_hms_opt(h, m, 0)).expect("date")
    }

    #[test]
    fn schedule_windows() {
        let t = |h, m| NaiveTime::from_hms_opt(h, m, 0);
        let happy = Schedule { active_time_from: t(17, 0), active_time_to: t(19, 0), ..Schedule::default() };
        assert!(happy.open_at(at(17, 0)) && happy.open_at(at(18, 59)));
        assert!(!happy.open_at(at(19, 0)) && !happy.open_at(at(16, 59)));
        let night = Schedule { active_time_from: t(22, 0), active_time_to: t(2, 0), ..Schedule::default() };
        assert!(night.open_at(at(23, 0)) && night.open_at(at(1, 0)) && !night.open_at(at(12, 0)));
        let weekend = Schedule { active_days: Some(vec!["fri".into(), "sat".into()]), ..Schedule::default() };
        assert!(weekend.open_at(at(10, 0)));
        let monday = Schedule { active_days: Some(vec!["mon".into()]), ..Schedule::default() };
        assert!(!monday.open_at(at(10, 0)));
        let dated = Schedule { starts_at: Some(at(10, 0)), ends_at: Some(at(12, 0)), ..Schedule::default() };
        assert!(!dated.open_at(at(9, 59)) && dated.open_at(at(10, 0)) && !dated.open_at(at(12, 0)));
    }

    #[test]
    fn states() {
        let s = Schedule { starts_at: Some(at(10, 0)), ends_at: Some(at(12, 0)), ..Schedule::default() };
        let z = Decimal::ZERO;
        assert_eq!(state("active", &s, at(9, 0), None, z, (0, None)), "scheduled");
        assert_eq!(state("active", &s, at(11, 0), None, z, (0, None)), "running");
        assert_eq!(state("active", &s, at(12, 0), None, z, (0, None)), "expired");
        assert_eq!(state("active", &s, at(11, 0), Some(Decimal::TEN), Decimal::TEN, (0, None)), "sold_out");
        assert_eq!(state("paused", &s, at(11, 0), None, z, (0, None)), "paused");
    }

    #[test]
    fn generated_codes() {
        let c = random_code("EID-", 8).expect("code");
        assert!(c.starts_with("EID-") && c.len() == 12);
        assert!(c[4..].bytes().all(|b| ALPHABET.contains(&b)));
    }
}
