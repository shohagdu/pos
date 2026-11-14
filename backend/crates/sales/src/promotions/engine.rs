//! The promotion engine (Final_docs/05_discount-system.md §4–5): turns a cart
//! and the promotions that are live for it into a discount amount per line.
//! Pure and deterministic; the POS screen runs the same algorithm
//! (frontend `lib/promotions.ts`) and both are locked by the shared vectors in
//! `frontend/src/lib/promotions.vectors.json`.
//!
//! The caller decides which promotions are live (status, schedule, branch,
//! customer eligibility, usage limits) and passes them in input order; the
//! engine only matches products, picks the best offer and does the money.
//!
//! Order (§5): automatic line promotions (product discount, flash) → Buy X
//! Get Y on the remaining lines → coupon → minimum selling price cap. The
//! result feeds `pricing::calculate` as each line's `promo_discount`, before
//! the cashier's own line discount, the invoice discount, VAT and rounding.

use pos_core::money::round2;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PromoKind {
    ProductDiscount,
    Flash,
    Bxgy,
    Coupon,
}

impl PromoKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ProductDiscount => "product_discount",
            Self::Flash => "flash",
            Self::Bxgy => "bxgy",
            Self::Coupon => "coupon",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BenefitType {
    /// `value` percent off.
    Percent,
    /// `value` off each unit (line promotions) or off the order (coupons).
    FixedAmount,
    /// Each unit sells for `value`.
    FixedPrice,
    /// The unit is free (Buy X Get Y).
    Free,
}

