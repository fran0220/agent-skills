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

## 品类固定映射（产品语义）

对 Agent 文档和能力设计而言，`asset-gateway` 以“品类 → 固定后端 → 固定命令”的方式工作。Agent 只应该按命令选能力，不应该思考 provider 选择。

| 品类 | 固定 Provider（内部） | Agent 命令 |
|------|----------------------|------------|
| 图片（生成 + 编辑 + 蒙版 + 风格） | Gemini | `generate image` |
| 视频 | Jimeng (Seedance) | `generate video` |
| 音效 / BGM | ElevenLabs | `generate audio` |
| 音乐 | ElevenLabs | `generate music` |
| 语音合成 | Qwen / DashScope | `generate tts` |
| 语音克隆 / 语音设计 | Qwen / DashScope | `voice clone` / `voice design` |
| 3D 模型 | Tripo3D | `generate model` + `process3d` |
| 文本 | LLM Proxy | `generate text` |
| 图片后处理 | 内置管线 | `process crop` / `resize` |

精灵动画仍是跨命令工作流：当前分支依赖 `generate image` + `process crop` / `resize` + 外部 ImageMagick 拼合。并行线程里设计过 `generate batch`、`process compose`、`process3d render-sprites`，但我核对当前代码后，这三项都还没有合入本分支。

## 技术栈选择

本项目使用 **Rust**（非 TypeScript），原因：

- 需要长期运行的网关服务（Axum + tokio）
- 需要更强的类型安全与并发可靠性
- 需要内建鉴权、DB 持久化的系统级能力

## 部署信息

| 项目 | 值 |
|------|-----|
| 服务器 | jpdata (185.200.65.233), user `root`, x86_64 |
| 二进制 | `/opt/asset-gateway/asset-gateway` |
| 配置文件 | `/opt/asset-gateway/config.toml` |
| 环境变量 | `/opt/asset-gateway/.env` |
| systemd | `asset-gateway.service` |
| 端口 | `6700` (Axum)，Nginx 反代 → upload.xiaomao.chat |
| 域名 | `upload.xiaomao.chat` |
| CI | push main → GitHub Actions → SSH deploy to jpdata |

### 服务器本地工具依赖

后处理管线依赖以下工具（已安装在 jpdata 服务器）：

| 工具 | 用途 | 安装方式 |
|------|------|---------|
| ImageMagick 6 | 裁剪/缩放/合成 | `dnf install ImageMagick`（用 `convert`/`identify`，非 `magick`） |
| Blender | 3D 动画渲染到 sprite frames（`process3d render-sprites` 线程规划中） | 当前分支未接入；待命令合入后再补安装步骤 |

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

[dashscope]
url = "https://dashscope-intl.aliyuncs.com"  # Singapore / Intl endpoint
key = "..."

