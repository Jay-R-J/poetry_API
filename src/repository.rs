//! 数据访问层：所有 SQL 查询集中在此。

use sqlx::{QueryBuilder, Sqlite, SqlitePool};

use crate::{
    error::AppError,
    models::{Poem, PoemRow},
};

/// 随机返回一首。
pub async fn random(pool: &SqlitePool) -> Result<Poem, AppError> {
    let row = sqlx::query_as::<_, PoemRow>("SELECT * FROM poems ORDER BY RANDOM() LIMIT 1")
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(row.into())
}

/// 按 ID 查询。
pub async fn by_id(pool: &SqlitePool, id: i64) -> Result<Poem, AppError> {
    let row = sqlx::query_as::<_, PoemRow>("SELECT * FROM poems WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(row.into())
}

/// 精选诗词数量（手工收录、带译文/标签的种子数据，`ext_key` 为空）。
pub async fn curated_count(pool: &SqlitePool) -> Result<i64, AppError> {
    let (total,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM poems WHERE ext_key IS NULL")
        .fetch_one(pool)
        .await?;
    Ok(total)
}

/// 在精选集内按偏移量取一首（用于「每日一诗」，保证每天都是有译文的经典篇目）。
pub async fn curated_by_offset(pool: &SqlitePool, offset: i64) -> Result<Poem, AppError> {
    let row = sqlx::query_as::<_, PoemRow>(
        "SELECT * FROM poems WHERE ext_key IS NULL ORDER BY id LIMIT 1 OFFSET ?",
    )
    .bind(offset)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(row.into())
}

/// 列表查询的筛选条件。
#[derive(Debug, Default)]
pub struct ListFilter {
    /// 朝代（精确匹配）
    pub dynasty: Option<String>,
    /// 作者（模糊匹配）
    pub author: Option<String>,
    /// 分类（诗 / 词，精确匹配）
    pub category: Option<String>,
    /// 关键词（标题/作者/朝代/内容/标签模糊匹配）
    pub keyword: Option<String>,
}

/// 分页列表 + 满足条件的总数。
pub async fn list(
    pool: &SqlitePool,
    filter: &ListFilter,
    page: i64,
    page_size: i64,
) -> Result<(Vec<Poem>, i64), AppError> {
    let mut qb = QueryBuilder::<Sqlite>::new("SELECT * FROM poems");
    apply_filters(&mut qb, filter);
    qb.push(" ORDER BY id LIMIT ")
        .push_bind(page_size)
        .push(" OFFSET ")
        .push_bind((page - 1) * page_size);

    let rows = qb.build_query_as::<PoemRow>().fetch_all(pool).await?;
    let items = rows.into_iter().map(Poem::from).collect();

    let mut cqb = QueryBuilder::<Sqlite>::new("SELECT COUNT(*) FROM poems");
    apply_filters(&mut cqb, filter);
    let (total,): (i64,) = cqb.build_query_as().fetch_one(pool).await?;

    Ok((items, total))
}

/// 把筛选条件追加到查询构建器（`?` 占位符与绑定值一一对应）。
fn apply_filters(qb: &mut QueryBuilder<'_, Sqlite>, filter: &ListFilter) {
    let mut has_where = false;

    if let Some(dynasty) = &filter.dynasty {
        qb.push(" WHERE dynasty = ").push_bind(dynasty.to_owned());
        has_where = true;
    }
    if let Some(category) = &filter.category {
        qb.push(if has_where { " AND " } else { " WHERE " });
        qb.push("category = ").push_bind(category.to_owned());
        has_where = true;
    }
    if let Some(author) = &filter.author {
        qb.push(if has_where {
            " AND author LIKE "
        } else {
            " WHERE author LIKE "
        });
        qb.push_bind(format!("%{author}%"));
        has_where = true;
    }
    if let Some(keyword) = &filter.keyword {
        qb.push(if has_where { " AND " } else { " WHERE " });
        // ≥ 3 字符走 FTS5 trigram 索引，更短的子串索引无法命中，退回 LIKE
        if keyword.chars().count() >= 3 {
            qb.push("id IN (SELECT rowid FROM poems_fts WHERE poems_fts MATCH ")
                .push_bind(fts_phrase(keyword))
                .push(")");
        } else {
            qb.push("(title LIKE ")
                .push_bind(format!("%{keyword}%"))
                .push(" OR author LIKE ")
                .push_bind(format!("%{keyword}%"))
                .push(" OR dynasty LIKE ")
                .push_bind(format!("%{keyword}%"))
                .push(" OR content LIKE ")
                .push_bind(format!("%{keyword}%"))
                .push(" OR tags LIKE ")
                .push_bind(format!("%{keyword}%"))
                .push(")");
        }
    }
}

/// 把关键词转成 FTS5 短语字面量（双引号包裹并转义），按原样子串匹配。
fn fts_phrase(keyword: &str) -> String {
    format!("\"{}\"", keyword.replace('"', "\"\""))
}

/// 朝代分布（数量降序）。
pub async fn dynasties(pool: &SqlitePool) -> Result<Vec<(String, i64)>, AppError> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT dynasty, COUNT(*) FROM poems GROUP BY dynasty ORDER BY COUNT(*) DESC, dynasty",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// 作者分布，可按朝代过滤（数量降序，最多 200 条）。
pub async fn authors(
    pool: &SqlitePool,
    dynasty: Option<&str>,
) -> Result<Vec<(String, i64)>, AppError> {
    let rows: Vec<(String, i64)> = match dynasty {
        Some(d) => {
            sqlx::query_as(
                "SELECT author, COUNT(*) FROM poems WHERE dynasty = ? \
                 GROUP BY author ORDER BY COUNT(*) DESC, author LIMIT 200",
            )
            .bind(d)
            .fetch_all(pool)
            .await?
        }
        None => {
            sqlx::query_as(
                "SELECT author, COUNT(*) FROM poems \
                 GROUP BY author ORDER BY COUNT(*) DESC, author LIMIT 200",
            )
            .fetch_all(pool)
            .await?
        }
    };
    Ok(rows)
}

/// 标签分布（数量降序，最多 200 条）。
pub async fn tags(pool: &SqlitePool) -> Result<Vec<(String, i64)>, AppError> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT je.value, COUNT(*) FROM poems p, json_each(p.tags) je \
         GROUP BY je.value ORDER BY COUNT(*) DESC, je.value LIMIT 200",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
