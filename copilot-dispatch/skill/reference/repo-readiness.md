# 目标 Repo 就绪检查

派发任务前，目标仓库必须备齐 4 个文件。**否则长任务约 50% 概率失败。**

## 检查清单

```bash
# 在目标 repo 根目录跑
ls .github/copilot-instructions.md AGENTS.md 2>/dev/null  # 至少一个
ls .github/copilot-setup-steps.yml
ls .github/ISSUE_TEMPLATE/agent-task.yml
ls .github/agents/ 2>/dev/null  # 可选：custom agent personas
```

下面给出 4 个文件的最小可用模板。

---

## 1. `.github/copilot-instructions.md` 或 `AGENTS.md`

**作用**：repo 级别的 agent 操作手册。GitHub 分析 2500+ 仓库的结论：成功的 agent 文件命中**6 个核心区块**。

```markdown
# Repository Instructions for AI Agents

This is a <Language> based <project type> for <one-line purpose>.

## Commands (放最前面，agent 高频引用)

- **Build**: `make build`
- **Test**: `make test` (跑 unit + integration)
- **Lint**: `make lint` (golangci-lint, 必须无警告)
- **Format**: `make fmt` (commit 前必跑)
- **Full CI check**: `make ci` (本地复现 GitHub Actions)

## Tech Stack

- **Language**: Go 1.22
- **Framework**: gRPC + buf
- **Database**: PostgreSQL 15, sqlx
- **Test**: testify + dockertest

## Project Structure

- `cmd/` — 各服务的 main entry
- `internal/` — 业务逻辑（不可被外部 import）
- `pkg/` — 可被外部使用的工具
- `proto/` — protobuf 定义，改完跑 `make proto`
- `migrations/` — 数据库迁移，**不要手改，用 `make migrate-new NAME=xxx`**
- `tests/integration/` — 集成测试

## Code Style

```go
// ✅ Good — 有 context、有错误包装
func GetUser(ctx context.Context, id string) (*User, error) {
    if id == "" {
        return nil, fmt.Errorf("getuser: empty id")
    }
    user, err := db.Query(ctx, id)
    if err != nil {
        return nil, fmt.Errorf("getuser: query: %w", err)
    }
    return user, nil
}

// ❌ Bad — 无 context、错误吞掉
func GetUser(id string) *User {
    u, _ := db.Query(id)
    return u
}
```

## Naming

- 函数：camelCase 小写开头（除非要 export）
- 类型：PascalCase
- 常量：MixedCaps（不用 SCREAMING_SNAKE）
- 测试：`TestXxx_<scenario>_<expected>`

## Git Workflow

- 分支：`feature/<issue-number>-<short-desc>`
- Commit：conventional commits（`feat(api): ...`）
- 必须 rebase main 后再开 PR

## Boundaries

### ✅ Always do
- Commit 前跑 `make fmt && make lint && make test`
- 改 proto 后跑 `make proto` 重新生成代码
- 写新 endpoint 前先看 `internal/api/` 已有 pattern

### ⚠️ Ask first
- 改 `migrations/` —— 必须确认是新增不是修改历史
- 加新 dependency —— 必须有理由
- 改 `Makefile` / CI 配置

### 🚫 Never do
- 提交 secrets / API keys（即使是 placeholder）
- 改 `vendor/` / `go.sum`（用 `go mod tidy`）
- 删失败的测试（fix 它，或问人）
- 改 `internal/legacy/` 任何文件
```

---

## 2. `.github/copilot-setup-steps.yml`

**作用**：在 ephemeral GitHub Actions runner 启动后**预装依赖**。否则 agent 自己 trial-and-error 装，慢且经常失败。

**最被低估的提效杠杆。**

```yaml
name: "Copilot Setup Steps"

on:
  workflow_dispatch:

jobs:
  copilot-setup-steps:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Set up Go
        uses: actions/setup-go@v5
        with:
          go-version: "1.22"
          cache: true

      - name: Cache deps
        uses: actions/cache@v4
        with:
          path: |
            ~/go/pkg/mod
            ~/.cache/go-build
          key: ${{ runner.os }}-go-${{ hashFiles('**/go.sum') }}

      - name: Install deps
        run: go mod download

      - name: Install tooling
        run: |
          go install github.com/bufbuild/buf/cmd/buf@latest
          go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest

      - name: Start postgres for tests
        run: docker run -d -p 5432:5432 -e POSTGRES_PASSWORD=test postgres:15

      - name: Run migrations
        run: make migrate-up
```

> 💡 把"开发新人 setup 文档"翻译成 yml 即可。

---

## 3. `.github/ISSUE_TEMPLATE/agent-task.yml`

**作用**：强制 issue 走结构化模板，杜绝"修一下 webhook 的 bug"这种烂 prompt。

