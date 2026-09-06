# 诗词 API（poem_api）

一个用 **Rust + Actix Web + SQLite** 构建的古诗词 REST API。开箱自带 116 首带译文的精选诗词，还可一键导入约 13 万首唐宋诗词；自带「每日一诗」网页、全文搜索、限流、API Key 认证与 Swagger 文档。单二进制 + 内嵌 SQLite，无需安装任何数据库，适合直接部署为诗词类产品（小程序、公众号、教育 App、内容网站等）的后端数据服务。

## 功能特性

- **海量语料**：内置约 116 首带译文和标签的精选诗词；可一键导入 [chinese-poetry](https://github.com/chinese-poetry/chinese-poetry) 开源数据集（唐诗 5.7 万+ / 宋诗 5.4 万+ / 宋词 2.1 万+，自动繁转简、去重）
- **每日一诗**：从精选经典名篇（带译文）中按日期确定性选取，同一天返回同一首，附优雅的网页演示
- **全文搜索**：关键词 ≥3 字符走 SQLite FTS5 trigram 索引，毫秒级命中；短关键词自动退回模糊匹配
- **随机一首 / 详情查询**：覆盖全部语料
- **聚合接口**：朝代分布、作者分布、标签分布
- **跨域友好**：内置 CORS 与 gzip 压缩，浏览器前端可直接调用
- **API Key 认证**：可选开启，支持 `X-API-Key` 与 `Authorization: Bearer` 两种方式
- **每 IP 限流**：滑动窗口算法，支持反向代理场景（`X-Forwarded-For`），超限返回 429
- **响应缓存**：每日一诗与按 ID 查询走内存缓存（moka）
- **OpenAPI 文档**：自动生成，内置 Swagger UI
- **零外部依赖**：SQLite 内嵌，数据文件自动生成；编译产物为单个二进制

## 快速开始

```bash
# 需要 Rust 1.93+（https://rustup.rs）
git clone https://github.com/Jay-R-J/poetry_API.git
cd poem_api

# 运行（首次启动自动建表并导入 116 首精选诗词）
cargo run
```

启动后：

- 网页演示（每日一诗）：<http://localhost:8080/>
- Swagger 接口文档：<http://localhost:8080/swagger-ui/>
- 健康检查：<http://localhost:8080/api/v1/health>

> 无需安装数据库——SQLite 已内嵌，数据文件自动生成在 `data/` 目录。

## API 一览

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/v1/health` | 健康检查 |
| GET | `/api/v1/poems` | 列表（分页 + 筛选 + 搜索） |
| GET | `/api/v1/poems/random` | 随机一首 |
| GET | `/api/v1/poems/daily` | 每日一诗（精选集，当天固定） |
| GET | `/api/v1/poems/{id}` | 按 ID 查询 |
| GET | `/api/v1/dynasties` | 朝代分布（含作品数） |
| GET | `/api/v1/authors?dynasty=唐` | 作者分布（可按朝代过滤） |
| GET | `/api/v1/tags` | 标签分布 |

`/poems` 列表接口参数：

| 参数 | 说明 | 示例 |
|------|------|------|
| `page` / `page_size` | 页码（从 1 开始）/ 每页条数（1~100） | `page=1&page_size=20` |
| `keyword` | 关键词（标题/作者/朝代/内容/标签） | `keyword=明月几时有` |
| `author` | 作者模糊匹配 | `author=李白` |
| `dynasty` | 朝代精确匹配 | `dynasty=唐` |
| `category` | 分类：诗 / 词 / 曲 | `category=词` |

### 调用示例

```bash
# 每日一诗
curl http://localhost:8080/api/v1/poems/daily

# 全文搜索
curl "http://localhost:8080/api/v1/poems?keyword=明月几时有"

# 只看宋词，每页 5 条
curl "http://localhost:8080/api/v1/poems?category=词&dynasty=宋&page_size=5"

# 朝代 / 作者分布
curl http://localhost:8080/api/v1/dynasties
curl "http://localhost:8080/api/v1/authors?dynasty=唐"
```

统一响应结构：

```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "id": 1,
    "title": "静夜思",
    "author": "李白",
    "dynasty": "唐",
    "category": "诗",
    "content": ["床前明月光", "疑是地上霜", "举头望明月", "低头思故乡"],
    "tags": ["思乡", "月"],
    "translation": "明亮的月光洒在床前……"
  }
}
```

列表接口的 `data` 为 `{ items, total, page, page_size }`；错误时 `code` 为 HTTP 状态码、`data` 为 `null`。

## 数据说明

语料分两层：

| 层级 | 数量 | 译文 | 标签 | 用途 |
|------|------|------|------|------|
| 精选集 | 约 116 首 |  全部有 |  有 | 每日一诗、网页演示 |
| 全量语料 | 约 13.3 万首 |  无 |  无 | 搜索、随机、列表、作者/朝代聚合 |

> 全量语料来自 [chinese-poetry](https://github.com/chinese-poetry/chinese-poetry)，数据源本身只提供原文，不含译文与标签，因此全量语料的 `translation` 为 `null`。

### 导入全量语料（可选）

仓库开箱即带 116 首精选。需要完整语料时：

```bash
# 1. 下载数据集（只需 json 目录；宋词 ci.song.* 在 master 分支的「宋词」目录下）
git clone --depth 1 https://github.com/chinese-poetry/chinese-poetry.git