impl BenefitType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Percent => "percent",
            Self::FixedAmount => "fixed_amount",
            Self::FixedPrice => "fixed_price",
            Self::Free => "free",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TargetType {
    Product,
    Category,
    Brand,
    AllProducts,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Target {
    pub target_type: TargetType,
    pub target_id: Option<Uuid>,
    #[serde(default)]
    pub is_exclusion: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum GetTargetType {
    SameProduct,
    Product,
    Category,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Bxgy {
    #[schema(value_type = String)]
    pub buy_quantity: Decimal,
    #[schema(value_type = String)]
    pub get_quantity: Decimal,
    pub get_target_type: GetTargetType,
    #[serde(default)]
    pub get_target_id: Option<Uuid>,
    /// `free` or `percent`.
    pub get_discount_type: BenefitType,
    #[serde(default)]
    #[schema(value_type = String)]
    pub get_discount_value: Decimal,
    /// Buy 2 get 2, buy 3 get 3… (else once per invoice).
    #[serde(default)]
    pub repeat: bool,
    /// Offer the free item when it is missing from the cart.
    #[serde(default)]
    pub auto_add_free_item: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Promotion {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub name_bn: Option<String>,
    pub kind: PromoKind,
    pub discount_type: BenefitType,
    #[schema(value_type = String)]
    pub discount_value: Decimal,
    /// Cap on what this promotion gives per invoice.
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub max_discount_amount: Option<Decimal>,
    /// Coupons: the cart's gross subtotal must reach this.
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub min_purchase_amount: Option<Decimal>,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub stackable: bool,
    pub targets: Vec<Target>,
    /// Flash: units still discountable (limit − used); `None` = no limit.
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub quantity_remaining: Option<Decimal>,
    #[serde(default)]
    pub bxgy: Option<Bxgy>,
    #[serde(default)]
    pub coupon_id: Option<Uuid>,
    #[serde(default)]
    pub coupon_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Line {
    pub variant_id: Uuid,
    pub product_id: Uuid,
    /// The product's category and its ancestors.
    #[serde(default)]
    pub category_ids: Vec<Uuid>,
    #[serde(default)]
    pub brand_id: Option<Uuid>,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    #[serde(default)]
    pub min_unit_price: Option<Decimal>,
    /// The cashier removed automatic promotions from this line.
    #[serde(default)]
    pub promotions_off: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// Stackable automatic promotions combine on a line (else the best one wins).
    pub promotions_stack: bool,
    /// A coupon combines with automatic promotions (else whichever gives more).
    pub coupon_stacks: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Applied {
    pub promotion_id: Uuid,
    pub coupon_id: Option<Uuid>,
    pub coupon_code: Option<String>,
    pub name: String,
    pub name_bn: Option<String>,
    pub kind: PromoKind,
    pub discount_type: BenefitType,
    #[schema(value_type = String)]
    pub discount_value: Decimal,
    #[schema(value_type = String)]
    pub amount: Decimal,
    /// Units this discount covers (flash units, free units).
    #[schema(value_type = String)]
    pub units: Decimal,
    #[schema(value_type = String)]
    pub free_quantity: Decimal,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct LineOut {
    pub discounts: Vec<Applied>,
    #[schema(value_type = String)]
    pub amount: Decimal,
    #[schema(value_type = String)]
    pub free_quantity: Decimal,
    /// Flash: only part of the quantity was discounted (limit reached).
    pub flash_partial: bool,
    /// Promotions were reduced so the price stays at the minimum selling price.
    pub capped_by_min_price: bool,
}

/// A free item the cart qualifies for but does not hold yet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Offer {
    pub promotion_id: Uuid,
    pub name: String,
    pub name_bn: Option<String>,
    pub get_target_type: GetTargetType,
    /// Variant for `same_product`, product or category id otherwise.
    pub target_id: Uuid,
    #[schema(value_type = String)]
    pub quantity: Decimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CouponOutcome {
    Applied,
    /// Automatic promotions already give more and the coupon does not stack.
    NotBetter,
    MinPurchase,
    NoQualifyingItems,
}

impl CouponOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::NotBetter => "not_better",
            Self::MinPurchase => "min_purchase",
            Self::NoQualifyingItems => "no_qualifying_items",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Output {
    pub lines: Vec<LineOut>,
    #[schema(value_type = String)]
    pub promotion_total: Decimal,
    #[schema(value_type = String)]
    pub coupon_total: Decimal,
    pub offers: Vec<Offer>,
    pub coupon: Option<CouponOutcome>,
}

fn target_matches(t: &Target, line: &Line) -> bool {
    match t.target_type {
        TargetType::AllProducts => true,
        TargetType::Product => t.target_id == Some(line.product_id),
        TargetType::Category => t.target_id.is_some_and(|id| line.category_ids.contains(&id)),
        TargetType::Brand => t.target_id.is_some() && t.target_id == line.brand_id,
    }
}

/// Included by a target and not excluded by one.
pub fn matches(targets: &[Target], line: &Line) -> bool {
    let included = targets.iter().any(|t| !t.is_exclusion && target_matches(t, line));
    included && !targets.iter().any(|t| t.is_exclusion && target_matches(t, line))
}

fn get_matches(b: &Bxgy, line: &Line) -> bool {
    match b.get_target_type {
        GetTargetType::SameProduct => false,
        GetTargetType::Product => b.get_target_id == Some(line.product_id),
        GetTargetType::Category => b.get_target_id.is_some_and(|id| line.category_ids.contains(&id)),
    }
}

/// What a line promotion takes off `units` units at `price`.
fn line_amount(kind: BenefitType, value: Decimal, price: Decimal, units: Decimal) -> Decimal {
    let amount = match kind {
        BenefitType::Percent => round2(price * units * value / Decimal::ONE_HUNDRED),
        BenefitType::FixedAmount => value.min(price).max(Decimal::ZERO) * units,
        BenefitType::FixedPrice => (price - value).max(Decimal::ZERO) * units,
        BenefitType::Free => price * units,
    };
    round2(amount).min(round2(price * units)).max(Decimal::ZERO)
}

/// Same, applied to what is left of the line after earlier stacked promotions.
fn stacked_amount(
    kind: BenefitType,
    value: Decimal,
    price: Decimal,
    quantity: Decimal,
    units: Decimal,
    left: Decimal,
) -> Decimal {
    if quantity.is_zero() {
        return Decimal::ZERO;
    }
    let amount = match kind {
        BenefitType::Percent => round2(left * units * value / (quantity * Decimal::ONE_HUNDRED)),
        _ => line_amount(kind, value, price, units),
    };
    amount.min(left).max(Decimal::ZERO)
}

fn free_unit_value(b: &Bxgy, price: Decimal, units: Decimal) -> Decimal {
    match b.get_discount_type {
        BenefitType::Percent => round2(price * units * b.get_discount_value / Decimal::ONE_HUNDRED),
        _ => round2(price * units),
    }
}

fn applied(p: &Promotion, amount: Decimal, units: Decimal, free: Decimal) -> Applied {
    Applied {
        promotion_id: p.id,
        coupon_id: p.coupon_id,
        coupon_code: p.coupon_code.clone(),
        name: p.name.clone(),
        name_bn: p.name_bn.clone(),
        kind: p.kind,
        discount_type: p.discount_type,
        discount_value: p.discount_value,
        amount,
        units,
        free_quantity: free,
    }
}

/// Spreads `total` over `weights` in proportion; the last non-zero weight takes the residue.
fn allocate(total: Decimal, weights: &[Decimal]) -> Vec<Decimal> {
    let sum: Decimal = weights.iter().copied().sum();
    if sum.is_zero() {
        return vec![Decimal::ZERO; weights.len()];
    }
    let last = weights.iter().rposition(|w| !w.is_zero()).unwrap_or(0);
    let mut out = Vec::with_capacity(weights.len());
    let mut given = Decimal::ZERO;
    for (i, w) in weights.iter().enumerate() {
        let share = if i == last { total - given } else { round2(total * *w / sum) };
        let share = share.min(*w).max(Decimal::ZERO);
        given += share;
        out.push(share);
    }
    out
}

fn sum_amounts(line: &LineOut) -> Decimal {
    line.discounts.iter().map(|d| d.amount).sum()
}

pub fn apply(lines: &[Line], promotions: &[Promotion], coupon: Option<&Promotion>, settings: Settings) -> Output {
    let mut out: Vec<LineOut> = vec![LineOut::default(); lines.len()];
    let gross: Vec<Decimal> = lines.iter().map(|l| round2(l.quantity * l.unit_price)).collect();

    // Order of evaluation: priority (high first), then input order.
    let mut order: Vec<usize> = (0..promotions.len()).collect();
    order.sort_by(|a, b| promotions[*b].priority.cmp(&promotions[*a].priority).then(a.cmp(b)));

    // --- 1. Automatic line promotions: product discount and flash.
    let mut remaining: Vec<Option<Decimal>> = promotions.iter().map(|p| p.quantity_remaining).collect();
    for (li, line) in lines.iter().enumerate() {
        if line.promotions_off || line.quantity <= Decimal::ZERO {
            continue;
        }
        let candidates: Vec<usize> = order
            .iter()
            .copied()
            .filter(|&pi| {
                let p = &promotions[pi];
                matches!(p.kind, PromoKind::ProductDiscount | PromoKind::Flash) && matches(&p.targets, line)
            })
            .filter(|&pi| remaining[pi].is_none_or(|r| r > Decimal::ZERO))
            .collect();
        if candidates.is_empty() {
            continue;
        }
        let units_of =
            |pi: usize, remaining: &[Option<Decimal>]| remaining[pi].map_or(line.quantity, |r| r.min(line.quantity));

        // The single best offer (ties: priority, then input order: `candidates` is already in that order).
        let mut best: Option<(usize, Decimal)> = None;
        for &pi in &candidates {
            let p = &promotions[pi];
            let a = line_amount(p.discount_type, p.discount_value, line.unit_price, units_of(pi, &remaining));
            if best.is_none_or(|(_, b)| a > b) {
                best = Some((pi, a));
            }
        }
        let mut chosen: Vec<(usize, Decimal)> = best.into_iter().filter(|(_, a)| *a > Decimal::ZERO).collect();

        // Stackable ones one after another, when the business allows it and it gives more.
        if settings.promotions_stack {
            let mut left = gross[li];
            let mut chain = Vec::new();
            for &pi in candidates.iter().filter(|&&pi| promotions[pi].stackable) {
                let p = &promotions[pi];
                let a = stacked_amount(
                    p.discount_type,
                    p.discount_value,
                    line.unit_price,
                    line.quantity,
                    units_of(pi, &remaining),
                    left,
                );
                if a > Decimal::ZERO {
                    left -= a;
                    chain.push((pi, a));
                }
            }
            let chain_total: Decimal = chain.iter().map(|(_, a)| *a).sum();
            let best_total: Decimal = chosen.iter().map(|(_, a)| *a).sum();
            if chain.len() > 1 && chain_total > best_total {
                chosen = chain;
            }
        }

        for (pi, a) in chosen {
            let units = units_of(pi, &remaining);
            if let Some(r) = remaining[pi].as_mut() {
                *r -= units;
                if units < line.quantity {
                    out[li].flash_partial = true;
                }
            }
            out[li].discounts.push(applied(&promotions[pi], a, units, Decimal::ZERO));
        }
    }

    // Per-invoice cap of each line promotion.
    for p in promotions.iter().filter(|p| matches!(p.kind, PromoKind::ProductDiscount | PromoKind::Flash)) {
        let Some(cap) = p.max_discount_amount else { continue };
        let hits: Vec<(usize, usize)> = out
            .iter()
            .enumerate()
            .flat_map(|(li, l)| {
                l.discounts.iter().enumerate().filter(|(_, d)| d.promotion_id == p.id).map(move |(di, _)| (li, di))
            })
            .collect();
        let total: Decimal = hits.iter().map(|&(li, di)| out[li].discounts[di].amount).sum();
        if total > cap {
            let weights: Vec<Decimal> = hits.iter().map(|&(li, di)| out[li].discounts[di].amount).collect();
            for (&(li, di), share) in hits.iter().zip(allocate(round2(cap.max(Decimal::ZERO)), &weights)) {
                out[li].discounts[di].amount = share;
            }
        }
    }

    // --- 2. Buy X Get Y on lines without a line promotion.
    let mut offers = Vec::new();
    let mut in_bxgy = vec![false; lines.len()];
    for &pi in order.iter().filter(|&&pi| promotions[pi].kind == PromoKind::Bxgy) {
        let p = &promotions[pi];
        let Some(b) = p.bxgy.as_ref() else { continue };
        if b.buy_quantity <= Decimal::ZERO || b.get_quantity <= Decimal::ZERO {
            continue;
        }
        let free_line = |li: usize, out: &[LineOut], in_bxgy: &[bool]| {
            let l = &lines[li];
            !l.promotions_off && l.quantity > Decimal::ZERO && out[li].discounts.is_empty() && !in_bxgy[li]
        };
        let buy_lines: Vec<usize> =
            (0..lines.len()).filter(|&li| free_line(li, &out, &in_bxgy) && matches(&p.targets, &lines[li])).collect();
        let group = b.buy_quantity + b.get_quantity;
        let grant = |li: usize, free: Decimal, out: &mut Vec<LineOut>| {
            let amount = free_unit_value(b, lines[li].unit_price, free);
            if free > Decimal::ZERO && amount > Decimal::ZERO {
                out[li].discounts.push(applied(p, amount, free, free));
                out[li].free_quantity += free;
            }
        };

        match b.get_target_type {
            GetTargetType::SameProduct => {
                let mut once = false;
                for &li in &buy_lines {
                    let q = lines[li].quantity;
                    let mut sets = (q / group).floor();
                    if !b.repeat {
                        sets = sets.min(if once { Decimal::ZERO } else { Decimal::ONE });
                    }
                    let free = sets * b.get_quantity;
                    if free > Decimal::ZERO {
                        once = true;
                        in_bxgy[li] = true;
                        grant(li, free, &mut out);
                    }
                    // One more free unit is within reach: offer it.
                    let rest = q - (q / group).floor() * group;
                    if b.auto_add_free_item && (b.repeat || sets.is_zero()) && rest >= b.buy_quantity {
                        offers.push(Offer {
                            promotion_id: p.id,
                            name: p.name.clone(),
                            name_bn: p.name_bn.clone(),
                            get_target_type: GetTargetType::SameProduct,
                            target_id: lines[li].variant_id,
                            quantity: group - rest,
                        });
                    }
                }
            }
            GetTargetType::Product | GetTargetType::Category => {
                let get_lines: Vec<usize> = (0..lines.len())
                    .filter(|&li| free_line(li, &out, &in_bxgy) && get_matches(b, &lines[li]))
                    .collect();
                let overlap = get_lines.iter().any(|li| buy_lines.contains(li));
                // Units on offer, cheapest first (then cart order).
                let mut pool: Vec<usize> = get_lines.clone();
                pool.sort_by(|a, c| lines[*a].unit_price.cmp(&lines[*c].unit_price).then(a.cmp(c)));
                let entitled = if overlap {
                    // One shared group (e.g. a category): of every buy+get units the cheapest get units are free.
                    let mut units: Vec<usize> = buy_lines.clone();
                    for li in &get_lines {
                        if !units.contains(li) {
                            units.push(*li);
                        }
                    }
                    let q: Decimal = units.iter().map(|&li| lines[li].quantity).sum();
                    let sets = (q / group).floor();
                    let sets = if b.repeat { sets } else { sets.min(Decimal::ONE) };
                    sets * b.get_quantity
                } else {
                    let q: Decimal = buy_lines.iter().map(|&li| lines[li].quantity).sum();
                    let sets = (q / b.buy_quantity).floor();
                    let sets = if b.repeat { sets } else { sets.min(Decimal::ONE) };
                    sets * b.get_quantity
                };
                let mut left = entitled;
                for li in pool {
                    if left <= Decimal::ZERO {
                        break;
                    }
                    let free = left.min(lines[li].quantity);
                    left -= free;
                    in_bxgy[li] = true;
                    grant(li, free, &mut out);
                }
                if entitled > left {
                    for li in &buy_lines {
                        in_bxgy[*li] = true;
                    }
                }
                if left > Decimal::ZERO
                    && !overlap
                    && b.auto_add_free_item
                    && let Some(target) = b.get_target_id
                {
                    offers.push(Offer {
                        promotion_id: p.id,
                        name: p.name.clone(),
                        name_bn: p.name_bn.clone(),
                        get_target_type: b.get_target_type,
                        target_id: target,
                        quantity: left,
                    });
                }
            }
        }
    }

    // --- 3. Coupon.
    let mut coupon_outcome = None;
    if let Some(c) = coupon {
        let subtotal: Decimal = gross.iter().copied().sum();
        let qualifying: Vec<usize> = (0..lines.len())
            .filter(|&li| lines[li].quantity > Decimal::ZERO && matches(&c.targets, &lines[li]))
            .collect();
        if c.min_purchase_amount.is_some_and(|m| subtotal < m) {
            coupon_outcome = Some(CouponOutcome::MinPurchase);
        } else if qualifying.is_empty() {
            coupon_outcome = Some(CouponOutcome::NoQualifyingItems);
        } else {
            let stacks = settings.coupon_stacks || c.stackable;
            let base = |li: usize, out: &[LineOut]| if stacks { gross[li] - sum_amounts(&out[li]) } else { gross[li] };
            let bases: Vec<Decimal> = qualifying.iter().map(|&li| base(li, &out).max(Decimal::ZERO)).collect();
            let base_sum: Decimal = bases.iter().copied().sum();
            let mut total = match c.discount_type {
                BenefitType::Percent => {
                    bases.iter().map(|b| round2(*b * c.discount_value / Decimal::ONE_HUNDRED)).sum()
                }
                _ => round2(c.discount_value).min(base_sum),
            };
            if let Some(cap) = c.max_discount_amount {
                total = total.min(round2(cap));
            }
            total = total.max(Decimal::ZERO);
            let auto_there: Decimal = qualifying.iter().map(|&li| sum_amounts(&out[li])).sum();
            if total.is_zero() {
                coupon_outcome = Some(CouponOutcome::NoQualifyingItems);
            } else if !stacks && total <= auto_there {
                coupon_outcome = Some(CouponOutcome::NotBetter);
            } else {
                if !stacks {
                    for &li in &qualifying {
                        out[li].discounts.clear();
                        out[li].free_quantity = Decimal::ZERO;
                        out[li].flash_partial = false;
                    }
                }
                let shares = match c.discount_type {
                    BenefitType::Percent if c.max_discount_amount.is_none_or(|cap| total < round2(cap)) => {
                        bases.iter().map(|b| round2(*b * c.discount_value / Decimal::ONE_HUNDRED)).collect()
                    }
                    _ => allocate(total, &bases),
                };
                for (&li, share) in qualifying.iter().zip(shares) {
                    if share > Decimal::ZERO {
                        out[li].discounts.push(applied(c, share, lines[li].quantity, Decimal::ZERO));
                    }
                }
                coupon_outcome = Some(CouponOutcome::Applied);
            }
        }
    }

    // --- 4. Never below the minimum selling price (free units excepted), never below zero.
    for (li, line) in lines.iter().enumerate() {
        let o = &mut out[li];
        let floor = line
            .min_unit_price
            .map_or(Decimal::ZERO, |m| round2(m * (line.quantity - o.free_quantity).max(Decimal::ZERO)));
        let room = (gross[li] - floor).max(Decimal::ZERO);
        let mut total = sum_amounts(o);
        if total > room {
            if line.min_unit_price.is_some() {
                o.capped_by_min_price = true;
            }
            for d in o.discounts.iter_mut().rev() {
                if total <= room {
                    break;
                }
                let cut = (total - room).min(d.amount);
                d.amount -= cut;
                total -= cut;
            }
            o.discounts.retain(|d| d.amount > Decimal::ZERO || d.free_quantity > Decimal::ZERO);
        }
        o.amount = sum_amounts(o);
    }

    let coupon_total =
        out.iter().flat_map(|l| &l.discounts).filter(|d| d.kind == PromoKind::Coupon).map(|d| d.amount).sum();
    let all: Decimal = out.iter().map(|l| l.amount).sum();
    Output { promotion_total: all - coupon_total, coupon_total, lines: out, offers, coupon: coupon_outcome }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    fn d(v: &Value) -> Decimal {
        v.as_str().unwrap_or("0").parse().expect("decimal")
    }

    /// The shared vectors are the contract between this engine and the POS screen.
    #[test]
    fn shared_test_vectors() {
        let raw = include_str!("../../../../../frontend/src/lib/promotions.vectors.json");
        let vectors: Vec<Value> = serde_json::from_str(raw).expect("vectors parse");
        assert!(!vectors.is_empty());
        for v in vectors {
            let name = v["name"].as_str().unwrap_or_default();
            let lines: Vec<Line> = serde_json::from_value(v["lines"].clone()).expect(name);
            let promotions: Vec<Promotion> = serde_json::from_value(v["promotions"].clone()).expect(name);
            let coupon: Option<Promotion> = serde_json::from_value(v["coupon"].clone()).expect(name);
            let settings: Settings = serde_json::from_value(v["settings"].clone()).expect(name);
            let out = apply(&lines, &promotions, coupon.as_ref(), settings);
            let e = &v["expected"];
            assert_eq!(out.promotion_total, d(&e["promotion_total"]), "{name}: promotion_total");
            assert_eq!(out.coupon_total, d(&e["coupon_total"]), "{name}: coupon_total");
            assert_eq!(out.coupon.map(|c| c.as_str()), e["coupon"].as_str(), "{name}: coupon");
            for (i, el) in e["lines"].as_array().expect("lines").iter().enumerate() {
                let l = &out.lines[i];
                assert_eq!(l.amount, d(&el["amount"]), "{name}: line {i} amount");
                assert_eq!(l.free_quantity, d(&el["free_quantity"]), "{name}: line {i} free");
                assert_eq!(
                    l.flash_partial,
                    el["flash_partial"].as_bool().unwrap_or(false),
                    "{name}: line {i} flash_partial"
                );
                assert_eq!(
                    l.capped_by_min_price,
                    el["capped_by_min_price"].as_bool().unwrap_or(false),
                    "{name}: line {i} capped"
                );
                let ids: Vec<String> = l.discounts.iter().map(|x| x.promotion_id.to_string()).collect();
                let want: Vec<String> = el["promotions"]
                    .as_array()
                    .expect("promotions")
                    .iter()
                    .map(|x| x.as_str().unwrap_or_default().to_owned())
                    .collect();
                assert_eq!(ids, want, "{name}: line {i} promotions");
            }
            let offers: Vec<(String, String, Decimal)> =
                out.offers.iter().map(|o| (o.promotion_id.to_string(), o.target_id.to_string(), o.quantity)).collect();
            let want: Vec<(String, String, Decimal)> = e["offers"]
                .as_array()
                .expect("offers")
                .iter()
                .map(|o| {
                    (
                        o["promotion_id"].as_str().unwrap_or_default().to_owned(),
                        o["target_id"].as_str().unwrap_or_default().to_owned(),
                        d(&o["quantity"]),
                    )
                })
                .collect();
            assert_eq!(offers, want, "{name}: offers");
        }
    }
}
