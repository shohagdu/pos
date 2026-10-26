//! Uploaded files (DB design §9.9). Any member of the business may read its
//! files (product photos are shown on the POS); each upload endpoint checks
//! its own permission.

use actix_web::{HttpResponse, get, http::header, web};
use uuid::Uuid;

use crate::{ApiError, AppState, auth::TenantAuth};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(get_file);
}

/// The file's bytes. Files never change (a new upload gets a new id), so the
/// browser may keep them privately for a day.
#[utoipa::path(get, path = "/api/v1/files/{id}", tag = "files", security(("bearer" = [])), params(("id" = Uuid, Path)),
    responses((status = 200, content_type = "application/octet-stream", body = Vec<u8>), (status = 404, body = crate::error::Problem)))]
#[get("/files/{id}")]
pub async fn get_file(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let mut tx = auth.begin(&state).await?;
    let file = pos_files::get(&mut tx, path.into_inner()).await?;
    tx.commit().await?;
    let bytes = state.storage.get(&file.storage_key).await?;
    Ok(HttpResponse::Ok()
        .content_type(file.content_type)
        .insert_header((header::CACHE_CONTROL, "private, max-age=86400, immutable"))
        .insert_header((header::CONTENT_DISPOSITION, "inline"))
        .body(bytes))
}
