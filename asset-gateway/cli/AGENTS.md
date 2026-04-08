# asset-gateway CLI — 开发指令

## 定位

通用资产生成网关（Universal Asset Gateway）。

这是一个面向 Agent 的基础设施型 Rust 服务端项目：
- Rust 二进制只负责运行网关服务：`asset-gateway serve`
- 所有面向用户和 Agent 的命令行操作，统一由 npm CLI `@doufunao123/asset-gateway` 承担

目标是把多提供商资产生成能力统一成一个稳定接口，供任意项目复用，而不是绑定单一业务项目。

**设计蓝图**：`BLUEPRINT.md`（所有设计决策的权威来源）

## 关键设计约束

1. **服务端单职责**：Rust 二进制只提供 `serve`，不再承载 HTTP 客户端命令。
2. **API 优先契约**：网关 API 与 npm CLI 共享统一 JSON 信封，`{"ok", "command", "data"|"error"}` 是对外契约。
3. **TOML 配置文件**：Provider 配置从 `config.toml` 读取，支持 Web 面板在线编辑 + 热重载。环境变量可覆盖配置文件。
4. **Provider 可插拔**：通过统一 `AssetProvider` trait 适配不同服务。
5. **策略路由优先**：显式 provider 覆盖 > 能力匹配 > 健康过滤 > 优先级排序 > 自动 fallback。

## 品类固定映射（产品语义）

对 Agent 文档和能力设计而言，`asset-gateway` 以“品类 → 固定后端 → 固定命令”的方式工作。Agent 只应该按 npm CLI 命令选能力，不应该思考 provider 选择。

| 品类 | 固定 Provider（内部） | npm CLI 命令 |
|------|----------------------|--------------|
| 图片（生成 + 编辑 + 蒙版 + 风格） | Gemini / GPT Image（透明图） | `asset-gateway generate image` |
| 视频 | Jimeng (Seedance) | `asset-gateway generate video` |
| 音效 / BGM | ElevenLabs | `asset-gateway generate audio` |
| 音乐 | Lyria 3 | `asset-gateway generate music` |
| 语音合成 | Qwen / DashScope | `asset-gateway generate tts` |
| 语音克隆 / 语音设计 | Qwen / DashScope | `asset-gateway voice clone` / `asset-gateway voice design` |
| 3D 模型 | Tripo3D | `asset-gateway generate model` + `asset-gateway process3d ...` |
| 文本 | LLM Proxy | `asset-gateway generate text` |
| 图片后处理 | 内置管线 | `asset-gateway process ...` |
| 精灵动画 | SpriteForge | `asset-gateway generate sprite` |
| 3D 世界/环境 | WorldLabs Marble | `asset-gateway generate world` |

精灵动画现在由 SpriteForge 直接走 **xAI Grok 视频生成**：文本/参考图 → `grok-imagine-video` → `ffmpeg` 抽帧 → 白底移除 → spritesheet/GIF。已移除旧版 Gemini 网格图方案，也不再做 LLM prompt 增强。该能力风格无关，不局限于像素风。

音乐现在由 **Google Lyria 3** 提供：`generate music` 走代理网关调用 `lyria-3-clip-preview`，输出 30 秒高质量音乐片段，成本约 `$0.04/clip`。ElevenLabs 保留给 SFX / 环境音等 `generate audio` 场景。

3D 世界通过 WorldLabs Marble API 生成：`generate world` 接受文本或图片输入，输出 Gaussian Splat (`.spz`) + 碰撞网格 (`.glb`) + 全景图。

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

后处理与渲染链路依赖以下工具（已安装在 jpdata 服务器，或应随部署一并保证）：

| 工具 | 用途 | 安装方式 |
|------|------|---------|
| ImageMagick 6 | 裁剪、缩放、拼合 | `dnf install ImageMagick`（用 `convert`/`identify`，非 `magick`） |
| ffmpeg | 视频抽帧、SpriteForge 视频后处理 | `dnf install ffmpeg` |
| Blender | `process3d render-sprites` 无头渲染 GLB/GLTF | 按服务器发行版安装；需可配合 `xvfb-run` 使用 |

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
url = "https://dashscope-intl.aliyuncs.com"
key = "..."

[grok2api]
url = "https://grok.xiaomao.chat"
key = "..."

