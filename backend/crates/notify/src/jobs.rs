//! Scheduled notification checks (NOT-R01), run by the worker every few
//! minutes for each active business in its own time zone. Each check builds
//! a digest and publishes it with a dedupe key, so running again is harmless:
//!
//! | type | when (local time) | key |
//! |---|---|---|
//! | `stock.low` | daily from 08:00 | `stock.low:<date>` |
//! | `sales.daily_summary` | daily from 07:00, for yesterday | `sales.daily:<yesterday>` |
//! | `customer.due`, `supplier.due` | Saturdays (start of the week) from 09:00 | `…:<date>` |
//! | `subscription.expiring` | 7, 3 and 1 days before the end, from 09:00 | `subscription.expiring:<date>` |
//! | `approval.request` | orders and expenses waiting (last 7 days) | `approval:<kind>:<id>` |
//! | `security.alert` | lockouts, password resets, token reuse (last day) | `security:<event id>` |

use chrono::{DateTime, Datelike, Duration, NaiveDate, Timelike, Utc, Weekday};
use chrono_tz::Tz;
use pos_core::{AppResult, RequestMeta, TenantCtx};
use pos_db::{TenantTx, map_db_error};
use pos_inventory::stock::{self, StockFilter};
use rust_decimal::Decimal;
use serde_json::json;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::{
    catalog::{self, Audience},
    publish::{Draft, Text, already_sent, publish, recipients},
};

/// Overdue = unpaid for more than this many days.
const OVERDUE_DAYS: i64 = 30;

#[derive(sqlx::FromRow)]
struct Business {
    id: Uuid,
    timezone: String,
    currency_code: String,
    owner_id: Option<Uuid>,
}

/// Runs every check for every active business; returns notifications created.
pub async fn run(pool: &MySqlPool, now: DateTime<Utc>) -> AppResult<usize> {
    let businesses: Vec<Business> = sqlx::query_as(
        "SELECT t.id, t.timezone, t.currency_code, (SELECT m.user_id FROM tenant_memberships m WHERE m.tenant_id = t.id AND m.is_owner \
           AND m.status = 'active' ORDER BY m.created_at LIMIT 1) AS owner_id \
         FROM tenants t WHERE t.status = 'active' \
         /* tenant-lint: allow the scheduled checks cover every business */",
    )
    .fetch_all(pool)
    .await
    .map_err(map_db_error)?;
    let mut created = 0;
    for b in businesses {
        let Some(owner_id) = b.owner_id else { continue };
        let tz: Tz = b.timezone.parse().unwrap_or(chrono_tz::Asia::Dhaka);
        match run_business(pool, b.id, owner_id, now, tz, b.currency_code).await {
            Ok(n) => created += n,
            // One business's failure must not stop the others.
            Err(err) => {
                tracing::error!(tenant = %b.id, error = %err, job = "notifications", "notification checks failed")
            }
        }
    }
    Ok(created)
}

/// Clock and currency of one business.
struct Local {
    date: NaiveDate,
    hour: u32,
    weekday: Weekday,
    now: DateTime<Utc>,
    currency: String,
}

async fn run_business(
    pool: &MySqlPool,
    tenant_id: Uuid,
    owner_id: Uuid,
    now: DateTime<Utc>,
    tz: Tz,
    currency: String,
) -> AppResult<usize> {
    let local_now = now.with_timezone(&tz);
    let local =
        Local { date: local_now.date_naive(), hour: local_now.hour(), weekday: local_now.weekday(), now, currency };
    let ctx = TenantCtx { tenant_id, user_id: owner_id, meta: RequestMeta::default() };
    let mut tx = pos_db::begin_tenant(pool, &ctx).await?;
    let ents = pos_billing::entitlements::resolve(&mut tx, tenant_id).await?;
    let sms = ents.can("notifications.sms");
    let mut n = 0;
    n += low_stock(&mut tx, &local, sms).await?;
    n += daily_sales(&mut tx, &local, sms).await?;
    n += dues(&mut tx, &local, sms).await?;
    if local.hour >= 9
        && let Some(days) = ents.days_left()
        && matches!(days, 1 | 3 | 7)
    {
        n += expiring(&mut tx, &local, days, sms).await?;
    }
    n += approvals(&mut tx, &local, sms).await?;
    n += security(&mut tx, &local, sms).await?;
    tx.commit().await?;
    Ok(n)
}

