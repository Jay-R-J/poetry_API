//! 元数据聚合接口：朝代 / 作者 / 标签分布。

use actix_web::{get, web, HttpResponse};
use utoipa::IntoParams;

use crate::{error::AppError, models::StatItem, repository};

use super::AppState;

/// 把 `(名称, 数量)` 行转成统一统计结构。
fn to_stats(rows: Vec<(String, i64)>) -> Vec<StatItem> {
    rows.into_iter()
        .map(|(name, count)| StatItem { name, count })
        .collect()
}

/// 朝代分布。
#[utoipa::path(
    get,
    path = "/api/v1/dynasties",
    responses(
        (status = 200, description = "朝代列表（按作品数量降序）", body = StatListResponse),
        (status = 500, description = "服务器错误", body = ErrorResponse),
    )
)]
#[get("/dynasties")]
pub async fn dynasties(state: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let stats = to_stats(repository::dynasties(&state.pool).await?);
    Ok(HttpResponse::Ok().json(StatListResponse::ok(stats)))
}

/// 作者分布，可按朝代过滤。
#[utoipa::path(
    get,
    path = "/api/v1/authors",
    params(AuthorQuery),
    responses(
        (status = 200, description = "作者列表（按作品数量降序，最多 200 条）", body = StatListResponse),
        (status = 500, description = "服务器错误", body = ErrorResponse),
    )
)]
#[get("/authors")]
pub async fn authors(
    state: web::Data<AppState>,
    query: web::Query<AuthorQuery>,
) -> Result<HttpResponse, AppError> {
    let stats = to_stats(repository::authors(&state.pool, query.dynasty.as_deref()).await?);
    Ok(HttpResponse::Ok().json(StatListResponse::ok(stats)))
}

/// 标签分布。
#[utoipa::path(
    get,
    path = "/api/v1/tags",
    responses(
        (status = 200, description = "标签列表（按作品数量降序，最多 200 条）", body = StatListResponse),
        (status = 500, description = "服务器错误", body = ErrorResponse),
    )
)]
#[get("/tags")]
pub async fn tags(state: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let stats = to_stats(repository::tags(&state.pool).await?);
    Ok(HttpResponse::Ok().json(StatListResponse::ok(stats)))
}

/// 作者查询参数。
#[derive(Debug, Default, serde::Deserialize, IntoParams)]
pub struct AuthorQuery {
    /// 朝代精确匹配（如：唐 / 宋）
    pub dynasty: Option<String>,
}

/// 统一成功响应：统计列表
#[derive(Debug, utoipa::ToSchema, serde::Serialize)]
pub struct StatListResponse {
    pub code: i32,
    pub message: String,
    pub data: Vec<StatItem>,
}

impl StatListResponse {
    fn ok(data: Vec<StatItem>) -> Self {
        Self {
            code: 0,
            message: "ok".to_string(),
            data,
        }
    }
}