[jimeng]
url = "http://127.0.0.1:5100"
token = "..."
```

### 部署流程

- **主要路径**：push 到 main 自动触发 CI 部署（GitHub Actions → SSH deploy to jpdata）
- **手动 fallback**：`./scripts/deploy.sh`（rsync 源码到服务器编译）

CI workflow：`.github/workflows/asset-gateway.yml`，Secrets：`JPDATA_SSH_KEY`、`JPDATA_HOST`

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
| `generate` | `image`, `video`, `audio`, `music`, `tts`, `model`, `text` |
| `generate` | `batch`（并行线程规划中，当前分支未合入） |
| `process` | `crop`, `resize` |
| `process` | `compose`（并行线程规划中，当前分支未合入） |
| `process3d` | `convert`, `texture`, `rig`, `animate`, `reduce`, `stylize`, `segment`, `prerigcheck`, `refine`, `import` |
| `process3d` | `render-sprites`（并行线程规划中，当前分支未合入） |
| `voice` | `clone`, `design`, `list`, `delete` |
| `upload` | `file`, `list`, `delete` |
| `provider` | `list`, `health` |
| `job` | `list`, `status`, `cancel` |
| `describe` | `[command]` |

Skill 安装在 `~/.config/amp/skills/asset-gateway` → `../skill/SKILL.md`。

## 当前实现状态 — 以当前分支代码为准

下面的状态按当前工作树核对；`generate batch`、`process compose`、`process3d render-sprites` 仅在并行线程里有设计/草案，当前分支并未出现对应文件、CLI 子命令或路由挂载。

### 命令面（当前能力覆盖）

| 命令组 | 子命令 | 说明 | 实现状态 |
|--------|--------|------|:--------:|
| `serve` | `--host --port --db --config` | 启动网关服务 | ✅ 完整 |
| `auth` | `login`, `logout`, `whoami` | 认证入口 | ✅ 完整 |
| `generate` | `image`, `video`, `audio`, `music`, `tts`, `model`, `text` | 资产生成 | ✅ 完整 |
| `generate` | `batch` | 批量生成 + 可选自动拼合 | ⏳ 线程规格已明确，当前分支未合入 |
| `process` | `crop`, `resize` | 图像后处理 | ✅ 完整 |
| `process` | `compose` | 多图拼合 sprite sheet | ⏳ 线程规格已明确，当前分支未合入 |
| `process3d` | `convert`, `texture`, `rig`, `animate`, `reduce`, `stylize`, `segment`, `prerigcheck`, `refine`, `import` | 3D 后处理管线 | ✅ 完整 |
| `process3d` | `render-sprites` | Blender 无头渲染动画 GLB 到 PNG 帧 | ⏳ 线程规格已明确，当前分支未合入 |
| `voice` | `clone`, `design`, `list`, `delete` | 自定义语音管理 | ✅ 完整 |
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
| Generate Batch | `POST /api/generate/batch` | ⏳ 线程设计为单 job + `buffer_unordered(4)`，当前分支未挂载 |
| Process | `POST /api/process` | ✅ Pipeline 引擎（当前仅 `smart_crop` / `resize`） |
| Process Compose | `POST /api/process` | ⏳ 线程设计复用同一路由处理 `compose`，当前分支未支持 |
| Process3d | `POST /api/process3d` | ✅ Tripo 3D 后处理管线（当前不含 `render-sprites`） |
| Process3d Render Sprites | `POST /api/process3d` | ⏳ 线程设计在同一路由上分支到 Blender，当前分支未支持 |
| Jobs | `GET /api/jobs`, `GET /api/jobs/:id`, `POST /api/jobs/:id/cancel` | ✅ 分页 + 筛选 + 详情 + 取消 |
| Health | `GET /api/health` | ✅ |
| Frontend | `GET /`, `GET /admin` | ✅ 内嵌 HTML 管理面板 |

### Provider 面（6 个）

| Provider ID | 类型 | 关键能力 | 测试状态 |
|-------------|------|---------|:--------:|
| `llm_proxy` | Text | 双协议（Anthropic + OpenAI 自动选择），SSE streaming | ✅ 1.3s |
| `gemini_image` | Image | 图片生成与编辑统一入口；支持多图参考(≤14)、多轮 session 编辑，且 `edit_mode` 会实际影响编辑语义（`inpaint` / `restyle` / `expand`） | ✅ 15s |
| `jimeng` | Image + Video | 图片生成 (jimeng-5.0) + 视频生成 (seedance-2.0-fast-vip / seedance-2.0-vip) | ✅ |
| `qwen_tts` | Tts/Voice | Qwen3-TTS via DashScope Intl：49+ 系统音色、指令控制、VC/VD | ✅ ~97ms 首包 |
| `elevenlabs` | Audio/Music | sound-generation（BGM/SFX）+ music-generation | ✅ 1.5s |
| `tripo3d` | Model3d | 完整 3D 管线：text/image/multiview → model, texture, rig, animate, convert, reduce, stylize, segment, prerigcheck, refine, import | ✅ |

> `llm_proxy` / `gemini_image` 通过 LLM proxy（api.xiaomao.chat）接入；`jimeng` 通过本地 jimeng-api Docker 容器（127.0.0.1:5100）接入；`qwen_tts` 使用独立 DashScope 国际站。

### 后处理与渲染管线（本地工具）

| 操作 | 工具 | 说明 | 测试状态 |
|------|------|------|:--------:|
| `smart_crop` (tightest) | ImageMagick `-trim +repage` | 裁剪透明边框 | ✅ 36ms |
| `smart_crop` (power_of2) | ImageMagick trim + extent | 裁剪后扩展到 2^n 尺寸 | ✅ 36ms |
| `resize` | ImageMagick `-resize` | 精确缩放 | ✅ 48ms |
| `compose` | ImageMagick `convert` / `montage` | 多图横向 / 纵向 / grid 拼合 sprite sheet | ⏳ 线程规格已明确，当前分支未合入 |
| `render_sprites` | Blender `--background --python` | 动画 GLB 渲染为 `dirNN_frameNNNN.png` 帧序列 | ⏳ 线程规格已明确，当前分支未合入 |

当前分支可链式组合的是 `smart_crop → resize`。并行线程计划把 `compose` 加入 `Pipeline::run()`，并让 `render_sprites` 在 `process3d` 内走 Blender 支路。

### 核心层

| 模块 | 功能 | 实现状态 |
|------|------|:--------:|
| Config | TOML 配置文件 + env var 覆盖 + 热重载 | ✅ |
| Registry | Provider 注册/发现/按类型查询 | ✅ |
| Dispatcher | 智能路由：能力匹配 → 健康过滤 → 优先级排序 → 自动 fallback | ✅ + 3 测试 |
| Pipeline | 后处理引擎：当前为 `smart_crop` / `resize`；线程中计划加入 `Compose` 和多输入 `ProcessRequest` | ✅ 当前分支 / ⏳ 线程中 |
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

对外产品语义优先遵循“固定品类映射”：Agent 按品类调用命令，不按 provider 做决策。底层 Dispatcher 仍保留统一路由与 fallback 机制，用于内部实现、调试和健康切换。

Dispatcher 选择链：
1. 显式 `--provider` → 仅用于内部 override / 调试，不作为 Agent 正常使用路径
2. 未显式指定时，先按 `asset_type` 命中该品类的固定后端族
3. 图片请求中，`transparent == true` 时优先支持透明图的候选；非透明图默认优先 `gemini_image`
4. 健康检查过滤 → 跳过不健康的
5. 按 `priority` 降序排列
6. 首选失败 → 自动尝试下一个候选

`POST /api/process3d` 不走多 Provider Dispatcher；它是 Tripo 专属的续处理链路，直接消费上一步返回的 `tripo_task_id`，把同一资产继续送入 texture / rig / animate / convert / reduce / stylize / segment / prerigcheck / refine / import。

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
4. 确认服务器已安装所需工具；图片类后处理默认走 ImageMagick，`render_sprites` 这类 3D 渲染需要 Blender
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
| refine | refine_model | task_id | 更高质量的细化模型 | 20-30 |
| import | import_model | file_url/file_path | 导入后的 Tripo task | 0 |

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
├── main.rs           (145)  CLI 入口：clap 解析 serve / client / process3d 命令
├── lib.rs              (9)  库入口
├── config.rs         (203)  TOML 配置文件 + env var 覆盖
├── db.rs              (10)  PostgreSQL 连接 + migration
├── error.rs          (225)  AppError 统一错误 + IntoResponse
├── output.rs          (32)  JSON 信封 success/error/print
│
├── core/
│   ├── mod.rs        (210)  AssetType, ProviderCapabilities, GenerateRequest/Response, trait AssetProvider
│   ├── pipeline.rs   (235)  后处理管线：当前仅 smart_crop / resize
│   ├── vault.rs       (66)  AES-256-GCM 加解密
│   ├── registry.rs    (51)  ProviderRegistry（线程安全注册表）
│   ├── dispatcher.rs (480)  策略路由 + 健康缓存(60s TTL) + 自动 fallback + 测试
│   └── queue.rs        (3)  异步队列占位
│
├── providers/
│   ├── mod.rs          (7)  pub mod 声明
│   ├── llm_proxy.rs  (359)  LLM 双协议 + streaming
│   ├── gemini_image.rs(378) Google 图像
│   ├── jimeng.rs    (237)  Jimeng 图片 + 视频（Seedance VIP）
│   ├── qwen_tts.rs   (548)  Qwen3-TTS：标准合成 + instruct + Voice Clone + Voice Design
│   ├── elevenlabs.rs (251)  音频 / 音乐
│   └── tripo3d.rs   (1045)  TripoClient + 全链路 3D 管线
│
├── server/
│   ├── mod.rs        (144)  ServerState + RwLock<AppConfig> + Provider 构建 + run()
│   └── routes/
│       ├── mod.rs      (35)  路由挂载
│       ├── auth.rs    (221)  注册/登录/JWT/CurrentUser extractor
│       ├── config_api.rs(82) GET/PUT /api/config (TOML 读写 + 热重载)
│       ├── generate.rs(236)  生成 + Job 持久化 + image session
│       ├── process.rs   (32) POST /api/process — 当前仅 smart_crop / resize
│       ├── process3d.rs(131)  POST /api/process3d — 当前 Tripo 3D 后处理
│       ├── assets.rs   (172) 文件上传 + 列表 + 删除
│       ├── providers.rs(127) Provider 列表 + 健康检查 + reload
│       ├── jobs.rs    (199) Job 列表/详情/取消
│       ├── users.rs   (396) 用户管理 CRUD
│       └── health.rs   (15) 健康检查
│
├── client/
│   ├── mod.rs        (971)  命令定义 + describe schema + handler 入口
│   ├── config.rs     (143)  Token 持久化（多网关 URL + 0600 权限）
│   ├── http.rs        (27)  authenticated_client（自动 Bearer 注入）
│   ├── auth_cmd.rs    (86)  登录/登出/whoami + token 写盘
│   ├── generate_cmd.rs(397) 生成 + 文件落盘（base64/URL → 本地文件）
│   ├── process_cmd.rs (90)  后处理 + 文件落盘
│   ├── process3d_cmd.rs(356) 3D 后处理命令 + 文件下载
│   ├── provider_cmd.rs(28)  Provider 列表/健康检查
│   └── job_cmd.rs     (47)  Job 列表/状态/取消
│
└── frontend/
    ├── mod.rs         (16)  内嵌 HTML 路由
    └── pages/
        ├── mod.rs      (1)
        └── admin.html(1363) 管理面板 SPA（Users/Config/Providers/Jobs）
```

