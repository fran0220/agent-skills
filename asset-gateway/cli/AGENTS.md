# asset-gateway CLI — 开发指令

## 定位

通用资产生成网关（Universal Asset Gateway）。

这是一个面向 Agent 的基础设施型 Rust CLI：
- `serve` 模式运行网关服务
- 其余命令作为客户端调用网关 API

目标是把多提供商资产生成能力统一成一个稳定接口，供任意项目复用，而不是绑定单一业务项目。

**设计蓝图**：`BLUEPRINT.md`（所有设计决策的权威来源）

## 关键设计约束

1. **单二进制双模式**：`asset-gateway serve` 跑服务，其他子命令走 HTTP 客户端。
2. **JSON 默认输出**：stdout 返回统一信封，不以人类可读格式作为契约。
3. **统一命令语义**：所有命令输出 `{"ok", "command", "data"|"error"}` 结构。
4. **凭据加密存储**：敏感凭据必须经 Vault（AES-256-GCM）加密后入库。
5. **Provider 可插拔**：通过统一 `AssetProvider` trait 适配不同服务。
6. **策略路由优先**：显式 provider 覆盖 > 透明度/能力匹配 > 健康过滤 > 优先级排序 > 自动 fallback。

## 技术栈选择

本项目使用 **Rust**（非 TypeScript），原因：

- 需要长期运行的网关服务（Axum + tokio）
- 需要更强的类型安全与并发可靠性
- 需要内建加密、鉴权、DB 持久化的系统级能力

## 当前实现状态 — ~4000 行 Rust, 4 测试通过

### 命令面（7 组 20 个子命令）

| 命令组 | 子命令 | 说明 | 实现状态 |
|--------|--------|------|:--------:|
| `serve` | `--host --port --db` | 启动网关服务 | ✅ 完整 |
| `auth` | `login`, `logout`, `whoami` | 认证入口 | ⚠️ 骨架 |
| `generate` | `image`, `video`, `audio`, `model`, `text` | 资产生成 | ⚠️ 骨架 |
| `provider` | `list`, `health` | Provider 发现与健康检查 | ⚠️ 骨架 |
| `credential` | `set`, `list`, `delete` | 凭据管理 | ⚠️ 骨架 |
| `job` | `list`, `status`, `cancel` | 任务管理 | ⚠️ 骨架 |
| `describe` | `[command]` | 命令自省 | ⚠️ 骨架 |

> ⚠️ 骨架 = CLI 客户端能发 HTTP 请求到网关，但缺少 token 持久化（`~/.config/asset-gateway/auth.json`）和认证 header 注入。

### Server 端（完整实现）

| 模块 | 路由 | 实现状态 |
|------|------|:--------:|
| Auth | `POST /auth/register`, `POST /auth/login` | ✅ Argon2 + JWT + API Key 双模式 |
| Auth extractor | `CurrentUser` from Bearer / api_key query | ✅ 完整 |
| Credentials | `GET/PUT /api/credentials`, `DELETE /api/credentials/:key` | ✅ Vault 加解密 + 脱敏 + Admin 限制 |
| Providers | `GET /api/providers`, `POST /api/providers`, `DELETE /api/providers/:id`, `GET /api/providers/:id/health` | ✅ CRUD + 动态注册 |
| Generate | `POST /api/generate` | ✅ Job 持久化 + Dispatcher 路由 |
| Jobs | `GET /api/jobs`, `GET /api/jobs/:id` | ✅ 分页 + 筛选 + 详情 |
| Health | `GET /api/health` | ✅ |
| Provider 加载 | 启动时从 DB 加载 + 凭据解密注入 | ✅ |

### Provider 面（6 个，全部实现）