/// Symbol for a business currency (codes from `pos_tenancy::settings::CURRENCIES`).
fn symbol(currency: &str) -> &str {
    match currency {
        "BDT" => "৳",
        "INR" => "₹",
        "USD" | "AUD" | "CAD" | "SGD" => "$",
        "EUR" => "€",
        "GBP" => "£",
        "PKR" | "NPR" | "LKR" => "Rs",
        "MYR" => "RM",
        other => other,
    }
}

/// `1234567.5` in BDT → `৳12,34,567.50` (South Asian grouping, as in the
/// app); in EUR → `€1,234,567.50`.
pub fn money(value: Decimal, currency: &str) -> String {
    let lakh = matches!(currency, "BDT" | "INR" | "PKR" | "NPR" | "LKR");
    let fixed = format!("{:.2}", value.round_dp(2).abs());
    let (int, frac) = fixed.split_once('.').unwrap_or((&fixed, "00"));
    let digits: Vec<char> = int.chars().collect();
    // The last three digits form a group; before them groups of 2 (lakh) or 3.
    let size = if lakh { 2 } else { 3 };
    let head = digits.len().saturating_sub(3);
    let mut grouped = String::new();
    for (i, c) in digits[..head].iter().enumerate() {
        if i > 0 && (head - i).is_multiple_of(size) {
            grouped.push(',');
        }
        grouped.push(*c);
    }
    if head > 0 {
        grouped.push(',');
    }
    grouped.extend(&digits[head..]);
    let sign = if value.is_sign_negative() && !value.is_zero() { "-" } else { "" };
    let symbol = symbol(currency);
    let space = if symbol.chars().all(char::is_alphabetic) { " " } else { "" };
    format!("{sign}{symbol}{space}{grouped}.{frac}")
}

fn bn(locale: &str) -> bool {
    locale.starts_with("bn")
}

async fn send(
    tx: &mut TenantTx,
    draft: &Draft<'_>,
    audience: Audience,
    subject: Option<Uuid>,
    sms: bool,
) -> AppResult<usize> {
    let to = recipients(tx, audience, subject).await?;
    publish(tx, draft, &to, sms).await
}

async fn low_stock(tx: &mut TenantTx, local: &Local, sms: bool) -> AppResult<usize> {
    let key = format!("stock.low:{}", local.date);
    if local.hour < 8 || already_sent(tx, &key).await? {
        return Ok(0);
    }
    let low = StockFilter { status: Some("low".into()), limit: Some(1), ..StockFilter::default() };
    let summary = stock::list(tx, &low, false).await?.summary;
    if summary.items == 0 {
        return Ok(0);
    }
    let out = summary.out_count;
    let (count, out_count) = (summary.items, out);
    let text = move |locale: &str| {
        if bn(locale) {
            Text {
                title: format!("{count}টি পণ্যের স্টক কম"),
                body: Some(format!("রিঅর্ডার লেভেলে বা তার নিচে {count}টি পণ্য, যার {out_count}টি শেষ হয়ে গেছে।")),
            }
        } else {
            Text {
                title: format!("{count} products are low on stock"),
                body: Some(format!(
                    "{count} products are at or below their reorder level; {out_count} are out of stock."
                )),
            }
        }
    };
    let draft = Draft {
        notif_type: catalog::STOCK_LOW,
        link: Some("/inventory?status=low".into()),
        data: json!({ "low_count": count, "out_count": out_count }),
        dedupe_key: Some(key),
        text: &text,
    };
    send(tx, &draft, Audience::Permission("inventory.view"), None, sms).await
}

