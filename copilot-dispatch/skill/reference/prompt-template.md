# Copilot 长任务 Prompt 完整模板

派发给 `gh agent-task create -F task-spec.md` 的标准结构。复制后逐节填空。

---

```markdown
# [AGENT] <一句话标题，动词开头>

## Context

<3-5 句业务背景：为什么要做、哪个用户场景、与什么系统相关。
不要假设 agent 知道 repo 业务，越具体越好。>

示例：
> 我们的支付服务在 v2.3 升级到 Stripe API 2024-06 后，
> webhook 签名验证经常误报失败，导致订单被错误回滚。
> 已知是 timestamp tolerance 从 5min 改成 30s 引起的。

## Goal / Outcome

<一句话目标。这个 PR 完成后，**用户/系统的可观测变化**是什么？>

示例：
> 把 webhook 签名验证的 tolerance 改回可配置（默认 5min），
> 并加单元测试覆盖三种 timestamp 场景。

## Scope

**In**:
- <明确包含的修改项 1>
- <明确包含的修改项 2>

**Out**:
- <明确不做的事 1>
- <明确不做的事 2>

> ⚠️ Out 比 In 更重要——防止 agent "顺手优化" 跑偏。

## Acceptance Criteria

写成 Given/When/Then，每条独立可验证：

- [ ] **Given** 配置 `webhook.tolerance_seconds=300`,
      **When** 收到 4min 前签名的 webhook,
      **Then** 验证通过，订单正常入库
- [ ] **Given** 配置 `webhook.tolerance_seconds=30`,
      **When** 收到 1min 前签名的 webhook,
      **Then** 返回 401，记录日志
- [ ] **Given** 配置缺失,
      **When** 收到任意 webhook,
      **Then** 默认 tolerance=300

## Files to Touch

**修改**:
- `src/payments/webhook.rs` — 加配置读取 + tolerance 参数
- `tests/integration/webhook_test.rs` — 加 3 个用例

**新增**:
- `docs/webhook-config.md` — 配置说明

**禁止修改**:
- `migrations/`
- `vendor/`
- `*.lock`
- 任何 `src/payments/legacy_*` 文件

## Tests Required

- [x] unit: tolerance 读取逻辑
- [x] integration: 三种 timestamp 场景
- [ ] e2e: 不需要

## Validation Commands

agent 必须在 PR ready 前跑通：

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
```

## Risk & Rollback

- **Risk**: 改错 tolerance 默认值会让生产签名验证全失败
- **Mitigation**: 默认值 300（与 v2.2 行为一致），通过 feature flag `WEBHOOK_TOLERANCE_V2` 控制
- **Rollback**: revert PR + 重启 service，无数据迁移

## Constraints

- 必须保持向后兼容：旧配置文件无 `webhook.tolerance_seconds` 字段时使用默认值
- 不引入新 crate 依赖
- 日志使用现有 `tracing::warn!` 格式
- 错误用现有 `WebhookError` enum，不新增变体

## References

- 上次类似改动：#1234
- 相关 issue：#5678
- Stripe 官方文档：https://stripe.com/docs/webhooks/signatures

---

## 执行流程（必须遵守）

1. **先 research**：
   - 读 `src/payments/webhook.rs` 了解现有签名验证流程
   - 读 `src/config/mod.rs` 了解配置加载约定
   - 读至少一个现有 test 了解测试风格
   - 在 PR description 顶部贴出调研结论

2. **再 plan**：
   - 在 PR description 写实施计划：改哪些函数、新增哪些测试、为什么
   - 计划含至少 3 个 commit 节点

3. **iterate on branch**：
   - 每个 commit 一个逻辑单元（配置读取 / tolerance 参数 / 测试 一一独立）
   - commit message 用 conventional commits（`fix(webhook): ...`）
   - 每个 commit 后跑 `cargo test`，全绿再下一步

4. **self-validate before ready**：
   - 跑 Validation Commands 全部命令
   - 在 PR description 贴出测试输出片段（pass count + 关键用例名）
   - 确认 Acceptance Criteria 每条都有对应测试

5. **PR description 必须包含**：
   - [ ] Research 结论
   - [ ] 实施计划
   - [ ] 测试输出截图
   - [ ] Acceptance Criteria checklist（已勾选）
   - [ ] Risk 评估更新（实现后是否有新发现的风险）
```

---

## 模板使用要点

| 节 | 长度 | 关键 |
|----|------|------|
| Context | 3-5 句 | 业务背景，agent 不懂业务 |
| Goal | 1 句 | 可观测变化 |
| Scope | 5-10 项 | **Out 比 In 重要** |
| Acceptance | 3-7 条 | Given/When/Then，**可验证** |
| Files | 全列 | 写明 **禁止修改** |
| Validation | 完整命令 | 不要省略 `--all-targets` 等 flag |
| Risk | 必填 | 防御性思维 |

## 反模式（不要这样写）

```markdown
❌ 修一下 webhook 的 bug
❌ 把 timeout 改成可配置
❌ 加点测试
❌ 重构 payment 模块让它更优雅
```

这种 prompt 会让 agent 自由发挥，长任务必翻车。

## 给 dispatch agent 自己的 checklist

派发前自检：

- [ ] Context 是否解释了"为什么"？
- [ ] Acceptance Criteria 每条都能用代码验证？
- [ ] Out 列表防止了 scope creep？
- [ ] Files 明确了 `禁止修改`？
- [ ] Validation Commands 复制粘贴可跑？
- [ ] 尾部"执行流程"有没有保留？
