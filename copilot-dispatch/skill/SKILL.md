---
name: copilot-dispatch
description: "Dispatches long-running coding tasks to GitHub Copilot Coding Agent (cloud, async, opens draft PR). Use when the user wants to hand off implementation work to Copilot, run background tasks that take minutes to hours, or delegate autonomous coding that produces a reviewable pull request. Triggers on: dispatch to copilot, send to copilot, copilot 执行, 派发到 copilot, gh agent-task."
---

# Copilot Dispatch — 派发长任务到 GitHub Copilot Coding Agent

## 何时使用

| 派发到 Copilot | 派发到 Codex | Amp 自己干 |
|---------------|-------------|-----------|
| 长任务（分钟～小时） | 短任务、本机环境依赖 | 探索 / 跨文件深度推理 |
| 必须有可 review 的 PR | 输出日志即可 | 边想边改 |
| 公开/可信仓库 | 敏感本地代码 | — |
| 已 commit 推送的代码 | 当前 worktree（含未提交） | — |

详见 `reference/decision-tree.md`。

## 前置依赖

1. **gh CLI ≥ 2.80**（`agent-task` 子命令）
   ```
   gh --version    # 不到 2.80 → brew upgrade gh
   ```

2. **PAT 权限**（不能用 GitHub App，Copilot 按用户计费）：
   - `actions` / `contents` / `issues` / `pull-requests` 全部 read & write
   - 通过 `gh auth login --scopes "repo,workflow"` 配置

3. **目标仓库就绪检查**（首次派发前必做，详见 `reference/repo-readiness.md`）：
   - [ ] `.github/copilot-instructions.md` 或 `AGENTS.md` 存在
   - [ ] `.github/copilot-setup-steps.yml` 预装依赖
   - [ ] `.github/ISSUE_TEMPLATE/agent-task.yml` 强制结构化
   - [ ] CI 跑通（agent 会用 CI 自验证）

> ⚠️ **跳过就绪检查 = 长任务 50% 概率失败**（agent 装依赖失败、风格不符、改了不该改的文件）。

## 核心命令

直接用 `gh agent-task`，不需要 MCP 包装。

### 派发任务（非阻塞）

```bash
# 内联 prompt
gh agent-task create "<task description>" --repo OWNER/REPO

# 从文件喂 prompt（推荐，避免 shell 转义）
gh agent-task create -F task-spec.md --repo OWNER/REPO

# 走自定义 agent persona（.github/agents/<name>.md）
gh agent-task create -F task-spec.md --custom-agent test-agent --repo OWNER/REPO

# 指定 base 分支
gh agent-task create -F task-spec.md --base develop --repo OWNER/REPO
```

返回 **session-id** + **PR URL**，立即返回，云端继续跑。

### 查看状态 / 拉日志

```bash
# 列出最近任务（默认你的）
gh agent-task list --json sessionId,status,prUrl,title

# 查看单个 session（接受 session-id / pr-number / pr-url）
gh agent-task view <session-id> --json status,prUrl,createdAt,completedAt

# 跟踪日志（阻塞，类似 tail -f）
gh agent-task view <session-id> --follow
```

### 终止任务

`gh agent-task` 暂无 `kill` 子命令。两种 workaround：
- **关 PR**：`gh pr close <pr-number>`，agent 会停止
- **GraphQL**：mutation `closeAgentSession`（需要自建脚本）

## Prompt 构建规则（长任务关键）

派发给 Copilot 的 prompt 写法**不同于 codex**：Copilot 把 prompt 当成 GitHub issue 处理，必须**结构化 + 验收标准 + 边界**。

完整模板见 `reference/prompt-template.md`，最小骨架：

```markdown
## Context
[业务背景、为什么要做]

## Goal / Outcome
[一句话目标]

## Scope
- In: [包含什么]
- Out: [明确不做什么]

## Acceptance Criteria
- Given <前置>, When <操作>, Then <预期>
- Given ..., When ..., Then ...

## Tests Required
- [ ] unit
- [ ] e2e
- [ ] contract

## Files
- 改 / 加：`src/...`, `tests/...`
- 不要碰：`vendor/`, `migrations/`, `*.lock`

## Validation
- 跑 `make test` 必须全绿
- 跑 `make lint` 必须无警告

## Risk & Rollback
[失败时如何 revert]
```

