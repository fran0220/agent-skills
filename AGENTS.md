# agent-skills — Agent 指令

## ⚠️ 项目性质

本仓库是一个 **Agent 工具链 monorepo**——我们在这里**编写、测试、发布** Skill 和 CLI。

**不要混淆**：
- 本仓库的项目目录是**我们正在开发的产出物源码**，不是当前 Agent 环境已安装的工具
- 修改这里的文件不会影响本地 Agent 环境，除非显式通过符号链接或安装

简言之：**这里是工厂，不是车间。我们在造工具，不是在用工具。**

## 仓库结构约定

本仓库按**项目**组织，每个顶级目录是一个独立项目：

```
agent-skills/
├── godot-forge/           # GodotForge — AI 驱动的 Godot 游戏开发
│   ├── skill/             #   Agent 技能
│   └── cli/               #   TypeScript CLI（52 个命令）
├── asset-gateway/         # Asset Gateway — 通用资产生成网关
│   ├── skill/             #   Agent 技能
│   └── cli/               #   Rust CLI + 网关服务
├── cognee-admin/          # Cognee Admin — 知识引擎管理
│   └── cli/               #   Rust CLI + HTMX Web 面板
├── bb-browser/            # BB Browser — 浏览器自动化
│   └── skill/             #   Agent 技能
├── jimeng-gateway/        # 即梦 Gateway — AI 图像/视频网关
│   └── skill/             #   Agent 技能
├── openclaw/              # OpenClaw — 网关最佳实践
│   └── skill/             #   知识库
├── pi-sdk-practices/      # Pi SDK Practices — SDK 开发最佳实践
│   └── skill/             #   知识库
├── slide-deck/            # Slide Deck — AI 幻灯片生成
│   └── skill/             #   Agent 技能
├── vercel-react-best-practices/  # React/Next.js 性能优化
│   └── skill/
├── better-auth-best-practices/   # Better Auth 认证集成
│   └── skill/
├── create-auth-skill/     # 应用认证服务创建
│   └── skill/
│
├── docs/                  # 跨项目开发规范
│   ├── skill-conventions.md
│   └── cli-conventions.md
├── .agent/skill/          # 开发用 Skill（辅助工具，非产出物）
├── AGENTS.md              # 本文件
└── README.md
```

### 项目内部结构

每个项目目录可包含以下组件（按需）：

| 子目录 | 用途 | 典型内容 |
|--------|------|---------|
| `skill/` | Agent 技能 | SKILL.md + reference/ + knowledge/ |
| `cli/` | 命令行工具 | package.json/Cargo.toml + src/ + tests/ |
| `web/` | Web 前端 | （未来扩展） |
| `api/` | API 服务 | （未来扩展） |

### Skill 与 CLI 的关系

- **Skill = 纯知识**：SKILL.md + reference/ + knowledge/，不含可执行代码
- **CLI = 纯工具**：独立包（TypeScript 或 Rust），通过 npm 或 cargo 安装
- **分离原则**：Skill 告诉 Agent 该做什么 → Agent 调 CLI 执行
- **同项目共存**：一个项目的 Skill 和 CLI 在同一顶级目录下

## 新建项目约定

```bash
mkdir <project-name>
# 按需添加组件：
mkdir <project-name>/skill   # 如果有 Agent 技能
mkdir <project-name>/cli     # 如果有 CLI
```

每个项目必须有 `README.md`，推荐有 `AGENTS.md`。

### Skill 组件规范

详见 [docs/skill-conventions.md](./docs/skill-conventions.md)。

### CLI 组件规范

详见 [docs/cli-conventions.md](./docs/cli-conventions.md)。

## AGENTS.md 层级体系

```
AGENTS.md                              # 本文件 — 仓库总览
├── godot-forge/
│   ├── skill/AGENTS.md                # Skill 开发指令
│   └── cli/AGENTS.md                  # CLI 开发指令
├── asset-gateway/
│   └── cli/AGENTS.md
├── cognee-admin/
│   └── cli/AGENTS.md
├── slide-deck/
│   └── skill/AGENTS.md
└── docs/
    ├── skill-conventions.md           # Skill 通用规范
    └── cli-conventions.md             # CLI 通用规范
```

子 AGENTS.md 继承上层约定，仅描述该目录的**特定规则**。

## 符号链接约定

Skill 通过符号链接安装：

```bash
ln -s /path/to/agent-skills/<project>/skill ~/.config/amp/skills/<project>
```

CLI 通过包管理器安装：

```bash
# TypeScript CLI
cd <project>/cli && npm install -g .

# Rust CLI
cd <project>/cli && cargo install --path .
```

## 开发用 Skill（.agent/skill/）

`.agent/skill/` 存放辅助开发的第三方 Skill，**不属于本仓库产出物**。

| Skill | 用途 |
|-------|------|
| `cli-design-framework` | CLI 设计与分类框架 |

### Skill 自动加载规则

- **设计/新建 CLI**：加载 `cli-design-framework` skill

## 命名规范

- 项目目录名：小写 + 连字符（`my-project`）
- Skill name 字段：与项目目录名一致
- CLI 包名/命令名：与项目目录名一致

## 版本管理

- 每个项目独立版本
- Skill 版本在 SKILL.md 中声明
- CLI 版本在 package.json/Cargo.toml 中声明