async fn daily_sales(tx: &mut TenantTx, local: &Local, sms: bool) -> AppResult<usize> {
    let day = local.date - Duration::days(1);
    let key = format!("sales.daily:{day}");
    if local.hour < 7 || already_sent(tx, &key).await? {
        return Ok(0);
    }
    let tenant_id = tx.tenant_id();
    let (invoices, total, due): (i64, Decimal, Decimal) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(grand_total), 0), COALESCE(SUM(due_total), 0) FROM sales \
         WHERE tenant_id = ? AND business_date = ? AND status <> 'cancelled'",
    )
    .bind(tenant_id)
    .bind(day)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if invoices == 0 {
        return Ok(0);
    }
    let returns: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(total), 0) FROM sale_returns WHERE tenant_id = ? AND business_date = ? AND status = 'posted'",
    )
    .bind(tenant_id)
    .bind(day)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let date = day.format("%d %b %Y").to_string();
    let cur = local.currency.clone();
    let text = move |locale: &str| {
        if bn(locale) {
            Text {
                title: format!("{date}-এর বিক্রয়: {}", money(total, &cur)),
                body: Some(format!("{invoices}টি ইনভয়েস · বাকি {} · ফেরত {}", money(due, &cur), money(returns, &cur))),
            }
        } else {
            Text {
                title: format!("Sales on {date}: {}", money(total, &cur)),
                body: Some(format!(
                    "{invoices} invoices · {} on credit · {} returned",
                    money(due, &cur),
                    money(returns, &cur)
                )),
            }
        }
    };
    let draft = Draft {
        notif_type: catalog::SALES_DAILY,
        link: Some(format!("/reports/sales?from={day}&to={day}")),
        data: json!({ "date": day, "invoices": invoices, "total": total, "due": due, "returns": returns }),
        dedupe_key: Some(key),
        text: &text,
    };
    send(tx, &draft, Audience::Permission("reports.sales"), None, sms).await
}

/// Weekly reminders of overdue customer and supplier balances.
async fn dues(tx: &mut TenantTx, local: &Local, sms: bool) -> AppResult<usize> {
    if local.weekday != Weekday::Sat || local.hour < 9 {
        return Ok(0);
    }
    let tenant_id = tx.tenant_id();
    let before = local.date - Duration::days(OVERDUE_DAYS);
    let mut n = 0;

    let key = format!("customer.due:{}", local.date);
    if !already_sent(tx, &key).await? {
        let (parties, amount): (i64, Decimal) = sqlx::query_as(
            "SELECT COUNT(DISTINCT customer_id), COALESCE(SUM(due_total), 0) FROM sales \
             WHERE tenant_id = ? AND status <> 'cancelled' AND due_total > 0 AND business_date < ?",
        )
        .bind(tenant_id)
        .bind(before)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
        if parties > 0 {
            let cur = local.currency.clone();
            let text = move |locale: &str| {
                if bn(locale) {
                    Text {
                        title: format!("{parties} জন গ্রাহকের {} বকেয়া ৩০ দিনের বেশি", money(amount, &cur)),
                        body: Some("বকেয়া আদায়ের জন্য এজিং রিপোর্ট দেখুন।".into()),
                    }
                } else {
                    Text {
                        title: format!("{} owed by {parties} customers for over 30 days", money(amount, &cur)),
                        body: Some("See the aging report to follow up.".into()),
                    }
                }
            };
            let draft = Draft {
                notif_type: catalog::CUSTOMER_DUE,
                link: Some("/reports/aging?party=customers".into()),
                data: json!({ "customers": parties, "amount": amount }),
                dedupe_key: Some(key),
                text: &text,
            };
            n += send(tx, &draft, Audience::Permission("customers.view"), None, sms).await?;
        }
    }

    let key = format!("supplier.due:{}", local.date);
    if !already_sent(tx, &key).await? {
        let (parties, amount): (i64, Decimal) = sqlx::query_as(
            "SELECT COUNT(DISTINCT supplier_id), COALESCE(SUM(due_total), 0) FROM purchases \
             WHERE tenant_id = ? AND status IN ('partially_received', 'received') AND due_total > 0 AND purchase_date < ?",
        )
        .bind(tenant_id)
        .bind(before)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
        if parties > 0 {
            let cur = local.currency.clone();
            let text = move |locale: &str| {
                if bn(locale) {
                    Text {
                        title: format!("{parties} জন সরবরাহকারীর {} পাওনা ৩০ দিনের বেশি", money(amount, &cur)),
                        body: Some("পরিশোধের জন্য এজিং রিপোর্ট দেখুন।".into()),
                    }
                } else {
                    Text {
                        title: format!("{} owed to {parties} suppliers for over 30 days", money(amount, &cur)),
                        body: Some("See the aging report to plan payments.".into()),
                    }
                }
            };
            let draft = Draft {
                notif_type: catalog::SUPPLIER_DUE,
                link: Some("/reports/aging?party=suppliers".into()),
                data: json!({ "suppliers": parties, "amount": amount }),
                dedupe_key: Some(key),
                text: &text,
            };
            n += send(tx, &draft, Audience::Permission("suppliers.view"), None, sms).await?;
        }
    }
    Ok(n)
}

