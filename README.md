# Agent Skills

可复用的 Agent 技能（Skill）与命令行工具（CLI）集合。按项目组织的 monorepo。

## 项目列表

| 项目 | 组件 | 说明 |
|------|------|------|
| [`godot-forge`](./godot-forge/) | Skill + CLI | AI 驱动的 Godot 4.x 游戏开发 |
| [`asset-gateway`](./asset-gateway/) | Skill + CLI | 通用资产生成网关（图像/视频/音频/3D/文本） |
| [`cognee-admin`](./cognee-admin/) | CLI | Cognee 知识引擎管理（CLI + Web 面板） |
| [`bb-browser`](./bb-browser/) | Skill | 浏览器自动化 |
| [`jimeng-gateway`](./jimeng-gateway/) | Skill + CLI + Web | 即梦/Seedance 视频生成网关 |
| [`openclaw`](./openclaw/) | Skill | OpenClaw 网关最佳实践 |
| [`pi-sdk-practices`](./pi-sdk-practices/) | Skill | Pi Agent SDK 开发最佳实践 |
| [`ai-search`](./ai-search/) | Skill + CLI + npm | AI 驱动的 Web 搜索网关（多源 + MCP） |
| [`slide-deck`](./slide-deck/) | Skill | AI 幻灯片生成 |

## 结构

每个项目是一个顶级目录，内部按组件类型分：

```
<project>/
├── skill/    # Agent 技能（SKILL.md + reference/ + knowledge/）
├── cli/      # 命令行工具（TypeScript 或 Rust）
├── npm/      # 可选：TypeScript / npm 客户端包
└── README.md
```

## 使用

### 安装 Skill

```bash
# 符号链接到 Amp
ln -s /path/to/agent-skills/<project>/skill ~/.config/amp/skills/<project>
```

### 安装 CLI

```bash
# 如果项目提供 npm/ 客户端包
cd <project>/npm && npm install -g .

# Rust CLI
cd <project>/cli && cargo install --path .
```

## 开发

参见 [AGENTS.md](./AGENTS.md) 了解项目约定，或查看 [docs/](./docs/) 下的规范文档。
