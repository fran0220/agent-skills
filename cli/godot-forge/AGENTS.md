# godot-forge CLI — 开发指令

## 定位

Agent-Primary Workflow CLI，用于 AI Agent 驱动的 Godot 4.x 游戏开发。

**设计蓝图**: `BLUEPRINT.md`（cli-design-framework 产出，所有设计决策的权威来源）

## 关键设计约束

1. **JSON 默认输出** — `--human` 才是可选
2. **Raw-payload-first** — 复杂对象通过 stdin JSON 传入，flags 只用于简单操作
3. **`describe` 优先于 `--help`** — Agent 用 `describe` 发现接口
4. **无 session** — CLI 是无状态执行器
5. **四层执行** — L1 纯文件 / L2 .tscn 文本 / L3a godot --headless / L3b 编辑器上下文

## 四层执行模型

```
L1: 纯文件操作（无 Godot 依赖）
    → project init/config/autoload/input/plugin, script create, design
    → 直接生成/修改 .md, .json, .gd, .cfg, project.godot, export_presets.cfg

L2: Godot 文本格式操作（无 Godot 依赖）
    → scene CRUD, node CRUD + 脚本绑定 + 实例化 + 信号连接, resource create/read/update
    → 解析/生成 .tscn, .tres 文件
    → 自研 parser（src/core/parser/）

L3a: Godot 引擎运行时操作（需要 godot 二进制）
    → engine validate/run/import, export build
    → describe node-types/resource-types/class/project-settings (ClassDB 自省)
    → child_process 调用 godot --headless -s script.gd

L3b: Godot 编辑器上下文操作（通过 ForgeSync 插件）
    → 场景打开/重载、节点选中、文件扫描、运行/停止
    → CLI 写入 .godot-forge/commands.json → 插件轮询执行
```

## 当前实现状态 — 43 个命令，49 个测试

### 命令总览

| 命令组 | 子命令 | 层级 |
|--------|--------|------|
| `project` | `init`, `info` | L1 |
| `project config` | `get <key>`, `set <key> <value>`, `list [section]` | L1 |
| `project autoload` | `add`, `remove`, `list` | L1 |
| `project input` | `add`, `remove`, `list` | L1 |
| `project plugin` | `list`, `enable`, `disable` | L1 |
| `scene` | `create`, `read`, `list`, `delete`, `update` | L2 |
| `node` | `add`, `remove`, `list`, `update`, `move` | L2 |
| `node` | `connect`, `disconnect`, `connections` | L2 |
| `node group` | `add`, `remove`, `list` | L2 |
| `script` | `create`, `list`, `validate`, `edit` | L1 |
| `engine` | `editor`, `validate`, `run`, `import`, `uid`, `preview` | L3a/L3b |
| `export` | `preset list`, `preset add`, `build` | L1/L3a |
| `resource` | `create`, `read`, `update`, `list`, `import`, `check` | L1/L2/L3a |
| `describe` | 43 个命令 schema + 引擎自省 (node-types, resource-types, class:*, project-settings) | L1/L3a |

### 基础设施

| 模块 | 文件 | 功能 |
|------|------|------|
| 输出层 | `src/utils/output.ts` | JSON 信封 + `--human` + `--fields` |
| 输入层 | `src/utils/input.ts` | stdin JSON + `--input` 文件 |
| 子进程 | `src/utils/subprocess.ts` | `findGodot()` (macOS/Linux/Windows) + `runGodotScript()` |
| 编辑器桥接 | `src/utils/editor-bridge.ts` | ForgeSync IPC |
| .tscn 解析器 | `src/core/parser/tscn.ts` | 解析/生成 + ext_resource/sub_resource/connection 管理 |
| INI 解析器 | `src/core/parser/godot-config.ts` | project.godot 读写（含多行值支持） |
| AST 类型 | `src/core/parser/types.ts` | TscnDocument, TscnNode, Connection |
| CLI 类型 | `src/types/index.ts` | GlobalOptions, SceneSpec, NodeSpec, etc. |

### 跨平台兼容

