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
4. **TOML 配置文件**：Provider 配置从 `config.toml` 读取，支持 Web 面板在线编辑 + 热重载。环境变量可覆盖配置文件。
5. **Provider 可插拔**：通过统一 `AssetProvider` trait 适配不同服务。
6. **策略路由优先**：显式 provider 覆盖 > 能力匹配 > 健康过滤 > 优先级排序 > 自动 fallback。

## 技术栈选择

本项目使用 **Rust**（非 TypeScript），原因：

- 需要长期运行的网关服务（Axum + tokio）
- 需要更强的类型安全与并发可靠性
- 需要内建鉴权、DB 持久化的系统级能力

## 部署信息

| 项目 | 值 |
|------|-----|
| 服务器 | Oracle (161.33.13.122), user `opc` |
| 二进制 | `/opt/asset-gateway/asset-gateway` |
| 配置文件 | `/opt/asset-gateway/config.toml` |
| 环境变量 | `/opt/asset-gateway/.env`（仅 DB/JWT，不含 provider keys） |
| systemd | `asset-gateway.service` |
| 端口 | 6700 (Axum)，Nginx 反代 443 |
| 域名 | `assets.xiaomao.chat`（CF proxy → Oracle Nginx → 6700） |
| 管理面板 | `https://assets.xiaomao.chat/admin` |
| Admin Token | `agk_admin_2484d6cec8ccc8b8d1eb076eac171e17` |
| DB | PostgreSQL (`assetgw@localhost:5432/asset_gateway`) |

### 服务器本地工具依赖

后处理管线依赖以下工具（已安装在 Oracle 服务器）：

| 工具 | 用途 | 安装方式 |
|------|------|---------|
| `rembg` + BiRefNet | AI 背景移除 | `pip3 install --user rembg[cli] onnxruntime`（通过 python3 -c 调用，避免 CLI gradio 冲突） |
| ImageMagick 6 | 裁剪/缩放/合成 | `dnf install ImageMagick`（用 `convert`/`identify`，非 `magick`） |
| Real-ESRGAN | AI 图像放大 | `realesrgan-ncnn-vulkan`（尚未安装，按需部署） |

### 配置优先级

**env var > config.toml > 默认值**。`.env` 中只保留基础变量（DB URL、JWT secret），所有 provider key 统一在 `config.toml` 管理，通过 Web 面板编辑。

### config.toml 结构

```toml
[server]
admin_token = "agk_admin_..."

[proxy]
url = "https://api.xiaomao.chat"
key = "..."
default_model = "claude-sonnet-4-6"

[elevenlabs]
key = "..."

[tripo3d]
key = "..."

[minimax]
url = "https://api.bltcy.ai"   # MiniMax 中转站（独立于 LLM proxy）
key = "..."                     # 留空则 fallback 到 [proxy].key
```

### 部署流程

```bash
rsync -az --exclude target --exclude .git cli/ opc@161.33.13.122:/tmp/asset-gateway-build/
ssh opc@161.33.13.122 'cd /tmp/asset-gateway-build && cargo build --release'
ssh opc@161.33.13.122 'sudo systemctl stop asset-gateway && sudo cp /tmp/asset-gateway-build/target/release/asset-gateway /opt/asset-gateway/ && sudo systemctl start asset-gateway'
```

## npm CLI 客户端

npm 包 `@doufunao123/asset-gateway`（v0.6.0）是面向 Agent 的轻量 HTTP 客户端：

```bash
npm install -g @doufunao123/asset-gateway
asset-gateway auth set <token>
```

源码在 `../npm/`，使用 TypeScript + commander，tsup 构建。

| 命令组 | 子命令 |
|--------|--------|
| `auth` | `set`, `status`, `clear` |
| `generate` | `image`, `video`, `audio`, `tts`, `model`, `text` |
| `process` | `remove-bg`, `crop`, `resize`, `upscale` |
| `process3d` | `convert`, `texture`, `rig`, `animate`, `reduce`, `stylize`, `segment`, `prerigcheck` |
| `upload` | `file`, `list`, `delete` |
| `provider` | `list`, `health` |
| `job` | `list`, `status`, `cancel` |
| `describe` | `[command]` |

