# 派发决策树：Copilot vs Codex vs Amp

## 核心问题：这个任务该交给谁？

```
任务来了
    │
    ▼
任务时长 > 30 分钟 ?
    │
    ├── 是 ──► 需要本机环境（DB / 私有服务 / 未提交代码）?
    │           │
    │           ├── 是 ──► 【Codex】（本机长跑）
    │           └── 否 ──► 仓库可信公开 / 已 push?
    │                       │
    │                       ├── 是 ──► 【Copilot】（云端长跑 + PR）  ⭐
    │                       └── 否 ──► 【Codex】
    │
    └── 否 ──► 需要边想边改 / 跨文件深度推理?
                │
                ├── 是 ──► 【Amp】（自己干）
                └── 否 ──► 任务明确有 spec?
                            │
                            ├── 是 ──► 【Codex】（短任务快出结果）
                            └── 否 ──► 【Amp】（先探索）
```

## 决策矩阵

| 维度 | Copilot Coding Agent | Codex CLI | Amp 自己干 |
|------|---------------------|-----------|-----------|
| **任务时长** | 分钟～小时 ✅ | 秒～分钟 ✅ | 实时 ✅ |
| **超长任务（>1h）** | ✅ 天然支持 | ⚠️ 占本机资源 | ❌ |
| **并行度** | ✅ 云端隔离，N 个 | ⚠️ 本机资源限 | ❌ 单线程 |
| **可审计性** | ✅✅ PR + commit + CI | ⚠️ 仅日志 | ⚠️ 仅日志 |
| **私有/敏感代码** | ⚠️ Actions 沙箱 | ✅ 全本地 | ✅ 全本地 |
| **未提交 worktree** | ❌ 必须先 push | ✅ 直接吃 | ✅ 直接吃 |
| **本地服务依赖** | ❌ 只能 Actions 装 | ✅ 完整本机 | ✅ 完整本机 |
| **冷启动延迟** | ⚠️ 30s～几分钟 | ✅ 秒级 | ✅ 立即 |
| **小改快速试错** | ❌ 每次开 PR | ✅ | ✅✅ |
| **跨文件深度推理** | ⚠️ 取决于 prompt | ⚠️ | ✅ Oracle/Librarian |
| **离线** | ❌ | ✅ | ✅ |
| **计费** | Copilot 订阅 + Actions 分钟 | API token | API token |
| **PR / commit 产出** | ✅ 必出 | ❌ 需要再 commit | ❌ 需要再 commit |

## 典型场景判断

### 派 Copilot

- ✅ "为认证模块加完整的单元测试覆盖"（明确目标 + 长任务 + 必出 PR）
- ✅ "把 deprecated API 调用全部迁到新版本"（机械 + 大量 + 可审计）
- ✅ "给 docs/ 下所有 markdown 文件加 frontmatter"（批量 + 简单）
- ✅ "依赖升级 + 修 break"（长 + 标准化）
- ✅ "实现这个明确 spec 的新 endpoint"（spec 已写、有 acceptance）
- ✅ "为这 5 个 sub-issue 各开一个 PR"（并行）

### 派 Codex

- ✅ "在本地 DB 里跑这个数据修复脚本"（本机环境）
- ✅ "改一下这个还没 commit 的实验分支"（worktree）
- ✅ "给我快速实现一个 prototype"（试错）
- ✅ "敏感仓库里的内部工具改造"（不能上 GitHub Actions）
- ✅ "需要本机 GPU 跑一段验证"（资源）

### Amp 自己干

- ✅ "这段代码有 bug，跨 5 个文件，帮我找根因"（推理 + Oracle）
- ✅ "调研一下这个库怎么集成"（探索 + 边问边定）
- ✅ "重构方案讨论 + 选型"（对话）
- ✅ "review 这个 PR 给意见"（一次性分析）
- ✅ "看下这个 thread 之前怎么决策的"（find_thread）

## 边界情况

### 想用 Copilot 但 repo 没就绪？

→ **先派 Copilot 一个 readiness 任务**，让它给自己加 `AGENTS.md` + `copilot-setup-steps.yml`，
然后再派真任务。

```bash
gh agent-task create --repo OWNER/REPO -F - <<'EOF'
# [READINESS] 准备 Copilot Coding Agent 工作环境

## Goal
为本仓库添加标准的 Copilot agent 工作环境文件。

## Files to Create
1. `AGENTS.md` —— 包含 6 个核心区块（commands, structure, stack, code style, git workflow, boundaries）
2. `.github/copilot-setup-steps.yml` —— 预装本仓库依赖
3. `.github/ISSUE_TEMPLATE/agent-task.yml` —— 任务派发模板

## Discovery
- 读 README.md, Makefile, package.json/Cargo.toml 等推断 stack
- 读 .github/workflows/ 了解 CI 命令
- 读 CONTRIBUTING.md 了解 git workflow

## Boundaries
- 不要改任何 src/ 代码
- 不要加新 dependency

## Validation
- AGENTS.md 必须 ≤ 200 行
- copilot-setup-steps.yml 必须能跑通（用 act 或描述）
EOF
```

### Codex 跑一半发现是长任务？

→ Codex `kill` 当前任务，把 prompt 改造成结构化 spec，重新派 Copilot。

### Copilot session 超时？

→ 立刻拆 sub-issues。**不要重试同一个 prompt**。

### 不知道选谁？

默认走 **Amp 自己干**，跑一会儿心里有数后再派出去。比派错强。

## 反模式

- ❌ **把 Codex 的 prompt 直接喂 Copilot**：Codex 吃日志反馈，Copilot 吃 PR 验收，结构完全不同
- ❌ **派 Copilot 处理未 push 的代码**：必然失败
- ❌ **派 Copilot 做探索性任务**：会得到看似漂亮但跑偏的 PR
- ❌ **派 Codex 跑 4 小时长任务**：占本机资源，关机即死
- ❌ **Amp 自己跑批量重构**：context window 撑不住
