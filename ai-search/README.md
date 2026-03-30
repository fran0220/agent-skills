# AI Search Gateway

多源 AI 搜索网关，统一封装 Grok、Exa、Tavily，并同时提供 Rust 单二进制、npm 客户端和 Amp Skill。

## 组件

| 组件 | 目录 | 说明 |
|------|------|------|
| **Skill** | `skill/` | Agent 技能，指导 Agent 通过网关执行搜索、MCP 集成与排障 |
| **CLI** | `cli/` | Rust 单二进制，支持本地直搜、HTTP Gateway、MCP stdio |
| **npm** | `npm/` | `@doufunao123/ai-search`，面向 Agent / Node.js 的轻量 HTTP 客户端 |

## 架构

| 模式 | 适用场景 | 说明 |
|------|----------|------|
| **Gateway 模式** | 多 Agent 共享搜索能力、网页管理面板、远程部署 | 通过 `https://search.xiaomao.chat` 访问统一 API，Bearer token 鉴权可选 |
| **本地模式** | 本地开发、离线调试、IDE MCP 接入 | 运行 `ai-search serve` 或 `ai-search serve --stdio`，直接使用本机配置文件与 provider key |

默认搜索模式：`fast`。深度研究时使用 `deep`，需要结构化 AI 总结时使用 `answer`。

## 安装

### Rust CLI

```bash
cd ai-search/cli
cargo build --release
cp target/release/ai-search ~/.local/bin/
```

### npm 客户端

```bash
npm install -g @doufunao123/ai-search
```

### Amp Skill

```bash
ln -s /path/to/agent-skills/ai-search/skill ~/.config/amp/skills/ai-search
```

## 搜索模式

| 模式 | Provider 组合 | 说明 |
|------|---------------|------|
| `fast` | Grok | 默认模式，低延迟网页搜索 |
| `deep` | Grok + Exa + Tavily | 并行多源召回，适合研究、对比、查证 |
| `answer` | Tavily | 使用 Tavily 的 AI answer 能力生成摘要结果 |

## Providers

| Provider | 接口 | 用途 |
|----------|------|------|
| **Grok** | `https://api.xiaomao.chat` | 默认搜索引擎，支持原生联网搜索 |
| **Exa** | `https://api.exa.ai` | 语义召回、网页发现、深度模式补充来源 |
| **Tavily** | `https://api.tavily.com` | 网页搜索 + AI answer，总结型查询优先 |

## 配置

主配置文件：`~/.config/ai-search/config.toml`

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

规则：环境变量覆盖 `config.toml`。`gateway_token` 留空时，HTTP Gateway 不要求 Bearer token。

## API 端点

| 方法 | 路径 | 说明 |
|------|------|------|
| `POST` | `/api/search` | 执行搜索，支持 `fast` / `deep` / `answer` |
| `GET` | `/api/models` | 返回可用模型列表 |
| `GET` | `/api/providers` | 返回 provider 列表 |
| `GET` | `/api/providers/health` | 返回 provider 健康状态 |
| `GET` | `/api/config` | 读取当前 TOML 配置 |
| `PUT` | `/api/config` | 更新配置并触发热重载 |
| `GET` | `/api/jobs` | 查询任务 / 搜索记录 |
| `GET` | `/health` | 网关健康检查 |
| `GET` | `/admin` | 内嵌管理面板 |

## 使用示例

```bash
# 默认快速搜索
ai-search "latest AI news" --mode fast

# 深度搜索
ai-search "compare Exa and Tavily for developer research" --mode deep --json

# MCP stdio（Claude Desktop / Cursor / Amp）
ai-search serve --stdio

# 本地网关
ai-search serve --port 6900 --config ~/.config/ai-search/config.toml

# 远程网关
curl https://search.xiaomao.chat/api/search \
  -H "Authorization: Bearer $AI_SEARCH_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query":"latest AI news","mode":"deep"}'
```

## 部署

| 项目 | 值 |
|------|-----|
| 服务器 | Oracle `161.33.13.122` (`opc`) |
| 域名 | `search.xiaomao.chat` |
| 端口 | `6900`（Axum），Nginx 反代 `443` |
| 二进制 | `/opt/ai-search/ai-search` |
| 配置文件 | `/opt/ai-search/config.toml` |
| systemd | `ai-search.service` |
| 管理面板 | `https://search.xiaomao.chat/admin` |

部署模板见 `cli/scripts/deploy.sh`、`cli/scripts/ai-search.service` 和 `cli/config.toml.example`。
