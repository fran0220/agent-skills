# cognee-admin CLI — 开发指令

## 定位

Cognee 知识引擎管理 CLI + Web 面板。

这是一个面向运维和 Agent 自动化的 Rust 项目：

- `serve` 模式运行 HTMX Web 管理面板（`cognee.xiaomao.chat`）
- 其余命令作为客户端调用 Cognee REST API 或直连 PostgreSQL 做运维查询

目标是为 API-only 的 Cognee 知识引擎提供可视化运维界面、批量数据导入能力和 Agent 友好的 JSON CLI。

**设计蓝图**：`BLUEPRINT.md`（所有设计决策的权威来源）

## 当前实现状态

### 命令面（v0.2）

| 命令组 | 子命令 / 参数 | 说明 | 状态 |
|--------|---------------|------|:----:|
| `serve` | `--host --port` | 启动 Web 管理面板 | ✅ |
| `health` | `--detailed --watch --interval` | Cognee 健康检查 + 快照记录 | ✅ |
| `dataset` | `list`, `create`, `delete`, `delete-all`, `status`, `graph` | 数据集管理 | ✅ |
| `data` | `add`, `add-file`, `add-dir`, `list`, `delete`, `raw`, `update` | 文本录入、单文件上传、批量导入、替换 | ✅ |
| `cognify` | `--dataset-id`, `--dataset-name`, `--custom-prompt`, `--custom-prompt-file`, `--background`, `--chunks-per-batch` | 触发知识构建 | ✅ |
| `search` | `<query>`, `--search-type`, `--top-k`, `--datasets`, `--verbose`, `history` | 搜索知识库与查看历史 | ✅ |
| `config` | `get`, `set` | Cognee 运行时配置 | ✅ |
| `log` | `list`, `stats` | 请求日志（PG 直读） | ✅ |
| `pipeline` | `list`, `detail` | Pipeline 运行记录（PG 直读） | ✅ |
| `token` | `create`, `list`, `revoke`, `delete` | `ca_xxx` token 管理 | ✅ |
| `login` | `--username --password` | 获取并持久化 Cognee JWT | ✅ |
| `ontology` | `upload`, `list` | Ontology 上传与查看 | ✅ |
| `describe` | `[command]` | 命令自省 JSON Schema | ✅ |

### Web 面板（当前已落地）

| 页面 | 路由 | 状态 |
|------|------|:----:|
| Dashboard | `/` | ✅ 健康概览 + 请求指标 |
| 知识图谱 | `/graph` | ✅ vis-network 交互式图谱 |
| 数据集 | `/datasets` | ✅ 列表与操作入口 |
| 搜索 | `/search` | ✅ HTMX 交互式搜索 |
| 请求日志 | `/logs` | ✅ 分页 + 筛选 |
| Pipeline | `/pipelines` | ✅ 运行记录 |
| 配置 | `/settings` | ✅ 查看/修改 Cognee 设置 |
| Token 管理 | `/tokens` | ✅ CRUD + 角色管理（admin only） |
| 登录 | `/login` | ✅ Token 登录 + Cookie |

Thread 4 仍在继续增强 Web 面板。涉及新页面或新路由时，务必同步更新本文件和 `README.md`。

## 认证模型

### `ca_xxx` Admin Token（统一凭证）

`ca_xxx` token 是唯一面向用户的凭证，由管理面板 `token create` 创建。

**两条认证路径**：

1. **Web 面板**（`cognee.xiaomao.chat`）：`ca_xxx` 直接认证
2. **Cognee API**（`cogneeapi.xiaomao.chat`）：Nginx `auth_request` → cognee-admin `/auth/validate` → 验证 `ca_xxx` → 放行

```
用户 --[ca_xxx Bearer]--> Nginx --[auth_request]--> cognee-admin /auth/validate
                            |                              ↓
                            |                     验证 ca_xxx hash
                            ↓ (通过)
                       Cognee API
```

| 信任域 | 凭证 | 说明 |
|--------|------|------|
| 用户 → API/面板 | `ca_xxx` token | 统一凭证，Bearer header 传递 |
| Web server → Cognee | `COGNEE_SERVICE_JWT` | 服务端内部凭证，环境变量 |
| Rust CLI → Cognee | `--cognee-jwt` / `COGNEE_JWT` | 可选，直连 Cognee 时使用 |

