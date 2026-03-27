# GodotForge

AI 驱动的 Godot 4.x 游戏开发工具链。

## 组件

| 组件 | 目录 | 说明 |
|------|------|------|
| **Skill** | `skill/` | Agent 技能 — 教 Agent 使用 GodotForge 完成游戏开发全流程 |
| **CLI** | `cli/` | 命令行工具 — 52 个命令，覆盖项目/场景/节点/脚本/资源/引擎/导出 |

## 安装

### Skill

```bash
ln -s /path/to/agent-skills/godot-forge/skill ~/.config/amp/skills/godot-forge
```

### CLI

```bash
cd godot-forge/cli
npm install -g .
```

## 详情

- Skill 指令：[skill/SKILL.md](./skill/SKILL.md)
- CLI 蓝图：[cli/BLUEPRINT.md](./cli/BLUEPRINT.md)
- CLI 开发指令：[cli/AGENTS.md](./cli/AGENTS.md)
