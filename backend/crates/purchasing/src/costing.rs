//! Purchase line costing (§8.4, PUR-R03): line discount, pro-rata bill
//! discount, VAT, and shipping/other charges allocated by line value into the
//! landed unit cost used for the moving average.

use pos_core::{
    AppError, AppResult,
    money::{round2, round4},
};
use rust_decimal::Decimal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineDiscount {
    Amount(Decimal),
    Percent(Decimal),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CostLine {
    pub quantity: Decimal,
    /// Base units per purchase unit (1 until unit conversions arrive).
    pub unit_factor: Decimal,
    pub unit_cost: Decimal,
    pub discount: Option<LineDiscount>,
    pub tax_rate_pct: Decimal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CostedLine {
    pub gross: Decimal,
    pub line_discount: Decimal,
    pub bill_discount: Decimal,
    /// After line and bill discounts, excluding VAT and charges.
    pub net: Decimal,
    pub tax: Decimal,
    pub charges: Decimal,
    /// Net + VAT (what the supplier bills for the line).
    pub line_total: Decimal,
    /// Per base unit: net + charges (+ VAT when it is not reclaimable).
    pub landed_unit_cost: Decimal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bill {
    pub lines: Vec<CostedLine>,
    pub subtotal: Decimal,
    pub item_discount_total: Decimal,
    pub bill_discount: Decimal,
    pub tax_total: Decimal,
    pub shipping: Decimal,
    pub other: Decimal,
    pub grand_total: Decimal,
    /// VAT recoverable as input tax (VAT-registered businesses).
    pub recoverable_tax: Decimal,
}

/// Splits `total` over `weights` pro-rata (2 decimals); the last non-zero
/// weight takes the residue. Equal shares when every weight is zero.
pub fn allocate(total: Decimal, weights: &[Decimal]) -> Vec<Decimal> {
    if weights.is_empty() || total.is_zero() {
        return vec![Decimal::ZERO; weights.len()];
    }
    let sum: Decimal = weights.iter().sum();
    let equal = sum.is_zero();
    let weights: Vec<Decimal> = if equal { vec![Decimal::ONE; weights.len()] } else { weights.to_vec() };
    let sum: Decimal = weights.iter().sum();
    let last = weights.iter().rposition(|w| !w.is_zero()).unwrap_or(0);
    let mut out = vec![Decimal::ZERO; weights.len()];
    let mut given = Decimal::ZERO;
    for (i, w) in weights.iter().enumerate() {
        if i == last {
            out[i] = total - given;
        } else {
            out[i] = round2(total * w / sum);
            given += out[i];
        }
    }
    out
}

pub fn cost(
    lines: &[CostLine],
    bill_discount: Decimal,
    shipping: Decimal,
    other: Decimal,
    tax_recoverable: bool,
) -> AppResult<Bill> {
    let mut gross_and_disc = Vec::with_capacity(lines.len());
    for (i, l) in lines.iter().enumerate() {
        let gross = round2(l.quantity * l.unit_cost);
        let disc = match l.discount {
            None => Decimal::ZERO,
            Some(LineDiscount::Amount(a)) => round2(a),
            Some(LineDiscount::Percent(p)) => round2(gross * p / Decimal::ONE_HUNDRED),
        };
        if disc > gross || disc < Decimal::ZERO {
            return Err(AppError::field(
                &format!("lines[{i}].discount_value"),
                "DISCOUNT_TOO_LARGE",
                "the discount is more than the line",
            ));
        }
        gross_and_disc.push((gross, disc));
    }
    let before_bill: Vec<Decimal> = gross_and_disc.iter().map(|(g, d)| g - d).collect();
    let net_sum: Decimal = before_bill.iter().sum();
    let bill_discount = round2(bill_discount);
    if bill_discount > net_sum {
        return Err(AppError::field("bill_discount", "DISCOUNT_TOO_LARGE", "the bill discount is more than the bill"));
    }
    let bill_shares = allocate(bill_discount, &before_bill);
    let nets: Vec<Decimal> = before_bill.iter().zip(&bill_shares).map(|(n, b)| n - b).collect();
    let charges = allocate(round2(shipping) + round2(other), &nets);
    let mut out = Vec::with_capacity(lines.len());
    for (i, l) in lines.iter().enumerate() {
        let net = nets[i];
        let tax = round2(net * l.tax_rate_pct / Decimal::ONE_HUNDRED);
        let base_qty = l.quantity * l.unit_factor;
        let cost_value = net + charges[i] + if tax_recoverable { Decimal::ZERO } else { tax };
        out.push(CostedLine {
            gross: gross_and_disc[i].0,
            line_discount: gross_and_disc[i].1,
            bill_discount: bill_shares[i],
            net,
            tax,
            charges: charges[i],
            line_total: net + tax,
            landed_unit_cost: if base_qty.is_zero() { Decimal::ZERO } else { round4(cost_value / base_qty) },
        });
    }
    let tax_total: Decimal = out.iter().map(|l| l.tax).sum();
    let grand_total = out.iter().map(|l| l.line_total).sum::<Decimal>() + round2(shipping) + round2(other);
    Ok(Bill {
        subtotal: out.iter().map(|l| l.gross).sum(),
        item_discount_total: out.iter().map(|l| l.line_discount).sum(),
        bill_discount,
        tax_total,
        shipping: round2(shipping),
        other: round2(other),
        grand_total,
        recoverable_tax: if tax_recoverable { tax_total } else { Decimal::ZERO },
        lines: out,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Decimal {
        s.parse().expect("decimal")
    }

    fn line(q: &str, c: &str) -> CostLine {
        CostLine {
            quantity: d(q),
            unit_factor: Decimal::ONE,
            unit_cost: d(c),
            discount: None,
            tax_rate_pct: Decimal::ZERO,
        }
    }

    #[test]
    fn ac_pur_01_shipping_goes_into_the_landed_cost() {
        let bill = cost(&[line("10", "100")], Decimal::ZERO, d("50"), Decimal::ZERO, false).expect("costs");
        assert_eq!(bill.lines[0].landed_unit_cost, d("105"));
        assert_eq!(bill.grand_total, d("1050"));
    }

    #[test]
    fn charges_and_bill_discount_split_by_value() {
        let bill = cost(&[line("2", "100"), line("1", "100")], d("30"), d("9"), d("0"), false).expect("costs");
        // Nets 200/100 → bill discount 20/10 → nets 180/90 → charges 6/3.
        assert_eq!(bill.lines.iter().map(|l| l.bill_discount).collect::<Vec<_>>(), vec![d("20"), d("10")]);
        assert_eq!(bill.lines.iter().map(|l| l.charges).collect::<Vec<_>>(), vec![d("6"), d("3")]);
        assert_eq!(bill.lines[0].landed_unit_cost, d("93"));
        assert_eq!(bill.grand_total, d("279"));
    }

    #[test]
    fn reclaimable_vat_stays_out_of_cost() {
        let mut l = line("4", "25");
        l.tax_rate_pct = d("15");
        let registered =
            cost(std::slice::from_ref(&l), Decimal::ZERO, Decimal::ZERO, Decimal::ZERO, true).expect("costs");
        let not_registered = cost(&[l], Decimal::ZERO, Decimal::ZERO, Decimal::ZERO, false).expect("costs");
        assert_eq!(registered.lines[0].landed_unit_cost, d("25"));
        assert_eq!(not_registered.lines[0].landed_unit_cost, d("28.75"));
        assert_eq!(registered.grand_total, d("115"));
        assert_eq!(registered.recoverable_tax, d("15"));
    }

    #[test]
    fn allocation_residue_goes_to_the_last_line() {
        let shares = allocate(d("10"), &[d("1"), d("1"), d("1")]);
        assert_eq!(shares, vec![d("3.33"), d("3.33"), d("3.34")]);
        assert_eq!(allocate(d("5"), &[Decimal::ZERO, Decimal::ZERO]), vec![d("2.5"), d("2.5")]);
    }
}
