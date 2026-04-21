# Cognee-Admin v0.2 升级计划

> 从运维 CLI 升级为全栈知识管理平台：文件上传、双轨认证、权限体系、Agent Skill

## 当前状态

- **代码量**: ~3,400 行 Rust
- **CLI 命令**: 10 组 + serve
- **Web 页面**: 8 + login
- **认证**: cognee-admin 自有 token (ca_xxx) + Cognee JWT (login 命令)
- **部署**: Oracle ARM VPS, systemd, Nginx 反代

## 架构决策

### 双轨认证模型（保持分离）

```
┌─────────────────────────────────────────────────────────────┐
│                      Trust Domain A                          │
│              Cognee API (cogneeapi.origingame.dev)             │
│                                                              │
│  Auth: Cognee JWT (fastapi-users)                           │
│  Used by: CLI data/search/cognify/config/dataset commands   │
│  Stored: ~/.config/cognee-admin/auth.json → cognee_jwt      │
│  Obtained: `cognee-admin login --username --password`        │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                      Trust Domain B                          │
│            cognee-admin Panel (cognee.origingame.dev)          │
│                                                              │
│  Auth: ca_xxx tokens (SHA256 hashed in PG)                  │
│  Used by: Web panel login, Nginx auth_request, admin CLI    │
│  Stored: ~/.config/cognee-admin/auth.json → admin_token     │
│  Obtained: Admin 创建后分发                                   │
│  Web上游: Server 用固定 service credential 调 Cognee API     │
└─────────────────────────────────────────────────────────────┘
```

**理由**: 两种 token 治理不同边界。Cognee JWT 管用户级 API 权限，ca_xxx 管运维面板访问。合并需要大型身份系统重构，当前不必要。

### 关键改动

1. `AuthConfig` 从单 token 拆为 `cognee_jwt` + `admin_token`
2. CLI 环境变量拆分: `COGNEE_JWT` / `COGNEE_ADMIN_TOKEN`
3. Web Server 用固定 service credential 调 Cognee API（不走 per-user JWT）
4. `CogneeClient` 新增 multipart 上传方法
5. 修复 request_logs 的 `MAX(id)` 竞态问题

---

## 工作流分解（5 个并行 Thread）

### Thread 1: Auth 重构 + Config 拆分
**预计改动**: ~200 行
**依赖**: 无（基础设施，其他 Thread 依赖此）
**优先级**: 🔴 最先完成

#### 任务清单

1. **拆分 AuthConfig**
   ```rust
   pub struct AuthConfig {
       pub cognee_jwt: Option<String>,   // Cognee API JWT
       pub admin_token: Option<String>,  // ca_xxx admin token
       pub cognee_url: Option<String>,
   }
   ```
   - 兼容迁移：旧 `token` 如果以 `ca_` 开头 → `admin_token`，否则 → `cognee_jwt`
   - 保存时设文件权限 `0600`

2. **拆分 CLI 参数和环境变量**
   ```
   --cognee-jwt / COGNEE_JWT          → Cognee API 认证
   --admin-token / COGNEE_ADMIN_TOKEN → 管理面板认证
   --token (旧) → 保留为 alias，优先映射到 cognee_jwt
   ```

3. **修改 main.rs token 解析逻辑**
   - Cognee API 命令 (health/dataset/data/cognify/search/config) 用 `cognee_jwt`
   - Admin 命令 (token/log/pipeline) 用 `admin_token`（或直连 PG 不需要）
   - `login` 命令：获取 Cognee JWT → 保存到 `cognee_jwt`

4. **修改 CogneeClient**
   - 明确 `with_token()` 接收的是 Cognee JWT
   - Web server 的 `CogneeClient` 使用固定 service JWT（env `COGNEE_SERVICE_JWT`）

5. **修复 request_logs 竞态**
   - `request_json()` 中 response_preview 更新改用 INSERT 返回的 id

#### 涉及文件
- `src/config.rs` — AuthConfig 拆分
- `src/main.rs` — token 解析 + 命令分发
- `src/cognee_client.rs` — 修复日志竞态
- `src/server/mod.rs` — service credential 注入

---

### Thread 2: CLI 文件上传 + 批量导入
**预计改动**: ~350 行新增
**依赖**: Thread 1（需要 cognee_jwt 认证）
**优先级**: 🔴 核心功能

#### 任务清单

1. **Cargo.toml 加 reqwest multipart**
   ```toml
   reqwest = { version = "0.12", features = ["json", "rustls-tls", "multipart", "stream"] }
   ```
   加 `indicatif` 做进度条:
   ```toml
   indicatif = "0.17"
   ```

