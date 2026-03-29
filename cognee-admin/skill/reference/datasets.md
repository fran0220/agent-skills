# Cognee 数据集参考

## 现有数据集

### main_dataset

| 字段 | 值 |
|------|-----|
| ID | `43eb2777-6e6a-54f0-a815-9b3be659226b` |
| 名称 | `main_dataset` |
| 状态 | `DATASET_PROCESSING_COMPLETED` |
| 内容 | Origin 平台知识（AI 游戏创作平台文档） |

### game

| 字段 | 值 |
|------|-----|
| ID | `e45f48b8-8ca9-5ab5-92a9-a7d11a49c9a3` |
| 名称 | `game` |
| 状态 | 处理中 |
| 文件数 | 293 个 Markdown 文件 |
| 来源 | GameDB v3 模块化游戏玩法知识库 |

#### game 数据集内容结构

GameDB v3 是一个结构化的游戏设计知识库，基于 Jesse Schell《游戏设计艺术》模块化理念构建。

```
atoms/md/     — 228 个玩法原子（可组合的最小游戏机制单元）
recipes/md/   — 50 个品类配方（游戏类型 Archetype）
systems/md/   — 15 个跨切面系统（变现、匹配、存档等）
+ 3 个总览文档（overview、audit report、v4 expansion plan）
```

**三层架构**：

| 层级 | 类型 | 数量 | 说明 |
|------|------|------|------|
| L1 | 玩法原子 (Atoms) | 228 | 最小可组合机制：跳跃、闪避、元素反应… |
| L2 | 品类配方 (Recipes) | 50 | 游戏类型：魂类、银河城、塔防、三消… |
| L3 | 跨切面系统 (Systems) | 15 | 变现、匹配、存档、无障碍… |

**原子分类**：
- A. Core Movement (21): 跳跃、冲刺、攀爬、游泳…
- B. Combat & Action (38): 连招、无敌帧、弹反、射击…
- C. Puzzle & Logic (16): 传送门、物理谜题、三消…
- D. Resource & Economy (33): 货币、抽卡、制造、拍卖行…
- E. Progression & Meta (23): 经验值、技能树、赛季排名…
- F. Narrative & Social (25): 分支对话、道德系统、公会…
- G. Structure & Level Design (48): 程序化生成、永久死亡、日夜循环…
- H. Input & Presentation (24): 触控、音游、自适应难度…

**每个原子文档包含**：
1. Identity（ID、名称、分类）
2. Core Verb & State Machine（状态转换图）
3. MDA Analysis（机制-动态-美学）
4. Parameter Space（参数表 + 基准值）
5. Benchmark Data（业界标杆游戏数据）
6. Common Variants（常见变体）
7. Compatibility Notes（兼容/不兼容原子）
8. Anti-Patterns（反模式）

#### game 数据集搜索示例

```bash
# 搜索移动机制
cognee-admin search "variable jump height coyote time" --datasets game --top-k 5

# 搜索战斗系统
cognee-admin search "souls-like combat stamina parry" --datasets game --top-k 10

# 搜索游戏配方
cognee-admin search "metroidvania recipe ability gating" --datasets game

# 搜索变现系统
cognee-admin search "gacha monetization pity system" --datasets game --search-type CHUNKS

# 快速搜索（不用 LLM，基于文本块匹配）
cognee-admin search "roguelike procedural generation" --datasets game --search-type CHUNKS
```

#### 组合查询（AI Agent 使用模式）

GameDB 的核心设计理念是"原子组合"。Agent 可以：

1. **查找品类配方** → 获取某类游戏需要的原子列表
2. **查找原子详情** → 获取具体机制的参数、状态机、基准数据
3. **检查兼容性** → 通过 `compatible_atoms` / `incompatible_atoms` 验证组合
4. **叠加系统** → 根据商业模式添加跨切面系统

示例：设计一个 Souls-like 游戏

```bash
# 1. 查找 souls-like 配方
cognee-admin search "souls-like recipe archetype" --datasets game

# 2. 查找核心原子
cognee-admin search "stamina combat dodge roll invincibility frames" --datasets game --top-k 10

# 3. 查找兼容系统
cognee-admin search "checkpoint bonfire save system" --datasets game
```

## 数据集管理 SOP

### 创建并导入新数据集

```bash
# 1. 创建
cognee-admin dataset create <name>

# 2. 批量导入 Markdown
cognee-admin data add-dir --dataset <name> --glob "**/*.md" /path/to/data/

# 3. 触发知识构建（后台 + 低并发避免限流）
cognee-admin cognify --dataset-name <name> --background --chunks-per-batch 5

# 4. 监控状态
cognee-admin dataset status
# 等待 DATASET_PROCESSING_COMPLETED

# 5. 验证搜索
cognee-admin search "test query" --datasets <name> --top-k 3
```

### 导入注意事项

- **只导入 `.md` 文件**——Cognee 对自然语言文本效果最好，避免同时导入 JSON 重复数据
- **大批量导入用 `--chunks-per-batch 3~5`** 降低 LLM 并发，避免限流
- **cognify 耗时**：293 文件约需 10-30 分钟（取决于 LLM 模型和限流）
- **Pipeline 卡死处理**：如果 status 长期停在 `STARTED`，可能需要清理 DB 中的 pipeline_runs 状态

### 当前 LLM 配置

| 配置项 | 值 | 说明 |
|--------|-----|------|
| LLM_MODEL | `gpt-5-mini` | 性价比好，structured output 兼容 |
| LLM_ENDPOINT | `https://api.xiaomao.chat/v1` | 小猫网关 |
| EMBEDDING_MODEL | `text-embedding-3-large` | OpenAI embedding |
| EMBEDDING_ENDPOINT | `https://api.xiaomao.chat/v1` | 小猫网关 |

**模型选择要点**：
- Cognee 图谱抽取需要 **structured output**（JSON schema 遵从）
- OpenAI 原生模型（gpt-*）效果最好
- Gemini 通过 LiteLLM 转接时 structured output 容易出格式错误
- `gpt-5-mini` 是当前最佳性价比选择
