//! chinese-poetry 数据集导入工具。
//!
//! 用法：`cargo run --release --bin import -- <chinese-poetry 仓库 json 目录路径>`
//!
//! 支持的数据源（文件名模式 -> 朝代 / 分类）：
//! - `poet.tang.*.json` -> 唐 / 诗
//! - `poet.song.*.json` -> 宋 / 诗
//! - `ci.song.*.json`   -> 宋 / 词
//!
//! 数据集原文为繁体，导入时统一转换为简体。
//! 通过 `ext_key`（标题|作者|内容的哈希）唯一索引去重，可安全重复执行。

use std::path::PathBuf;

use sqlx::sqlite::SqlitePool;

#[derive(serde::Deserialize)]
struct RawPoem {
    /// 旧版 poet.* 文件的标题
    #[serde(default)]
    title: Option<String>,
    /// 新版 ci.song 文件的词牌名（代替 title）
    #[serde(default)]
    rhythmic: Option<String>,
    #[serde(default)]
    paragraphs: Vec<String>,
    #[serde(default)]
    author: String,
}

impl RawPoem {
    fn effective_title(&self) -> Option<&str> {
        self.title
            .as_deref()
            .filter(|t| !t.trim().is_empty())
            .or(self.rhythmic.as_deref().filter(|t| !t.trim().is_empty()))
    }
}

/// 数据源规则：文件名前缀 -> (朝代, 分类)
const SOURCES: &[(&str, &str, &str)] = &[
    ("poet.tang.", "唐", "诗"),
    ("poet.song.", "宋", "诗"),
    ("ci.song.", "宋", "词"),
];

fn main() -> std::io::Result<()> {
    let mut args = std::env::args().skip(1);
    let dir = args.next().unwrap_or_else(|| {
        eprintln!("用法: cargo run --release --bin import -- <chinese-poetry json 目录>");
        std::process::exit(2);
    });

    #[actix_web::main]
    async fn run(dir: String) -> std::io::Result<()> {
        inner_run(dir)
            .await
            .map_err(|e| std::io::Error::other(e.to_string()))
    }

    run(dir)
}

async fn inner_run(dir: String) -> Result<(), Box<dyn std::error::Error>> {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    let config = poem_api::config::Config::from_env();
    let pool = poem_api::db::init(&config).await?;
    tracing::info!(db = %config.database_url, "数据库已就绪（迁移 + 种子导入完成）");

    // 繁体 -> 简体转换器（OpenCC，内置字典，整个导入过程复用一个实例）
    let opencc = ferrous_opencc::OpenCC::from_config(ferrous_opencc::config::BuiltinConfig::T2s)?;

    let mut total_inserted = 0usize;
    for (prefix, dynasty, category) in SOURCES {
        let inserted = import_source(&pool, &opencc, &dir, prefix, dynasty, category).await?;
        tracing::info!(prefix, dynasty, category, inserted, "该数据源导入完成");
        total_inserted += inserted;
    }

    // 删除与种子数据（ext_key 为 NULL 的手工精修行）同名同作者的导入副本，
    // 保留带译文和标签的种子版本，避免检索时出现重复条目
    let removed = sqlx::query(
        "DELETE FROM poems WHERE ext_key IS NOT NULL AND EXISTS (\
             SELECT 1 FROM poems s \
             WHERE s.ext_key IS NULL AND s.title = poems.title AND s.author = poems.author)",
    )
    .execute(&pool)
    .await?
    .rows_affected();
    if removed > 0 {
        tracing::info!(removed, "已删除与种子重复的导入副本");
    }

    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM poems")
        .fetch_one(&pool)
        .await?;
    tracing::info!(
        total_inserted,
        removed,
        library_total = count,
        "全部导入完成"
    );
    Ok(())
}

/// 按文件名前缀导入一组数据文件。
async fn import_source(
    pool: &SqlitePool,
    opencc: &ferrous_opencc::OpenCC,
    dir: &str,
    prefix: &str,
    dynasty: &str,
    category: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(prefix) && n.ends_with(".json"))
        })
        .collect();
    files.sort();

    if files.is_empty() {
        tracing::warn!(prefix, "未找到匹配的数据文件");
        return Ok(0);
    }

    let mut inserted = 0usize;
    for (i, path) in files.iter().enumerate() {
        let raw = std::fs::read_to_string(path)?;
        let poems: Vec<RawPoem> = serde_json::from_str(&raw)?;

        let mut tx = pool.begin().await?;
        for poem in poems {
            // 数据集为繁体或简体，统一转简体入库；新版宋词文件用词牌名作标题
            let Some(raw_title) = poem.effective_title() else {
                continue;
            };
            let title = opencc.convert(raw_title);
            let author = opencc.convert(&poem.author);
            let content: Vec<String> = poem
                .paragraphs
                .into_iter()
                .map(|l| opencc.convert(l.trim()))
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();
            if content.is_empty() {
                continue;
            }
            let ext_key = format!(
                "fnv1a:{:016x}",
                fnv1a(&format!("{}|{}|{}", title, author, content.join("\n")))
            );
            let res = sqlx::query(
                "INSERT OR IGNORE INTO poems (title, author, dynasty, content, tags, translation, category, ext_key)
                 VALUES (?, ?, ?, ?, '[]', NULL, ?, ?)",
            )
            .bind(&title)
            .bind(&author)
            .bind(dynasty)
            .bind(serde_json::to_string(&content)?)
            .bind(category)
            .bind(&ext_key)
            .execute(&mut *tx)
            .await?;
            inserted += res.rows_affected() as usize;
        }
        tx.commit().await?;

        if (i + 1) % 50 == 0 {
            tracing::info!(
                prefix,
                progress = format!("{}/{}", i + 1, files.len()),
                inserted
            );
        }
    }
    Ok(inserted)
}

/// FNV-1a 64 位哈希（与 daily.rs 中实现一致，用于生成去重键）。
fn fnv1a(bytes: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    hash
}
