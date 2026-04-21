# GameDB API 文档

> 游戏原子化知识库语义搜索服务

**Base URL**: `https://gamedb.zhangfan0220.workers.dev`

---

## 概览

GameDB 存储了 **293 个游戏设计知识单元**，支持语义搜索：

| 类型 | 数量 | 前缀 | 示例 |
|------|------|------|------|
| **atom** （玩法原子） | 228 | `atom_` | `atom_variable_jump`, `atom_gacha_banner` |
| **recipe**（品类配方） | 50 | 无 | `souls-like`, `metroidvania`, `auto-battler` |
| **system**（跨切面系统） | 15 | `sys_` | `sys_f2p_energy`, `sys_battle_pass` |

---

## 接口

### 1. 健康检查

```
GET /health
```

```bash
curl https://gamedb.zhangfan0220.workers.dev/health
```

```json
{
  "ok": true,
  "service": "gamedb",
  "version": "1.0.0",
  "vectors": 293,
  "embedding_model": "@cf/baai/bge-large-en-v1.5",
  "dimensions": 1024
}
```

---

### 2. 语义搜索（核心接口）

```
GET /search?q={query}&top_k={n}&type={type}&category={category}
```

| 参数 | 必填 | 默认 | 说明 |
|------|:----:|------|------|
| `q` | ✅ | — | 搜索语句（自然语言） |
| `top_k` | ❌ | 10 | 返回数量（最大 50） |
| `type` | ❌ | — | 过滤类型：`atom` / `recipe` / `system` |
| `category` | ❌ | — | 过滤分类（atom 的类别 A-H） |

**示例**：

```bash
# 基础搜索
curl "https://gamedb.zhangfan0220.workers.dev/search?q=open+world+traversal&top_k=5"

# 只搜配方
curl "https://gamedb.zhangfan0220.workers.dev/search?q=roguelike+combat&type=recipe"

# 只搜战斗相关原子
curl "https://gamedb.zhangfan0220.workers.dev/search?q=dodge+mechanic&type=atom&category=B"
```

**响应**：

```json
{
  "ok": true,
  "query": "open world traversal",
  "count": 5,
  "results": [
    {
      "id": "atom_interconnected_map",
      "score": 0.738,
      "metadata": {
        "type": "atom",
        "category": "G",
        "name_en": "Interconnected Map",
        "name_cn": "互联地图"
      }
    },
    {
      "id": "open-world-rpg",
      "score": 0.704,
      "metadata": {
        "type": "recipe",
        "category": "recipe",
        "name_en": "Open World RPG",
        "name_cn": "开放世界RPG"
      }
    }
  ]
}
```

**Atom 分类（category 值）**：

| Category | 领域 | 数量 |
|----------|------|------|
| A | Core Movement & Traversal | 21 |
| B | Combat & Action | 38 |
| C | Puzzle & Logic | 16 |
| D | Resource & Economy | 33 |
| E | Progression & Meta | 23 |
| F | Narrative & Social | 25 |
| G | Structure & Level Design | 48 |
| H | Input & Presentation | 24 |

---

### 3. 获取完整知识单元

```
GET /get/{id}
```

**示例**：

```bash
curl "https://gamedb.zhangfan0220.workers.dev/get/atom_variable_jump"
curl "https://gamedb.zhangfan0220.workers.dev/get/souls-like"
curl "https://gamedb.zhangfan0220.workers.dev/get/sys_f2p_energy"
```

**响应**：

```json
{
  "ok": true,
  "id": "atom_variable_jump",
  "json": {
    "atom_id": "atom_variable_jump",
    "name_en": "Variable Jump Height",
    "name_cn": "可变跳跃高度",
    "definition": "...",
    "parameters": [...],
    "benchmarks": [...],
    "compatible_atoms": ["atom_coyote_time", "atom_jump_buffer", "atom_double_jump"],
    "mda": { "mechanics": "...", "dynamics": "...", "aesthetics": [...] }
  },
  "markdown": "# Gameplay Atom: Variable Jump Height\n\n## 1. Identity\n..."
}
```

每个知识单元返回两种格式：
- `json` — 结构化数据（参数空间、基准数据、兼容性、MDA 分析）
- `markdown` — 完整人类可读文档

---

