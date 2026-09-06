//! 健康检查接口。

use actix_web::{get, HttpResponse};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct Health {
    pub status: String,
}

/// 健康检查。
#[utoipa::path(
    get,
    path = "/api/v1/health",
    responses((status = 200, description = "服务正常", body = Health))
)]
#[get("/health")]
pub async fn health() -> HttpResponse {
    HttpResponse::Ok().json(Health {
        status: "ok".to_string(),
    })
}
