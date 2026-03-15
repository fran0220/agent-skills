# agent-skills — Agent 指令

## ⚠️ 项目性质

本仓库是一个 **Skill 和 CLI 的开发项目**——我们在这里**编写、测试、发布** Skill 和 CLI 产出物。

**不要混淆**：
- 本仓库的 `skills/` 目录是**我们正在开发的 Skill 源码**，不是当前 Agent 环境已安装的 Skill
- 本仓库的 `cli/` 目录是**我们正在开发的 CLI 源码**，不是当前系统已安装的工具
- 修改这里的文件不会影响本地 Agent 环境，除非显式通过符号链接或 npm install 安装

简言之：**这里是工厂，不是车间。我们在造工具，不是在用工具。**

## 仓库结构约定

本仓库是 **Skill + CLI monorepo**，顶层目录：

```
agent-skills/
├── skills/        # 所有 Skill（知识层，产出物）
├── cli/           # 所有 CLI（能力层，产出物）
└── .agent/skill/  # 开发用 Skill（辅助开发的第三方 Skill，不属于产出物）
```

### Skill 与 CLI 的关系

- **Skill = 纯知识**：SKILL.md + reference/ + knowledge/，不含可执行代码
- **CLI = 纯工具**：独立 TypeScript 包，通过 npm 安装
- **分离原则**：Skill 告诉 Agent 该做什么 → Agent 调 CLI 执行
- **多对多**：一个 Skill 可以依赖多个 CLI，一个 CLI 可以被多个 Skill 使用

### AGENTS.md 层级体系

```
AGENTS.md                          # 本文件 — 仓库总览与全局约定
├── skills/AGENTS.md               # Skill 开发通用规范
│   ├── skills/godotforge/AGENTS.md
│   ├── skills/openclaw-best-practices/AGENTS.md
│   ├── skills/pi-agent-sdk/AGENTS.md
│   └── skills/slide-deck/AGENTS.md
└── cli/AGENTS.md                  # CLI 开发通用规范
    └── cli/godot-forge/AGENTS.md
```

子 AGENTS.md 继承上层约定，仅描述该目录的**特定规则**，不重复全局约定。

## 新建 Skill 约定

每个 Skill 是 `skills/` 下的一个目录，必须包含：

```
skills/<skill-name>/
├── SKILL.md          # 必须 — Agent 入口（YAML frontmatter + 指令）
├── README.md         # 必须 — 人类文档
├── AGENTS.md         # 推荐 — 该 Skill 的开发指令
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

每个 CLI 是 `cli/` 下的一个独立 TypeScript 包：

```
cli/<cli-name>/
├── package.json      # 必须 — 包配置 + 依赖 + bin
├── tsconfig.json     # 必须 — TypeScript 配置
├── BLUEPRINT.md      # 推荐 — CLI 设计蓝图（cli-design-framework 产出）
├── AGENTS.md         # 推荐 — 该 CLI 的开发指令
├── README.md         # 必须 — 使用说明
├── src/              # 必须 — TypeScript 源码
│   ├── index.ts      # CLI 入口
│   ├── commands/     # 子命令实现
│   ├── core/         # 核心逻辑
│   └── utils/        # 工具函数
└── tests/            # 必须 — 测试
```

### CLI 设计原则

1. **JSON 默认输出**：Agent-Primary CLI 以 JSON 为默认输出，`--human` 为可选人类可读格式
2. **退出码规范**：0 = 成功，1 = 命令错误，2 = 系统错误
3. **幂等性**：相同输入 + 相同状态 = 相同输出
4. **无交互模式**：Agent 调用时不能有 prompt，所有参数通过命令行或 stdin JSON 传入
5. **错误信息可操作**：报错要说明怎么修复，不只是"失败了"
6. **Schema 自省**：提供 `describe` 命令，让 Agent 可发现命令结构和输入格式

### 命名规范

- 目录名：小写 + 连字符（`godot-forge`）
- npm 包名：与目录名一致（`godot-forge`）
- CLI 命令名：与目录名一致（`godot-forge`）

## Skill 如何引用 CLI

在 SKILL.md 的 Prerequisites 中声明依赖：

```markdown
## Prerequisites

1. **godot-forge CLI**: `npm install -g @doufunao123/godot-forge`
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

# CLI 通过 npm 安装
npm install -g /path/to/agent-skills/cli/godot-forge
```

## 开发用 Skill（.agent/skill/）

`.agent/skill/` 存放辅助开发的第三方 Skill，**不属于本仓库产出物**，仅供 Agent 在开发过程中参考。

当前已安装：

| Skill | 用途 | 来源 |
|-------|------|------|
| `cli-design-framework` | CLI 设计与分类框架，设计新 CLI 或审查现有 CLI 时使用 | [Wangnov/cli-design-framework](https://github.com/Wangnov/cli-design-framework) |

### Skill 自动加载规则

- **设计/新建 CLI**：涉及 CLI 设计、分类、命令结构决策时，加载 `cli-design-framework` skill

## 版本管理

- 每个 Skill 和 CLI 独立版本
- Skill 版本在 SKILL.md 或 manifest.yaml 中声明
- CLI 版本在 package.json 中声明
- Skill 和 CLI 版本独立迭代，互不耦合
