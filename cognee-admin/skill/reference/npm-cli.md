# cognee-admin npm CLI

轻量 TypeScript 客户端，纯 Cognee API 操作，无需 Rust 或 PostgreSQL。

## 安装

```bash
# 一次性使用
npx @doufunao123/cognee-admin@0.4.0 <command>

# 全局安装
npm install -g @doufunao123/cognee-admin
cognee-admin <command>
```

要求 Node.js ≥ 20。

## 认证

npm CLI 使用 `ca_xxx` admin token，保存在 `~/.config/cognee-admin/auth.json`。

```bash
# 保存 token（只需一次）
cognee-admin auth set ca_xxx_your_token

# 检查认证状态
cognee-admin auth status

# 清除本地凭据
cognee-admin auth clear
```

Token 保存后所有命令自动加载，无需每次传递 `--admin-token`。

## 命令速查

### 健康检查

```bash
cognee-admin health              # 基础健康
cognee-admin health --detailed   # 组件级详情
```

### 数据集管理

```bash
cognee-admin dataset list
cognee-admin dataset create <name>
cognee-admin dataset delete <id>
cognee-admin dataset delete-all
cognee-admin dataset status        # 查看处理状态
cognee-admin dataset graph <id>    # 获取知识图谱
```

### 数据操作

```bash
# 添加文本
cognee-admin data add --dataset <name> "文本内容"

# 上传单文件
cognee-admin data add-file --dataset <name> ./file.md

# 批量上传（递归扫描，每批 10 个，失败自动逐文件重试）
cognee-admin data add-dir --dataset <name> --glob "**/*.md" ./directory/

# 查看数据
cognee-admin data list <dataset-id>
cognee-admin data raw --dataset-id <id> --data-id <id>

# 删除 / 更新
cognee-admin data delete --dataset-id <id> --data-id <id>
cognee-admin data update --dataset-id <id> --data-id <id> ./new-file.md
```

### 知识构建 (Cognify)

```bash
# 基础 cognify
cognee-admin cognify --dataset-name <name>

# 后台 + 降低并发
cognee-admin cognify --dataset-name <name> --background --chunks-per-batch 5

# 自定义 prompt
cognee-admin cognify --dataset-name <name> --custom-prompt "提取游戏机制和系统关系"
cognee-admin cognify --dataset-name <name> --custom-prompt-file ./prompt.txt
```

### 搜索

```bash
# 基础搜索
cognee-admin search "query"

# 指定数据集 + 搜索类型
cognee-admin search "dodge roll mechanics" --datasets game --top-k 10
cognee-admin search "crafting system" --search-type CHUNKS --datasets game

# 搜索历史
cognee-admin search history
```

搜索类型：
- `CHUNKS` — 快速，基于 embedding 的文本块匹配（~1-2s）
- `SUMMARIES` — 快速，基于摘要匹配（~1-2s）
- `GRAPH_COMPLETION` — 慢，通过知识图谱 + LLM 生成答案（~10-30s，默认）
- `RAG_COMPLETION` — 慢，RAG pipeline + LLM（~10-30s）

### 配置

```bash
cognee-admin config get
cognee-admin config set '{"llm":{"provider":"openai","model":"gpt-5-mini"}}'
```

### Ontology

```bash
cognee-admin ontology upload --key <name> ./ontology.owl
cognee-admin ontology list
```

### 自省

```bash
cognee-admin describe           # 所有命令 JSON Schema
cognee-admin describe search    # 指定命令
```

## 输出格式

所有命令输出统一 JSON 信封：

```json
{"ok": true, "command": "dataset.list", "data": [...]}
{"ok": false, "command": "search", "error": {"code": "NOT_FOUND", "message": "..."}}
```

使用 `--human` 输出人类可读格式。使用 `--raw` 输出不带信封的原始数据。

## npm CLI vs Rust CLI

npm CLI **只做 API 客户端**，以下功能仅 Rust CLI 支持：

| 功能 | npm CLI | Rust CLI |
|------|:-------:|:--------:|
| `serve` (Web 面板) | ❌ | ✅ |
| `log` (请求日志) | ❌ | ✅ |
| `pipeline` (运行记录) | ❌ | ✅ |
| `token` (Token 管理) | ❌ | ✅ |
| `login` (JWT 获取) | ❌ | ✅ |
