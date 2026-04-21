# copilot-dispatch — 项目指令

## 项目性质

本项目是一个 **Skill-only 项目**（无 CLI 组件）。

- Skill 直接封装 `gh agent-task` CLI，**不需要单独 CLI 包**
- 不需要 MCP 服务器
- `skill/SKILL.md` 是 Agent 入口，必须保持 < 500 行
- 详细内容拆到 `skill/reference/` 按需加载

## 维护原则

1. **跟踪上游 gh CLI 变化**：`gh agent-task` 子命令在 preview 阶段，新版本可能改变 flag
2. **跟踪 GitHub Copilot Coding Agent 更新**：定期 review GitHub Blog changelog
3. **Skill 描述要触发明确**：`description` 字段必须包含触发短语（`派发到 copilot`、`dispatch to copilot`）

## 修改流程

修改 `skill/SKILL.md` 后：

1. 检查行数：`wc -l skill/SKILL.md` 应 < 500
2. 检查孤立文件：`reference/` 下每个文件必须在 SKILL.md 中被引用
3. 检查 frontmatter：`name` 必须等于项目名 `copilot-dispatch`

## 与 codex-dispatch 的协作

两个 dispatch skill 是**对偶关系**：

| 决策点 | codex-dispatch | copilot-dispatch |
|--------|---------------|------------------|
| 任务时长 | 短 | 长 |
| 执行环境 | 本机 | 云端 GitHub Actions |
| 输入约束 | 当前 worktree | 已 push 代码 |
| 产出物 | 日志 | draft PR |
| 工具入口 | `tb__codex-dispatch` MCP | `gh agent-task` CLI |

修改任一方时，要同步更新 `skill/reference/decision-tree.md`，避免决策歧义。

## 不要做

- ❌ 不要把 codex-dispatch 的 prompt 模板直接搬过来——两边 prompt 风格完全不同
- ❌ 不要假设 GitHub Coding Agent API 会稳定——这是 preview，要在 SKILL.md 里说清楚
- ❌ 不要在 skill 里硬编码具体仓库名 / 用户名
- ❌ 不要为这个 skill 加 MCP 服务器——`gh` CLI 已经够用，加 MCP 是过度设计
