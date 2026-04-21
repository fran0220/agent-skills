# Copilot Dispatch 典型工作流

## 工作流 1：Research → Plan → Iterate（推荐默认）

不直接 "implement X"，让 agent 先研究 + 出方案，你 review 后再让它继续。

### 阶段 1：派发 research-only 任务

```bash
gh agent-task create --repo OWNER/REPO -F - <<'EOF'
# [RESEARCH] 调研支付 webhook 签名验证问题

## Goal
**只做调研，不写代码**。产出一份报告 + 实施方案。

## 调研任务
1. 读 `src/payments/webhook.rs` 现有签名验证流程
2. 读 `src/config/mod.rs` 了解配置加载
3. 查 git log，找出 tolerance 从 5min 改 30s 的 commit
4. 对比 v2.2 与 v2.3 的行为差异

## 产出
在 PR description 里写：
- 现状分析（500 字内）
- 根因（必须有具体 commit hash）
- 三个候选方案，每个含：改动点、风险、估算时间
- 推荐方案 + 理由

## 边界
- ❌ 不要改任何代码
- ❌ 不要建新文件
- ✅ 只更新 PR description
EOF
```

### 阶段 2：你 review 方案，然后追加实施 prompt

PR 评论框里写（**用 Start a review 批量提交**，不要单条）：

```
@copilot 方案 B 通过。请按以下计划实施：

1. 加配置字段 `webhook.tolerance_seconds: u64`，默认 300
2. webhook.rs 读配置，传给 verify_signature
3. 加单元测试覆盖 3 个场景（见上方 Acceptance Criteria）
4. 跑 `cargo test --workspace` 全绿后 ready PR

不要改 default 行为以外的任何东西。
```

### 阶段 3：跟进直到 PR ready

```bash
# 跟踪日志
gh agent-task view <session-id> --follow

# CI 失败的话追加 fix
gh pr comment <pr-number> --body "@copilot CI 失败了，看一下 lint 错误"
```

---

## 工作流 2：Sub-issue 拆分（长任务必走）

单 issue 大于 4 小时或超 5 个文件改动 → **强制拆**。

### 拆分原则

- 每个 sub-issue **独立可 review**（一个逻辑单元）
- 有依赖时按 ordered 编号（subtask-01, subtask-02...）
- 同 base 分支的并行任务**不能改同一文件**

### 操作

```bash
# 1. 先建父 epic
parent_url=$(gh issue create \
  --title "Epic: 重构认证模块到 OAuth 2.1" \
  --body-file epic.md \
  --label "epic,agent")
parent_num=$(echo "$parent_url" | grep -oE '[0-9]+$')

# 2. 拆 5 个 sub-issue 并派发
for i in 01 02 03 04 05; do
  spec="subtasks/subtask-${i}.md"
  # 创建 sub-issue 关联父 issue
  sub_url=$(gh issue create \
    --title "[AGENT] subtask ${i}: $(head -1 "$spec" | sed 's/# //')" \
    --body "$(cat "$spec")\n\nPart of #${parent_num}" \
    --label "agent")

  # 派发到 Copilot
  gh agent-task create \
    --repo OWNER/REPO \
    -F "$spec" \
    --base "feature/auth-oauth21-${i}"

  sleep 2  # 避免 rate limit
done

# 3. 监控所有 session
watch -n 30 'gh agent-task list --json sessionId,status,title \
  --jq ".[] | select(.title | startswith(\"[AGENT] subtask\"))"'
```

### sub-issue 拆分模板

```markdown
# subtask-01: 提取 token 解析器到独立模块

## Parent
#1234 (Epic: OAuth 2.1 迁移)

## Depends on
None (第一个任务)

## Blocks
- subtask-02 (新 token 验证器)
- subtask-03 (refresh flow)

## Goal
把 `src/auth/legacy_token.rs` 中的 token 解析逻辑提取到独立 `src/auth/token_parser.rs`，
**不改变行为**。纯重构。

## Files
- 新增：`src/auth/token_parser.rs`
- 修改：`src/auth/legacy_token.rs`（改成调用新模块）
- 修改：`src/auth/mod.rs`（加 module 声明）

## Acceptance
- [ ] 所有现有测试通过（行为不变）
- [ ] 新模块 100% 覆盖率
- [ ] 旧模块行数减少 ≥ 30%

## Validation
cargo test -p auth
```