```yaml
name: Agent Task
description: 派发给 Copilot Coding Agent 的任务
title: "[AGENT] "
labels: ["agent", "needs-spec"]

body:
  - type: textarea
    id: context
    attributes:
      label: Context
      description: 业务背景，agent 不懂业务必须解释
      placeholder: |
        我们的支付服务在 v2.3 升级到 Stripe 后...
    validations:
      required: true

  - type: textarea
    id: goal
    attributes:
      label: Goal / Outcome
      description: 一句话目标，可观测变化
    validations:
      required: true

  - type: textarea
    id: scope
    attributes:
      label: Scope
      description: 必须明确 In / Out
      placeholder: |
        **In**:
        - ...
        **Out**:
        - ...
    validations:
      required: true

  - type: textarea
    id: acceptance
    attributes:
      label: Acceptance Criteria
      description: 写成 Given/When/Then，每条独立可验证
      placeholder: |
        - [ ] Given ..., When ..., Then ...
    validations:
      required: true

  - type: textarea
    id: files
    attributes:
      label: Files to Touch
      placeholder: |
        修改：src/...
        新增：tests/...
        禁止修改：vendor/, migrations/
    validations:
      required: true

  - type: textarea
    id: validation
    attributes:
      label: Validation Commands
      description: agent 在 PR ready 前必须跑通的命令
      placeholder: |
        ```bash
        make fmt lint test
        ```
    validations:
      required: true

  - type: textarea
    id: risk
    attributes:
      label: Risk & Rollback
      placeholder: |
        - Risk: ...
        - Mitigation: ...
        - Rollback: ...
```

---

## 4. `.github/agents/<name>.md`（可选 但强烈推荐）

**作用**：定义专家 persona，用 `gh agent-task create --custom-agent <name>` 调用。

GitHub 推荐的 6 个起手 agent：

| Agent | 职责 | 边界 |
|-------|------|------|
| `docs-agent` | 写文档 | 写 `docs/`，不碰 `src/` |
| `test-agent` | 写测试 | 写 `tests/`，**永不删失败测试** |
| `lint-agent` | 修 style | 只改格式，不动逻辑 |
| `api-agent` | 写 endpoint | 改路由，schema 改动必须先问 |
| `security-agent` | 安全扫描 | 只读 + 报告 |
| `dev-deploy-agent` | dev 环境部署 | 只能 dev，prod 必须人审 |

### 模板：`.github/agents/test-agent.md`

```markdown
---
name: test-agent
description: Writes unit and integration tests, never modifies source code
---

You are a senior QA engineer for this Go project.

## Your role
- Write unit tests in `*_test.go` next to source files
- Write integration tests in `tests/integration/`
- Use table-driven tests with subtests
- Aim for >80% line coverage on touched files

## Project knowledge
- Test framework: testify (`require` + `assert`)
- Mock framework: gomock (generated by `make mocks`)
- Integration: dockertest with postgres:15

## Commands you can use
- `make test` — 跑全部测试
- `make test-unit` — 只跑 unit
- `make test-integration` — 只跑 integration
- `make coverage` — 出覆盖率报告

## Test style example

```go
func TestGetUser(t *testing.T) {
    tests := []struct {
        name    string
        id      string
        want    *User
        wantErr error
    }{
        {"valid id", "u_123", &User{ID: "u_123"}, nil},
        {"empty id", "", nil, ErrInvalidID},
    }
    for _, tt := range tests {
        t.Run(tt.name, func(t *testing.T) {
            got, err := GetUser(context.Background(), tt.id)
            require.ErrorIs(t, err, tt.wantErr)
            assert.Equal(t, tt.want, got)
        })
    }
}
```

## Boundaries

- ✅ **Always do**：写新测试、改既有测试 setup、加 fixture
- ⚠️ **Ask first**：改 mock 接口（可能影响 source 实现）
- 🚫 **Never do**：
  - 修改 `src/` 任何文件
  - 删除失败的测试（fix 它，或在 PR 里说明为何 skip）
  - 改 CI 配置
  - 提 commit 前不跑 `make test`
```

---

## 自动化就绪检查脚本

考虑在派发前先跑：

```bash
# scripts/check-copilot-ready.sh
set -e
repo_root=$(git rev-parse --show-toplevel)

check() {
  if [ -e "$repo_root/$1" ]; then
    echo "✅ $1"
  else
    echo "❌ $1 — 缺失"
    return 1
  fi
}

check ".github/copilot-instructions.md" || check "AGENTS.md"
check ".github/copilot-setup-steps.yml"
check ".github/ISSUE_TEMPLATE/agent-task.yml"

echo "📋 Custom agents:"
ls "$repo_root/.github/agents/" 2>/dev/null || echo "  (none, 可选)"
```

不过 4 项 → 不要派发，先补齐。