### 本地凭据存储

文件：`~/.config/cognee-admin/auth.json`（权限 `0600`）

```json
{
  "admin_token": "ca_...",
  "cognee_url": "https://cogneeapi.xiaomao.chat"
}
```

npm CLI 通过 `cognee-admin auth set <ca_xxx>` 保存，Rust CLI 通过 `--admin-token` / `COGNEE_ADMIN_TOKEN` 传入。

## npm CLI（轻量客户端）

包名：`@doufunao123/cognee-admin`，纯 TypeScript，无 Rust 依赖。

```bash
npx @doufunao123/cognee-admin auth set ca_xxx   # 保存 token
npx @doufunao123/cognee-admin dataset list       # 之后免 token
```

**npm CLI 只做 API 客户端**，不含 `serve`/`log`/`pipeline`/`token` 等需要 PG 直连的命令。

| npm CLI 命令 | 对应 Rust CLI |
|-------------|--------------|
| `auth set/status/clear` | `--admin-token` 参数 |
| `health` | `health` |
| `dataset *` | `dataset *` |
| `data *` | `data *` |
| `cognify` | `cognify` |
| `search` | `search` |
| `config get/set` | `config get/set` |
| `ontology *` | `ontology *` |
| `describe` | `describe` |
| ❌ 无 | `serve`, `log`, `pipeline`, `token`, `login` |

## 源码导航

```
src/
├── main.rs                CLI 入口：clap 解析、双轨认证解析、命令分发
├── config.rs              AppConfig + AuthConfig（cognee_jwt/admin_token/cognee_url）
├── db.rs                  PG 连接 + migrations
├── error.rs               AppError 统一错误 + IntoResponse
├── output.rs              JSON 信封 + human 模式输出
├── lib.rs                 模块声明
├── auth.rs                `ca_xxx` token 生成/验证/CRUD（SHA256 哈希）
├── cognee_client.rs       Cognee REST API 客户端 + 请求日志 + multipart 上传
│
├── client/
│   ├── mod.rs             子模块声明
│   ├── health_cmd.rs      健康检查 + watch 模式
│   ├── dataset_cmd.rs     数据集 CRUD + delete-all
│   ├── data_cmd.rs        data add/add-file/add-dir/update 等
│   ├── cognify_cmd.rs     cognify 参数扩展 + prompt 文件读取
│   ├── search_cmd.rs      数据集过滤、verbose、history
│   ├── config_cmd.rs      配置 get/set
│   ├── log_cmd.rs         请求日志查询 + 统计
│   ├── pipeline_cmd.rs    Pipeline 运行记录查询
│   ├── login_cmd.rs       登录并保存 Cognee JWT
│   ├── token_cmd.rs       Token CRUD
│   ├── ontology_cmd.rs    Ontology 上传与列表
│   └── describe_cmd.rs    命令自省 JSON Schema
│
├── server/
│   ├── mod.rs             AppState + `COGNEE_SERVICE_JWT` 注入 + run()
│   ├── middleware.rs      Token/Cookie 认证中间件
│   └── routes/
│       ├── mod.rs         路由挂载 + base_html + `/auth/validate`
│       ├── login.rs       登录页 + Cookie 设置
│       ├── tokens.rs      Token 管理页（admin only）
│       ├── dashboard.rs   Dashboard + 健康快照
│       ├── graph.rs       vis-network 知识图谱
│       ├── datasets.rs    数据集页
│       ├── search.rs      搜索页
│       ├── logs.rs        请求日志页
│       ├── pipelines.rs   Pipeline 页
│       ├── settings.rs    配置页（LLM 设置等）
│       └── api.rs         预留
│
├── static/
│   ├── css/custom.css     自定义样式
│   └── js/graph.js        vis-network 初始化
│
└── migrations/
    ├── 001_create_admin_schema.sql   request_logs + health_snapshots
    └── 002_create_api_tokens.sql     api_tokens
```

## 关键设计约束

