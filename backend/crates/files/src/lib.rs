//! Uploaded files (DB design §9.9). The `files` table records each file; its
//! bytes live in [`Storage`] under `tenants/{tenant_id}/{purpose}/{id}.{ext}`
//! (R-TEN-06). Local disk for now; an S3-compatible store slots in behind the
//! same three calls.
//!
//! Bytes are written before the row is committed, so a rolled-back request
//! can leave an unreferenced object behind; it is harmless (nothing points at
//! it) and a later sweep can remove it. Deleting goes the other way: the row
//! goes first, the object after commit ([`Storage::delete`]).

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use pos_core::{AppError, AppResult, id::new_id};
use pos_db::{TenantTx, map_db_error};
use serde::Serialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Product images and logos (Requirement Analysis §8: ≤ 2 MB, jpg/png/webp).
pub const MAX_IMAGE_BYTES: usize = 2 * 1024 * 1024;
const MB: i64 = pos_tenancy::entitlements::MB;

/// Where file bytes are kept.
#[derive(Debug, Clone)]
pub struct Storage {
    root: PathBuf,
}

impl Storage {
    /// Files under a local directory (created on first write).
    pub fn local(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn path(&self, key: &str) -> AppResult<PathBuf> {
        // Keys are built here from ids, but never let one climb out of the root.
        if key.split('/').any(|part| part.is_empty() || part == "." || part == "..") {
            return Err(AppError::internal(format!("bad storage key {key}")));
        }
        Ok(self.root.join(Path::new(key)))
    }

    pub async fn put(&self, key: &str, bytes: &[u8]) -> AppResult<()> {
        let path = self.path(key)?;
        if let Some(dir) = path.parent() {
            tokio::fs::create_dir_all(dir).await.map_err(AppError::internal)?;
        }
        tokio::fs::write(&path, bytes).await.map_err(AppError::internal)
    }

    pub async fn get(&self, key: &str) -> AppResult<Vec<u8>> {
        match tokio::fs::read(self.path(key)?).await {
            Ok(bytes) => Ok(bytes),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(AppError::NotFound),
            Err(e) => Err(AppError::internal(e)),
        }
    }

    /// Best effort: a missing object is fine, other failures are logged.
    pub async fn delete(&self, key: &str) {
        let Ok(path) = self.path(key) else { return };
        if let Err(e) = tokio::fs::remove_file(&path).await
            && e.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(key, error = %e, "could not delete a stored file");
        }
    }
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct FileRecord {
    pub id: Uuid,
    #[serde(skip)]
    pub storage_key: String,
    pub file_name: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub purpose: String,
    pub created_at: DateTime<Utc>,
}

/// The image type from the file's first bytes (the declared type is not trusted).
pub fn sniff_image(bytes: &[u8]) -> Option<(&'static str, &'static str)> {
    match bytes {
        [0xFF, 0xD8, 0xFF, ..] => Some(("image/jpeg", "jpg")),
        [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, ..] => Some(("image/png", "png")),
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => Some(("image/webp", "webp")),
        _ => None,
    }
}

/// Checks and stores an image for an entity (e.g. a product) and records it.
pub async fn store_image(
    tx: &mut TenantTx,
    storage: &Storage,
    purpose: &'static str,
    entity: (&'static str, Uuid),
    file_name: Option<&str>,
    bytes: &[u8],
) -> AppResult<FileRecord> {
    if bytes.is_empty() {
        return Err(AppError::field("file", "REQUIRED", "choose an image"));
    }
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(AppError::field("file", "FILE_TOO_LARGE", "the image must be 2 MB or smaller"));
    }
    let (content_type, ext) = sniff_image(bytes)
        .ok_or_else(|| AppError::field("file", "UNSUPPORTED_TYPE", "use a JPG, PNG or WebP image"))?;
    let size = i64::try_from(bytes.len()).map_err(AppError::internal)?;
    let used = pos_tenancy::entitlements::storage_used_mb(tx).await?;
    let after = (used * MB + size + MB - 1) / MB;
    pos_tenancy::entitlements::ensure_within(tx, "storage.mb", used, after - used).await?;

    let tenant_id = tx.tenant_id();
    let id = new_id();
    let key = format!("tenants/{tenant_id}/{purpose}/{id}.{ext}");
    let name: String = file_name
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .unwrap_or("image")
        .chars()
        .filter(|c| !c.is_control() && *c != '/' && *c != '\\')
        .take(255)
        .collect();
    storage.put(&key, bytes).await?;
    let saved = sqlx::query(
        "INSERT INTO files (tenant_id, id, storage_key, file_name, content_type, size_bytes, checksum_sha256, purpose, \
         entity_type, entity_id, uploaded_by) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(id)
    .bind(&key)
    .bind(&name)
    .bind(content_type)
    .bind(size)
    .bind(Sha256::digest(bytes).to_vec())
    .bind(purpose)
    .bind(entity.0)
    .bind(entity.1)
    .bind(tx.user_id())
    .execute(&mut **tx)
    .await
    .map_err(map_db_error);
    if let Err(e) = saved {
        storage.delete(&key).await;
        return Err(e);
    }
    get(tx, id).await
}

pub async fn get(tx: &mut TenantTx, id: Uuid) -> AppResult<FileRecord> {
    sqlx::query_as(
        "SELECT id, storage_key, file_name, content_type, size_bytes, purpose, created_at FROM files \
         WHERE tenant_id = ? AND id = ?",
    )
    .bind(tx.tenant_id())
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(AppError::NotFound)
}

/// Deletes the row and returns the storage key; delete the object after commit.
pub async fn remove(tx: &mut TenantTx, id: Uuid) -> AppResult<String> {
    let file = get(tx, id).await?;
    sqlx::query("DELETE FROM files WHERE tenant_id = ? AND id = ?")
        .bind(tx.tenant_id())
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(file.storage_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn images_are_recognised_by_their_first_bytes() {
        assert_eq!(sniff_image(&[0xFF, 0xD8, 0xFF, 0xE0, 0]), Some(("image/jpeg", "jpg")));
        assert_eq!(sniff_image(b"\x89PNG\r\n\x1a\n...."), Some(("image/png", "png")));
        assert_eq!(sniff_image(b"RIFF\x10\0\0\0WEBPVP8 "), Some(("image/webp", "webp")));
        assert_eq!(sniff_image(b"GIF89a"), None);
        assert_eq!(sniff_image(b"<svg"), None);
    }

    #[tokio::test]
    async fn storage_round_trips_and_refuses_climbing_keys() {
        let dir = std::env::temp_dir().join(format!("pos-files-{}", new_id()));
        let storage = Storage::local(&dir);
        storage.put("tenants/t/product_image/a.png", b"abc").await.expect("put");
        assert_eq!(storage.get("tenants/t/product_image/a.png").await.expect("get"), b"abc");
        storage.delete("tenants/t/product_image/a.png").await;
        assert!(matches!(storage.get("tenants/t/product_image/a.png").await, Err(AppError::NotFound)));
        assert!(storage.put("../escape", b"x").await.is_err());
        let _ = std::fs::remove_dir_all(dir);
    }
}
