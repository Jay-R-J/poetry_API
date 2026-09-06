-- 扩展迁移：分类字段、导入去重键、FTS5 全文索引

-- 诗 / 词 分类
ALTER TABLE poems ADD COLUMN category TEXT NOT NULL DEFAULT '诗';

-- 导入去重键（title|author|content 的哈希），重复导入时由唯一索引拦截
ALTER TABLE poems ADD COLUMN ext_key TEXT;
CREATE UNIQUE INDEX IF NOT EXISTS idx_poems_ext_key ON poems (ext_key);

-- FTS5 trigram 全文索引（外部内容表），支持中文子串检索
CREATE VIRTUAL TABLE IF NOT EXISTS poems_fts USING fts5(
    title, author, dynasty, content, tags,
    content = 'poems',
    content_rowid = 'id',
    tokenize = 'trigram'
);

-- 增删改同步触发器（外部内容表必须手工同步）
CREATE TRIGGER IF NOT EXISTS poems_fts_ai AFTER INSERT ON poems BEGIN
    INSERT INTO poems_fts(rowid, title, author, dynasty, content, tags)
    VALUES (new.id, new.title, new.author, new.dynasty, new.content, new.tags);
END;

CREATE TRIGGER IF NOT EXISTS poems_fts_ad AFTER DELETE ON poems BEGIN
    INSERT INTO poems_fts(poems_fts, rowid, title, author, dynasty, content, tags)
    VALUES ('delete', old.id, old.title, old.author, old.dynasty, old.content, old.tags);
END;

CREATE TRIGGER IF NOT EXISTS poems_fts_au AFTER UPDATE ON poems BEGIN
    INSERT INTO poems_fts(poems_fts, rowid, title, author, dynasty, content, tags)
    VALUES ('delete', old.id, old.title, old.author, old.dynasty, old.content, old.tags);
    INSERT INTO poems_fts(rowid, title, author, dynasty, content, tags)
    VALUES (new.id, new.title, new.author, new.dynasty, new.content, new.tags);
END;

-- 回填存量数据
INSERT INTO poems_fts(poems_fts) VALUES ('rebuild');
