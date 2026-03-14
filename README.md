# Agent Skills

可复用的 Agent 技能集合，适配 Amp / Claude Code 等 agent 框架。

## 技能列表

| Skill | 用途 | 依赖 |
|-------|------|------|
| [`slide-deck`](./slide-deck/) | AI 幻灯片生成（内容 → 图片 → PPTX/PDF） | Python 3.9+, google-genai, python-pptx, Pillow |
| [`openclaw-best-practices`](./openclaw-best-practices/) | OpenClaw 网关部署、路由、模型、记忆、沙箱、技能体系最佳实践 | — |
| [`pi-agent-sdk`](./pi-agent-sdk/) | Pi Agent SDK 开发指南（会话、工具、Provider、RPC、Web UI） | — |

## 使用

```bash
git clone https://github.com/fran0220/agent-skills.git

# Symlink 单个 skill 到项目
ln -s /path/to/agent-skills/slide-deck .agents/skills/slide-deck

# 或用户级
ln -s /path/to/agent-skills/slide-deck ~/.config/amp/skills/slide-deck
```

每个 skill 目录包含 `SKILL.md`（agent 入口）、`README.md`（人类文档）和 `.env.example`（环境变量模板）。