Skill 安装在 `~/.config/amp/skills/asset-gateway` → `../skill/SKILL.md`。

## 当前实现状态 — ~6860 行 Rust, 18 测试通过

### 命令面（8 组 30 个子命令）

| 命令组 | 子命令 | 说明 | 实现状态 |
|--------|--------|------|:--------:|
| `serve` | `--host --port --db --config` | 启动网关服务 | ✅ 完整 |
| `auth` | `login`, `logout`, `whoami` | 认证入口 | ✅ 完整 |
| `generate` | `image`, `video`, `audio`, `tts`, `model`, `text` | 资产生成 | ✅ 完整 |
| `process` | `remove-bg`, `crop`, `resize`, `upscale` | 图像后处理 | ✅ 完整 |
| `process3d` | `convert`, `texture`, `rig`, `animate`, `reduce`, `stylize`, `segment`, `prerigcheck` | 3D 后处理管线 | ✅ 完整 |
| `provider` | `list`, `health` | Provider 发现与健康检查 | ✅ 完整 |
| `job` | `list`, `status`, `cancel` | 任务管理 | ✅ 完整 |
| `describe` | `[command]` | 命令自省 | ✅ 完整 |

> CLI 客户端完整实现了 token 持久化（`~/.config/asset-gateway/auth.json`，多网关 URL 支持，Unix 0600 权限保护）和自动认证 header 注入。

### Server 端（完整实现）

| 模块 | 路由 | 实现状态 |
|------|------|:--------:|
| Auth | `POST /auth/register`, `POST /auth/login` | ✅ Argon2 + JWT + API Key + Admin Token |
| Auth extractor | `CurrentUser` from Bearer / api_key query | ✅ 完整 |
| Users | `GET /api/users`, `PUT /api/users/:id`, `DELETE /api/users/:id` | ✅ 用户管理 + Admin 限制 |
| Config | `GET /api/config`, `PUT /api/config` | ✅ TOML 读写 + 热重载 |
| Providers | `GET /api/providers`, `GET /api/providers/health`, `POST /api/providers/reload` | ✅ 列表 + 健康检查 + 热重载 |
| Assets | `POST /api/assets/upload`, `GET /api/assets`, `DELETE /api/assets/:name` | ✅ 文件上传 + 列表 + 删除 |
| Static | `GET /uploads/:filename` | ✅ 上传文件静态访问（无需认证） |
| Generate | `POST /api/generate` | ✅ Job 持久化 + Dispatcher 路由 |
| Process | `POST /api/process` | ✅ Pipeline 引擎（rembg + ImageMagick + Real-ESRGAN） |
| Process3d | `POST /api/process3d` | ✅ Tripo 3D 后处理管线 |
| Jobs | `GET /api/jobs`, `GET /api/jobs/:id`, `POST /api/jobs/:id/cancel` | ✅ 分页 + 筛选 + 详情 + 取消 |
| Health | `GET /api/health` | ✅ |
| Frontend | `GET /`, `GET /admin` | ✅ 内嵌 HTML 管理面板 |

### Provider 面（6 个）

| Provider ID | 类型 | 关键能力 | 测试状态 |
|-------------|------|---------|:--------:|
| `llm_proxy` | Text | 双协议（Anthropic + OpenAI 自动选择），SSE streaming | ✅ 1.3s |
| `gemini_image` | Image | Google generateContent，gemini-3.1-flash-image-preview，支持 text-to-image + image editing (inlineData) | ✅ 15s |
| `grok_image` | Image/Video | Grok 图片生成/编辑 + 视频生成（OpenAI 兼容格式） | ⚠️ 502 upstream |
| `minimax_tts` | Tts | MiniMax 中文/多语种 TTS，同步 API（api.bltcy.ai 中转） | ✅ 1.5s |
| `elevenlabs` | Audio | sound-generation (BGM/SFX) | ✅ 1.5s |
| `tripo3d` | Model3d | 完整 3D 管线：text/image/multiview → model, texture, rig, animate, convert, reduce, stylize, segment, prerigcheck | ✅ |

