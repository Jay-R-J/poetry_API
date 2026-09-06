//! 诗词 API 核心库。
//!
//! 把业务逻辑放在 lib 中，`main.rs` 只负责装配与启动，
//! 这样集成测试（`tests/`）可以直接复用这些模块。

pub mod cache;
pub mod config;
pub mod daily;
pub mod db;
pub mod error;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod openapi;
pub mod repository;
