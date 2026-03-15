# agent-skills — Agent 指令

## 仓库结构约定

本仓库是 **Skill + CLI monorepo**，顶层两个目录：

```
agent-skills/
├── skills/     # 所有 Skill（知识层）
└── cli/        # 所有 CLI（能力层）
```

### Skill 与 CLI 的关系

- **Skill = 纯知识**：SKILL.md + reference/ + knowledge/，不含可执行代码
- **CLI = 纯工具**：独立 Python 包（或其他语言），可通过 pip install 安装
- **分离原则**：Skill 告诉 Agent 该做什么 → Agent 调 CLI 执行
- **多对多**：一个 Skill 可以依赖多个 CLI，一个 CLI 可以被多个 Skill 使用

## 新建 Skill 约定

每个 Skill 是 `skills/` 下的一个目录，必须包含：

```
skills/<skill-name>/
├── SKILL.md          # 必须 — Agent 入口（YAML frontmatter + 指令）
├── README.md         # 必须 — 人类文档
├── .env.example      # 可选 — 环境变量模板（如需 API key）
├── reference/        # 可选 — 规范文档（Agent 按需读取）
└── knowledge/        # 可选 — 领域知识（Agent 按需读取）
```

### SKILL.md 格式

```yaml
---
name: <skill-name>          # 必须，与目录名一致
description: "<描述>"        # 必须，说明用途和触发条件
---

# Skill Title

（Agent 指令内容，<500 行）
```

### 命名规范

- 目录名：小写 + 连字符（`my-skill-name`）
- `name` 字段：与目录名一致

## 新建 CLI 约定

每个 CLI 是 `cli/` 下的一个独立 Python 包：

```
cli/<cli-name>/
├── pyproject.toml    # 必须 — 包配置 + 依赖 + entry_points
├── README.md         # 必须 — 使用说明
├── <package_name>/   # 必须 — Python 源码
│   ├── __init__.py
│   ├── cli.py        # Click 入口
│   ├── commands/     # 子命令实现
│   ├── core/         # 核心逻辑
│   └── utils/        # 工具函数
└── tests/            # 必须 — 测试
```

### CLI 设计原则

1. **`--json` 全覆盖**：每个命令支持 `--json` 输出，Agent 可解析
2. **退出码规范**：0 = 成功，1 = 用户错误，2 = 系统错误
3. **幂等性**：相同输入 + 相同状态 = 相同输出
4. **无交互模式**：Agent 调用时不能有 prompt，所有参数通过命令行传入
5. **错误信息可操作**：报错要说明怎么修复，不只是"失败了"

### 命名规范

- 目录名：小写 + 连字符（`godot-forge`）
- Python 包名：下划线（`godot_forge`）
- CLI 命令名：与目录名一致（`godot-forge`）

## Skill 如何引用 CLI

在 SKILL.md 的 Prerequisites 中声明依赖：

```markdown
## Prerequisites

1. **godot-forge CLI**: `pip install godot-forge`
2. Verify: `godot-forge --version`
```

然后在工作流中指导 Agent 调用 CLI 命令：

```markdown
## Workflow

### Step 1: Initialize project
Run `godot-forge project init my-game --template platformer-2d`
```

## 符号链接约定

安装 Skill 时只链接 `skills/<name>/`，不链接 `cli/`：

```bash
# Skill 通过符号链接安装
ln -s /path/to/agent-skills/skills/godotforge ~/.config/amp/skills/godotforge

# CLI 通过 pip 安装
pip install -e /path/to/agent-skills/cli/godot-forge
```

## 版本管理

- 每个 Skill 和 CLI 独立版本
- Skill 版本在 SKILL.md 或 manifest.yaml 中声明
- CLI 版本在 pyproject.toml 中声明
- Skill 和 CLI 版本独立迭代，互不耦合
