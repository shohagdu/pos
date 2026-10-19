//! Barcode validation (Requirement Analysis §7.5): EAN-13, EAN-8 and UPC-A
//! check digits are verified when the format is recognised; anything else is
//! accepted as free format.

use pos_core::{AppError, AppResult};

/// The detected symbology, stored in `product_barcodes.symbology`.
pub fn symbology(code: &str) -> &'static str {
    if !code.bytes().all(|b| b.is_ascii_digit()) {
        return "other";
    }
    match code.len() {
        13 => "ean13",
        12 => "upca",
        8 => "ean8",
        _ => "other",
    }
}

/// GS1 check digit: weights 3 and 1 from the right, excluding the check digit.
fn check_digit_ok(code: &str) -> bool {
    let digits: Vec<u32> = code.bytes().map(|b| u32::from(b - b'0')).collect();
    let Some((&check, body)) = digits.split_last() else { return false };
    let sum: u32 = body.iter().rev().enumerate().map(|(i, d)| if i % 2 == 0 { d * 3 } else { *d }).sum();
    (10 - sum % 10) % 10 == check
}

/// Trims and validates one barcode, returning it with its symbology.
pub fn validate(raw: &str) -> AppResult<(String, &'static str)> {
    let code = raw.trim();
    if code.is_empty() || code.len() > 64 || !code.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(AppError::field("barcodes", "INVALID_BARCODE", "barcodes are 1–64 characters without spaces"));
    }
    let kind = symbology(code);
    if kind != "other" && !check_digit_ok(code) {
        return Err(AppError::field("barcodes", "INVALID_CHECK_DIGIT", format!("{code}: the check digit is wrong")));
    }
    Ok((code.to_owned(), kind))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognised_formats_need_a_valid_check_digit() {
        assert_eq!(validate("4006381333931").ok(), Some(("4006381333931".to_owned(), "ean13")));
        assert_eq!(validate("036000291452").ok(), Some(("036000291452".to_owned(), "upca")));
        assert_eq!(validate("96385074").ok(), Some(("96385074".to_owned(), "ean8")));
        for bad in ["4006381333932", "036000291453", "96385075"] {
            assert!(validate(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn free_format_is_accepted() {
        assert_eq!(validate(" ABC-123 ").ok(), Some(("ABC-123".to_owned(), "other")));
        assert_eq!(validate("12345").ok(), Some(("12345".to_owned(), "other")));
        assert!(validate("has space").is_err());
        assert!(validate("").is_err());
    }
}