async fn expiring(tx: &mut TenantTx, local: &Local, days: i64, sms: bool) -> AppResult<usize> {
    let key = format!("subscription.expiring:{}", local.date);
    if already_sent(tx, &key).await? {
        return Ok(0);
    }
    let text = move |locale: &str| {
        if bn(locale) {
            Text {
                title: format!("সাবস্ক্রিপশন {days} দিনের মধ্যে শেষ হবে"),
                body: Some("বিক্রয় চালু রাখতে প্ল্যান পাতা থেকে নবায়ন করুন।".into()),
            }
        } else {
            Text {
                title: format!("Your subscription ends in {days} day{}", if days == 1 { "" } else { "s" }),
                body: Some("Renew from the plan page to keep selling without interruption.".into()),
            }
        }
    };
    let draft = Draft {
        notif_type: catalog::SUBSCRIPTION_EXPIRING,
        link: Some("/plan".into()),
        data: json!({ "days_left": days }),
        dedupe_key: Some(key),
        text: &text,
    };
    send(tx, &draft, Audience::Owners, None, sms).await
}

#[derive(sqlx::FromRow)]
struct Pending {
    id: Uuid,
    number: Option<String>,
    party: String,
    amount: Decimal,
}

/// Documents waiting for someone's approval.
async fn approvals(tx: &mut TenantTx, local: &Local, sms: bool) -> AppResult<usize> {
    let tenant_id = tx.tenant_id();
    let since = local.now - Duration::days(7);
    let mut n = 0;
    let orders: Vec<Pending> = sqlx::query_as(
        "SELECT p.id, p.purchase_no AS number, s.name AS party, p.grand_total AS amount FROM purchases p \
         JOIN suppliers s ON s.tenant_id = p.tenant_id AND s.id = p.supplier_id \
         WHERE p.tenant_id = ? AND p.status = 'ordered' AND p.ordered_at >= ?",
    )
    .bind(tenant_id)
    .bind(since)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    for o in orders {
        let key = format!("approval:purchase:{}", o.id);
        if already_sent(tx, &key).await? {
            continue;
        }
        let (no, party, amount) = (o.number.unwrap_or_default(), o.party, o.amount);
        let cur = local.currency.clone();
        let text = move |locale: &str| {
            if bn(locale) {
                Text {
                    title: format!("ক্রয় অর্ডার {no} অনুমোদনের অপেক্ষায়"),
                    body: Some(format!("{party} · {}", money(amount, &cur))),
                }
            } else {
                Text {
                    title: format!("Purchase order {no} needs approval"),
                    body: Some(format!("{party} · {}", money(amount, &cur))),
                }
            }
        };
        let draft = Draft {
            notif_type: catalog::APPROVAL,
            link: Some(format!("/purchases/{}", o.id)),
            data: json!({ "kind": "purchase", "id": o.id }),
            dedupe_key: Some(key),
            text: &text,
        };
        n += send(tx, &draft, Audience::Permission("purchases.approve"), None, sms).await?;
    }
    let expenses: Vec<Pending> = sqlx::query_as(
        "SELECT e.id, e.expense_no AS number, c.name AS party, e.amount FROM expenses e \
         JOIN expense_categories c ON c.tenant_id = e.tenant_id AND c.id = e.category_id \
         WHERE e.tenant_id = ? AND e.status = 'pending_approval' AND e.created_at >= ?",
    )
    .bind(tenant_id)
    .bind(since)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    for e in expenses {
        let key = format!("approval:expense:{}", e.id);
        if already_sent(tx, &key).await? {
            continue;
        }
        let (no, party, amount) = (e.number.unwrap_or_default(), e.party, e.amount);
        let cur = local.currency.clone();
        let text = move |locale: &str| {
            if bn(locale) {
                Text {
                    title: format!("খরচ {no} অনুমোদনের অপেক্ষায়"),
                    body: Some(format!("{party} · {}", money(amount, &cur))),
                }
            } else {
                Text {
                    title: format!("Expense {no} needs approval"),
                    body: Some(format!("{party} · {}", money(amount, &cur))),
                }
            }
        };
        let draft = Draft {
            notif_type: catalog::APPROVAL,
            link: Some("/expenses".into()),
            data: json!({ "kind": "expense", "id": e.id }),
            dedupe_key: Some(key),
            text: &text,
        };
        n += send(tx, &draft, Audience::Permission("expenses.approve"), None, sms).await?;
    }
    Ok(n)
}

