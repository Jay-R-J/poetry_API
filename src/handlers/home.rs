//! 首页：内嵌的「每日一诗」网页。

use actix_web::{get, HttpResponse, Responder};

/// 返回编译期内嵌的静态页面（`static/index.html`）。
#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../../static/index.html"))
}
