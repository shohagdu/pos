//! Cursor pagination (Requirement Analysis API-03): `?limit=50&cursor=…`,
//! responses `{ "data": [...], "next_cursor": "…" }`, max limit 200.
//!
//! Lists are ordered newest first by `(time, id)`; the cursor is the key of
//! the last row returned, opaque to clients (DB design §11 keyset pagination).

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{AppError, AppResult};

pub const DEFAULT_LIMIT: u32 = 50;
pub const MAX_LIMIT: u32 = 200;

/// The requested page size, defaulted and checked against `1..=MAX_LIMIT`.
pub fn limit(requested: Option<u32>) -> AppResult<u32> {
    match requested.unwrap_or(DEFAULT_LIMIT) {
        n @ 1..=MAX_LIMIT => Ok(n),
        _ => Err(AppError::field("limit", "OUT_OF_RANGE", format!("limit must be between 1 and {MAX_LIMIT}"))),
    }
}

/// Position after the last row of a page ordered by `(at DESC, id DESC)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Keyset {
    pub at: DateTime<Utc>,
    pub id: Uuid,
}

impl Keyset {
    pub fn encode(&self) -> String {
        URL_SAFE_NO_PAD.encode(format!("{}.{}", self.at.timestamp_micros(), self.id.simple()))
    }

    pub fn decode(cursor: &str) -> AppResult<Self> {
        let invalid =
            || AppError::field("cursor", "INVALID_CURSOR", "cursor is not valid; start again from the first page");
        let raw = URL_SAFE_NO_PAD.decode(cursor).map_err(|_| invalid())?;
        let text = std::str::from_utf8(&raw).map_err(|_| invalid())?;
        let (micros, id) = text.split_once('.').ok_or_else(invalid)?;
        let at = micros.parse().ok().and_then(DateTime::from_timestamp_micros).ok_or_else(invalid)?;
        let id = Uuid::try_parse(id).map_err(|_| invalid())?;
        Ok(Self { at, id })
    }
}

/// Position in a list ordered by something other than time (names, codes):
/// an opaque offset. Use [`Keyset`] for time-ordered lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OffsetCursor(pub u32);

impl OffsetCursor {
    pub fn encode(&self) -> String {
        URL_SAFE_NO_PAD.encode(format!("o{}", self.0))
    }

    pub fn decode(cursor: &str) -> AppResult<Self> {
        let invalid =
            || AppError::field("cursor", "INVALID_CURSOR", "cursor is not valid; start again from the first page");
        let raw = URL_SAFE_NO_PAD.decode(cursor).map_err(|_| invalid())?;
        let text = std::str::from_utf8(&raw).map_err(|_| invalid())?;
        text.strip_prefix('o').and_then(|n| n.parse().ok()).map(Self).ok_or_else(invalid)
    }

    /// The offset to start at for an optional cursor.
    pub fn start(cursor: Option<&str>) -> AppResult<u32> {
        Ok(cursor.map(Self::decode).transpose()?.map_or(0, |c| c.0))
    }

    /// The cursor of the next page, if `fetched` (limit + 1 rows) had one more row.
    pub fn next(start: u32, limit: u32, fetched: usize) -> Option<String> {
        (fetched > limit as usize).then(|| Self(start + limit).encode())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyset_round_trips() {
        let key =
            Keyset { at: DateTime::from_timestamp_micros(1_792_000_000_123_456).expect("valid"), id: Uuid::now_v7() };
        assert_eq!(Keyset::decode(&key.encode()).expect("decodes"), key);
    }

    #[test]
    fn bad_cursors_are_validation_errors() {
        for bad in ["", "!!", "bm9wZQ", &URL_SAFE_NO_PAD.encode("12.not-a-uuid")] {
            assert_eq!(Keyset::decode(bad).expect_err("rejected").code(), "VALIDATION_FAILED", "{bad}");
        }
    }

    #[test]
    fn offset_cursor_round_trips() {
        assert_eq!(OffsetCursor::decode(&OffsetCursor(150).encode()).ok(), Some(OffsetCursor(150)));
        assert!(OffsetCursor::decode(&Keyset { at: DateTime::UNIX_EPOCH, id: Uuid::nil() }.encode()).is_err());
        assert_eq!(OffsetCursor::next(0, 50, 51), Some(OffsetCursor(50).encode()));
        assert_eq!(OffsetCursor::next(0, 50, 50), None);
    }

    #[test]
    fn limit_bounds() {
        assert_eq!(limit(None).ok(), Some(DEFAULT_LIMIT));
        assert_eq!(limit(Some(MAX_LIMIT)).ok(), Some(MAX_LIMIT));
        assert!(limit(Some(0)).is_err());
        assert!(limit(Some(MAX_LIMIT + 1)).is_err());
    }
}