---

## 工作流 3：多任务并行（不同 repo / 不同子目录）

云端 session 天然隔离，可以无脑并发。

```bash
# 并行派发 3 个独立任务
gh agent-task create --repo org/web -F tasks/web-i18n.md &
gh agent-task create --repo org/api -F tasks/api-rate-limit.md &
gh agent-task create --repo org/worker -F tasks/worker-retry.md &
wait

# 列出全部进行中的
gh agent-task list --json sessionId,status,prUrl,title \
  --jq '.[] | select(.status=="running")'
```

### ⚠️ 同 repo 并行的约束

```bash
# ❌ 不要：同 base 同子目录
gh agent-task create --repo org/api -F task-a.md  # 改 src/users/
gh agent-task create --repo org/api -F task-b.md  # 也改 src/users/  → rebase 冲突

# ✅ 可以：同 base 不同子目录
gh agent-task create --repo org/api -F task-a.md  # 改 src/users/
gh agent-task create --repo org/api -F task-b.md  # 改 src/orders/

# ✅ 也可以：不同 base
gh agent-task create --repo org/api -F task-a.md --base develop
gh agent-task create --repo org/api -F task-b.md --base feature/xxx
```

---

## 工作流 4：迭代式追加 review comments

### ⚠️ 关键规则：永远 "Start a review"，不要 "Add single comment"

Copilot 看到一条 comment 就立刻开始干活。逐条 add → agent 被打断 N 次反复重启，又慢又乱。

### 正确做法

```bash
# 在 GitHub UI：
# 1. 点 Files changed
# 2. 在多处加 comment（点 +）
# 3. 全部点 "Start a review"，不是 "Add single comment"
# 4. 最后点 "Submit review" → agent 一次性收到所有反馈

# 或用 gh CLI 批量：
gh pr review <pr-number> --comment --body "$(cat << 'EOF'
@copilot 几处需要改：

1. src/auth.rs:42 — error 应该用 Result 不是 panic
2. src/auth.rs:88 — 这个 unwrap 不安全
3. tests/auth_test.rs — 缺少 timeout 场景

修完后 ping 我再 review。
EOF
)"
```

---

## 工作流 5：自动 CI 修复

```bash
# 派发任务后跟进
session_id=$(gh agent-task create --repo OWNER/REPO -F task.md --json sessionId -q .sessionId)
pr_url=$(gh agent-task view "$session_id" --json prUrl -q .prUrl)
pr_num=$(echo "$pr_url" | grep -oE '[0-9]+$')

# 等 PR ready
gh agent-task view "$session_id" --follow

# 检查 CI
while true; do
  status=$(gh pr checks "$pr_num" --json state -q '[.[] | .state] | unique')
  case "$status" in
    *FAILURE*)
      echo "CI failed, asking Copilot to fix..."
      gh pr comment "$pr_num" --body "@copilot CI 失败了：\n$(gh pr checks "$pr_num" --json name,state,link --jq '[.[] | select(.state==\"FAILURE\")]')"
      sleep 60  # 等 agent 处理
      ;;
    *SUCCESS*)
      echo "✅ CI green, ready to merge"
      break
      ;;
    *)
      echo "still running..."
      sleep 30
      ;;
  esac
done
```

---

## 工作流 6：紧急 stop & cleanup

`gh agent-task` 没有原生 kill。降级方案：

```bash
# 方案 A：关 PR（agent 会停止）
gh pr close <pr-number> --delete-branch

# 方案 B：在 PR 留 comment 让 agent 主动退出
gh pr comment <pr-number> --body "@copilot 取消这个任务，回退所有改动并关闭 PR"

# 方案 C：GraphQL 直接关 session（需要 PAT）
gh api graphql -f query='
  mutation($sessionId: ID!) {
    closeAgentSession(input: { sessionId: $sessionId }) {
      session { state }
    }
  }
' -F sessionId="$session_id"
```

---

## 工作流速查

| 场景 | 工作流 |
|------|--------|
| 任务复杂 / 不熟悉 codebase | Research → Plan → Iterate |
| 任务大 (>4h or >5 files) | Sub-issue 拆分 |
| 多个独立小任务 | 多任务并行 |
| Review PR 时多处要改 | Start a review 批量提交 |
| 全自动 ship | Sub-issue + 自动 CI 修复 + 人工 final review |