# 2. 导入（自动繁转简、按内容哈希去重，可安全重复执行）
cargo run --release --bin import -- ../chinese-poetry/json
```

导入工具支持 `poet.tang.*`（唐诗）、`poet.song.*`（宋诗）、`ci.song.*`（宋词）三类文件；
与精选诗词重名的导入副本会被自动清理，保留带译文的精选版本。

## 配置

所有配置通过环境变量（不设置也能直接运行）：

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `HOST` | `127.0.0.1` | 监听地址（部署时改为 `0.0.0.0`） |
| `PORT` | `8080` | 监听端口 |
| `DATABASE_URL` | `sqlite://data/poems.db` | SQLite 连接串 |
| `API_KEY` | （空） | 设置后开启 API Key 认证 |
| `RATE_LIMIT` | `100` | 每 IP 窗口内最大请求数 |
| `RATE_WINDOW_SECS` | `60` | 限流窗口（秒） |
| `TRUST_PROXY` | `false` | 反向代理后设为 `true`，从 `X-Forwarded-For` 取真实 IP |
| `RUST_LOG` | `poem_api=debug,info` | 日志级别 |

开启认证后，`/api/v1/*` 接口需携带密钥（网页和 Swagger 不受影响）：

```bash
curl -H "X-API-Key: your-secret" http://localhost:8080/api/v1/poems/daily
# 或
curl -H "Authorization: Bearer your-secret" http://localhost:8080/api/v1/poems/daily
```

## 部署到服务器

1. 编译发布版二进制（可在本地编译后上传，也可在服务器上编译）：

   ```bash
   cargo build --release
   # 产物：target/release/poem_api
   ```

   > 如需全量语料，先按上一节导入，再把 `data/poems.db` 与二进制一起上传。

2. 在服务器上运行（建议用 systemd 或 `nohup` 守护）：

   ```bash
   HOST=0.0.0.0 PORT=8080 RUST_LOG=info ./poem_api
   ```

3. 正式对外开放建议：用 Nginx/Caddy 反代并配置 HTTPS（Caddy 可自动申请证书），
   同时设置 `API_KEY`、`TRUST_PROXY=true` 并按需调整 `RATE_LIMIT`，
   云服务器安全组放行对应端口。

## 项目结构

```
poem_api/
├── src/
│   ├── main.rs            # 入口：装配与启动
│   ├── lib.rs             # 库根（供测试复用）
│   ├── config.rs          # 环境变量配置
│   ├── error.rs           # 统一错误类型与响应
│   ├── models.rs          # 数据模型 / 响应结构
│   ├── db.rs              # 连接池、迁移、种子导入
│   ├── repository.rs      # 数据访问层（SQL）
│   ├── daily.rs           # 每日一诗确定性选取
│   ├── cache.rs           # 内存缓存
│   ├── middleware/        # 认证、限流中间件
│   ├── handlers/          # 路由处理
│   ├── bin/import.rs      # chinese-poetry 数据集导入工具
│   └── openapi.rs         # OpenAPI 定义
├── migrations/            # SQL 迁移（含 FTS5 全文索引）
├── data/poems.json        # 精选种子数据（编译期内嵌）
├── static/index.html      # 「每日一诗」网页（编译期内嵌）
├── tests/api.rs           # 集成测试
└── .github/workflows/     # CI（fmt + clippy + test）
```

### 架构分层

```
客户端 → 中间件层（限流 / 认证 / CORS / 压缩）
       → Handler 层（参数校验、响应封装）
       → Repository 层（SQL）
       → SQLite（含 FTS5 全文索引）
       ↘ 内存缓存（每日一诗 / 详情）
```

## 测试与代码质量

```bash
cargo test                      # 单元 + 集成测试
cargo clippy -- -D warnings     # 静态检查
cargo fmt --check               # 格式检查
```

## License

[MIT](./LICENSE)