### 4. 列表

```
GET /list?type={type}
```

| 参数 | 必填 | 说明 |
|------|:----:|------|
| `type` | ❌ | 过滤类型：`atom` / `recipe` / `system`。不传返回全部 |

```bash
# 所有 293 个知识单元
curl "https://gamedb.zhangfan0220.workers.dev/list"

# 只列配方
curl "https://gamedb.zhangfan0220.workers.dev/list?type=recipe"

# 只列系统
curl "https://gamedb.zhangfan0220.workers.dev/list?type=system"
```

**响应**：

```json
{
  "ok": true,
  "count": 50,
  "items": [
    { "id": "souls-like", "type": "recipe", "name_en": "Souls-like", "name_cn": "魂类游戏" },
    { "id": "metroidvania", "type": "recipe", "name_en": "Metroidvania", "name_cn": "银河城类" }
  ]
}
```

---

### 5. 批量导入（管理员）

```
POST /ingest
Authorization: Bearer {GAMEDB_API_KEY}
```

需要 API Key 认证。用于更新或新增知识单元。

```bash
curl -X POST "https://gamedb.zhangfan0220.workers.dev/ingest" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -d '{
    "items": [{
      "id": "atom_new_mechanic",
      "json": { "name_en": "New Mechanic", "name_cn": "新机制", "definition": "..." },
      "markdown": "# New Mechanic\n..."
    }]
  }'
```

---

## 使用场景

### Agent 集成（curl）

```bash
# 设计一个 Souls-like 游戏时，查找相关原子
RESULTS=$(curl -s "https://gamedb.zhangfan0220.workers.dev/search?q=stamina+based+combat+dodge+roll&top_k=10")
echo "$RESULTS" | jq '.results[].metadata.name_en'
```

### TypeScript / Node.js

```typescript
const GAMEDB = "https://gamedb.zhangfan0220.workers.dev";

// 搜索
const res = await fetch(`${GAMEDB}/search?q=${encodeURIComponent("gacha monetization")}&top_k=5`);
const { results } = await res.json();

// 获取详情
const detail = await fetch(`${GAMEDB}/get/${results[0].id}`);
const { json, markdown } = await detail.json();
```

### Python

```python
import requests

GAMEDB = "https://gamedb.zhangfan0220.workers.dev"

# 搜索
results = requests.get(f"{GAMEDB}/search", params={"q": "open world exploration", "top_k": 5}).json()

# 获取详情
for r in results["results"]:
    detail = requests.get(f"{GAMEDB}/get/{r['id']}").json()
    print(f"{r['id']}: {detail['json']['name_cn']}")
```

### 游戏设计组合查询

```bash
# 1. 搜索一个品类的配方
curl -s "https://gamedb.zhangfan0220.workers.dev/get/souls-like" | jq '.json.atoms[].atom_id'

# 2. 查看配方的耦合矩阵
curl -s "https://gamedb.zhangfan0220.workers.dev/get/souls-like" | jq '.json.coupling_matrix'

# 3. 搜索兼容的原子
curl -s "https://gamedb.zhangfan0220.workers.dev/get/atom_dodge_roll" | jq '.json.compatible_atoms'
```

---

## 认证

| 端点 | 认证 |
|------|------|
| `/health`, `/search`, `/list`, `/get/:id` | **无需认证**（公开） |
| `/ingest` | **需要** `Authorization: Bearer {GAMEDB_API_KEY}` |

---

## 错误处理

所有错误返回统一格式：

```json
{
  "ok": false,
  "error": "Missing ?q= parameter"
}
```

| HTTP 状态码 | 含义 |
|------------|------|
| 200 | 成功 |
| 400 | 参数错误 |
| 401 | 未认证（ingest） |
| 404 | 知识单元不存在 |
| 500 | 服务器内部错误 |

---

## 技术规格

| 项 | 值 |
|----|-----|
| Embedding 模型 | `@cf/baai/bge-large-en-v1.5` |
| 向量维度 | 1024 |
| 距离度量 | Cosine |
| 向量数量 | 293 |
| 存储 | Cloudflare R2 |
| 索引 | Cloudflare Vectorize |
| 部署 | Cloudflare Workers |
| 延迟 | ~50-100ms（全球边缘） |