| Provider ID | 类型 | 关键能力 | 行数 |
|-------------|------|---------|:----:|
| `llm_proxy` | Text | 双协议（Anthropic + OpenAI 自动选择），SSE streaming | 343 |
| `gpt_image` | Image | 透明背景，多格式（png/jpeg/webp），多尺寸 | 192 |
| `gemini_image` | Image | Google generateContent，成本优先 | 152 |
| `jimeng` | Image/Video | 图像直出 + 视频异步轮询 + 指数退避 | 316 |
| `elevenlabs` | Audio | sound-generation + text-to-speech 双路径 | 202 |
| `tripo3d` | Model3d | image-to-model + text-to-model，异步轮询 + 超时 | 247 |

### 核心层

| 模块 | 功能 | 实现状态 |
|------|------|:--------:|
| Vault | AES-256-GCM 加解密 | ✅ + 测试 |
| Registry | Provider 注册/发现/按类型查询 | ✅ |
| Dispatcher | 智能路由：透明度优先 → 健康过滤 → 优先级排序 → 自动 fallback | ✅ + 3 测试 |
| Queue | 异步任务队列 | 🔲 占位符 |

### SQLite 四表

| 表 | 字段 | 说明 |
|---|------|------|
| `users` | id, username, password_hash, role, api_key | 用户 + 认证 |
| `providers` | id, display_name, adapter, asset_types, config, priority, enabled | Provider 注册 |
| `credentials` | key, encrypted_value, nonce, provider_id, description | 加密凭据 |
| `jobs` | id, asset_type, provider_id, status, request, response, error_message, output_path, cost_usd | 生成任务 |

### 测试覆盖

| 测试 | 内容 |
|------|------|
| `vault::encrypt_decrypt_roundtrip` | AES-256-GCM 加解密往返 |
| `dispatcher::transparent_requests_prefer_gpt_image` | 透明图路由策略 |
| `dispatcher::skips_unhealthy_provider` | 健康检查过滤 |
| `dispatcher::falls_back_to_next_healthy_provider_when_primary_fails` | 失败 fallback |

## 路由策略

Dispatcher 选择链：
1. 显式 `--provider` → 直接路由，不做 fallback
2. 按 `asset_type` 过滤候选 Provider
3. 透明图（`params.transparent == true`）→ 优先 `supports_transparency` 的 Provider
4. 健康检查过滤 → 跳过不健康的
5. 按 `priority` 降序排列
6. 首选失败 → 自动尝试下一个候选

## 已知缺口（v1 未完成）

### 高优先级

1. **CLI token 持久化**：`auth login` 成功后保存 token 到 `~/.config/asset-gateway/auth.json`
2. **CLI 认证注入**：所有客户端命令自动读取 token 并注入 `Authorization` header
3. **生成文件落盘**：`generate` 结果写入本地文件（base64 → 二进制），返回 `output_path`
4. **describe 完整 schema**：按命令返回 JSON Schema，不只是命令列表

### 中优先级

5. **Job cancel**：`POST /api/jobs/:id/cancel`（当前仅 DB 状态更新，无法终止运行中任务）
6. **Provider 热重载**：credential 更新后自动刷新 Provider 实例
7. **错误建议**：AppError 增加 `suggestion` 字段
8. **E2E 集成测试**：启动 server → register → login → set credential → add provider → generate

### 低优先级（v2）

9. Leptos 管理前端
10. WebSocket 任务推送
11. 异步任务队列（视频/3D 长任务）
12. 限流与配额
13. 成本统计与可视化
14. RBAC 细化（user 级生成配额）

## 开发约定

### 新增 Provider Checklist

1. 在 `src/providers/<name>.rs` 实现 `AssetProvider` trait
2. 明确 `id()`, `display_name()`, `asset_types()`, `capabilities()`
3. `generate()` 返回统一 `GenerateResponse`，不得泄漏密钥
4. `health_check()` 必须实现，不做空壳
5. 在 `src/server/mod.rs` 的 `build_provider()` match 中加入新 adapter
6. 在 `src/providers/mod.rs` 中加 `pub mod`
7. 在 README.md、BLUEPRINT.md、`describe` 输出中同步更新
8. 补充 smoke test（成功 + 错误路径）