**尾部固定指令**：

```markdown
---
请按以下流程执行：
1. **先 research**：扫一遍相关文件，列出依赖关系
2. **再 plan**：在 PR description 里写实施计划（改哪些文件、为什么）
3. **iterate on branch**：分多个 commit 逐步实现，每个 commit 一个逻辑单元
4. **self-validate**：每个 commit 后跑 `make test`，全绿再继续
5. **PR ready 前**：在 description 里贴出测试输出、覆盖率变化、风险评估
```

## 长任务的拆分策略

**单 issue > 4 小时 / > 5 个文件改动 → 必须拆 sub-issues**（社区共识，session 长会超时被关）。

```bash
# 创建父 issue
parent=$(gh issue create --title "Epic: 重构认证模块" --body "$(cat epic.md)" --json number -q .number)

# 创建 sub-issues + 派发（每个一个 PR）
for spec in subtasks/*.md; do
  gh agent-task create -F "$spec" --repo OWNER/REPO
done

# 跟踪所有 session
gh agent-task list --json sessionId,status,title --jq '.[] | select(.status=="running")'
```

详见 `reference/workflows.md` 的 "research-plan-iterate" 与 "sub-issue 拆分" 章节。

## 多任务并行

Copilot 云端 session 天然隔离（各自独立 branch），可以无脑并发：

```bash
gh agent-task create -F task-a.md --repo org/repo-a &
gh agent-task create -F task-b.md --repo org/repo-b &
gh agent-task create -F task-c.md --repo org/repo-a --base feature/x &
wait
gh agent-task list --json sessionId,status
```

> ⚠️ **同 repo 同 base** 的并行任务会互相 rebase 冲突，要么改不同子目录，要么用不同 base 分支。

## 适用场景对照

**适合派发**：
- ✅ 明确的功能实现（带验收标准）
- ✅ 测试覆盖率提升（"为 X 模块加 N 个测试"）
- ✅ 文档生成 / API 文档同步
- ✅ 依赖升级 + 修复 break
- ✅ 批量重构（lint 修复、import 整理、命名规范化）
- ✅ Bug 修复（issue 描述清晰）

**不适合派发**：
- ❌ 探索性 spike（不知道要改啥）
- ❌ 涉及未提交代码或本地服务
- ❌ 敏感仓库（PAT 必须有 repo 写权限）
- ❌ 需要 Amp 独有工具（Oracle、Librarian、find_thread）
- ❌ 需要实时人机对话调整方向

## 失败排查

- **session 立刻 fail**：检查 PAT 权限是否包含 `actions`
- **"agent encountered an error"**：基本是 PAT 用了 GitHub App token；改用 user PAT
- **跑很久没动静**：缺 `copilot-setup-steps.yml`，agent 在 trial-and-error 装依赖
- **PR 改了一堆不该改的**：`AGENTS.md` 里 Boundaries 写得不够清楚
- **session 超时**：单任务太大，回去拆 sub-issues

## 进阶能力

- **自定义 agent persona**：`.github/agents/<name>.md` 定义专家角色（test / docs / lint / security），用 `--custom-agent` 调用。模板见 `reference/repo-readiness.md`。
- **MCP 扩展**：默认开 GitHub MCP + Playwright MCP，可在 cloud-agent 设置里挂自己的 MCP 让 agent 访问内部 API/DB。
- **自定义 firewall / dev env**：在仓库 Settings → Copilot → coding agent 配置出网白名单和 runner 镜像。
- **Hooks**：`pre-commit` / `pre-push` 在关键节点拦截 agent 的危险操作。

详见 [GitHub Copilot Coding Agent docs](https://docs.github.com/copilot/concepts/agents/coding-agent)。

## 与 codex-dispatch 的关系

两者**互补不替代**。决策树详见 `reference/decision-tree.md`。简记：

> 长 + 云端 + 要 PR → copilot-dispatch
> 短 + 本机 + 要日志 → codex-dispatch
> 边想边改 + 跨文件推理 → Amp 自己干
