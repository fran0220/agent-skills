# Jimeng Gateway

即梦/Seedance AI 视频生成网关 — Rust 后端 (axum) + React 前端。

## 组件

| 组件 | 目录 | 说明 |
|------|------|------|
| **Skill** | `skill/` | Agent 技能 — 即梦 API 网关配置与使用 |
| **CLI** | `cli/` | Rust 后端 — axum 服务 + 即梦 API 集成 + SQLite |
| **Web** | `web/` | React 前端 — 管理面板 (Vite + TailwindCSS) |

## 安装

### Skill

```bash
ln -s /path/to/agent-skills/jimeng-gateway/skill ~/.config/amp/skills/jimeng-gateway
```

### 后端

```bash
cd jimeng-gateway/cli
cargo build --release
```

### 前端

```bash
cd jimeng-gateway/web
npm install && npm run build
```

## 详情

- Skill 指令：[skill/SKILL.md](./skill/SKILL.md)
- 项目指令：[AGENTS.md](./AGENTS.md)
- 部署文档：[cli/docs/](./cli/docs/)
