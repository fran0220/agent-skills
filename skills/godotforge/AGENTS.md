# godotforge Skill — 开发指令

## 定位

AI 驱动的 Godot 4.x 游戏开发 Skill。教 Agent 使用 `godot-forge` CLI 完成从概念到发布的全流程。

## 对应 CLI

`cli/godot-forge/` — TypeScript CLI，详见其 BLUEPRINT.md。

## 内容维护规则

### reference/ — 规范文档

| 文件 | 内容 | 维护时机 |
|------|------|---------|
| `workflow.md` | 19 步 SOP | CLI 命令结构变化时同步 |
| `cli-commands.md` | CLI 完整参考 | CLI 新增/修改命令时同步 |
| `godot-formats.md` | .tscn/.tres 格式规范 | Godot 版本升级时检查 |
| `quality-gates.md` | 质量门禁定义 | 工作流调整时更新 |

### knowledge/ — 领域知识

| 文件 | 内容 | 来源 |
|------|------|------|
| `game-design.md` | 核心循环、MDA 框架 | 游戏设计理论 |
| `godot-patterns.md` | 场景/信号/状态机模式 | Godot 最佳实践 |
| `scene-organization.md` | 项目结构约定 | 社区共识 |
| `feel-presets.md` | 游戏手感参数预设 | 实践积累 |
| `anti-patterns.md` | 常见错误 | 实践积累 |

## Skill 与 CLI 同步

- CLI 命令接口变更时，必须同步更新 SKILL.md 中的 `CLI Commands` 段
- CLI 新增命令组时，评估是否需要新增 reference/ 或 knowledge/ 文件
- Godot 版本升级时，检查 `godot-formats.md` 和 `godot-patterns.md` 是否需要更新
