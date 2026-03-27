# bb-browser Skill

引导 AI Agent 使用 [bb-browser](https://github.com/epiral/bb-browser) — 通过用户真实浏览器的登录态获取公域和私域信息。

## 什么是 bb-browser

bb-browser 让 AI Agent 直接使用你已登录的 Chrome 浏览器，无需 API key、无需模拟登录、不触发反爬检测。

架构：`CLI → Daemon (localhost:19824) → Chrome Extension → 你的真实浏览器`

## 前置条件

1. **安装 CLI**: `npm install -g bb-browser`
2. **安装 Chrome 扩展**: 从 [Releases](https://github.com/epiral/bb-browser/releases/latest) 下载，解压后在 `chrome://extensions/` 加载
3. **启动 Daemon**: `bb-browser daemon`（macOS 如遇连接问题用 `--host 127.0.0.1`）

## Skill 内容

| 文件 | 说明 |
|------|------|
| `SKILL.md` | Agent 入口 — 命令速查、核心工作流、使用模式 |
| `references/snapshot-refs.md` | Ref 生命周期、最佳实践、常见问题 |

## 来源

本 Skill 内容来自 [epiral/bb-browser](https://github.com/epiral/bb-browser/tree/main/skills/bb-browser) 官方仓库。
