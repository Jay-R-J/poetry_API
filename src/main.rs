//! 程序入口：装配配置、日志、数据库、缓存、中间件与路由。

use actix_web::{web, App, HttpServer};
use tracing_subscriber::EnvFilter;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use poem_api::{
    cache::AppCache,
    config::Config,
    db,
    handlers::{self, AppState},
    middleware::{Auth, RateLimit},
    openapi,
};

fn init_tracing() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("poem_api=debug,info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_tracing();

    let config = Config::from_env();

    let pool = db::init(&config).await.unwrap_or_else(|e| {
        tracing::error!(error = %e, "数据库初始化失败");
        std::process::exit(1);
    });

    let app_state = web::Data::new(AppState {
        pool,
        cache: AppCache::new(),
    });

    let rate_limit = RateLimit::new(
        config.rate_limit,
        config.rate_window_secs,
        config.trust_proxy,
    );
    let auth = Auth::new(config.api_key.clone());

    let bind = format!("{}:{}", config.host, config.port);
    tracing::info!(address = %bind, "🚀 诗词 API 服务启动");
    if config.api_key.is_some() {
        tracing::info!("API Key 认证已开启");
    } else {
        tracing::info!("API Key 认证未开启（开放模式）");
    }

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .wrap(tracing_actix_web::TracingLogger::default())
            .wrap(actix_web::middleware::Compress::default())
            .wrap(actix_cors::Cors::permissive())
            .service(
                web::scope("/api/v1")
                    .wrap(rate_limit.clone())
                    .wrap(auth.clone())
                    .configure(handlers::configure_routes),
            )
            .service(handlers::home::index)
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", openapi::ApiDoc::openapi()),
            )
    })
    .bind(bind)?
    .run()
    .await
}
