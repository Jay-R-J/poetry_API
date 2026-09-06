//! 数据库初始化：连接池、迁移、种子数据导入。

use std::str::FromStr;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};

use crate::{config::Config, error::AppError, models::SeedPoem};

/// 初始化连接池：建立连接、执行迁移、必要时导入种子数据。
pub async fn init(config: &Config) -> Result<SqlitePool, AppError> {
    ensure_db_dir(&config.database_url)?;

    // 内存库只能单连接共享（每个连接是独立的内存数据库）
    let max_connections = if config.database_url.contains(":memory:") {
        1
    } else {
        5
    };

    // create_if_missing(true)：首次启动时自动创建数据库文件
    let options = SqliteConnectOptions::from_str(&config.database_url)?.create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(max_connections)
        .connect_with(options)
        .await?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    seed_if_empty(&pool).await?;

    Ok(pool)
}

/// 确保数据库文件所在目录存在。
fn ensure_db_dir(url: &str) -> Result<(), AppError> {
    let path = url
        .strip_prefix("sqlite://")
        .or_else(|| url.strip_prefix("sqlite:"))
        .unwrap_or(url);

    if path == ":memory:" {
        return Ok(());
    }

    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| AppError::Internal(e.to_string()))?;
        }
    }
    Ok(())
}

/// 表为空时导入 `data/poems.json`（编译期内嵌，无运行时路径依赖）。
async fn seed_if_empty(pool: &SqlitePool) -> Result<(), AppError> {
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM poems")
        .fetch_one(pool)
        .await?;
    if count > 0 {
        return Ok(());
    }

    let poems: Vec<SeedPoem> = serde_json::from_str(include_str!("../data/poems.json"))?;
    let total = poems.len();

    // 单个事务批量写入，避免逐条自动提交带来的开销
    let mut tx = pool.begin().await?;
    for poem in poems {
        sqlx::query(
            "INSERT INTO poems (title, author, dynasty, content, tags, translation, category)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(poem.title)
        .bind(poem.author)
        .bind(poem.dynasty)
        .bind(serde_json::to_string(&poem.content)?)
        .bind(serde_json::to_string(&poem.tags)?)
        .bind(poem.translation)
        .bind(poem.category.unwrap_or_else(|| "诗".to_string()))
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    tracing::info!(count = total, "已导入种子诗词数据");
    Ok(())
}
