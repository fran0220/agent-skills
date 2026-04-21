# GameDB

游戏原子化知识库 — 基于 Cloudflare Workers + Vectorize + R2 的语义搜索服务。

**替代原 cognee-admin 全栈**，从 ~3400 行 Rust + Python Docker + PostgreSQL + Nginx 简化到 ~200 行 TypeScript Worker。

## 架构

```
CF Worker (gamedb) → Vectorize (293 vectors × 1024d) + R2 (原文存储)
                   → Workers AI (@cf/baai/bge-large-en-v1.5) embedding
```

| 组件 | 说明 |
|------|------|
| `worker/` | CF Worker API（TypeScript） |
| `data/` | GameDB v3 知识库原始数据 |
| `scripts/` | 一次性 ingest 脚本 |

## API

**Base URL**: `https://gamedb.zhangfan0220.workers.dev`

| 端点 | 方法 | 说明 |
|------|------|------|
| `/health` | GET | 健康检查 |
| `/search?q=...&top_k=10&type=atom` | GET | 语义搜索 |
| `/list?type=atom\|recipe\|system` | GET | 列表 |
| `/get/:id` | GET | 获取完整数据（JSON + Markdown） |
| `/ingest` | POST | 批量导入（需 Bearer auth） |

### 搜索示例

```bash
# 语义搜索
curl "https://gamedb.zhangfan0220.workers.dev/search?q=open+world+traversal&top_k=5"

# 按类型过滤
curl "https://gamedb.zhangfan0220.workers.dev/search?q=combat+system&type=recipe"

# 获取单个知识单元
curl "https://gamedb.zhangfan0220.workers.dev/get/atom_variable_jump"

# 列表
curl "https://gamedb.zhangfan0220.workers.dev/list?type=atom"
```

## 数据

293 个知识单元：
- **228 atoms** — 玩法原子（8 类：Movement, Combat, Puzzle, Economy, Progression, Narrative, Structure, Input）
- **50 recipes** — 品类配方（Souls-like, Metroidvania, Auto-Battler 等）
- **15 systems** — 跨切面系统（F2P Economy, Battle Pass, Matchmaking 等）

## 部署

```bash
cd worker
npm install
wrangler deploy
```

## 数据更新

```bash
npx tsx scripts/ingest.ts \
  --data ./data/extracted \
  --endpoint https://gamedb.zhangfan0220.workers.dev \
  --key <GAMEDB_API_KEY>
```

## 成本

**$0/月** — 全部在 CF 免费额度内：
- Vectorize: 293 × 1024 = ~30 万维度（免费额度 1000 万）
- R2: ~3MB（免费额度 10GB）
- Workers AI: ~300 次 embedding（免费额度充足）
- Worker: 按需调用（免费额度 10 万次/天）
