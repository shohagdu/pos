//! Login identifiers: a mobile number or an email address (US-AUTH-02).

use pos_core::{AppError, AppResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Identifier {
    /// E.164, e.g. `+8801712345678`.
    Phone(String),
    /// Lower-cased address.
    Email(String),
}

impl Identifier {
    /// Parses what a user typed into the identifier field.
    pub fn parse(raw: &str, field: &str) -> AppResult<Self> {
        let raw = raw.trim();
        if raw.contains('@') {
            normalize_email(raw)
                .map(Self::Email)
                .ok_or_else(|| AppError::field(field, "INVALID_EMAIL", "enter a valid email address"))
        } else {
            normalize_phone(raw)
                .map(Self::Phone)
                .ok_or_else(|| AppError::field(field, "INVALID_PHONE", "enter a valid mobile number"))
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Phone(s) | Self::Email(s) => s,
        }
    }

    pub fn masked(&self) -> String {
        mask(self.as_str())
    }
}

/// Normalises a mobile number to E.164. Local Bangladeshi numbers
/// (`01712-345678`, `8801712345678`) get the `+880` prefix and must be valid
/// mobile numbers; other international numbers need 8–15 digits.
pub fn normalize_phone(raw: &str) -> Option<String> {
    let compact: String = raw.chars().filter(|c| !matches!(c, ' ' | '-' | '(' | ')' | '.')).collect();
    let digits = if let Some(rest) = compact.strip_prefix('+') {
        rest.to_owned()
    } else if let Some(rest) = compact.strip_prefix("00") {
        rest.to_owned()
    } else if compact.len() == 11 && compact.starts_with("01") {
        format!("88{compact}")
    } else {
        compact
    };
    if !digits.bytes().all(|b| b.is_ascii_digit()) || digits.starts_with('0') {
        return None;
    }
    if let Some(national) = digits.strip_prefix("880") {
        // Bangladeshi mobile: 1 + operator digit 3–9 + 8 digits.
        let b = national.as_bytes();
        let valid = b.len() == 10 && b[0] == b'1' && (b'3'..=b'9').contains(&b[1]);
        return valid.then(|| format!("+{digits}"));
    }
    (8..=15).contains(&digits.len()).then(|| format!("+{digits}"))
}

pub fn normalize_email(raw: &str) -> Option<String> {
    let email = raw.trim().to_lowercase();
    let (local, domain) = email.split_once('@')?;
    let valid = !local.is_empty()
        && email.len() <= 254
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !domain.contains('@')
        && !email.chars().any(char::is_whitespace);
    valid.then_some(email)
}

/// `+8801712345678` → `+880*******678`; `rahim@example.com` → `ra***@example.com`.
pub fn mask(destination: &str) -> String {
    if let Some((local, domain)) = destination.split_once('@') {
        let keep: String = local.chars().take(2).collect();
        return format!("{keep}***@{domain}");
    }
    let chars: Vec<char> = destination.chars().collect();
    if chars.len() <= 7 {
        return "*".repeat(chars.len());
    }
    let head: String = chars[..4].iter().collect();
    let tail: String = chars[chars.len() - 3..].iter().collect();
    format!("{head}{}{tail}", "*".repeat(chars.len() - 7))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalises_bangladeshi_numbers() {
        for raw in ["01712345678", "01712-345678", "+8801712345678", "8801712345678", "008801712345678"] {
            assert_eq!(normalize_phone(raw).as_deref(), Some("+8801712345678"), "{raw}");
        }
    }

    #[test]
    fn rejects_invalid_phones() {
        for raw in ["", "12345", "0171234567", "+8801212345678", "+88017123456789", "01712abc678", "+0123456789"] {
            assert_eq!(normalize_phone(raw), None, "{raw}");
        }
        assert_eq!(normalize_phone("+447911123456").as_deref(), Some("+447911123456"));
    }

    #[test]
    fn parses_and_lowercases_emails() {
        assert_eq!(
            Identifier::parse(" Rahim@Example.COM ", "id").ok(),
            Some(Identifier::Email("rahim@example.com".into()))
        );
        assert!(Identifier::parse("rahim@localhost", "id").is_err());
        assert!(Identifier::parse("a b@example.com", "id").is_err());
    }

    #[test]
    fn masks_destinations() {
        assert_eq!(mask("+8801712345678"), "+880*******678");
        assert_eq!(mask("rahim@example.com"), "ra***@example.com");
    }
}