2. **CogneeClient 新增 multipart 方法**
   ```rust
   pub async fn upload_file(&self, dataset_name: &str, file_path: &Path) -> Result<Value, AppError>
   pub async fn upload_files(&self, dataset_name: &str, files: &[PathBuf]) -> Result<Value, AppError>
   pub async fn update_data(&self, dataset_id: &str, data_id: &str, file_path: &Path) -> Result<Value, AppError>
   ```
   - 日志记录文件名和大小，不记录文件内容
   - 流式上传，不全量缓存

3. **新增 data 子命令**
   ```
   data add-file  --dataset <name> <file_path>         # 单文件上传
   data add-dir   --dataset <name> --glob "*.md" <dir>  # 批量目录上传
   data update    --dataset-id <id> --data-id <id> <file_path>  # 替换数据
   ```
   - `add-dir` 显示进度条（indicatif）
   - `add-dir` 分批上传（每批 10 个文件），带 1s 间隔
   - 输出上传统计：成功/失败/跳过

4. **更新 DataAction enum 和 dispatch**

5. **更新 describe_cmd.rs schema**

#### 涉及文件
- `Cargo.toml` — 新依赖
- `src/cognee_client.rs` — multipart 方法
- `src/client/data_cmd.rs` — 新子命令实现
- `src/main.rs` — DataAction 新增 + dispatch
- `src/client/describe_cmd.rs` — schema 更新

---

### Thread 3: CLI 命令补全 + 增强
**预计改动**: ~400 行新增/修改
**依赖**: Thread 1（认证拆分）
**优先级**: 🟡 重要

#### 任务清单

1. **cognify 命令增强**
   ```
   cognify [--dataset-id <id>] [--dataset-name <name>]
           [--custom-prompt <text>]
           [--background]
           [--chunks-per-batch <n>]
   ```
   - `--background` 返回 run_id（不等待完成）
   - `--custom-prompt` 支持从 stdin 读取（`--custom-prompt -` 或 `--custom-prompt-file <path>`）

2. **search 命令增强**
   ```
   search <query> [--search-type <type>] [--top-k <n>]
          [--datasets <name,...>]
          [--verbose]
   search history [--limit <n>]
   ```

3. **health 命令增强**
   ```
   health [--detailed] [--watch] [--interval <secs>]
   ```
   - `--watch` 循环打印，Ctrl+C 退出
   - 每次 health check 记录到 health_snapshots

4. **dataset 命令增强**
   ```
   dataset delete-all [--yes]    # 危险操作，必须 --yes 或交互确认
   ```

5. **新增 ontology 命令组**
   ```
   ontology upload --key <name> <owl_file>    # multipart 上传 OWL 文件
   ontology list                               # 列出所有 ontology
   ```
   - 需要 CogneeClient 新增 ontology 方法

6. **更新 describe_cmd.rs**

#### 涉及文件
- `src/main.rs` — Commands enum 扩展
- `src/client/cognify_cmd.rs` — 参数扩展
- `src/client/search_cmd.rs` — datasets 过滤 + history
- `src/client/health_cmd.rs` — watch 模式
- `src/client/dataset_cmd.rs` — delete-all
- `src/client/ontology_cmd.rs` — 新文件
- `src/client/mod.rs` — 新模块声明
- `src/cognee_client.rs` — 新 API 方法
- `src/client/describe_cmd.rs` — schema 更新

---

### Thread 4: Web 面板增强 + 数据上传页
**预计改动**: ~500 行新增
**依赖**: Thread 1（service credential）、Thread 2（multipart client 方法）
**优先级**: 🟡 重要

#### 任务清单

1. **数据上传页面** (`/upload`)
   - 文件拖拽上传区域（HTMX + JS FileReader）
   - 目标 dataset 选择（下拉，HTMX 动态加载 dataset 列表）
   - 上传进度显示
   - 上传完成后一键 Cognify 按钮
   - 新增路由: `GET /upload` (页面) + `POST /api/upload` (处理)

2. **Ontology 管理页面** (`/ontologies`)
   - 列出已上传 ontology
   - 上传新 ontology（OWL 文件）
   - 新增路由: `GET /ontologies` + `POST /api/ontologies`

3. **Dataset 页面增强**
   - 增加 "Delete All" 按钮（需确认）
   - 增加每个 dataset 的 "Cognify" 按钮（支持 custom prompt）
   - 显示 cognify 状态（background 运行时轮询）

4. **Search 页面增强**
   - 增加 dataset 过滤下拉
   - 增加搜索历史标签页

5. **侧边栏更新**
   - 添加 Upload 和 Ontologies 导航项

6. **Server 路由挂载**

#### 涉及文件
- `src/server/routes/upload.rs` — 新文件
- `src/server/routes/ontologies.rs` — 新文件
- `src/server/routes/mod.rs` — 挂载 + 侧边栏更新
- `src/server/routes/datasets.rs` — cognify/delete-all 按钮
- `src/server/routes/search.rs` — dataset 过滤 + history

---

