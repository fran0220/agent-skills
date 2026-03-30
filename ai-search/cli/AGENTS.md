# ai-search CLI — 开发指令

## 定位

AI 驱动的 Web 搜索 CLI + Gateway + MCP 服务器。

轻量级 Rust 二进制，双模式运行：
- 直接搜索：`ai-search "query"`
- Gateway / MCP 服务：`ai-search serve`

通过小猫 AI 网关（`api.xiaomao.chat`）+ Exa + Tavily 统一实现多源搜索、评分排序和网关 API。

## 关键设计约束

1. **单二进制双模式**：`ai-search serve` 跑 HTTP / MCP 服务，其他命令直接搜索或调用网关。
2. **无数据库**：当前实现不引入 PostgreSQL；配置与状态以 TOML / 运行时内存为主。
3. **JSON 默认输出**：统一信封 `{"ok", "command", "data"|"error"}`。
4. **Gateway 鉴权可选**：`gateway_token` 留空则不开启 Bearer token 鉴权。
5. **MCP 兼容**：stdio 模式实现 JSON-RPC 2.0，供 Claude Desktop、Cursor、Amp 集成。
6. **TOML + env 覆盖**：配置文件为主，环境变量覆盖配置文件。
7. **多源搜索策略**：`fast` 走 Grok，`deep` 并行 Grok + Exa + Tavily，`answer` 走 Tavily AI summary。

## 部署信息

| 项目 | 值 |
|------|-----|
| 服务器 | Oracle (161.33.13.122), user `opc` |
| 二进制 | `/opt/ai-search/ai-search` |
| 配置文件 | `/opt/ai-search/config.toml` |
| systemd | `ai-search.service` |
| 端口 | `6900` (Axum)，Nginx 反代 `443` |
| 域名 | `search.xiaomao.chat`（CF proxy → Oracle Nginx → 6900） |
| 管理面板 | `https://search.xiaomao.chat/admin` |

## 技术栈

| 层 | 选择 |
|----|------|
| 语言 | Rust (edition 2021) |
| CLI | clap 4 (derive) |
| HTTP 服务 | Axum 0.8 + tokio |
| HTTP 客户端 | reqwest 0.12 |
| 配置 | TOML + env var |
| 序列化 | serde + serde_json |

## 配置优先级

**env var > config.toml > 默认值**

## config.toml 结构

```toml
[server]
port = 6900
gateway_token = ""

[proxy]
url = "https://api.xiaomao.chat"
key = ""
search_model = "grok-4.1-fast"
analysis_model = "gemini-3-flash-preview"

[exa]
key = ""

[tavily]
key = ""

[search]
max_split = 10
timeout_secs = 180
default_mode = "fast"
```

## 搜索模式

| 模式 | Provider 组合 | 说明 |
|------|---------------|------|
| `fast` | Grok | 默认模式，低延迟结果 |
| `deep` | Grok + Exa + Tavily | 并行多源召回 + 评分 |
| `answer` | Tavily | AI 答案 / 摘要优先 |

## Provider 列表

| Provider | 上游 | 角色 | 状态 |
|----------|------|------|------|
| `grok` | `api.xiaomao.chat` | 默认搜索源，支撑 `fast` | 主线路径 |
| `exa` | `api.exa.ai` | 语义召回、补充深度来源 | `deep` 模式 |
| `tavily` | `api.tavily.com` | 网页搜索 + AI answer | `deep` / `answer` 模式 |

## 源码导航（重构后）

```
src/
├── main.rs              CLI 入口：direct search / serve / MCP
├── config.rs            TOML 配置加载 + env 覆盖
├── output.rs            JSON 信封工具
├── search.rs            搜索编排与模式分发
├── scoring.rs           多源结果评分 / 合并逻辑
├── mcp.rs               MCP stdio handler
├── providers/
│   ├── mod.rs           Provider trait + 注册
│   ├── grok.rs          Grok provider（经 xiaomao proxy）
│   ├── exa.rs           Exa provider
│   └── tavily.rs        Tavily provider
└── server/
    ├── mod.rs           Axum app + shared state
    ├── auth.rs          Bearer token middleware
    └── routes/
        ├── mod.rs       路由挂载
        ├── search.rs    POST /api/search
        ├── models.rs    GET /api/models
        ├── providers.rs GET /api/providers + /health
        ├── config.rs    GET/PUT /api/config
        ├── jobs.rs      GET /api/jobs
        ├── health.rs    GET /health
        └── admin.rs     GET /admin（内嵌 HTML SPA）
```

## 部署流程

```bash
rsync -az --exclude target --exclude .git cli/ opc@161.33.13.122:/tmp/ai-search-build/
ssh opc@161.33.13.122 'cd /tmp/ai-search-build && cargo build --release'
ssh opc@161.33.13.122 'sudo systemctl stop ai-search || true'
ssh opc@161.33.13.122 'sudo mkdir -p /opt/ai-search && sudo cp /tmp/ai-search-build/target/release/ai-search /opt/ai-search/'
ssh opc@161.33.13.122 'sudo cp /tmp/ai-search-build/scripts/ai-search.service /etc/systemd/system/ai-search.service && sudo systemctl daemon-reload && sudo systemctl start ai-search'
```

## 管理面板

内嵌 HTML SPA（`/admin`），包含 4 个标签页：

| 标签 | 功能 |
|------|------|
| Search Test | 手工测试 `fast` / `deep` / `answer` 请求 |
| Providers | 查看 provider 列表与健康状态 |
| Config Editor | 在线编辑 `config.toml` |
| Logs | 查看任务 / 查询历史 |

## 退出码

- `0` 成功
- `1` 命令/输入错误
- `2` 系统错误（配置/服务内部异常）
- `3` 外部服务错误（上游 provider / gateway 调用失败）
