# Cognee Admin

Cognee 知识引擎管理工具，提供一个 Rust CLI 和一个基于 HTMX 的 Web 管理面板。

## 组件

| 组件 | 目录 | 说明 |
|------|------|------|
| CLI + Web | `cli/` | 单二进制双模式：`serve` 跑面板，其余命令走 Cognee API 客户端 |
| Skill | `skill/` | 面向 Agent 的使用说明，指导如何用 `cognee-admin` 做知识管理 |

## 安装

```bash
cd cognee-admin/cli
cargo install --path .
```

## 环境变量

| 变量 | 用途 |
|------|------|
| `COGNEE_URL` | Cognee API 地址，默认 `https://cogneeapi.xiaomao.chat` |
| `COGNEE_JWT` | Cognee API JWT，供 `health`/`dataset`/`data`/`cognify`/`search`/`config`/`ontology` 使用 |
| `COGNEE_ADMIN_TOKEN` | Web 面板与 Nginx `auth_request` 使用的 `ca_xxx` token |
| `COGNEE_ADMIN_DB` | PostgreSQL 连接串，供日志、pipeline、token 管理和 Web 面板使用 |
| `COGNEE_SERVICE_JWT` | `serve` 模式必须，用于 Web 服务器上游访问 Cognee API |

## 命令总览

| 命令 | 说明 |
|------|------|
| `serve` | 启动 Web 管理面板 |
| `health` | 健康检查，支持 `--detailed --watch --interval` |
| `login` | 用用户名密码换取 Cognee JWT 并保存到本地配置 |
| `dataset` | 数据集管理：`list/create/delete/delete-all/status/graph` |
| `data` | 数据录入与维护：`add/add-file/add-dir/list/delete/raw/update` |
| `cognify` | 触发知识构建，支持 `--dataset-name`、自定义 prompt、后台运行 |
| `search` | 知识搜索，支持 `--datasets` 过滤、`--verbose` 和 `history` |
| `config` | 查看和更新 Cognee 运行时配置 |
| `log` | 查询请求日志和统计 |
| `pipeline` | 查看 pipeline 运行记录 |
| `token` | 管理 Web 面板 API token |
| `ontology` | 上传和列出 ontology |
| `describe` | 输出命令自省 JSON Schema |

## 快速开始

```bash
# 1. 登录，保存 Cognee JWT
cognee-admin login --username alice --password '<password>'

# 2. 创建数据集
cognee-admin dataset create gamedb-atoms

# 3. 上传文件
cognee-admin data add-file --dataset gamedb-atoms ./docs/zelda-botw.md

# 4. 触发 cognify
cognee-admin cognify --dataset-name gamedb-atoms --background

# 5. 搜索结果
cognee-admin search "open world traversal" --datasets gamedb-atoms --top-k 10
```

## 文档

- CLI 使用说明：[cli/README.md](./cli/README.md)
- CLI 蓝图：[cli/BLUEPRINT.md](./cli/BLUEPRINT.md)
- CLI 开发指令：[cli/AGENTS.md](./cli/AGENTS.md)
- Agent Skill：[skill/SKILL.md](./skill/SKILL.md)
- Skill 人类文档：[skill/README.md](./skill/README.md)
