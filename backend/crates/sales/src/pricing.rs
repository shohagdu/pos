//! The pricing engine (Requirement Analysis §8.2, E3.1). The server always
//! recalculates totals from quantities, prices and discounts (POS-R01); the
//! POS screen runs the same algorithm (frontend `lib/pricing.ts`) for display,
//! and both are locked by the shared test vectors in
//! `frontend/src/lib/pricing.vectors.json`.

use pos_core::{
    AppError, AppResult,
    money::{round_to_step, round2},
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiscountType {
    Amount,
    Percent,
}

impl DiscountType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Amount => "amount",
            Self::Percent => "percent",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Discount {
    pub kind: DiscountType,
    pub value: Decimal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LineInput {
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub discount: Option<Discount>,
    /// Percent, e.g. 15.
    pub tax_rate_pct: Decimal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CartInput {
    pub lines: Vec<LineInput>,
    pub invoice_discount: Option<Discount>,
    pub shipping: Decimal,
    pub other: Decimal,
    pub prices_include_tax: bool,
    /// 0 = none, else 0.50 or 1.00.
    pub cash_rounding: Decimal,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LineResult {
    pub gross: Decimal,
    pub line_discount: Decimal,
    pub net_before_invoice: Decimal,
    pub invoice_discount: Decimal,
    /// Base the tax is computed on (includes the tax for tax-inclusive prices).
    pub taxable: Decimal,
    pub tax: Decimal,
    pub line_total: Decimal,
}

impl LineResult {
    /// Revenue of the line excluding VAT.
    pub fn net_of_tax(&self) -> Decimal {
        self.line_total - self.tax
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Totals {
    pub lines: Vec<LineResult>,
    pub subtotal: Decimal,
    pub item_discount_total: Decimal,
    pub invoice_discount: Decimal,
    pub tax_total: Decimal,
    pub shipping: Decimal,
    pub other: Decimal,
    pub pre_round_total: Decimal,
    pub rounding_adjustment: Decimal,
    pub grand_total: Decimal,
}

fn discount_amount(base: Decimal, discount: Option<Discount>, field: &str) -> AppResult<Decimal> {
    let Some(d) = discount else { return Ok(Decimal::ZERO) };
    if d.value < Decimal::ZERO {
        return Err(AppError::field(field, "INVALID_DISCOUNT", "a discount cannot be negative"));
    }
    let value = match d.kind {
        DiscountType::Percent => {
            if d.value > Decimal::ONE_HUNDRED {
                return Err(AppError::field(field, "INVALID_DISCOUNT", "a percentage discount is at most 100"));
            }
            round2(base * d.value / Decimal::ONE_HUNDRED)
        }
        DiscountType::Amount => round2(d.value),
    };
    if value > base {
        return Err(AppError::field(field, "DISCOUNT_TOO_LARGE", "the discount is more than the amount"));
    }
    Ok(value)
}

pub fn calculate(cart: &CartInput) -> AppResult<Totals> {
    let mut lines = Vec::with_capacity(cart.lines.len());
    for (i, line) in cart.lines.iter().enumerate() {
        let gross = round2(line.quantity * line.unit_price);
        let line_discount = discount_amount(gross, line.discount, &format!("lines[{i}].discount_value"))?;
        lines.push(LineResult {
            gross,
            line_discount,
            net_before_invoice: gross - line_discount,
            invoice_discount: Decimal::ZERO,
            taxable: Decimal::ZERO,
            tax: Decimal::ZERO,
            line_total: Decimal::ZERO,
        });
    }
    let net_sum: Decimal = lines.iter().map(|l| l.net_before_invoice).sum();
    let invoice_discount = discount_amount(net_sum, cart.invoice_discount, "invoice_discount_value")?;

    // Pro-rata allocation; the last line with a net amount takes the residue.
    if !invoice_discount.is_zero() {
        let last = lines.iter().rposition(|l| !l.net_before_invoice.is_zero()).unwrap_or(0);
        let mut allocated = Decimal::ZERO;
        for (i, line) in lines.iter_mut().enumerate() {
            let share = if i == last {
                invoice_discount - allocated
            } else {
                round2(invoice_discount * line.net_before_invoice / net_sum)
            };
            line.invoice_discount = share.min(line.net_before_invoice).max(Decimal::ZERO);
            allocated += line.invoice_discount;
        }
        if allocated != invoice_discount {
            return Err(AppError::field(
                "invoice_discount_value",
                "DISCOUNT_TOO_LARGE",
                "the discount is more than the amount",
            ));
        }
    }

    for (line, input) in lines.iter_mut().zip(&cart.lines) {
        line.taxable = line.net_before_invoice - line.invoice_discount;
        let rate = input.tax_rate_pct;
        if cart.prices_include_tax {
            line.tax = round2(line.taxable * rate / (Decimal::ONE_HUNDRED + rate));
            line.line_total = line.taxable;
        } else {
            line.tax = round2(line.taxable * rate / Decimal::ONE_HUNDRED);
            line.line_total = line.taxable + line.tax;
        }
    }

    let shipping = round2(cart.shipping);
    let other = round2(cart.other);
    let pre_round_total: Decimal = lines.iter().map(|l| l.line_total).sum::<Decimal>() + shipping + other;
    let rounded = round_to_step(pre_round_total, cart.cash_rounding);
    Ok(Totals {
        subtotal: lines.iter().map(|l| l.gross).sum(),
        item_discount_total: lines.iter().map(|l| l.line_discount).sum(),
        invoice_discount,
        tax_total: lines.iter().map(|l| l.tax).sum(),
        shipping,
        other,
        pre_round_total,
        rounding_adjustment: rounded - pre_round_total,
        grand_total: rounded,
        lines,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    fn d(s: &str) -> Decimal {
        s.parse().expect("decimal")
    }

    fn discount(v: &Value) -> Option<Discount> {
        let kind = match v.get("type")?.as_str()? {
            "percent" => DiscountType::Percent,
            _ => DiscountType::Amount,
        };
        Some(Discount { kind, value: d(v.get("value")?.as_str()?) })
    }

    /// The shared vectors are the contract between this engine and the POS screen.
    #[test]
    fn shared_test_vectors() {
        let raw = include_str!("../../../../frontend/src/lib/pricing.vectors.json");
        let vectors: Vec<Value> = serde_json::from_str(raw).expect("vectors parse");
        assert!(!vectors.is_empty());
        for v in vectors {
            let name = v["name"].as_str().unwrap_or_default();
            let input = &v["input"];
            let cart = CartInput {
                lines: input["lines"]
                    .as_array()
                    .expect("lines")
                    .iter()
                    .map(|l| LineInput {
                        quantity: d(l["quantity"].as_str().expect("qty")),
                        unit_price: d(l["unit_price"].as_str().expect("price")),
                        discount: l.get("discount").and_then(discount),
                        tax_rate_pct: d(l["tax_rate_pct"].as_str().expect("rate")),
                    })
                    .collect(),
                invoice_discount: input.get("invoice_discount").and_then(discount),
                shipping: d(input["shipping"].as_str().unwrap_or("0")),
                other: d(input["other"].as_str().unwrap_or("0")),
                prices_include_tax: input["prices_include_tax"].as_bool().unwrap_or(false),
                cash_rounding: d(input["cash_rounding"].as_str().unwrap_or("0")),
            };
            let expected = &v["expected"];
            if let Some(code) = expected.get("error").and_then(Value::as_str) {
                let err = calculate(&cart).expect_err(name);
                assert_eq!(err.field_errors().first().map(|e| e.code.as_str()), Some(code), "{name}");
                continue;
            }
            let t = calculate(&cart).unwrap_or_else(|e| panic!("{name}: {e}"));
            for key in [
                "subtotal",
                "item_discount_total",
                "invoice_discount",
                "tax_total",
                "rounding_adjustment",
                "grand_total",
            ] {
                let got = match key {
                    "subtotal" => t.subtotal,
                    "item_discount_total" => t.item_discount_total,
                    "invoice_discount" => t.invoice_discount,
                    "tax_total" => t.tax_total,
                    "rounding_adjustment" => t.rounding_adjustment,
                    _ => t.grand_total,
                };
                assert_eq!(got, d(expected[key].as_str().expect(key)), "{name}: {key}");
            }
            for (i, line) in expected["lines"].as_array().into_iter().flatten().enumerate() {
                for key in ["invoice_discount", "tax", "line_total"] {
                    let got = match key {
                        "invoice_discount" => t.lines[i].invoice_discount,
                        "tax" => t.lines[i].tax,
                        _ => t.lines[i].line_total,
                    };
                    assert_eq!(got, d(line[key].as_str().expect(key)), "{name}: line {i} {key}");
                }
            }
        }
    }

    #[test]
    fn worked_example_from_the_requirements() {
        let line = |q: &str, p: &str, disc: Option<Discount>| LineInput {
            quantity: d(q),
            unit_price: d(p),
            discount: disc,
            tax_rate_pct: d("15"),
        };
        let cart = CartInput {
            lines: vec![
                line("2", "420", None),
                line("1", "190", Some(Discount { kind: DiscountType::Amount, value: d("10") })),
                line("3", "60", None),
            ],
            invoice_discount: Some(Discount { kind: DiscountType::Amount, value: d("50") }),
            shipping: Decimal::ZERO,
            other: Decimal::ZERO,
            prices_include_tax: false,
            cash_rounding: d("1.00"),
        };
        let t = calculate(&cart).expect("calculates");
        assert_eq!(t.lines.iter().map(|l| l.invoice_discount).collect::<Vec<_>>(), vec![d("35"), d("7.5"), d("7.5")]);
        assert_eq!(t.tax_total, d("172.51"));
        assert_eq!(t.pre_round_total, d("1322.51"));
        assert_eq!(t.rounding_adjustment, d("0.49"));
        assert_eq!(t.grand_total, d("1323.00"));
    }
}
