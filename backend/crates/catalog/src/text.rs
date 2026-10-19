//! Input cleaning shared by the catalogue.

use pos_core::{AppError, AppResult};

/// A required name: trimmed, `min..=max` characters.
pub(crate) fn name(value: &str, field: &str, min: usize, max: usize) -> AppResult<String> {
    let value = value.trim();
    let len = value.chars().count();
    if len < min || len > max {
        return Err(AppError::field(field, "INVALID_LENGTH", format!("use {min}–{max} characters")));
    }
    Ok(value.to_owned())
}

/// An optional text: trimmed, empty becomes `None`.
pub(crate) fn optional(value: Option<String>, field: &str, max: usize) -> AppResult<Option<String>> {
    let Some(value) = value.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty()) else {
        return Ok(None);
    };
    if value.chars().count() > max {
        return Err(AppError::field(field, "TOO_LONG", format!("use at most {max} characters")));
    }
    Ok(Some(value))
}

/// A status from `allowed`; `None` means the first (the default).
pub(crate) fn status(value: Option<String>, allowed: &[&str]) -> AppResult<String> {
    match value {
        None => Ok(allowed[0].to_owned()),
        Some(v) if allowed.contains(&v.as_str()) => Ok(v),
        Some(_) => Err(AppError::field("status", "INVALID_STATUS", format!("use one of: {}", allowed.join(", ")))),
    }
}
