//! 内存缓存（moka）：缓存「每日一诗」与「按 ID 查询」的结果。
//!
//! 随机接口刻意不缓存——它本就应每次返回不同结果。

use std::time::Duration;

use moka::future::Cache;

use crate::models::Poem;

#[derive(Clone)]
pub struct AppCache {
    /// 每日一诗：按日期缓存，避免同一天内重复查库
    daily: Cache<String, Poem>,
    /// 按 ID 查询：缓存热门诗词详情
    by_id: Cache<i64, Poem>,
}

impl AppCache {
    pub fn new() -> Self {
        Self {
            daily: Cache::builder()
                .time_to_live(Duration::from_secs(6 * 3600))
                .build(),
            by_id: Cache::builder()
                .max_capacity(1024)
                .time_to_live(Duration::from_secs(5 * 60))
                .build(),
        }
    }

    pub fn daily(&self) -> &Cache<String, Poem> {
        &self.daily
    }

    pub fn by_id(&self) -> &Cache<i64, Poem> {
        &self.by_id
    }
}

impl Default for AppCache {
    fn default() -> Self {
        Self::new()
    }
}
