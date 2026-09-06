//! 统一错误类型与错误响应。

use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use thiserror::Error;

use crate::models::ErrorResponse;

/// 应用统一错误类型。
///
/// 通过实现 [`ResponseError`]，actix 会在 handler 返回 `Err` 时
/// 自动把错误转换为符合规范的 JSON 响应。
#[derive(Debug, Error)]
pub enum AppError {
    #[error("资源不存在")]
    NotFound,

    #[error("请求参数错误：{0}")]
    BadRequest(String),

    #[error("未授权：缺少或错误的 API Key")]
    Unauthorized,

    #[error("请求过于频繁，请稍后再试")]
    RateLimited,

    #[error("服务器内部错误：{0}")]
    Internal(String),
}

impl AppError {
    /// 业务状态码，与 HTTP 状态码保持一致，便于前端按 `code` 判断。
    fn code(&self) -> i32 {
        match self {
            AppError::NotFound => 404,
            AppError::BadRequest(_) => 400,
            AppError::Unauthorized => 401,
            AppError::RateLimited => 429,
            AppError::Internal(_) => 500,
        }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Internal(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Internal(e.to_string())
    }
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).json(ErrorResponse {
            code: self.code(),
            message: self.to_string(),
            data: None,
        })
    }
}
