# cognee-admin Skill

Agent-facing usage guide for the `cognee-admin` project.

This skill explains how to use the `cognee-admin` CLI（npm 轻量客户端 + Rust 完整版）to:

- add data into Cognee datasets
- run `cognify` with custom prompts
- search across one or more datasets (including GameDB game design knowledge)
- manage ontology uploads
- configure Cognee's LLM settings
- operate the web dashboard and admin tokens

## File Structure

```
skill/
├── SKILL.md              # Agent 入口（工作流、命令速查、反模式）
├── README.md             # 本文件
└── reference/
    ├── npm-cli.md        # npm CLI 完整命令参考
    └── datasets.md       # 数据集详情（game 知识库结构、搜索示例、导入 SOP）
```

Primary entrypoint: [SKILL.md](./SKILL.md)

The skill follows the monorepo convention of keeping executable logic in [`../cli/`](../cli/) / [`../npm/`](../npm/) and usage guidance in `skill/`.
