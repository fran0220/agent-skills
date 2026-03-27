# cognee-admin CLI — 开发指令

## 定位

Cognee 知识引擎管理 CLI + Web 面板。

这是一个面向运维的 Rust CLI：
- `serve` 模式运行 HTMX Web 管理面板（`cognee.xiaomao.chat`）
- 其余命令作为客户端调用 Cognee REST API（`cogneeapi.xiaomao.chat`）

目标是为 API-only 的 Cognee 知识引擎提供可视化运维界面和 Agent 可调用的 CLI 接口。

**设计蓝图**：`BLUEPRINT.md`（所有设计决策的权威来源）

## 技术栈选择

本项目使用 **Rust**（非 TypeScript），原因：

- 与 asset-gateway 共享同一部署模式（单二进制双模式）
- 需要长期运行的 Web 服务（Axum + tokio）
- 需要直连 PostgreSQL 进行高效读取（sqlx）
- 部署目标为 Oracle ARM VPS，Rust 单二进制更适合

## 当前实现状态 — ~3,400 行 Rust，功能完整

### 命令面（9 组 + serve）

| 命令组 | 子命令 | 说明 | 状态 |
|--------|--------|------|:----:|
| `serve` | `--host --port` | 启动 Web 管理面板 | ✅ |
| `health` | `--detailed` | Cognee 健康检查 | ✅ |
| `dataset` | `list`, `create`, `delete`, `status`, `graph` | 数据集管理 | ✅ |
| `data` | `add`, `list`, `delete`, `raw` | 数据操作 | ✅ |
| `cognify` | `--dataset-id` | 触发知识构建 | ✅ |
| `search` | `--search-type --top-k` | 搜索知识库 | ✅ |
| `config` | `get`, `set` | Cognee 运行时配置 | ✅ |
| `log` | `list`, `stats` | 请求日志（PG 直读） | ✅ |
| `pipeline` | `list`, `detail` | Pipeline 运行记录（PG 直读） | ✅ |
| `token` | `create`, `list`, `revoke`, `delete` | API Token 管理 | ✅ |
| `login` | `--username --password` | Cognee 认证 | ✅ |
| `describe` | `[command]` | 命令自省 JSON Schema | ✅ |

### Web 面板（8 页面）

| 页面 | 路由 | 状态 |
|------|------|:----:|
| Dashboard | `/` | ✅ 6 组件状态 + Chart.js 延迟图 |
| 知识图谱 | `/graph` | ✅ vis-network 交互式图谱 |
| 数据集 | `/datasets` | ✅ 列表 + 状态 |
| 搜索 | `/search` | ✅ HTMX 交互式搜索 |
| 请求日志 | `/logs` | ✅ 分页 + 筛选 |
| Pipeline | `/pipelines` | ✅ 运行记录 |
| 配置 | `/settings` | ✅ 查看/修改 |
| Token 管理 | `/tokens` | ✅ CRUD + 角色管理（admin only） |
| 登录 | `/login` | ✅ Token 登录 + Cookie |

### 认证系统

| 层 | 机制 | 说明 |
|----|------|------|
| Web 面板 | Bearer Token + Cookie | `/login` 页面输入 token → HttpOnly Cookie |
| CLI | `--token` flag / `COGNEE_ADMIN_TOKEN` env | 每次请求携带 Bearer |
| Cognee API | Nginx `auth_request` | BWG 反代通过 cognee-admin 验证 token |
| Token 存储 | PG `cognee_admin.api_tokens` | SHA256 哈希存储，原始 token 仅创建时显示 |

**两种角色**：
- `admin`：全部权限，可管理 token
- `user`：读写 Cognee 数据，不可管理 token 和 settings

## 源码导航

