-- 诗词表：content 与 tags 以 JSON 数组文本存储
CREATE TABLE IF NOT EXISTS poems (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    title       TEXT NOT NULL,
    author      TEXT NOT NULL,
    dynasty     TEXT NOT NULL,
    content     TEXT NOT NULL,            -- JSON 数组：逐句文本
    tags        TEXT NOT NULL DEFAULT '[]', -- JSON 数组：标签
    translation TEXT                      -- 可选译文
);

CREATE INDEX IF NOT EXISTS idx_poems_dynasty ON poems (dynasty);
CREATE INDEX IF NOT EXISTS idx_poems_author  ON poems (author);