### 新增 API 路由 Checklist

1. 在 `src/server/routes/<module>.rs` 增加路由与 handler
2. 返回统一 JSON 信封；错误走 `AppError`
3. 明确是否需要鉴权（`CurrentUser` extractor + `require_admin()`）
4. 需要持久化时写入 SQLite
5. 更新 `src/server/routes/mod.rs` 挂载
6. 更新 CLI 客户端命令映射（`src/client/*_cmd.rs`）
7. 更新 `describe` schema 和 README

### 退出码

- `0` 成功
- `1` 命令/输入错误
- `2` 系统错误（DB/配置/内部异常）
- `3` 外部服务错误（Provider 调用失败）

## 源码导航

```
src/
├── main.rs           (116)  CLI 入口：clap 解析 serve / client 命令
├── config.rs          (26)  AppConfig 环境变量
├── db.rs              (11)  SQLite 连接 + migration
├── error.rs           (72)  AppError 统一错误 + IntoResponse
├── output.rs          (32)  JSON 信封 success/error/print
│
├── core/
│   ├── mod.rs        (161)  AssetType, ProviderCapabilities, GenerateRequest/Response, trait AssetProvider
│   ├── vault.rs       (66)  AES-256-GCM 加解密
│   ├── registry.rs    (46)  ProviderRegistry（线程安全注册表）
│   ├── dispatcher.rs (345)  策略路由 + 健康过滤 + 自动 fallback + 测试
│   └── queue.rs        (3)  异步队列占位
│
├── providers/
│   ├── mod.rs          (6)  pub mod 声明
│   ├── llm_proxy.rs  (343)  LLM 双协议 + streaming
│   ├── gpt_image.rs  (192)  OpenAI 图像 + 透明
│   ├── gemini_image.rs(152) Google 图像
│   ├── jimeng.rs     (316)  即梦图像 + Seedance 视频
│   ├── elevenlabs.rs (202)  音频
│   └── tripo3d.rs    (247)  3D 模型
│
├── server/
│   ├── mod.rs        (284)  ServerState + Provider 动态加载 + build_provider + run()
│   └── routes/
│       ├── mod.rs     (23)  路由挂载
│       ├── auth.rs   (268)  注册/登录/JWT/CurrentUser extractor
│       ├── generate.rs(98)  生成 + Job 持久化
│       ├── providers.rs(273) Provider CRUD + 健康检查
│       ├── credentials.rs(156) Vault 加解密 CRUD
│       ├── jobs.rs   (112)  Job 列表/详情
│       └── health.rs  (15)  健康检查
│
├── client/
│   ├── mod.rs        (187)  命令定义 + handler 入口
│   ├── auth_cmd.rs    (52)  ⚠️ 缺 token 持久化
│   ├── generate_cmd.rs(73)  ⚠️ 缺 auth header
│   ├── provider_cmd.rs(27)  ⚠️ 缺 auth header
│   ├── credential_cmd.rs(43) ⚠️ 缺 auth header
│   └── job_cmd.rs     (35)  ⚠️ 缺 auth header
│
└── frontend/                 🔲 Phase 4 Leptos 占位
    ├── mod.rs          (5)
    └── pages/mod.rs    (2)
```

## 技术栈

| 层 | 选择 |
|----|------|
| 语言 | Rust (edition 2021) |
| 服务框架 | Axum 0.8 + tower-http + tokio |
| CLI 框架 | clap 4 (derive) |
| 数据库 | SQLite + sqlx 0.8 + migrations |
| 认证 | jsonwebtoken + argon2 |
| 凭据加密 | aes-gcm (AES-256-GCM) |
| HTTP 客户端 | reqwest 0.12 |
| 序列化 | serde + serde_json |
