//! Platform-unique business slugs generated from the name (ONB-R02).

use pos_auth::token::random_bytes;
use pos_core::AppResult;

const MAX_BASE_LEN: usize = 50;
const FALLBACK: &str = "shop";

/// `"Rahim General Store"` → `rahim-general-store`. Names without Latin letters
/// or digits (e.g. Bengali only) fall back to `shop`.
pub fn base(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
        if out.len() >= MAX_BASE_LEN {
            break;
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.len() < 3 { FALLBACK.to_owned() } else { trimmed.to_owned() }
}

/// `base` plus a random suffix, used when the base is taken.
pub fn with_suffix(base: &str) -> AppResult<String> {
    const ALPHABET: &[u8] = b"abcdefghijkmnpqrstuvwxyz23456789";
    let suffix: String =
        random_bytes::<5>()?.iter().map(|b| ALPHABET[usize::from(*b) % ALPHABET.len()] as char).collect();
    Ok(format!("{base}-{suffix}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_readable_slugs() {
        assert_eq!(base("Rahim General Store"), "rahim-general-store");
        assert_eq!(base("  M/S. Karim & Sons (Pvt) Ltd.  "), "m-s-karim-sons-pvt-ltd");
        assert_eq!(base("রহিম স্টোর"), "shop");
        assert_eq!(base("AB"), "shop");
        assert!(base(&"x".repeat(80)).len() <= MAX_BASE_LEN);
    }

    #[test]
    fn suffixed_slugs_match_the_schema_pattern() {
        let slug = with_suffix(&base(&"long name ".repeat(10))).expect("slug");
        assert!(slug.len() <= 63);
        assert!(slug.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'));
    }
}