> 所有 provider 通过 LLM proxy（api.xiaomao.chat）统一接入，minimax_tts 通过独立中转站（api.bltcy.ai）。

### 后处理管线（本地工具）

| 操作 | 工具 | 说明 | 测试状态 |
|------|------|------|:--------:|
| `remove_bg` | rembg + BiRefNet-general | AI 背景移除，通过 python3 -c 调用 | ✅ ~38s |
| `smart_crop` (tightest) | ImageMagick `-trim +repage` | 裁剪透明边框 | ✅ 36ms |
| `smart_crop` (power_of2) | ImageMagick trim + extent | 裁剪后扩展到 2^n 尺寸 | ✅ 36ms |
| `resize` | ImageMagick `-resize` | 精确缩放 | ✅ 48ms |
| `upscale` | Real-ESRGAN ncnn-vulkan | AI 2x/4x 放大 | ⚠️ 未安装 |

操作可链式组合：`remove_bg → smart_crop → resize`。

### 核心层

| 模块 | 功能 | 实现状态 |
|------|------|:--------:|
| Config | TOML 配置文件 + env var 覆盖 + 热重载 | ✅ |
| Registry | Provider 注册/发现/按类型查询 | ✅ |
| Dispatcher | 智能路由：能力匹配 → 健康过滤 → 优先级排序 → 自动 fallback | ✅ + 3 测试 |
| Pipeline | 后处理引擎：tokio::process::Command 调 rembg/ImageMagick/ESRGAN | ✅ |
| Vault | AES-256-GCM 加解密（保留但不再用于 provider key） | ✅ + 测试 |

### PostgreSQL 表

| 表 | 字段 | 说明 |
|---|------|------|
| `users` | id, username, password_hash, role, api_key | 用户 + 认证 |
| `jobs` | id, asset_type, provider_id, status, request, response, error_message, output_path, cost_usd | 生成任务 |

### 管理面板

内嵌 HTML SPA（`/admin`），4 个页面：

| 页面 | 功能 |
|------|------|
| Users & Keys | 用户审批、API Key 生成/撤销 |
| Config | TOML 编辑器，保存后自动重载 provider |
| Providers | Provider 列表、健康检查 |
| Job Logs | 任务历史、状态筛选、详情查看 |

## 路由策略

Dispatcher 选择链：
1. 显式 `--provider` → 直接路由，不做 fallback
2. 按 `asset_type` 过滤候选 Provider
3. 透明图（`params.transparent == true`）→ 优先 `supports_transparency` 的 Provider
4. 非透明图 → 优先 `gemini_image`
5. 健康检查过滤 → 跳过不健康的
6. 按 `priority` 降序排列
7. 首选失败 → 自动尝试下一个候选

`POST /api/process3d` 不走多 Provider Dispatcher；它是 Tripo 专属的续处理链路，直接消费上一步返回的 `tripo_task_id`，把同一资产继续送入 texture / rig / animate / convert / reduce / stylize / segment / prerigcheck。

## 开发约定

### 新增 Provider Checklist

1. 在 `src/providers/<name>.rs` 实现 `AssetProvider` trait
2. 明确 `id()`, `display_name()`, `asset_types()`, `capabilities()`
3. `generate()` 返回统一 `GenerateResponse`，不得泄漏密钥
4. `health_check()` 必须实现，不做空壳
5. 在 `src/server/mod.rs` 的 `build_providers_from_config()` 中加入构建逻辑
6. 在 `src/config.rs` 添加对应 TOML section
7. 在 `src/providers/mod.rs` 中加 `pub mod`
8. 更新 `config.toml.example` 和本文档

### 新增 Pipeline 操作 Checklist

1. 在 `src/core/pipeline.rs` 的 `ProcessOp` 枚举增加变体
2. 在 `Pipeline::run()` 的 match 中增加执行分支
3. 实现对应的 `async fn`（调用外部 CLI 工具）
4. 确认服务器已安装所需工具
5. 更新 `src/client/mod.rs` 的 `ProcessCommands` 和 describe schema
6. 更新 `src/client/process_cmd.rs` 的 handler
7. 更新 npm `src/commands/process.ts`
8. 更新 SKILL.md 和本文档

