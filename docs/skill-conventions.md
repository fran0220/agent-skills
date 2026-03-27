# Skill 开发规范

## 通用规范

- 每个 Skill 位于 `<project>/skill/` 目录下
- 必须有 `SKILL.md`（Agent 入口）和 `README.md`（人类文档）
- SKILL.md 控制在 500 行以内，大量内容放 `reference/` 和 `knowledge/` 按需加载
- reference/ 放规范性文档（格式规范、API 参考、SOP 流程）
- knowledge/ 放领域知识（设计模式、最佳实践、反模式）
- 不在 Skill 目录中放可执行代码，CLI 放同项目的 `cli/` 目录

## 质量检查

- SKILL.md 的 YAML frontmatter 必须有 `name` 和 `description`
- `name` 必须与项目名一致
- `description` 必须说明触发条件（什么时候该加载这个 Skill）
- reference/ 和 knowledge/ 中的文件必须在 SKILL.md 中被引用，不留孤立文件

## 与 CLI 的协作

- Skill 通过 `Prerequisites` 声明依赖的 CLI
- Skill 通过 `Workflow` 指导 Agent 如何调用 CLI
- Skill 不假设 CLI 的实现细节，只依赖 CLI 的公开命令接口
