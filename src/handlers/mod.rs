//! HTTP 层：路由注册与共享状态。

pub mod health;
pub mod home;
pub mod meta;
pub mod poem;

use actix_web::web;

use crate::cache::AppCache;

/// 各 handler 共享的应用状态。
#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::SqlitePool,
    pub cache: AppCache,
}

/// 注册 `/api/v1` 下的具体路由。
///
/// scope 与中间件（限流、认证）在 `main.rs` 中装配，此处只声明资源。
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(health::health)
        .service(poem::list_poems)
        .service(poem::random_poem)
        .service(poem::daily_poem)
        .service(poem::get_poem)
        .service(meta::dynasties)
        .service(meta::authors)
        .service(meta::tags);
}