### Process3d 操作说明

所有 process3d 操作通过 Tripo API 执行，基于 tripo_task_id 链式调用：

| 操作 | Tripo task type | 输入 | 输出 | 估算 Credits |
|------|----------------|------|------|:------------:|
| convert | convert_model | task_id + format + quad/face_limit | FBX/USDZ/OBJ/STL/GLTF/3MF | 5-10 |
| texture | texture_model | task_id + prompt + pbr/quality | 重新贴图的模型 | 20-40 |
| rig | animate_rig | task_id + spec(mixamo/tripo) | 带骨骼的模型 | ~25 |
| animate | animate_retarget | task_id + animation preset | 带动画的模型 | ~25 |
| reduce | highpoly_to_lowpoly | task_id + face_limit + quad | 低面数模型 | ~30 |
| stylize | stylize_model | task_id + style | 风格化模型 | ~5 |
| segment | mesh_segmentation | task_id | 部件分割信息 | ~5 |
| prerigcheck | animate_prerigcheck | task_id | 可否绑骨 + rig_type | ~1 |

### 新增 API 路由 Checklist

1. 在 `src/server/routes/<module>.rs` 增加路由与 handler
2. 返回统一 JSON 信封；错误走 `AppError`
3. 明确是否需要鉴权（`CurrentUser` extractor + `require_admin()`）
4. 需要持久化时写入 PostgreSQL
5. 更新 `src/server/routes/mod.rs` 挂载
6. 更新 CLI 客户端命令映射（`src/client/*_cmd.rs`）
7. 更新 `describe` schema

### 退出码

- `0` 成功
- `1` 命令/输入错误
- `2` 系统错误（DB/配置/内部异常）
- `3` 外部服务错误（Provider 调用失败）

## 源码导航

```
src/
├── main.rs           (138)  CLI 入口：clap 解析 serve / client / process3d 命令
├── lib.rs              (9)  库入口
├── config.rs         (162)  TOML 配置文件 + env var 覆盖
├── db.rs              (10)  PostgreSQL 连接 + migration
├── error.rs          (225)  AppError 统一错误 + IntoResponse
├── output.rs          (32)  JSON 信封 success/error/print
│
├── core/
│   ├── mod.rs        (177)  AssetType, ProviderCapabilities, GenerateRequest/Response, trait AssetProvider
│   ├── pipeline.rs   (284)  后处理管线：rembg/ImageMagick/ESRGAN via tokio::process::Command
│   ├── vault.rs       (66)  AES-256-GCM 加解密
│   ├── registry.rs    (46)  ProviderRegistry（线程安全注册表）
│   ├── dispatcher.rs (384)  策略路由 + 健康缓存(60s TTL) + 自动 fallback + 测试
│   └── queue.rs        (3)  异步队列占位
│
├── providers/
│   ├── mod.rs          (7)  pub mod 声明
│   ├── llm_proxy.rs  (343)  LLM 双协议 + streaming
│   ├── gemini_image.rs(152) Google 图像
│   ├── grok_image.rs (409)  Grok 图片生成/编辑 + 视频（OpenAI 兼容）+ URL 提取 11 测试
│   ├── minimax.rs    (230)  MiniMax TTS 中文/多语种（同步 API + hex/url 输出）
│   ├── elevenlabs.rs (202)  音频 (BGM/SFX)
│   └── tripo3d.rs    (773)  TripoClient + 全链路 3D 管线（14 task types）+ 3 测试
│
├── server/
│   ├── mod.rs        (142)  ServerState + RwLock<AppConfig> + Provider 构建 + run()
│   └── routes/
│       ├── mod.rs     (29)  路由挂载
│       ├── auth.rs   (346)  注册/登录/JWT/CurrentUser extractor
│       ├── config_api.rs(87) GET/PUT /api/config (TOML 读写 + 热重载)
│       ├── generate.rs(136) 生成 + Job 持久化
│       ├── process.rs  (32) POST /api/process — 后处理管线
│       ├── process3d.rs(131)  POST /api/process3d — 3D 后处理管线
│       ├── assets.rs  (120) 文件上传 + 列表 + 删除
│       ├── providers.rs(121) Provider 列表 + 健康检查 + reload
│       ├── jobs.rs   (199)  Job 列表/详情/取消
│       ├── users.rs  (325)  用户管理 CRUD
│       └── health.rs  (15)  健康检查
│
├── client/
│   ├── mod.rs        (860)  命令定义 + describe schema + handler 入口
│   ├── config.rs     (143)  Token 持久化（多网关 URL + 0600 权限）
│   ├── http.rs        (27)  authenticated_client（自动 Bearer 注入）
│   ├── auth_cmd.rs   (101)  登录/登出/whoami + token 写盘
│   ├── generate_cmd.rs(254) 生成 + 文件落盘（base64/URL → 本地文件，含 data URI strip）
│   ├── process_cmd.rs(110)  后处理 + 文件落盘
│   ├── process3d_cmd.rs(280)  3D 后处理命令 + 文件下载
│   ├── provider_cmd.rs(28)  Provider 列表/健康检查
│   └── job_cmd.rs     (47)  Job 列表/状态/取消
│
└── frontend/
    ├── mod.rs         (16)  内嵌 HTML 路由
    └── pages/
        ├── mod.rs      (1)
        └── admin.html(1290) 管理面板 SPA（Users/Config/Providers/Jobs）
```

