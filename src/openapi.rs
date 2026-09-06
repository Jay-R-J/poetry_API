//! OpenAPI 文档定义（utoipa）。

use utoipa::OpenApi;

use crate::handlers::health::Health;
use crate::handlers::meta::StatListResponse;
use crate::models::{ErrorResponse, Poem, PoemList, PoemListResponse, PoemResponse, StatItem};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::handlers::health::health,
        crate::handlers::poem::list_poems,
        crate::handlers::poem::random_poem,
        crate::handlers::poem::daily_poem,
        crate::handlers::poem::get_poem,
        crate::handlers::meta::dynasties,
        crate::handlers::meta::authors,
        crate::handlers::meta::tags,
    ),
    components(schemas(
        Health,
        Poem,
        PoemList,
        PoemResponse,
        PoemListResponse,
        ErrorResponse,
        StatItem,
        StatListResponse,
    )),
    info(
        title = "诗词 API",
        version = "0.1.0",
        description = "基于 Rust + Actix Web + SQLite 的古诗词 REST API",
    )
)]
pub struct ApiDoc;
