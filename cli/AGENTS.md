# cli/ — CLI 开发指令

## 技术栈

- **语言**: TypeScript
- **CLI 框架**: Commander.js（轻量）或 oclif（重量级，需插件系统时）
- **测试**: Vitest
- **构建**: tsup / esbuild（单文件打包，快速启动）
- **分发**: npm（`npm install -g` / `npx`）

## 开发流程

1. **先设计再编码** — 使用 `cli-design-framework` skill 产出 BLUEPRINT.md
2. **先骨架再填充** — `package.json` + `src/index.ts` + 一个可运行的子命令
3. **先核心再扩展** — 遵守 BLUEPRINT.md 中的 v1 边界

## Agent-Primary CLI 约定

本仓库的 CLI 主要面向 AI Agent 调用，遵循以下约定：

### 输出

- **JSON 默认**：stdout 输出 JSON，统一信封 `{"ok": bool, "command": str, "data": {} | "error": {}}`
- **`--human`**：可选的人类可读格式（调试用，非契约）
- **`--fields`**：字段选择，控制输出体积（Agent 上下文窗口纪律）
- **stderr**：日志/进度信息输出到 stderr，不污染 JSON stdout

### 输入

- **Raw-payload-first**：主路径通过 stdin JSON 或 `--input file.json`
- **Flags 辅助**：简单操作可用 flags，复杂对象用 JSON payload
- **未知字段拒绝**：不静默忽略，报错列出未知字段

### 自省

- **`describe` 命令**：返回命令的 JSON Schema，Agent 用此发现接口
- **`--help`**：存在但简洁，不是主要发现途径

### 错误处理

- 错误输出为 JSON：`{"ok": false, "error": {"code": "ERROR_CODE", "message": "...", "suggestion": "..."}}`
- 退出码有语义：0 成功，1 命令错误，2 系统错误，3 外部服务错误

## 项目结构约定

```
cli/<cli-name>/
├── package.json          # name, version, bin, dependencies
├── tsconfig.json
├── BLUEPRINT.md          # 设计蓝图
├── AGENTS.md             # 开发指令（本层级）
├── README.md             # 使用说明
├── src/
│   ├── index.ts          # CLI 入口（Commander 配置）
│   ├── commands/         # 每个命令组一个文件
│   │   ├── project.ts
│   │   ├── scene.ts
│   │   └── ...
│   ├── core/             # 业务逻辑（不依赖 CLI 框架）
│   │   ├── parser/       # 格式解析器（如 .tscn）
│   │   └── ...
│   ├── utils/            # 工具函数
│   │   ├── output.ts     # JSON/human 输出统一处理
│   │   ├── subprocess.ts # 子进程封装
│   │   └── ...
│   └── types/            # TypeScript 类型定义
└── tests/
    ├── unit/
    └── e2e/
```

## 测试要求

- 核心逻辑（core/）必须有单元测试
- 命令（commands/）至少有 smoke test（命令能运行、输出合法 JSON）
- 涉及 Godot 引擎的测试标记为 `@engine`，CI 中可选跳过