并行线程目标中的以下文件当前分支尚不存在：`src/server/routes/generate_batch.rs`、`src/client/batch_cmd.rs`、`scripts/render_sprites.py`。

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
| 后处理 | ImageMagick 6 |
| 3D 精灵渲染 | Blender（`render-sprites` 线程规划中，当前分支未接入） |
| 前端 | 内嵌 HTML SPA（Vanilla JS） |

## 已知问题

- **Jimeng Video**: Seedance 视频生成需要图片输入，不支持纯文生视频；Docker 容器需设置 shm_size >= 1GB
- **Nginx Timeout**: 长时间后处理请求可能触发 Nginx 60s 超时，需调整 `proxy_read_timeout`

## 已完成优化（2026-03-30）

1. **Dispatcher 健康检查缓存** — `RwLock<HashMap>` + 60s TTL，避免每次 generate 请求都做真实 HTTP 健康检查。`invalidate_health_cache()` 在 provider reload 时清缓存
2. **Gemini Image 编辑支持** — `input_file` 存在时，下载图片并以 `inlineData` 传入 Gemini API contents 数组，支持图片编辑
3. **Gemini `edit_mode` 语义生效** — `edit_mode` 不再只是透传字段，现已实际参与图片编辑请求语义，覆盖 `inpaint` / `restyle` / `expand`
4. **Grok URL 提取加固** — 替换了 `split_whitespace` 和 `find("src=\"")` 为多策略提取：Markdown `![](url)` → HTML `<img src>` / `<a href>` → `<video>` / `<source>` tag → 裸 URL 扫描。11 个测试覆盖
5. **cost_usd 成本估算** — 所有 6 个 provider 均返回估算成本：gemini $0.04-0.08, grok $0.07-0.10, llm $0.02-0.10, qwen3-tts ~$0.115/10K chars, elevenlabs $0.05, tripo3d $0.20
6. **Image 编辑 CLI 支持** — Rust CLI 和 npm CLI 均新增 `--input` 参数，用于传入待编辑图片 URL
7. **Music 资产类型接入** — 新增 `Music` asset type，并通过 ElevenLabs `/v1/music-generation` 暴露 `generate music`
8. **Tripo 3D 全管线接入** — 从 2/14 task type 扩展到全部 14 个：multiview 生成、P1 参数增强、convert/texture/rig/animate/reduce/stylize/segment/prerigcheck/refine/import。完整的 text → model → rig → animate → export 链路
