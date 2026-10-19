use actix_web::{HttpResponse, get, web};
use serde::Serialize;
use utoipa::{OpenApi, ToSchema};

use crate::{AppState, openapi::ApiDoc};

#[derive(Serialize, ToSchema)]
pub struct Meta {
    pub name: &'static str,
    pub version: &'static str,
    pub environment: String,
}

/// Build information for support and the frontend "about" screen.
#[utoipa::path(get, path = "/api/v1/meta", tag = "ops", responses((status = 200, body = Meta)))]
#[get("/meta")]
pub async fn meta(state: web::Data<AppState>) -> HttpResponse {
    HttpResponse::Ok().json(Meta {
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
        environment: format!("{:?}", state.config.env).to_lowercase(),
    })
}

/// OpenAPI 3.1 document; the frontend client is generated from it.
#[get("/openapi.json")]
pub async fn openapi_json() -> HttpResponse {
    HttpResponse::Ok().json(ApiDoc::openapi())
}
