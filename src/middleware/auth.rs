//! API Key 认证中间件。
//!
//! 未配置 `API_KEY` 时为开放模式（直接放行）；配置后要求请求携带
//! `X-API-Key: <key>` 或 `Authorization: Bearer <key>`。

use std::future::{ready, Ready};
use std::task::{Context, Poll};

use actix_web::{
    body::BoxBody,
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    http::header,
    Error, ResponseError,
};
use futures_util::future::LocalBoxFuture;

use crate::error::AppError;

#[derive(Clone)]
pub struct Auth {
    /// 配置的 API Key；为 `None` 时跳过认证。
    key: Option<String>,
}

impl Auth {
    pub fn new(key: Option<String>) -> Self {
        Self { key }
    }

    fn is_authorized(&self, req: &ServiceRequest) -> bool {
        let expected = match &self.key {
            None => return true,
            Some(k) => k.as_str(),
        };

        let check = |provided: &str| constant_time_eq(provided, expected);

        // 优先检查 X-API-Key 头
        if let Some(v) = req.headers().get("x-api-key").and_then(|v| v.to_str().ok()) {
            return check(v);
        }

        // 再检查 Authorization: Bearer <key>
        req.headers()
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(check)
            .unwrap_or(false)
    }
}

/// 常量时间字符串比较，避免通过比较耗时推测 Key 内容。
fn constant_time_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut diff = (a.len() ^ b.len()) as u8;
    let n = a.len().max(b.len());
    for i in 0..n {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        diff |= x ^ y;
    }
    diff == 0
}

impl<S> Transform<S, ServiceRequest> for Auth
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error> + 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = AuthMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddleware {
            service,
            auth: self.clone(),
        }))
    }
}

pub struct AuthMiddleware<S> {
    service: S,
    auth: Auth,
}

impl<S> Service<ServiceRequest> for AuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error> + 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        if self.auth.is_authorized(&req) {
            Box::pin(self.service.call(req))
        } else {
            let resp = AppError::Unauthorized
                .error_response()
                .map_into_boxed_body();
            let resp = ServiceResponse::new(req.request().clone(), resp);
            Box::pin(async move { Ok(resp) })
        }
    }
}