```
src/
├── main.rs           (430)  CLI 入口：clap 解析 + 命令分发
├── config.rs          (63)  AppConfig 环境变量 + AuthConfig 持久化
├── db.rs              (18)  PG 连接 + 2 个 migration
├── error.rs           (77)  AppError 统一错误 + IntoResponse + suggestion
├── output.rs          (71)  JSON 信封 success/error + human 模式
├── lib.rs              (8)  模块声明
├── auth.rs           (117)  Token 生成/验证/CRUD（SHA256 哈希）
├── cognee_client.rs  (261)  Cognee REST API 客户端 + 自动日志
│
├── client/
│   ├── mod.rs         (11)  子模块声明
│   ├── health_cmd.rs  (12)  健康检查
│   ├── dataset_cmd.rs (24)  数据集 CRUD
│   ├── data_cmd.rs    (23)  数据操作
│   ├── cognify_cmd.rs (12)  触发知识构建
│   ├── search_cmd.rs   (8)  搜索
│   ├── config_cmd.rs  (14)  配置管理
│   ├── log_cmd.rs    (104)  请求日志查询 + 统计
│   ├── pipeline_cmd.rs(75)  Pipeline 记录查询
│   ├── login_cmd.rs   (20)  Cognee 认证
│   ├── token_cmd.rs   (51)  Token CRUD
│   └── describe_cmd.rs(153) 命令自省 JSON Schema
│
├── server/
│   ├── mod.rs         (38)  AppState + 认证中间件挂载 + run()
│   ├── middleware.rs   (97)  Token/Cookie 认证中间件
│   └── routes/
│       ├── mod.rs    (154)  路由挂载 + base_html 布局 + /auth/validate
│       ├── login.rs  (102)  登录页 + Cookie 设置
│       ├── tokens.rs (271)  Token 管理页（admin only, HTMX CRUD）
│       ├── dashboard.rs(277) Dashboard + Chart.js + 健康快照
│       ├── graph.rs  (137)  vis-network 知识图谱
│       ├── datasets.rs(115) 数据集列表
│       ├── search.rs (144)  交互式搜索
│       ├── logs.rs   (215)  请求日志 + 分页
│       ├── pipelines.rs(133) Pipeline 运行记录
│       ├── settings.rs(141) 配置管理
│       └── api.rs      (3)  保留
│
├── static/
│   ├── css/custom.css (63)  自定义样式
│   └── js/graph.js    (64)  vis-network 初始化
│
└── migrations/
    ├── 001_create_admin_schema.sql (31)  request_logs + health_snapshots
    └── 002_create_api_tokens.sql   (11)  api_tokens
```

## 关键设计约束

1. **单二进制双模式**：`cognee-admin serve` 跑 Web 面板，其他子命令走 Cognee API 客户端。
2. **双数据源**：写操作通过 Cognee REST API，读操作直连 PostgreSQL。
3. **Schema 隔离**：`cognee_admin` schema 与 Cognee 的 `public` schema 共存于同一 PG。
4. **JSON 默认输出**：CLI stdout 返回统一信封 `{"ok", "command", "data"|"error"}`。
5. **请求日志记录**：所有 Cognee API 调用自动记录到 `cognee_admin.request_logs`。
6. **Token 认证**：SHA256 哈希存储，admin/user 双角色，Nginx auth_request 联动。
7. **内联 HTML**：Web 面板使用 `format!()` 生成 HTML，不依赖模板引擎。

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
       │  (request_logs, health_snapshots,        │  (Cognee 29 表)
       │   api_tokens)                            │
       └──────────────── PG ──────────────────────┘
```

### cognee_admin Schema（3 表）

| 表 | 说明 |
|---|------|
| `request_logs` | API 请求日志（method, endpoint, latency_ms, status_code） |
| `health_snapshots` | 健康状态快照（components JSONB, uptime） |
| `api_tokens` | Token 管理（token_hash SHA256, name, role, enabled, expires_at） |

## 域名映射

| 域名 | 用途 | 端口 | 认证 |
|------|------|------|------|
| `cognee.xiaomao.chat` | Web 管理面板 | 9847 | Token + Cookie |
| `cogneeapi.xiaomao.chat` | Cognee REST API | 8847 | Nginx auth_request → cognee-admin |

## 部署信息

- **服务器**：Oracle VPS (4C ARM / 22G RAM)
- **二进制**：`/usr/local/bin/cognee-admin`
- **systemd**：`cognee-admin.service`（enabled, auto-restart）
- **构建**：Oracle 上 `cargo build --release`（~2min）
- **更新流程**：rsync → build → stop → cp → start

## 开发约定

### 新增 CLI 命令 Checklist

1. 在 `src/client/<name>_cmd.rs` 实现命令逻辑
2. 在 `src/client/mod.rs` 添加 `pub mod`
3. 在 `src/main.rs` 的 Commands enum 添加子命令 + dispatch
4. 通过 `CogneeClient` 调用 Cognee API（自动记录日志）
5. 返回 `Result<Value, AppError>`；统一 JSON 信封
6. 更新 `describe_cmd.rs` 的 schema
7. 更新 README.md

### 新增 Web 页面 Checklist

1. 在 `src/server/routes/<page>.rs` 创建 page + api handler
2. 使用 `base_html()` 生成布局，`format!()` 内联 HTML
3. 在 `src/server/routes/mod.rs` 挂载路由 + 更新侧边栏导航
4. HTMX 实现交互（`hx-get`, `hx-post`, `hx-swap`）
5. Admin-only 页面检查 `Extension<ApiToken>` 的 role

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
| 序列化 | serde + serde_json |
