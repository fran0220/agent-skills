# Godot MCP 生态调研报告

> 日期：2026-03-15 | 上下文：评估 MCP 对 godot-forge 四层架构的补充/替代价值

## 1. 生态概览

社区已有 **6+ 个 Godot MCP 服务器**，可分为两种架构范式：

| 架构 | 代表项目 | Stars | 通信方式 | 需要编辑器 |
|------|---------|-------|----------|:---:|
| **外部 headless 派** | [Coding-Solo/godot-mcp](https://github.com/Coding-Solo/godot-mcp) | 2.3k | stdio → godot --headless | ❌ |
| | [bradypp/godot-mcp](https://github.com/bradypp/godot-mcp) | 53 | 同上（fork 增强版） | ❌ |
| **编辑器插件派** | [@satelliteoflove/godot-mcp](https://github.com/satelliteoflove/godot-mcp) | npm 216/wk | stdio → WebSocket:6550 → 编辑器插件 | ✅ |
| | [tomyud1/godot-mcp](https://github.com/tomyud1/godot-mcp) | Godot Asset Library | 同上（Python MCP server） | ✅ |
| | [ee0pdt/Godot-MCP](https://github.com/ee0pdt/Godot-MCP) | 479 | TCP → 编辑器插件 | ✅ |
| | [Dokujaa/Godot-MCP](https://github.com/Dokujaa/Godot-MCP) | 38 | Python → 编辑器插件 + Meshy API | ✅ |

另有纯文档类 MCP（提供 Godot 4.x 文档查询），与我们无功能重叠。

## 2. 两种架构范式对比

### 2.1 外部 headless 派（Coding-Solo 模式）

```
[AI Agent] ←stdio/MCP→ [Node.js Server] ←child_process→ [godot --headless -s script.gd]
```

- **原理**：每次操作生成 GDScript，通过 `godot --headless` 执行
- **优点**：无需打开编辑器，适合 CI/批处理
- **缺点**：每次调用有 1-3s Godot 启动开销，无实时反馈
- **与 godot-forge 对比**：和我们的 L3a 层几乎完全重叠（都是 child_process 调 godot --headless），但：
  - Coding-Solo 使用**单一 bundled GDScript**（`godot_operations.gd`），避免临时文件
  - 我们使用**动态生成 temp script**，更灵活但有文件清理开销
  - 功能覆盖度：他们有 ~14 tools，我们有 43 commands，我们远超

### 2.2 编辑器插件派（satelliteoflove 模式）

```
[AI Agent] ←stdio/MCP→ [Node.js Server] ←WebSocket:6550→ [Godot 编辑器插件]
                                                                ↕ (Debugger Wire)
                                                         [运行中的游戏进程]
```

- **原理**：编辑器内运行 WebSocket 服务器，MCP server 作为中间层转发
- **独特能力**：
  - 🎯 **运行时游戏调试**：截图、性能计数器、输入注入
  - 🎬 **Animation 完整 CRUD**：14 个子操作，覆盖轨道/关键帧
  - 📸 **编辑器/游戏截图**：Base64 PNG 直接返回
  - 🔍 **实时节点查找**：在运行的游戏进程中搜索场景树
  - ⌨️ **输入注入**：模拟键盘/鼠标事件到运行中的游戏
  - 📊 **性能监控**：FPS、draw calls、内存使用
- **通信协议**：
  - `{id, command, params}` → `{id, status, result/error}`
  - 30s 心跳 + 45s 超时 + 僵尸连接自动替换
  - 16MB 缓冲区（支持截图 payload）

## 3. 与 godot-forge 四层架构的映射

| godot-forge 层级 | MCP 生态覆盖 | 评估 |
|:---:|:---:|:---|
| **L1** 纯文件 | ❌ 无对应 | MCP 不做纯文件操作，我们独占优势 |
| **L2** tscn 解析 | ❌ 无对应 | MCP 用编辑器 API 而非文本解析，我们独占优势 |
| **L3a** headless | ⚠️ 功能重叠 | Coding-Solo 做了类似的事，但我们更完整 |
| **L3b** ForgeSync | ✅ **严格优于** | WebSocket 双向通信 >> 文件轮询 |

### 关键发现：MCP 编辑器插件严格优于 ForgeSync

| 维度 | ForgeSync (当前) | MCP WebSocket Bridge |
|------|:---:|:---:|
| 通信延迟 | 0.5s 轮询间隔 | <16ms（每帧处理） |
| 方向性 | 单向（CLI→编辑器） | 双向（可返回结果） |
| 错误反馈 | 无（fire-and-forget） | 结构化 JSON 错误 |
| 操作种类 | 6 个 action | 50+ 个 command |
| 运行时调试 | ❌ | ✅ 截图/性能/输入注入 |
| 连接管理 | 无 | 心跳/超时/重连 |
| Undo 集成 | ❌ | ✅ 编辑器撤销栈 |

## 4. 战略建议

### 方案 A：集成现有 MCP Server（推荐先行评估）

**思路**：让 godotforge Skill 同时推荐安装 `@satelliteoflove/godot-mcp` 作为编辑器桥接。

```
Agent
  ├── godot-forge CLI（L1/L2/L3a）→ 项目搭建、场景/节点 CRUD、headless 验证
  └── godot-mcp Server（MCP）→ 实时编辑器交互、调试、截图、动画
```

- **优势**：零开发成本，社区维护，能力互补
- **劣势**：Agent 需要同时管理两个工具接口，操作语义可能重叠（如 create_scene 两边都有）
- **适用场景**：Skill 中引导 Agent 选择正确工具

### 方案 B：升级 ForgeSync 为 WebSocket Bridge

**思路**：参考 MCP 编辑器插件的架构，将 ForgeSync 从 `commands.json` 轮询升级为 WebSocket 双向通信。

- 保留我们的 Godot 插件（`forge_sync.gd`），改为 WebSocket 服务器
- CLI 通过 WebSocket 发送命令并等待响应
- 不引入 MCP 协议，保持 CLI 的 stdin/stdout JSON 信封模式

```
godot-forge CLI ←stdin/stdout→ Agent
godot-forge CLI ←WebSocket→ Godot Editor Plugin (升级版 ForgeSync)
```

- **优势**：统一的 CLI 接口，Agent 只需学一套 API
- **劣势**：开发量较大，需要重写 ForgeSync 插件和 CLI 的编辑器桥接层

### 方案 C：godot-forge 自身成为 MCP Server

**思路**：在 godot-forge CLI 之上添加 MCP transport，使其可以作为 MCP server 运行。

```bash
# CLI 模式（现有）
echo '{"scene":"main","name":"Player","type":"CharacterBody2D"}' | godot-forge node add

# MCP 模式（新增）
godot-forge serve --mcp  # 启动 MCP server
```

- **优势**：统一 43+ 命令为 MCP tools，原生支持任何 MCP 客户端
- **劣势**：MCP 协议有开销，CLI 模式和 MCP 模式需要并存维护
- **实现要点**：用 `@modelcontextprotocol/sdk` 包装现有 command handlers

### 推荐路径

1. **短期**（Phase 3）：**方案 A** — 在 Skill 中推荐 godot-mcp 作为编辑器桥接
2. **中期**（Phase 4）：**方案 B** — 升级 ForgeSync 为 WebSocket，参考 MCP 插件的架构
3. **长期**（v2）：**方案 C** — 评估是否将 godot-forge 本身作为 MCP server 发布

## 5. 关键经验（可借鉴到 godot-forge）

### 从 Coding-Solo/godot-mcp 学到的

1. **Bundled GDScript 模式**：用单一 `godot_operations.gd` 接收 JSON 参数分发操作，避免临时文件
   - 我们当前每次 L3a 操作都写 temp script → 执行 → 清理
   - 可考虑合并为一个通用 operations.gd，减少文件 I/O

### 从 satelliteoflove/godot-mcp 学到的

1. **Debugger Wire Bridge**：通过 `EngineDebugger.send_message()` 在编辑器和游戏进程间通信
   - 这是访问运行时游戏数据的最优方式（截图、性能、输入注入）
   - ForgeSync 如果要升级，应采用此模式而非文件轮询
2. **连接健壮性**：心跳+超时+僵尸替换三层保护
3. **16MB 缓冲区**：编辑器截图的 base64 payload 可能很大
4. **Dual-path node 查询**：场景树（编辑时）vs 运行时场景树（通过 debugger bridge）

### 从 tomyud1/godot-mcp 学到的

1. **可视化工具**：将项目的脚本/场景关系渲染为 2D 图（有趣但非核心）
2. **资产生成集成**：Meshy API 调用直接导入 Godot（对应我们 Phase 4 的 `generate *`）

## 6. 对 Skill 编写的启示

当前 `skills/godotforge/` 还是空的。Skill 应该包含：

1. **工具选择指导**：何时用 `godot-forge` CLI vs 何时用 MCP
2. **MCP 推荐配置**：如果用户有 MCP client（Cursor/Claude Desktop），推荐安装 godot-mcp
3. **互补工作流**：
   - 项目搭建 → godot-forge CLI（L1/L2，不需要编辑器）
   - 实时调试 → godot-mcp（需要编辑器打开）
   - CI/构建 → godot-forge CLI（L3a，headless）