### Thread 5: Skill 创建 + 部署
**预计改动**: ~200 行 SKILL.md + 部署脚本
**依赖**: Thread 1-4 全部完成
**优先级**: 🟢 最后

#### 5a: Skill 创建

创建 `cognee-admin/skill/SKILL.md`：

```yaml
---
name: cognee-admin
description: "Cognee knowledge engine management. Use when the user wants to add data to Cognee, build knowledge graphs (cognify), search knowledge bases, manage datasets, or administer the Cognee instance."
---
```

内容结构：
- **Prerequisites** — CLI 安装、环境变量配置
- **Authentication** — 双轨认证说明
- **Core Workflows**
  - 数据录入流程: upload → cognify → search
  - 批量导入流程: add-dir → cognify --custom-prompt --background
  - 知识图谱管理: dataset CRUD + graph 查看
  - Ontology 管理: upload OWL → cognify with ontology
- **Command Quick Reference** — 所有命令速查表
- **GameDB 导入 SOP** — 具体的 GameDB 导入标准操作流程
- **Anti-Patterns** — 常见错误和避免方法

#### 5b: 部署更新

1. 更新 `.env` 文件
   ```bash
   COGNEE_JWT=...          # 新变量名
   COGNEE_ADMIN_TOKEN=...  # 不变
   COGNEE_SERVICE_JWT=...  # Web server 用的 service credential
   ```

2. 构建部署脚本
   ```bash
   # 本地交叉编译或 VPS 上编译
   rsync -avz ./cognee-admin/cli/ oracle:~/cognee-admin-cli/
   ssh oracle 'cd ~/cognee-admin-cli && cargo build --release'
   ssh oracle 'sudo systemctl stop cognee-admin'
   ssh oracle 'sudo cp ~/cognee-admin-cli/target/release/cognee-admin /usr/local/bin/'
   ssh oracle 'sudo systemctl start cognee-admin'
   ```

3. Smoke test checklist
   - [ ] `cognee-admin health` 正常
   - [ ] `cognee-admin login --username X --password Y` → 保存 JWT
   - [ ] `cognee-admin data add-file --dataset test /tmp/test.md` → 上传成功
   - [ ] `cognee-admin cognify --dataset-name test` → 触发成功
   - [ ] `cognee-admin search "test query"` → 返回结果
   - [ ] Web 面板 `cognee.origingame.dev` 登录正常
   - [ ] Web 面板上传功能正常
   - [ ] Nginx auth_request 仍然正常

4. Skill 安装
   ```bash
   ln -s /path/to/agent-skills/cognee-admin/skill ~/.config/amp/skills/cognee-admin
   ```

---

## Thread 依赖图

```
Thread 1 (Auth 重构)
    │
    ├──→ Thread 2 (文件上传)
    │        │
    ├──→ Thread 3 (命令补全)
    │        │
    └──→ Thread 4 (Web 增强) ──→ 依赖 Thread 2 的 multipart client
              │
              └──→ Thread 5 (Skill + 部署) ← 等待 1-4 全部完成
```

实施顺序：
1. **Thread 1** 先行（~1h），产出稳定的认证基础
2. **Thread 2 + Thread 3** 并行（各 ~2h）
3. **Thread 4** 在 Thread 1+2 完成后启动（~2h）
4. **Thread 5** 最后（~1h）

---

## 代码量预估

| Thread | 新增/修改行数 | 新文件 | 修改文件 |
|--------|-------------|--------|---------|
| 1. Auth 重构 | ~200 | 0 | 4 |
| 2. 文件上传 | ~350 | 0 | 5 |
| 3. 命令补全 | ~400 | 1 | 7 |
| 4. Web 增强 | ~500 | 2 | 4 |
| 5. Skill + 部署 | ~200 | 2 | 0 |
| **合计** | **~1,650** | **5** | **~15** |

最终代码量: ~3,400 + 1,650 = **~5,050 行 Rust** + Skill

---

## 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| JWT 过期无 refresh | CLI 命令突然失败 | 401 时提示 "please run login again" |
| 大文件上传内存压力 | VPS OOM (22G RAM) | 流式上传，单文件限制 100MB |
| dataset delete-all 误操作 | 数据全丢 | 必须 `--yes` flag |
| request_logs 竞态 | 日志关联错误 | INSERT RETURNING id |
| Cognee API 变更 | 命令失败 | describe 命令 + 版本检测 |

## 完成标准

- [ ] 所有 CLI 命令可正常工作（含 multipart 上传）
- [ ] Auth 双轨模型清晰、配置文件兼容迁移
- [ ] Web 面板可上传文件、触发 cognify
- [ ] GameDB v3 的 293 个 .md 可通过 `data add-dir` 一键导入
- [ ] Skill 已创建并可被 Agent 加载
- [ ] 线上部署更新且 smoke test 通过
