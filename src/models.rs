//! 数据模型与统一响应结构。
//!
//! 对外 API 模型（[`Poem`]）与数据库行（[`PoemRow`]）分离：
//! 数据库中 `content`/`tags` 以 JSON 文本存储，API 层则暴露为 `Vec<String>`。

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 诗词（对外 API 模型）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Poem {
    pub id: i64,
    pub title: String,
    pub author: String,
    pub dynasty: String,
    /// 分类：诗 / 词
    pub category: String,
    /// 逐句文本
    pub content: Vec<String>,
    pub tags: Vec<String>,
    /// 可选译文
    #[schema(nullable = true)]
    pub translation: Option<String>,
}

/// 诗词列表 + 分页信息
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PoemList {
    pub items: Vec<Poem>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

/// 种子诗词（data/poems.json）
#[derive(Debug, serde::Deserialize)]
pub struct SeedPoem {
    pub title: String,
    pub author: String,
    pub dynasty: String,
    pub content: Vec<String>,
    pub tags: Vec<String>,
    #[serde(default)]
    pub translation: Option<String>,
    /// 分类：诗 / 词 / 曲，缺省按「诗」处理
    #[serde(default)]
    pub category: Option<String>,
}

/// 统一成功响应：诗词
#[derive(Debug, Serialize, ToSchema)]
pub struct PoemResponse {
    pub code: i32,
    pub message: String,
    pub data: Poem,
}

impl PoemResponse {
    pub fn ok(data: Poem) -> Self {
        Self {
            code: 0,
            message: "ok".to_string(),
            data,
        }
    }
}

/// 统一成功响应：诗词列表
#[derive(Debug, Serialize, ToSchema)]
pub struct PoemListResponse {
    pub code: i32,
    pub message: String,
    pub data: PoemList,
}

impl PoemListResponse {
    pub fn ok(data: PoemList) -> Self {
        Self {
            code: 0,
            message: "ok".to_string(),
            data,
        }
    }
}

/// 统一错误响应
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub code: i32,
    pub message: String,
    /// 错误时恒为 null
    #[schema(nullable = true)]
    pub data: Option<String>,
}

/// 聚合统计项（作者 / 朝代 / 标签通用）
#[derive(Debug, Serialize, ToSchema)]
pub struct StatItem {
    pub name: String,
    pub count: i64,
}

/// 数据库行（content/tags 以 JSON 文本存储，经 sqlx 的 `Json` 类型自动编解码）
#[derive(Debug, sqlx::FromRow)]
pub struct PoemRow {
    pub id: i64,
    pub title: String,
    pub author: String,
    pub dynasty: String,
    #[sqlx(default)]
    pub category: String,
    pub content: sqlx::types::Json<Vec<String>>,
    pub tags: sqlx::types::Json<Vec<String>>,
    pub translation: Option<String>,
    #[sqlx(default)]
    pub ext_key: Option<String>,
}

impl From<PoemRow> for Poem {
    fn from(row: PoemRow) -> Self {
        Self {
            id: row.id,
            title: row.title,
            author: row.author,
            dynasty: row.dynasty,
            category: row.category,
            content: row.content.0,
            tags: row.tags.0,
            translation: row.translation,
        }
    }
}