1. **单二进制双模式**：`cognee-admin serve` 跑 Web 面板，其他子命令走 Cognee API 客户端。
2. **双数据源**：写操作通过 Cognee REST API，读操作直连 PostgreSQL。
3. **统一 `ca_xxx` 认证**：用户只用 `ca_xxx` token，通过 Nginx `auth_request` 同时覆盖 Web 面板和 Cognee API。
4. **Web server 固定上游凭证**：`serve` 必须使用 `COGNEE_SERVICE_JWT`，不要改成 per-user JWT 代理模式。
5. **JSON 默认输出**：CLI stdout 返回统一信封 `{"ok", "command", "data"|"error"}`。
6. **请求日志记录**：所有 Cognee API 调用自动记录到 `cognee_admin.request_logs`；日志更新必须使用 `INSERT ... RETURNING id` 避免竞态。
7. **文件上传**：通过 `reqwest` multipart + 流式上传，不缓存整文件内容；日志只记录文件名和大小，不记录文件正文。
8. **内联 HTML**：Web 面板使用 `format!()` 生成 HTML，不引入模板引擎。
9. **设置接口边界**：Cognee Settings API 只接收 `llm.provider/model/api_key`。`LLM_ENDPOINT` 属于 Cognee 服务端环境变量，不属于设置 JSON。

## LLM / Embedding 配置

### ⚠️ 关键：Cognee 运行时只读环境变量，不读 Settings API

Settings API (`/api/v1/settings`) 改的值**不影响实际 LLM 调用**。Cognee 的 LLM/Embedding 配置来自启动时的环境变量。修改后必须重启容器。

### 当前生产配置（Oracle VPS `.env`）

```bash
# LLM — OpenAI 原生模型，structured output 兼容性最佳
LLM_PROVIDER="openai"
LLM_MODEL="gpt-5-mini"
LLM_API_KEY="<xiaomao-gateway-key>"
LLM_ENDPOINT="https://api.xiaomao.chat/v1"

# Embedding — 通过小猫网关
EMBEDDING_PROVIDER="openai"
EMBEDDING_MODEL="text-embedding-3-large"
EMBEDDING_API_KEY="<xiaomao-gateway-key>"
EMBEDDING_ENDPOINT="https://api.xiaomao.chat/v1"
EMBEDDING_DIMENSIONS=3072
EMBEDDING_MAX_TOKENS=8191
```

### LLM 模型选择

Cognee 图谱抽取依赖 **structured output**（JSON schema 遵从），模型选择很重要：

| 模型 | Structured Output | 性价比 | 推荐场景 |
|------|:-:|:-:|------|
| `gpt-5-mini` | ✅ 原生 | ⭐⭐⭐ | **当前生产配置**，性价比最佳 |
| `gpt-4.1-mini` | ✅ 原生 | ⭐⭐⭐ | 备选，更便宜 |
| `gpt-5.4` | ✅ 最强 | ⭐ | 复杂抽取，成本高 |
| `openai/gemini-*` | ⚠️ 不稳定 | ⭐⭐ | LiteLLM 转接易出 pydantic 错误 |
| `openai/claude-*` | ⚠️ 不稳定 | ⭐⭐ | 同上 |

**关键教训**：Gemini/Claude 通过 LiteLLM OpenAI 兼容层转接时，structured output 容易返回错误类型（float 代替 object），导致 graph extraction 失败。优先用 OpenAI 原生模型。

### LiteLLM 模型名陷阱

非 OpenAI 模型**必须加 `openai/` 前缀**强制走 OpenAI 协议：

| 模型名 | LiteLLM 行为 | 正确写法 |
|--------|-------------|---------|
| `gemini-3-flash-preview` | ❌ 走 vertex_ai，需要 google.auth | `openai/gemini-3-flash-preview` |
| `claude-sonnet-4-6` | ❌ 走 anthropic 原生 | `openai/claude-sonnet-4-6` |
| `gpt-5-mini` | ✅ 默认走 openai | `gpt-5-mini`（无需前缀） |

### 可热改 vs 需重启

| 配置项 | 可通过 Settings API 改？ | 实际效果 |
|--------|:-:|------|
| `llm.provider / model / api_key` | ✅ 能存 | ❌ 不影响运行时 |
| `LLM_ENDPOINT` | ❌ | 需改 `.env` + 重启 |
| `EMBEDDING_*` | ❌ | 需改 `.env` + 重启 |
| `vectorDb.*` | ❌ | 需改 `.env` + 重启 |

### Web 面板 Settings 页已知 bug

`saveLlmSettings()` JS 中读旧 API key 用 `llm.api_key`（snake_case），但 Cognee GET 返回 `llm.apiKey`（camelCase），导致保存时 API key 被清空。已在本地修复（`llm.apiKey || llm.api_key`），待部署。