#[derive(sqlx::FromRow)]
struct SecurityEvent {
    id: Uuid,
    user_id: Uuid,
    event: String,
    created_at: DateTime<Utc>,
}

/// Lockouts, password resets and refresh-token reuse of the business's members.
async fn security(tx: &mut TenantTx, local: &Local, sms: bool) -> AppResult<usize> {
    let tenant_id = tx.tenant_id();
    let events: Vec<SecurityEvent> = sqlx::query_as(
        "SELECT e.id, e.user_id, e.event, e.created_at FROM auth_events e \
         JOIN tenant_memberships m ON m.user_id = e.user_id AND m.tenant_id = ? AND m.status = 'active' \
         WHERE e.event IN ('auth.lockout', 'password.reset', 'refresh.reuse_detected') AND e.created_at >= ? \
         /* tenant-lint: allow auth events are per user; joined to this business's members */",
    )
    .bind(tenant_id)
    .bind(local.now - Duration::days(1))
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let mut n = 0;
    for e in events {
        let key = format!("security:{}", e.id);
        if already_sent(tx, &key).await? {
            continue;
        }
        let (event, at) = (e.event.clone(), e.created_at.format("%d %b %Y %H:%M UTC").to_string());
        let text = move |locale: &str| {
            let (en, bn_title) = match event.as_str() {
                "auth.lockout" => {
                    ("Your account was locked after failed sign-ins", "ব্যর্থ সাইন-ইনের পর আপনার অ্যাকাউন্ট লক হয়েছে")
                }
                "password.reset" => ("Your password was reset", "আপনার পাসওয়ার্ড রিসেট করা হয়েছে"),
                _ => (
                    "A suspicious sign-in was stopped and your sessions were ended",
                    "সন্দেহজনক সাইন-ইন থামানো হয়েছে এবং সব সেশন বন্ধ করা হয়েছে",
                ),
            };
            if bn(locale) {
                Text {
                    title: bn_title.into(), body: Some(format!("{at}। আপনি না করে থাকলে পাসওয়ার্ড বদলান।"))
                }
            } else {
                Text { title: en.into(), body: Some(format!("{at}. If this wasn't you, change your password.")) }
            }
        };
        let draft = Draft {
            notif_type: catalog::SECURITY,
            link: Some("/account/sessions".into()),
            data: json!({ "event": e.event, "at": e.created_at }),
            dedupe_key: Some(key),
            text: &text,
        };
        n += send(tx, &draft, Audience::Subject, Some(e.user_id), sms).await?;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_money_in_the_business_currency() {
        assert_eq!(money(Decimal::new(123456750, 2), "BDT"), "৳12,34,567.50");
        assert_eq!(money(Decimal::new(999, 0), "BDT"), "৳999.00");
        assert_eq!(money(Decimal::new(1000, 0), "BDT"), "৳1,000.00");
        assert_eq!(money(Decimal::new(-1550, 1), "BDT"), "-৳155.00");
        assert_eq!(money(Decimal::new(123456750, 2), "EUR"), "€1,234,567.50");
        assert_eq!(money(Decimal::new(5, 0), "MYR"), "RM 5.00");
    }
}