`findGodot()` 支持：
- **macOS**: PATH + `/Applications/Godot.app`
- **Linux**: PATH + `/usr/bin/godot` + `/usr/local/bin/godot` + Flatpak + Snap + AppImage
- **Windows**: PATH + `Program Files` + `%LOCALAPPDATA%`

### node add 增强能力

```bash
# 基本节点
echo '{"scene":"main","name":"Player","type":"CharacterBody2D"}' | godot-forge node add

# 附加脚本（自动注册 ext_resource）
echo '{"scene":"main","name":"Player","type":"CharacterBody2D","script":"res://scripts/player.gd"}' | godot-forge node add

# 场景实例化（自动注册 PackedScene ext_resource）
echo '{"scene":"main","name":"Enemy1","instance":"res://scenes/enemy.tscn"}' | godot-forge node add

# 嵌套子节点
echo '{"scene":"main","name":"Player","type":"CharacterBody2D","children":[
  {"name":"Sprite","type":"Sprite2D"},
  {"name":"Collision","type":"CollisionShape2D"}
]}' | godot-forge scene create
```

---

## Parser API

### tscn.ts 导出函数

```typescript
// 解析/生成
parseTscn(content: string): TscnDocument
generateTscn(doc: TscnDocument): string
createEmptyScene(rootType: string, rootName: string): TscnDocument

// 节点操作
addNode(doc, node): TscnDocument
removeNode(doc, nodePath): TscnDocument

// ext_resource 管理（自动 ID 分配，路径去重）
addExtResource(doc, type, path, uid?): { doc, id }
findExtResource(doc, path): string | null
removeExtResource(doc, id): TscnDocument

// sub_resource 管理
addSubResource(doc, type, properties): { doc, id }
removeSubResource(doc, id): TscnDocument

// 连接管理
addConnection(doc, connection): TscnDocument
removeConnection(doc, signal, from, to): TscnDocument

// 工具
recalcLoadSteps(doc): TscnDocument
```

### godot-config.ts 导出函数

```typescript
parseGodotConfig(content: string): GodotConfig
generateGodotConfig(config: GodotConfig): string
configGet(config, section, key): string | undefined
configSet(config, section, key, value): GodotConfig
configUnset(config, section, key): GodotConfig
configListSection(config, section): Map<string, string> | undefined
configListSections(config): string[]
```

### TscnNode 类型

```typescript
interface TscnNode {
  name: string;
  type?: string;
  parent?: string;
  instance?: string;              // ExtResource("N") 引用
  groups?: string[];              // 节点组
  unique_name_in_owner?: boolean; // %Name 唯一名称
  index?: number;                 // 子节点排序
  properties: Record<string, unknown>;
}
```

---

## ForgeSync 编辑器桥接

### 架构

```
CLI (TypeScript)                  Godot Editor
     │                                │
     │ sendEditorCommand()            │
     │ ──write──> .godot-forge/       │
     │            commands.json       │
     │                    ┌───poll────>│ ForgeSync plugin (0.5s)
     │                    │           │ - 执行动作
     │                    │           │ - 删除命令文件
     │                    │           │ - 自动 scan 文件系统
```

### 支持的动作

| action | 描述 | 自动触发 |
|--------|------|----------|
| `open_scene` | 打开场景 | `scene create` |
| `reload_scene` | 重载当前场景 | `node add/remove/update/move/connect`, `scene update` |
| `select_node` | 选中节点 | — |
| `scan` | 扫描文件系统 | — |
| `run_scene` | 运行场景 | — |
| `stop` | 停止运行 | — |

### 自动安装

`project init` 自动将 ForgeSync 嵌入项目：
- 写入 `addons/forge_sync/plugin.cfg` + `forge_sync.gd`
- 在 `project.godot` 的 `[editor_plugins]` 段启用
- `.gitignore` 包含 `.godot-forge/`

---

## 开发约定

### 新增命令检查清单

1. 在 `src/commands/<group>.ts` 中添加子命令
2. 使用 `readInput<T>()` 读取 stdin JSON
3. 使用 `success()` / `error()` 构建输出信封
4. 支持 `--dry-run`（mutation 命令必须）
5. 支持 `--force`（覆盖/删除命令必须）
6. 支持 `--fields` 字段过滤
7. 发送 ForgeSync 命令（如果操作影响编辑器显示）
8. 在 `describe.ts` 的 `SCHEMAS` 中注册 schema
9. 添加 E2E 测试到 `tests/e2e/cli.test.ts`