## 数据架构

```
┌─────────────┐     REST API (writes)     ┌──────────────────┐
│ cognee-admin │ ──────────────────────── → │ Cognee API       │
│              │                            │ cogneeapi.       │
│              │     PG direct (reads)      │ xiaomao.chat     │
│              │ ← ──────────────────────── │                  │
└─────────────┘                            └──────────────────┘
       │                                          │
       │  cognee_admin schema                     │  public schema
       │  (request_logs, health_snapshots,        │  (Cognee tables)
       │   api_tokens)                            │
       └──────────────── PG ──────────────────────┘
```

### cognee_admin Schema（3 表）

| 表 | 说明 |
|---|------|
| `request_logs` | API 请求日志（method, endpoint, latency_ms, status_code, preview） |
| `health_snapshots` | 健康状态快照（components JSONB, uptime） |
| `api_tokens` | Token 管理（token_hash SHA256, name, role, enabled, expires_at） |

## 域名映射

| 域名 | 用途 | 端口 | 认证 |
|------|------|------|------|
| `cognee.xiaomao.chat` | Web 管理面板 | 9847 | `ca_xxx` token + Cookie |
| `cogneeapi.xiaomao.chat` | Cognee REST API | 8847 | Nginx `auth_request` → cognee-admin |

## 部署信息

| 项目 | 值 |
|------|-----|
| 服务器 | Oracle (161.33.13.122), user `opc`, aarch64 |
| 二进制 | `/usr/local/bin/cognee-admin` |
| systemd | `cognee-admin.service` |
| 端口 | `9847` (Axum)，Nginx 反代 → cognee.xiaomao.chat |
| 域名 | `cognee.xiaomao.chat`（CF proxy → Oracle Nginx → 9847） |
| CI | push main → GitHub Actions → cross-compile aarch64 → SSH deploy to Oracle |

## 开发约定

### 新增 CLI 命令 Checklist

1. 在 `src/client/<name>_cmd.rs` 实现命令逻辑。
2. 在 `src/client/mod.rs` 添加 `pub mod`。
3. 在 `src/main.rs` 的 `Commands` enum 添加子命令和 dispatch。
4. 通过 `CogneeClient` 调用 Cognee API；需要上传时优先复用现有 multipart helper。
5. 返回 `Result<Value, AppError>`；统一走 JSON 信封。
6. 更新 `src/client/describe_cmd.rs` 的 schema。
7. 更新 `README.md`、必要时同步 `skill/SKILL.md`。

### 新增 Web 页面 Checklist

1. 在 `src/server/routes/<page>.rs` 创建 page + API handler。
2. 使用 `base_html()` 生成布局，`format!()` 内联 HTML。
3. 在 `src/server/routes/mod.rs` 挂载路由并更新侧边栏导航。
4. 交互优先用 HTMX；必要的客户端逻辑保持最小化内联 JS。
5. Admin-only 页面必须检查角色，不要只依赖前端隐藏按钮。
6. 如果页面依赖 Cognee 写操作，确认其上游认证走的是 `COGNEE_SERVICE_JWT`。

### 文档与 Skill Checklist

1. 命令面变化后同步 `README.md`、`../README.md`、`describe_cmd.rs`。
2. 如果变更影响 Agent 使用路径，同步 `../skill/SKILL.md` 和 `../skill/README.md`。
3. Skill 目录只放文档，不放可执行脚本或实现代码。

### 退出码

- `0` 成功
- `1` 命令/输入错误
- `2` 系统错误（DB/配置/内部异常）
- `3` 外部服务错误（Cognee API 不可达）

## 技术栈

| 层 | 选择 |
|----|------|
| 语言 | Rust (edition 2021) |
| 服务框架 | Axum 0.8 + tower-http + tokio |
| CLI 框架 | clap 4 (derive) |
| 数据库 | PostgreSQL + sqlx 0.8 |
| 认证 | SHA256 token hash + Axum middleware |
| 前端 | HTMX + Tailwind CSS (CDN) + Chart.js + vis-network |
| HTTP 客户端 | reqwest 0.12 |
| 上传流 | reqwest multipart + tokio-util `ReaderStream` |
| 批量导入反馈 | indicatif |
| 序列化 | serde + serde_json |