## 技术栈

| 层 | 选择 |
|----|------|
| 语言 | Rust (edition 2021) |
| 服务框架 | Axum 0.8 + tower-http + tokio |
| CLI 框架 | clap 4 (derive) |
| 数据库 | PostgreSQL + sqlx 0.8 + migrations |
| 配置 | TOML (toml 0.8) + dotenvy |
| 认证 | jsonwebtoken + argon2 + admin token |
| HTTP 客户端 | reqwest 0.12 |
| 序列化 | serde + serde_json |
| 后处理 | rembg (BiRefNet) + ImageMagick 6 + Real-ESRGAN |
| 前端 | 内嵌 HTML SPA（Vanilla JS） |

## 已知问题

- **Grok Image**: 通过 proxy 调用时偶发 502 upstream error，proxy 侧问题
- **Real-ESRGAN**: 服务器尚未安装 `realesrgan-ncnn-vulkan`，upscale 操作暂不可用
- **rembg 首次调用**: BiRefNet 模型 973MB，首次加载 ~40s，后续请求复用 Python 进程无缓存仍需 ~30s（每次 spawn 新进程）。计划改为持久 FastAPI 服务
- **Nginx Timeout**: 长时间后处理请求可能触发 Nginx 60s 超时，需调整 `proxy_read_timeout`

## 已完成优化（2026-03-30）

1. **Dispatcher 健康检查缓存** — `RwLock<HashMap>` + 60s TTL，避免每次 generate 请求都做真实 HTTP 健康检查。`invalidate_health_cache()` 在 provider reload 时清缓存
2. **Gemini Image 编辑支持** — `input_file` 存在时，下载图片并以 `inlineData` 传入 Gemini API contents 数组，支持图片编辑
3. **Grok URL 提取加固** — 替换了 `split_whitespace` 和 `find("src=\"")` 为多策略提取：Markdown `![](url)` → HTML `<img src>` / `<a href>` → `<video>` / `<source>` tag → 裸 URL 扫描。11 个测试覆盖
4. **cost_usd 成本估算** — 所有 6 个 provider 均返回估算成本：gemini $0.04-0.08, grok $0.07-0.10, llm $0.02-0.10, minimax $0.005, elevenlabs $0.05, tripo3d $0.20
5. **Image 编辑 CLI 支持** — Rust CLI 和 npm CLI 均新增 `--input` 参数，用于传入待编辑图片 URL
6. **Tripo 3D 全管线接入** — 从 2/14 task type 扩展到全部 14 个：multiview 生成、P1 参数增强、convert/texture/rig/animate/reduce/stylize/segment/prerigcheck。完整的 text → model → rig → animate → export 链路
