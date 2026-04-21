# Copilot Dispatch

把长时间运行的编码任务派发到 **GitHub Copilot Coding Agent**（云端、异步、产出 draft PR）。

是 [`codex-dispatch`](https://github.com/.../codex-dispatch) 的兄弟工具——两者互补：
- **codex-dispatch**：本机短任务、未提交代码、日志输出
- **copilot-dispatch**：云端长任务、已 push 代码、PR 输出

## 项目结构

```
copilot-dispatch/
├── README.md                       # 本文件（人类读）
├── AGENTS.md                       # 子目录指令
└── skill/                          # Agent Skill
    ├── SKILL.md                    # Agent 入口（< 500 行）
    └── reference/                  # 按需加载
        ├── prompt-template.md      # 长任务 prompt 完整模板
        ├── repo-readiness.md       # 目标 repo 4 件套（AGENTS.md / setup-steps / issue template / agent personas）
        ├── workflows.md            # 6 大典型工作流
        └── decision-tree.md        # Copilot vs Codex vs Amp 决策树
```

## 核心理念

不引入 MCP，**直接封装 `gh agent-task` CLI**（GitHub CLI ≥ 2.80 内置）：

| Amp action | gh 命令 |
|------------|---------|
| dispatch | `gh agent-task create -F spec.md --repo OWNER/REPO` |
| list | `gh agent-task list --json sessionId,status,prUrl` |
| status | `gh agent-task view <session-id> [--follow]` |
| kill | （无原生）→ `gh pr close` 或 GraphQL |

## 安装

### 1. 升级 gh CLI 到 ≥ 2.80

```bash
brew upgrade gh
gh --version    # 必须 ≥ 2.80
```

### 2. 配置 PAT（Personal Access Token）

⚠️ Copilot Coding Agent **不能用 GitHub App**（按用户计费），必须用 PAT：

```bash
gh auth login --scopes "repo,workflow,actions"
```

PAT 必须有：`actions` / `contents` / `issues` / `pull-requests` 全部 read & write。

### 3. 安装 Skill

```bash
ln -sf "$(pwd)/skill" ~/.config/amp/skills/copilot-dispatch
```

### 4. 验证

下次 Amp 会话里说"派发到 copilot"或"dispatch to copilot"，应自动加载本 skill。

## 快速上手

```bash
# 1. 检查目标 repo 是否就绪
ls TARGET_REPO/.github/copilot-instructions.md \
   TARGET_REPO/.github/copilot-setup-steps.yml \
   TARGET_REPO/.github/ISSUE_TEMPLATE/agent-task.yml

# 缺啥？看 skill/reference/repo-readiness.md 补齐

# 2. 写结构化 prompt（用 skill/reference/prompt-template.md 套模板）
cat > task.md << 'EOF'
# [AGENT] 给 webhook 模块加 tolerance 配置

## Context ...
## Goal ...
## Acceptance Criteria ...
EOF

# 3. 派发
gh agent-task create -F task.md --repo OWNER/REPO

# 4. 跟踪
gh agent-task view <session-id> --follow
```

## 长任务成功要素（最被低估）

按重要性排序：

1. **目标 repo 有 `copilot-setup-steps.yml`** —— 不然 agent 在 trial-and-error 装依赖
2. **prompt 用 Given/When/Then 写 acceptance criteria** —— 不是"修一下 bug"
3. **大任务必拆 sub-issues** —— 单 session > 4h 会超时被关
4. **明确 "禁止修改" 列表** —— 防止 scope creep
5. **AGENTS.md 里写 Boundaries（✅/⚠️/🚫 三档）** —— GitHub 分析 2500+ 仓库的结论

详见 `skill/reference/`。

## 与 codex-dispatch 的关系

两者**共存不替代**。决策树见 `skill/reference/decision-tree.md`，简记：

> 长 + 云端 + 要 PR → copilot-dispatch
> 短 + 本机 + 要日志 → codex-dispatch
> 边想边改 + 跨文件推理 → Amp 自己干

## 参考文档

- [GitHub Copilot Coding Agent docs](https://docs.github.com/copilot/concepts/agents/coding-agent)
- [gh agent-task manual](https://cli.github.com/manual/gh_agent-task)
- [How to write a great agents.md (GitHub Blog)](https://github.blog/ai-and-ml/github-copilot/how-to-write-a-great-agents-md-lessons-from-over-2500-repositories/)
- [Best practices for using Copilot to work on tasks](https://docs.github.com/copilot/how-tos/agents/copilot-coding-agent/best-practices-for-using-copilot-to-work-on-tasks)
