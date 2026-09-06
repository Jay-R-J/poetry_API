//! 集成测试：内存 SQLite + `actix_web::test`。

use actix_web::body::BoxBody;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::{test, web, App};

use poem_api::{
    cache::AppCache,
    config::Config,
    db,
    handlers::{self, AppState},
    middleware::Auth,
};

/// 构建测试应用（内存数据库 + 种子数据）。
async fn test_app(
    api_key: Option<String>,
) -> impl Service<actix_http::Request, Response = ServiceResponse<BoxBody>, Error = actix_web::Error>
{
    let config = Config {
        host: "127.0.0.1".to_string(),
        port: 0,
        database_url: "sqlite::memory:".to_string(),
        api_key,
        rate_limit: 1000,
        rate_window_secs: 60,
        trust_proxy: false,
    };
    let pool = db::init(&config).await.expect("初始化数据库");
    let app_state = web::Data::new(AppState {
        pool,
        cache: AppCache::new(),
    });

    test::init_service(
        App::new().app_data(app_state).service(
            web::scope("/api/v1")
                .wrap(Auth::new(config.api_key))
                .configure(handlers::configure_routes),
        ),
    )
    .await
}

#[actix_web::test]
async fn health_returns_ok() {
    let app = test_app(None).await;
    let req = test::TestRequest::get().uri("/api/v1/health").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

#[actix_web::test]
async fn daily_poem_is_stable_within_a_day() {
    let app = test_app(None).await;

    let first: serde_json::Value = {
        let req = test::TestRequest::get()
            .uri("/api/v1/poems/daily")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        test::read_body_json(resp).await
    };

    let second: serde_json::Value = {
        let req = test::TestRequest::get()
            .uri("/api/v1/poems/daily")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        test::read_body_json(resp).await
    };

    assert_eq!(first["data"]["id"], second["data"]["id"]);
}

#[actix_web::test]
async fn random_returns_a_poem() {
    let app = test_app(None).await;
    let req = test::TestRequest::get()
        .uri("/api/v1/poems/random")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["data"]["title"].is_string());
}

#[actix_web::test]
async fn list_poems_returns_paginated_items() {
    let app = test_app(None).await;
    let req = test::TestRequest::get()
        .uri("/api/v1/poems?page=1&page_size=5")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["data"]["total"].as_i64().unwrap() > 0);
    assert!(body["data"]["items"].as_array().unwrap().len() <= 5);
}

#[actix_web::test]
async fn get_poem_by_id_and_not_found() {
    let app = test_app(None).await;

    let req = test::TestRequest::get().uri("/api/v1/poems/1").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["id"].as_i64().unwrap(), 1);

    let req = test::TestRequest::get()
        .uri("/api/v1/poems/999999")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);
}

#[actix_web::test]
async fn search_by_keyword() {
    let app = test_app(None).await;
    let req = test::TestRequest::get()
        .uri("/api/v1/poems?keyword=%E6%9C%88")
        .to_request(); // keyword=月
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["data"]["total"].as_i64().unwrap() > 0);
}

#[actix_web::test]
async fn auth_enforced_when_configured() {
    let app = test_app(Some("secret".to_string())).await;

    // 未携带 API Key -> 401
    let req = test::TestRequest::get()
        .uri("/api/v1/poems/daily")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);

    // 携带正确的 X-API-Key -> 200
    let req = test::TestRequest::get()
        .uri("/api/v1/poems/daily")
        .insert_header(("X-API-Key", "secret"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // 错误的 Key -> 401
    let req = test::TestRequest::get()
        .uri("/api/v1/poems/daily")
        .insert_header(("X-API-Key", "wrong"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}