### 退出码

- `0` = 成功
- `1` = 命令错误（无效输入、文件不存在、验证失败）
- `2` = 引擎错误（Godot headless 失败、导入错误）
- `3` = 外部服务错误（v2+）

---

## 后续开发计划

### Phase 3 — 领域功能包（按游戏需求选装）

Phase 1（核心基础设施）和 Phase 2（通用资源系统 + 自省）已全部完成。以下领域现在可通过已有通用命令覆盖（`node add` + `resource create` + `project config set`），未来可添加便捷封装：

| 领域 | 通过现有命令可做 | 未来便捷封装 |
|------|-----------------|-------------|
| **Physics** | `node add --type CollisionShape2D` + `resource create --type RectangleShape2D` | 子资源自动关联 |
| **UI** | `scene create --root-type Control` + `node add --type Button` | 锚点/布局预设 |
| **Animation** | `resource create --type Animation` + `node add --type AnimationPlayer` | 轨道/关键帧 CRUD |
| **Audio** | `node add --type AudioStreamPlayer2D` + `resource create --type AudioBusLayout` | 总线/效果器管理 |
| **Navigation** | `node add --type NavigationRegion2D` + `resource create --type NavigationPolygon` | 烘焙命令 |
| **TileMap** | `resource create --type TileSet` + `node add --type TileMapLayer` | TileSet 源/瓦片 CRUD |
| **Rendering** | `resource create --type StandardMaterial3D` + `resource create --type Environment` | Shader 参数管理 |
| **Design** | Agent 直接生成 .md 文件 | `design gdd/art/narrative` 模板 |

### Phase 4 — 高级自动化（v2+）

| 功能 | 层级 | 说明 |
|------|------|------|
| Multiplayer 脚手架 | L1/L2 | RPC 注解、Synchronizer/Spawner 节点 |
| C# 支持 | L1/L3a | .csproj 管理、C# 脚本生成 |
| GDExtension | L1/L3a | .gdextension 配置 |
| XR/AR 支持 | L1/L2 | OpenXR 配置、XR 节点 |
| `generate *` | L1+外部 | AI 服务集成（图片/音频/模型） |
| 调试/性能分析 | L3a | 运行日志收集、截图测试 |
| 增量构建 | L3a | 变更检测 |

### Parser 待增强（按需）

- **Variant 序列化增强**：PackedArray、Dictionary 字面量、多行值
- **TileSet/TileMap 专用序列化**（格式密集且版本敏感）
- **Animation 轨道/关键帧打包格式**
- **Theme 资源结构**

### Godot 功能全景 vs 自动化层级

| Godot 领域 | L1 | L2 | L3a | L3b | GUI-only |
|------------|:---:|:---:|:---:|:---:|:---:|
| 项目配置 | ✅ 主力 | — | 自省 | 插件启停 | — |
| 场景/节点 | — | ✅ 主力 | 验证 | 打开/重载 | 可视化放置 |
| 脚本 | ✅ 主力 | — | 验证 | — | — |
| 资源 (.tres) | — | ✅ 主力 | 导入 | — | 可视化编辑 |
| 物理 | — | ✅ 节点+资源 | 测试 | — | 形状绘制 |
| UI | — | ✅ 节点+属性 | 验证 | — | WYSIWYG |
| 动画 | — | ✅ 资源 | 验证 | — | 时间轴 |
| 音频 | — | ✅ 节点+总线 | 导入 | — | 试听 |
| 渲染 | — | ✅ 材质/Shader | 编译 | — | 实时预览 |
| 导航 | — | ✅ 节点+资源 | 烘焙 | — | 可视化 |
| TileMap | — | ✅ 复杂 | 验证 | — | 绘画工具 |
| 导入管线 | 放置源文件 | — | ✅ 主力 | 重导入 | 导入面板 |
| 导出/构建 | 预设配置 | — | ✅ 主力 | — | 一键部署 |

**结论**：真正 GUI-only 的仅是**可视化放置/预览**。所有项目数据都可通过 L1-L3 自动化。
