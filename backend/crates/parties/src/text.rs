use pos_core::{AppError, AppResult};

pub fn required(value: &str, field: &str, max: usize) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::field(field, "REQUIRED", "this field is required"));
    }
    if value.chars().count() > max {
        return Err(AppError::field(field, "TOO_LONG", format!("use at most {max} characters")));
    }
    Ok(value.to_owned())
}

pub fn optional(value: Option<String>, field: &str, max: usize) -> AppResult<Option<String>> {
    let Some(value) = value.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty()) else { return Ok(None) };
    if value.chars().count() > max {
        return Err(AppError::field(field, "TOO_LONG", format!("use at most {max} characters")));
    }
    Ok(Some(value))
}

/// Digits with an optional leading `+`, 6–20 characters (spaces and dashes removed).
pub fn phone(value: Option<String>) -> AppResult<Option<String>> {
    let Some(raw) = value.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty()) else { return Ok(None) };
    let compact: String = raw.chars().filter(|c| !matches!(c, ' ' | '-')).collect();
    let digits = compact.strip_prefix('+').unwrap_or(&compact);
    if !(6..=20).contains(&compact.len()) || digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(AppError::field("phone", "INVALID_PHONE", "enter a phone number with digits only"));
    }
    Ok(Some(compact))
}

pub fn email(value: Option<String>) -> AppResult<Option<String>> {
    let Some(email) = optional(value, "email", 254)? else { return Ok(None) };
    let valid = email.split_once('@').is_some_and(|(local, domain)| !local.is_empty() && domain.contains('.'));
    if !valid {
        return Err(AppError::field("email", "INVALID_EMAIL", "enter a valid email address"));
    }
    Ok(Some(email.to_lowercase()))
}

/// `%` and `_` are LIKE wildcards; match them literally.
pub fn like_contains(text: &str) -> String {
    let escaped = text.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    format!("%{escaped}%")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phones() {
        assert_eq!(phone(Some(" 017-1234 5678 ".into())).ok(), Some(Some("01712345678".into())));
        assert_eq!(phone(Some("+8801712345678".into())).ok(), Some(Some("+8801712345678".into())));
        assert!(phone(Some("abc123".into())).is_err());
        assert_eq!(phone(Some("  ".into())).ok(), Some(None));
    }
}
