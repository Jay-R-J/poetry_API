//! 诗词相关接口。

use actix_web::{get, web, HttpResponse};
use utoipa::IntoParams;

// `ErrorResponse` 仅供下方 #[utoipa::path] 宏解析类型路径使用
#[allow(unused_imports)]
use crate::{
    error::AppError,
    models::{ErrorResponse, PoemList, PoemListResponse, PoemResponse},
    repository::{self, ListFilter},
};

use super::AppState;

/// 列表查询参数。
#[derive(Debug, Default, serde::Deserialize, IntoParams)]
pub struct ListQuery {
    /// 页码，从 1 开始
    pub page: Option<i64>,
    /// 每页条数（1~100）
    pub page_size: Option<i64>,
    /// 朝代精确匹配
    pub dynasty: Option<String>,
    /// 分类精确匹配（诗 / 词）
    pub category: Option<String>,
    /// 作者模糊匹配
    pub author: Option<String>,
    /// 关键词（标题/作者/朝代/内容/标签，≥3 字符走全文索引）
    pub keyword: Option<String>,
}

/// 诗词列表（分页 + 筛选 + 搜索）。
#[utoipa::path(
    get,
    path = "/api/v1/poems",
    params(ListQuery),
    responses(
        (status = 200, description = "诗词列表", body = PoemListResponse),
        (status = 400, description = "参数错误", body = ErrorResponse),
        (status = 500, description = "服务器错误", body = ErrorResponse),
    )
)]
#[get("/poems")]
pub async fn list_poems(
    state: web::Data<AppState>,
    query: web::Query<ListQuery>,
) -> Result<HttpResponse, AppError> {
    let query = query.into_inner();
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    let filter = ListFilter {
        dynasty: query.dynasty,
        category: query.category,
        author: query.author,
        keyword: query.keyword,
    };

    let (items, total) = repository::list(&state.pool, &filter, page, page_size).await?;
    let data = PoemList {
        items,
        total,
        page,
        page_size,
    };

    Ok(HttpResponse::Ok().json(PoemListResponse::ok(data)))
}

/// 随机一首。
#[utoipa::path(
    get,
    path = "/api/v1/poems/random",
    responses(
        (status = 200, description = "随机诗词", body = PoemResponse),
        (status = 500, description = "服务器错误", body = ErrorResponse),
    )
)]
#[get("/poems/random")]
pub async fn random_poem(state: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let poem = repository::random(&state.pool).await?;
    Ok(HttpResponse::Ok().json(PoemResponse::ok(poem)))
}

/// 每日一诗（当天固定，带缓存）。
///
/// 从精选集（带译文/标签的经典篇目）中按日期确定性选取，
/// 保证每天推荐的都是完整可读的名篇，而非语料库中的残篇。
#[utoipa::path(
    get,
    path = "/api/v1/poems/daily",
    responses(
        (status = 200, description = "每日一诗（精选经典篇目，当天固定）", body = PoemResponse),
        (status = 500, description = "服务器错误", body = ErrorResponse),
    )
)]
#[get("/poems/daily")]
pub async fn daily_poem(state: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let date = crate::daily::date_string();

    if let Some(poem) = state.cache.daily().get(&date).await {
        tracing::debug!(date = %date, "每日一诗命中缓存");
        return Ok(HttpResponse::Ok().json(PoemResponse::ok(poem)));
    }

    tracing::debug!(date = %date, "每日一诗未命中缓存");
    let total = repository::curated_count(&state.pool).await?;
    let offset = crate::daily::stable_index(&date, total as usize) as i64;
    let poem = repository::curated_by_offset(&state.pool, offset).await?;

    state.cache.daily().insert(date, poem.clone()).await;

    Ok(HttpResponse::Ok().json(PoemResponse::ok(poem)))
}

/// 按 ID 查询（带缓存）。
#[utoipa::path(
    get,
    path = "/api/v1/poems/{id}",
    params(("id" = i64, Path, description = "诗词 ID")),
    responses(
        (status = 200, description = "诗词详情", body = PoemResponse),
        (status = 404, description = "不存在", body = ErrorResponse),
        (status = 500, description = "服务器错误", body = ErrorResponse),
    )
)]
#[get("/poems/{id}")]
pub async fn get_poem(
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();

    if let Some(poem) = state.cache.by_id().get(&id).await {
        return Ok(HttpResponse::Ok().json(PoemResponse::ok(poem)));
    }

    let poem = repository::by_id(&state.pool, id).await?;
    state.cache.by_id().insert(id, poem.clone()).await;

    Ok(HttpResponse::Ok().json(PoemResponse::ok(poem)))
}
