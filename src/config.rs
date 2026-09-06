//! 应用配置：从环境变量 / `.env` 文件读取，带合理默认值。

use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    /// 监听地址
    pub host: String,
    /// 监听端口
    pub port: u16,
    /// SQLite 连接串
    pub database_url: String,
    /// 可选 API Key；为 None 时认证关闭（开放模式）
    pub api_key: Option<String>,
    /// 每个 IP 在窗口内允许的最大请求数
    pub rate_limit: u32,
    /// 限流窗口时长（秒）
    pub rate_window_secs: u64,
    /// 是否信任反向代理的 X-Forwarded-For 头（用于限流取真实客户端 IP）
    pub trust_proxy: bool,
}

impl Config {
    /// 从环境变量读取配置；缺失的项使用默认值。
    ///
    /// 如果存在 `.env` 文件，`dotenvy` 会先加载其中的变量（不会覆盖已有环境变量）。
    pub fn from_env() -> Self {
        let _ = dotenvy::dotenv();

        Self {
            host: env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: env::var("PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(8080),
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://data/poems.db".to_string()),
            api_key: env::var("API_KEY").ok().filter(|s| !s.trim().is_empty()),
            rate_limit: env::var("RATE_LIMIT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(100),
            rate_window_secs: env::var("RATE_WINDOW_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(60),
            trust_proxy: env::var("TRUST_PROXY")
                .ok()
                .map(|s| matches!(s.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
                .unwrap_or(false),
        }
    }
}