[jimeng]
url = "http://127.0.0.1:5100"
token = "..."

[worldlabs]
key = "..."

[xai]
key = "..."
```

### 部署流程

- **主要路径**：push 到 main 自动触发 CI 部署（GitHub Actions runner 编译 → scp 二进制 → 重启 systemd）
- **重要**：不在 jpdata 服务器上编译（性能差），在 runner (`ubuntu-latest` x86_64) 上编译后推送二进制

CI workflow：`.github/workflows/asset-gateway.yml`，Secrets：`JPDATA_SSH_KEY`、`JPDATA_HOST`

## npm CLI 客户端（唯一客户端）

npm 包 `@doufunao123/asset-gateway` 是面向 Agent 的唯一命令行客户端：

```bash
npm install -g @doufunao123/asset-gateway
asset-gateway auth set <token>
```

源码在 `../npm/`，使用 TypeScript + commander，tsup 构建。

| 命令组 | 子命令 |
|--------|--------|
| `auth` | `set`, `status`, `clear` |
| `generate` | `image`, `video`, `audio`, `music`, `tts`, `model`, `text`, `sprite`, `world`, `batch` |
| `process` | `crop`, `resize`, `compose`, `extract-frames`, `remove-bg` |
| `process3d` | `convert`, `texture`, `rig`, `animate`, `render-sprites`, `reduce`, `stylize`, `segment`, `prerigcheck`, `refine`, `import` |
| `voice` | `clone`, `design`, `list`, `delete` |
| `upload` | `file`, `list`, `delete` |
| `provider` | `list`, `health` |
| `job` | `list`, `status`, `cancel` |
| `describe` | `[command]` |

Sprite 命令的当前行为：

- 已移除 `--grid-size`
- `--animation-type` 默认值为 `walk`
- 新增 `--duration`，默认 `2`

Skill 安装在 `~/.config/amp/skills/asset-gateway` → `../skill/SKILL.md`。

## 当前实现状态

### Rust 二进制命令面

| 命令组 | 说明 | 实现状态 |
|--------|------|:--------:|
| `serve` | 启动网关服务（`--host --port --database-url --config`） | ✅ 完整 |

Rust 客户端命令已退出产品职责范围；所有用户操作统一走 npm CLI。

### Server 端

| 模块 | 路由 | 实现状态 |
|------|------|:--------:|
| Auth | `POST /auth/register`, `POST /auth/login` | ✅ Argon2 + JWT + API Key + Admin Token |
| Auth extractor | Bearer / `api_key` query → `CurrentUser` | ✅ |
| Users | `GET /api/users`, `PUT /api/users/:id`, `DELETE /api/users/:id` | ✅ 用户管理 + Admin 限制 |
| Config | `GET /api/config`, `PUT /api/config` | ✅ TOML 读写 + 热重载 |
| Providers | `GET /api/providers`, `GET /api/providers/health`, `POST /api/providers/reload` | ✅ 列表 + 健康检查 + 热重载 |
| Assets | `POST /api/assets/upload`, `GET /api/assets`, `DELETE /api/assets/:name` | ✅ 文件上传 + 列表 + 删除 |
| Static | `GET /uploads/:filename` | ✅ 上传文件静态访问（无需认证） |
| Generate | `POST /api/generate` | ✅ Job 持久化 + Dispatcher 路由 |
| Generate Batch | `POST /api/generate/batch` | ✅ 批量生成 + `buffer_unordered(4)` + 可选 compose |
| Process | `POST /api/process` | ✅ `smart_crop` / `resize` / `compose` / `extract_frames` / `remove_bg` |
| Process3d | `POST /api/process3d` | ✅ Tripo 3D 后处理 + `render_sprites` |
| Voice | `POST /api/voice/clone`, `POST /api/voice/design`, `GET /api/voice/list`, `DELETE /api/voice/:id` | ✅ Qwen 自定义语音管理 |
| Jobs | `GET /api/jobs`, `GET /api/jobs/:id`, `POST /api/jobs/:id/cancel` | ✅ 分页 + 筛选 + 详情 + 取消 |
| Tripo Admin | `GET /api/tripo/balance` | ✅ 管理员余额查询 |
| Health | `GET /api/health` | ✅ |
| Frontend | `GET /`, `GET /admin` | ✅ 内嵌 HTML 管理面板 |

### Provider 面（10 个产品语义 Provider）

| Provider ID | 类型 | 关键能力 |
|-------------|------|---------|
| `llm_proxy` | Text | 双协议（Anthropic + OpenAI 自动选择），支持流式文本生成 |
| `gemini_image` | Image | 默认图片生成与编辑入口；支持多图参考、多轮 session 编辑、`edit_mode` 语义 |
| `gpt_image` | Image | 透明背景图片与 alpha 输出，作为透明图优先候选 |
| `jimeng` | Image + Video | Jimeng 图片生成 + Seedance 图生视频 |
| `qwen_tts` | Tts / Voice | Qwen3-TTS via DashScope Intl：系统音色、指令控制、VC/VD |
| `elevenlabs` | Audio | SFX / BGM / 环境音生成；不再承担 `generate music` |
| `lyria` | Music | Google Lyria 3 (`lyria-3-clip-preview`) via proxy，30 秒片段，约 `$0.04/clip`，优先级高于 ElevenLabs |
| `tripo3d` | Model3d | 完整 3D 管线：text/image/multiview → model, texture, rig, animate, convert, reduce, stylize, segment, prerigcheck, refine, import |
| `spriteforge` | Sprite | xAI `grok-imagine-video` → `ffmpeg` 抽帧 → 白底移除 → spritesheet/GIF；支持文本和参考图，风格无关，成本 `$0.05/sec`（默认 2 秒约 `$0.10`） |
| `worldlabs` | World | WorldLabs Marble API：文本/图片 → Gaussian Splat 3D 世界，输出 SPZ + collider GLB + panorama |

> `llm_proxy` / `gemini_image` / `gpt_image` / `lyria` 都通过小猫 AI 代理网关（`api.xiaomao.chat`）接入；`jimeng` 通过本地 `jimeng-api` Docker 容器（`127.0.0.1:5100`）接入；`spriteforge` 直接使用 `[xai]` 配置的 API Key。

### 后处理与渲染管线

| 操作 | 工具 / 服务 | 说明 |
|------|-------------|------|
| `smart_crop` (tightest) | ImageMagick `-trim +repage` | 裁剪透明边框 |
| `smart_crop` (power_of2) | ImageMagick trim + extent | 裁剪后扩展到 2^n 尺寸 |
| `resize` | ImageMagick `-resize` | 精确缩放 |
| `compose` | ImageMagick `convert` / `montage` | 多图横向 / 纵向 / grid 拼合 |
| `extract_frames` | `ffmpeg` | 从视频抽取 PNG 帧序列 |
| `remove_bg` | BgSweep API | 移除背景并返回透明 PNG |
| `render_sprites` | Blender + `scripts/render_sprites.py` | GLB / GLTF 动画无头渲染为精灵帧序列 |

### 核心层

| 模块 | 功能 | 实现状态 |
|------|------|:--------:|
| Config | TOML 配置文件 + env var 覆盖 + 热重载 | ✅ |
| Registry | Provider 注册/发现/按类型查询 | ✅ |
| Dispatcher | 智能路由：能力匹配 → 健康过滤 → 优先级排序 → 自动 fallback | ✅ |
| Pipeline | 后处理引擎：`smart_crop` / `resize` / `compose` / `extract_frames` / `remove_bg` | ✅ |
| Vault | AES-256-GCM 加解密（保留但不再用于 provider key） | ✅ |

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

对外产品语义优先遵循“固定品类映射”：Agent 按命令调用能力，不按 provider 做决策。底层 Dispatcher 仍保留统一路由与 fallback 机制，用于内部实现、调试和健康切换。

Dispatcher 选择链：
1. 显式 `--provider` → 仅用于内部 override / 调试，不作为 Agent 正常使用路径
2. 未显式指定时，先按 `asset_type` 命中该品类的固定后端族
3. 图片请求中，`transparent == true` 时优先 `gpt_image`；非透明图默认优先 `gemini_image`
4. 音乐默认优先 `lyria`
5. 健康检查过滤 → 跳过不健康的
6. 按 `priority` 降序排列
7. 首选失败 → 自动尝试下一个候选

`POST /api/process3d` 不走多 Provider Dispatcher；它是 Tripo 专属的续处理链路，直接消费上一步返回的 `tripo_task_id`，把同一资产继续送入 `texture` / `rig` / `animate` / `convert` / `render_sprites` / `reduce` / `stylize` / `segment` / `prerigcheck` / `refine` / `import`。

## 开发约定

### 新增 Provider Checklist

1. 在 `src/providers/<name>.rs` 实现 `AssetProvider` trait
2. 明确 `id()`, `display_name()`, `asset_types()`, `capabilities()`
3. `generate()` 返回统一 `GenerateResponse`，不得泄漏密钥
4. `health_check()` 必须实现，不做空壳
5. 在 `src/server/mod.rs` 的 `build_providers_from_config()` 中加入构建逻辑
6. 在 `src/config.rs` 添加对应 TOML section
7. 在 `src/providers/mod.rs` 中加 `pub mod`
8. 如对外能力有变化，更新 npm CLI 客户端（`../npm/`）及其 describe schema
9. 更新 `config.toml.example`、SKILL 文档和本文档

### 新增 Pipeline 操作 Checklist

1. 在 `src/core/pipeline.rs` 的 `ProcessOp` 枚举增加变体
2. 在 `Pipeline::run()` 的 match 中增加执行分支
3. 实现对应的 `async fn`（调用外部工具或服务）
4. 确认服务器已安装所需工具；图片类后处理默认走 ImageMagick，视频抽帧走 `ffmpeg`，3D 渲染走 Blender
5. 更新 npm CLI `../npm/src/commands/process.ts`
6. 更新 npm CLI `../npm/src/describe-schemas.ts`
7. 更新 SKILL.md、reference 文档和本文档

### Process3d 操作说明

所有 `process3d` 操作通过 Tripo API 执行，基于 `tripo_task_id` 链式调用：

| 操作 | Tripo task type | 输入 | 输出 | 估算 Credits |
|------|----------------|------|------|:------------:|
| convert | convert_model | task_id + format + quad/face_limit | FBX/USDZ/OBJ/STL/GLTF/3MF | 5-10 |
| texture | texture_model | task_id + prompt + pbr/quality | 重新贴图的模型 | 20-40 |
| rig | animate_rig | task_id + spec(mixamo/tripo) | 带骨骼的模型 | ~25 |
| animate | animate_retarget | task_id + animation preset | 带动画的模型 | ~25 |
| render_sprites | 本地 Blender 渲染 | task_id + frame_count + resolution + camera_angle + directions | PNG 帧序列 / sprite 数据 | 本地计算 |
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
6. 更新 npm CLI 客户端命令映射（`../npm/src/commands/*`）
7. 更新 npm CLI describe schema 与文档

## 源码导航

```text
asset-gateway/
├── cli/
│   ├── src/
│   │   ├── main.rs                Rust 服务入口；仅解析 `serve`
│   │   ├── lib.rs                 库入口
│   │   ├── config.rs              TOML 配置 + env 覆盖 + 热重载
│   │   ├── db.rs                  PostgreSQL 连接 + migration
│   │   ├── error.rs               AppError 统一错误 + IntoResponse
│   │   ├── output.rs              JSON 信封辅助
│   │   ├── core/
│   │   │   ├── mod.rs             AssetType / GenerateRequest / AssetProvider
│   │   │   ├── pipeline.rs        图片/视频后处理管线
│   │   │   ├── registry.rs        ProviderRegistry
│   │   │   ├── dispatcher.rs      策略路由 + fallback
│   │   │   ├── vault.rs           AES-256-GCM 工具
│   │   │   └── queue.rs           异步队列占位
│   │   ├── providers/
│   │   │   ├── llm_proxy.rs       文本生成
│   │   │   ├── gemini_image.rs    默认图片生成/编辑
│   │   │   ├── gpt_image.rs       透明图 / alpha 输出
│   │   │   ├── jimeng.rs          图片 + Seedance 视频
│   │   │   ├── qwen_tts.rs        TTS / Voice Clone / Voice Design
│   │   │   ├── elevenlabs.rs      SFX / Audio
│   │   │   ├── lyria.rs           Google Lyria 3 音乐生成
│   │   │   ├── spriteforge.rs     xAI 视频 → 抽帧 → spritesheet/GIF
│   │   │   ├── tripo3d.rs         Tripo 全链路 3D 管线
│   │   │   └── worldlabs.rs       WorldLabs Marble
│   │   ├── server/
│   │   │   ├── mod.rs             ServerState + Provider 构建 + run()
│   │   │   └── routes/
│   │   │       ├── auth.rs        注册 / 登录 / CurrentUser
│   │   │       ├── config_api.rs  `/api/config`
│   │   │       ├── generate.rs    `/api/generate`
│   │   │       ├── generate_batch.rs `/api/generate/batch`
│   │   │       ├── process.rs     `/api/process`
│   │   │       ├── process3d.rs   `/api/process3d`
│   │   │       ├── voice.rs       `/api/voice/*`
│   │   │       ├── providers.rs   `/api/providers*`
│   │   │       ├── jobs.rs        `/api/jobs*`
│   │   │       ├── assets.rs      `/api/assets*`
│   │   │       ├── users.rs       `/api/users*`
│   │   │       ├── tripo.rs       `/api/tripo/balance`
│   │   │       └── health.rs      `/api/health`
│   │   └── frontend/
│   │       ├── mod.rs             内嵌 HTML 路由
│   │       └── pages/admin.html   管理面板 SPA
│   ├── scripts/
│   │   ├── render_sprites.py      Blender 渲染脚本
│   │   ├── setup-server.sh        服务器初始化
│   │   └── deploy.sh              部署脚本
│   ├── config.toml.example
│   ├── BLUEPRINT.md
│   └── README.md
└── npm/
    └── src/                       唯一受支持的 CLI 客户端
```

## 技术栈

| 层 | 选择 |
|----|------|
| 语言 | Rust (edition 2021) |
| 服务框架 | Axum 0.8 + tower-http + tokio |
| 数据库 | PostgreSQL + sqlx 0.8 + migrations |
| 配置 | TOML (toml 0.8) + dotenvy |
| 认证 | jsonwebtoken + argon2 + admin token |
| HTTP 客户端 | reqwest 0.12 |
| 序列化 | serde + serde_json |
| 图片后处理 | ImageMagick 6 |
| 视频后处理 | ffmpeg |
| 3D 精灵渲染 | Blender + `xvfb-run` |
| 前端 | 内嵌 HTML SPA（Vanilla JS） |

## 提交前检查

- 提交前先运行 `cargo fmt`
- CI 检查以 `cargo test --lib`、`cargo clippy`、`cargo fmt --check` 为准

## 已知问题

- **Jimeng Video**: Seedance 视频生成需要图片输入，不支持纯文生视频；Docker 容器需设置 `shm_size >= 1GB`
- **Nginx Timeout**: 长时间后处理请求可能触发 Nginx 60s 超时，需调整 `proxy_read_timeout`
- **SpriteForge**: 依赖 `[xai]` 配置和服务器上的 `ffmpeg`；缺任一项都会导致 `generate sprite` 不可用

## 已完成优化（2026-04）

1. **Rust CLI 客户端退出职责** — Rust 二进制收敛为 `asset-gateway serve`，npm CLI 成为唯一客户端。
2. **SpriteForge 全量重写** — 从旧版 Gemini 网格图方案切换到 xAI `grok-imagine-video` → `ffmpeg` 抽帧 → 白底移除 → spritesheet/GIF，不再做 LLM 增强。
3. **Sprite CLI 参数更新** — 移除 `--grid-size`，`--animation-type` 默认值改为 `walk`，新增 `--duration`（默认 2 秒）。
4. **Lyria Provider 接入** — `generate music` 迁移到 Google Lyria 3，30 秒音乐片段约 `$0.04/clip`，优先级高于 ElevenLabs。
5. **ElevenLabs 职责收缩** — 保留 SFX / Audio，不再承担 `generate music`。
6. **新增 `[xai]` 配置段** — 允许 SpriteForge 直接接入 xAI 官方 API。
7. **后处理管线扩展** — `process` 现支持 `compose`、`extract_frames`、`remove_bg`，不再局限于 `smart_crop` / `resize`。
8. **`process3d render_sprites` 落地** — 通过 Blender 无头渲染 GLB / GLTF 动画到帧序列。
9. **Dispatcher 健康检查缓存** — `RwLock<HashMap>` + 60s TTL，减少重复真实健康探测。
10. **Tripo 3D 全管线接入** — 扩展到完整 task type 集合，覆盖 text → model → rig → animate → export 链路。
