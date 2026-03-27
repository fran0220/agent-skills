# Asset Gateway

通用资产生成网关 — 统一多提供商资产生成能力（图像/视频/音频/3D/文本）。

## 组件

| 组件 | 目录 | 说明 |
|------|------|------|
| **Skill** | `skill/` | Agent 技能 — 教 Agent 使用 Asset Gateway API |
| **CLI** | `cli/` | Rust CLI + 网关服务 — 单二进制双模式（serve + client） |

## 安装

### Skill

```bash
ln -s /path/to/agent-skills/asset-gateway/skill ~/.config/amp/skills/asset-gateway
```

### CLI

```bash
cd asset-gateway/cli
cargo install --path .
```

## 详情

- Skill 指令：[skill/SKILL.md](./skill/SKILL.md)
- CLI 蓝图：[cli/BLUEPRINT.md](./cli/BLUEPRINT.md)
- CLI 开发指令：[cli/AGENTS.md](./cli/AGENTS.md)
