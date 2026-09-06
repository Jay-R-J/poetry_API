//! 基于滑动窗口的每 IP 限流中间件。
//!
//! - 窗口外的空 IP 条目会随周期清扫回收，避免长期运行内存增长
//! - 部署在反向代理后时设置 `TRUST_PROXY=true`，从 `X-Forwarded-For`
//!   取真实客户端 IP（取最左侧一跳）

use std::collections::{HashMap, VecDeque};
use std::future::{ready, Ready};
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use actix_web::{
    body::BoxBody,
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse,
};
use futures_util::future::LocalBoxFuture;

use crate::models::ErrorResponse;

#[derive(Clone)]
pub struct RateLimit {
    limit: u32,
    window: Duration,
    trust_proxy: bool,
    /// 客户端 IP -> 请求时间戳队列，以及上次清扫时间
    state: Arc<Mutex<HashMap<IpAddr, VecDeque<Instant>>>>,
    last_sweep: Arc<Mutex<Instant>>,
}

impl RateLimit {
    pub fn new(limit: u32, window_secs: u64, trust_proxy: bool) -> Self {
        Self {
            limit,
            window: Duration::from_secs(window_secs),
            trust_proxy,
            state: Arc::new(Mutex::new(HashMap::new())),
            last_sweep: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// 解析客户端 IP：受信代理模式下优先取 `X-Forwarded-For` 最左侧地址。
    fn client_ip(&self, req: &ServiceRequest) -> Option<IpAddr> {
        if self.trust_proxy {
            if let Some(ip) = req
                .headers()
                .get("x-forwarded-for")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.split(',').next())
                .map(|s| s.trim())
                .and_then(|s| s.parse::<IpAddr>().ok())
            {
                return Some(ip);
            }
        }
        req.peer_addr().map(|addr| addr.ip())
    }

    /// 清扫窗口内无访问的 IP 条目，周期为一个窗口时长。
    fn sweep_if_due(&self, guard: &mut HashMap<IpAddr, VecDeque<Instant>>, now: Instant) {
        let due = {
            let mut last = self.last_sweep.lock().unwrap();
            if now.saturating_duration_since(*last) > self.window {
                *last = now;
                true
            } else {
                false
            }
        };
        if due {
            guard.retain(|_, queue| {
                queue
                    .back()
                    .is_some_and(|t| now.saturating_duration_since(*t) <= self.window)
            });
        }
    }
}

impl<S> Transform<S, ServiceRequest> for RateLimit
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error> + 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = RateLimitMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RateLimitMiddleware {
            service,
            inner: self.clone(),
        }))
    }
}

pub struct RateLimitMiddleware<S> {
    service: S,
    inner: RateLimit,
}

impl<S> Service<ServiceRequest> for RateLimitMiddleware<S>
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
        let mut retry_after: Option<u64> = None;

        if let Some(ip) = self.inner.client_ip(&req) {
            // 临界区很短且不含 await，使用 std 同步锁是合适的
            let mut guard = self.inner.state.lock().unwrap();
            let now = Instant::now();
            self.inner.sweep_if_due(&mut guard, now);

            let queue = guard.entry(ip).or_default();

            // 清理窗口外的旧记录
            while let Some(&t) = queue.front() {
                if now.saturating_duration_since(t) > self.inner.window {
                    queue.pop_front();
                } else {
                    break;
                }
            }

            if queue.len() as u32 >= self.inner.limit {
                let oldest = queue.front().copied().unwrap_or(now);
                let elapsed = now.saturating_duration_since(oldest);
                retry_after = Some(self.inner.window.saturating_sub(elapsed).as_secs().max(1));
            } else {
                queue.push_back(now);
            }
        }

        if let Some(secs) = retry_after {
            let resp = HttpResponse::TooManyRequests()
                .insert_header(("Retry-After", secs))
                .json(ErrorResponse {
                    code: 429,
                    message: "请求过于频繁，请稍后再试".to_string(),
                    data: None,
                })
                .map_into_boxed_body();
            let resp = ServiceResponse::new(req.request().clone(), resp);
            Box::pin(async move { Ok(resp) })
        } else {
            Box::pin(self.service.call(req))
        }
    }
}
