# Jimeng Gateway — Seedance 2.0 API 完整指南

> 面向第三方开发者的全面 API 文档，涵盖接入、调用、素材上传、任务管理、错误处理与内容审核机制。

---

## 目录

1. [接入概览](#1-接入概览)
2. [认证与鉴权](#2-认证与鉴权)
3. [可用模型](#3-可用模型)
4. [创建视频生成任务](#4-创建视频生成任务)
5. [任务状态查询与轮询](#5-任务状态查询与轮询)
6. [任务管理](#6-任务管理)
7. [素材上传与引用](#7-素材上传与引用)
8. [参数参考](#8-参数参考)
9. [内容审核机制](#9-内容审核机制)
10. [错误处理](#10-错误处理)
11. [速率限制与配额](#11-速率限制与配额)
12. [系统架构](#12-系统架构)
13. [最佳实践](#13-最佳实践)
14. [完整调用示例](#14-完整调用示例)

---

## 1. 接入概览

| 项目 | 值 |
|------|-----|
| **Base URL** | `http://185.200.65.233:5100` |
| **协议** | HTTP/1.1 |
| **认证方式** | Bearer Token (API Key) |
| **任务模式** | 异步（提交 → 轮询 → 获取结果） |
| **视频生成耗时** | 通常 2–5 分钟，高峰期可能更长 |

### 快速验证

```bash
# 健康检查（无需认证）
curl http://185.200.65.233:5100/ping
# → pong
```

---

## 2. 认证与鉴权

### API Key 格式

API Key 由平台方分配，格式为 `gw_` + 32 位十六进制字符（共 35 字符），例如：

```
gw_0e51468c39bc3cc3e604e474755a7b40
```

### 请求头

所有 API 请求（除 `/ping`）需携带：

```http
Authorization: Bearer <API_KEY>
```

### 认证错误

| HTTP 状态码 | 含义 |
|------------|------|
| `401` | 缺少 `Authorization` 头 / API Key 无效 |
| `403` | API Key 已禁用 / 已过期 / 缺少所需权限 |
| `429` | 速率限制超额 |

### 权限范围 (Scopes)

API Key 可配置以下权限：

| Scope | 说明 |
|-------|------|
| `video:create` | 创建视频生成任务（必需） |
| `admin` | 管理员权限（会话管理、Key 管理等） |

---

## 3. 可用模型

```bash
# 查询可用模型列表
curl http://185.200.65.233:5100/v1/models \
  -H 'Authorization: Bearer <API_KEY>'
```

**响应：**

```json
{
  "object": "list",
  "data": [
    { "id": "seedance-2.0", "object": "model" },
    { "id": "seedance-2.0-pro", "object": "model" },
    { "id": "seedance-2.0-fast", "object": "model" }
  ]
}
```

| 模型名 | 说明 | 生成速度 | 画质 |
|--------|------|---------|------|
| `seedance-2.0` | 标准版（等同 pro） | 较慢 | 高 |
| `seedance-2.0-pro` | 专业版 | 较慢 | 高 |
| `seedance-2.0-fast` | 快速版 | 快 | 标准 |

> **别名兼容**：`jimeng-video-seedance-2.0` 等价于 `seedance-2.0`，系统会自动映射。

---

## 4. 创建视频生成任务

**端点：** `POST /v1/videos/generations`

支持两种请求格式：JSON 和 Multipart。

### 4.1 JSON 格式（纯文本生成）

```bash
curl -X POST 'http://185.200.65.233:5100/v1/videos/generations' \
  -H 'Authorization: Bearer <API_KEY>' \
  -H 'Content-Type: application/json' \
  -d '{
    "prompt": "一只金色的柴犬在樱花树下奔跑，慢动作，电影感光影",
    "model": "seedance-2.0-fast",
    "duration": 5,
    "ratio": "16:9"
  }'
```

### 4.2 Multipart 格式（含素材上传）

```bash
curl -X POST 'http://185.200.65.233:5100/v1/videos/generations' \
  -H 'Authorization: Bearer <API_KEY>' \
  -F 'prompt=让@1中的人物缓慢转头微笑，电影感光影' \
  -F 'model=seedance-2.0' \
  -F 'duration=5' \
  -F 'ratio=16:9' \
  -F 'files=@/path/to/image.png'
```

### 成功响应 (HTTP 202 Accepted)

```json
{
  "code": 0,
  "message": "Task queued",
  "data": [{
    "task_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
    "status": "queued"
  }],
  "task": {
    "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
    "status": "queued",
    "poll_url": "/api/v1/tasks/a1b2c3d4-e5f6-7890-abcd-ef1234567890"
  }
}
```

---

## 5. 任务状态查询与轮询

### 5.1 查询单个任务

**端点：** `GET /api/v1/tasks/{task_id}`

```bash
curl 'http://185.200.65.233:5100/api/v1/tasks/<task_id>' \
  -H 'Authorization: Bearer <API_KEY>'
```

**响应示例（进行中）：**

```json
{
  "task": {
    "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
    "status": "polling",
    "model": "seedance-2.0-fast",
    "prompt": "一只金色的柴犬在樱花树下奔跑",
    "duration": 5,
    "ratio": "16:9",
    "queue_position": 156,
    "queue_total": 892,
    "queue_eta": "3m25s",
    "video_url": null,
    "error_message": null,
    "error_kind": null,
    "created_at": "2026-03-01 10:00:00",
    "updated_at": "2026-03-01 10:02:15",
    "started_at": "2026-03-01 10:00:02",
    "finished_at": null
  }
}
```

**响应示例（成功）：**

```json
{
  "task": {
    "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
    "status": "succeeded",
    "video_url": "https://v3-dreamnia.jimeng.com/obj/dreamina-sign/...",
    "finished_at": "2026-03-01 10:04:30"
  }
}
```

### 5.2 任务状态生命周期

```
queued → submitting → polling → downloading → succeeded
                  ↘              ↘                ↘
                   → failed       → failed         → failed
                                   → cancelled
```

| 状态 | 说明 |
|------|------|
| `queued` | 已入队，等待处理 |
| `submitting` | 正在提交到上游（含素材上传 + a_bogus 签名） |
| `polling` | 已提交成功，轮询上游生成进度 |
| `downloading` | 生成完成，正在获取高清视频 URL |
| `succeeded` | 任务完成，`video_url` 可用 |
| `failed` | 任务失败，见 `error_message` 和 `error_kind` |
| `cancelled` | 用户手动取消 |

### 5.3 推荐轮询策略

```
初始等待 3 秒
循环:
    GET /api/v1/tasks/{id}
    if status == "succeeded" → 下载 video_url
    if status == "failed"    → 读取 error_message / error_kind
    if status == "cancelled" → 结束
    等待 5 秒后重试
    最长轮询 10 分钟
```

### 5.4 队列进度字段

在 `polling` 状态时，任务会返回上游排队信息：

| 字段 | 类型 | 说明 |
|------|------|------|
| `queue_position` | `int` | 当前排队位置（如 156） |
| `queue_total` | `int` | 队列总长度（如 892） |
| `queue_eta` | `string` | 预计剩余等待时间（如 `"3m25s"`、`"1h20m"`） |

---

## 6. 任务管理

### 6.1 列出任务

**端点：** `GET /api/v1/tasks`

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `status` | `string` | - | 按状态筛选：`queued`/`polling`/`succeeded`/`failed`/`cancelled` |
| `limit` | `int` | 50 | 返回数量上限 |

```bash
# 列出最近失败的任务
curl 'http://185.200.65.233:5100/api/v1/tasks?status=failed&limit=10' \
  -H 'Authorization: Bearer <API_KEY>'
```

### 6.2 取消任务

**端点：** `POST /api/v1/tasks/{task_id}/cancel`

仅可取消 `queued`、`submitting`、`polling` 状态的任务。

```bash
curl -X POST 'http://185.200.65.233:5100/api/v1/tasks/<task_id>/cancel' \
  -H 'Authorization: Bearer <API_KEY>'
```

**成功响应：** `{ "ok": true }`

### 6.3 重试任务

**端点：** `POST /api/v1/tasks/{task_id}/retry`

克隆原始任务参数（含素材数据），创建新的排队任务。

```bash
curl -X POST 'http://185.200.65.233:5100/api/v1/tasks/<task_id>/retry' \
  -H 'Authorization: Bearer <API_KEY>'
```

**响应 (HTTP 201)：**

```json
{
  "task": {
    "id": "new-task-uuid",
    "status": "queued"
  },
  "retry_of": "original-task-uuid"
}
```

### 6.4 统计概览

**端点：** `GET /api/v1/stats`

```bash
curl 'http://185.200.65.233:5100/api/v1/stats' \
  -H 'Authorization: Bearer <API_KEY>'
```

**响应：**

```json
{
  "total": 1234,
  "queued": 5,
  "running": 2,
  "succeeded": 1100,
  "failed": 120,
  "cancelled": 7
}
```

---

## 7. 素材上传与引用

### 7.1 支持的素材类型

| 类型 | 支持格式 | 上传通道 | 说明 |
|------|---------|---------|------|
| **图片** | JPG, JPEG, PNG, WebP, GIF, BMP | ImageX | 生成图片驱动的视频 |
| **视频** | MP4, MOV, M4V | VOD | 视频到视频（风格转换等） |
| **音频** | MP3, WAV | VOD | 音频驱动视频（口型同步等） |

### 7.2 Multipart 上传

通过 `-F 'files=@path'` 附加素材文件，支持多文件：

```bash
curl -X POST 'http://185.200.65.233:5100/v1/videos/generations' \
  -H 'Authorization: Bearer <API_KEY>' \
  -F 'prompt=使用@1和@2素材，创建动态场景切换' \
  -F 'model=seedance-2.0' \
  -F 'duration=5' \
  -F 'ratio=16:9' \
  -F 'files=@/path/to/image1.png' \
  -F 'files=@/path/to/video1.mp4'
```

### 7.3 Prompt 中引用素材

在 prompt 中使用占位符引用上传的素材：

| 占位符格式 | 说明 |
|-----------|------|
| `@1`, `@2`, `@3` | 按上传顺序引用第 N 个素材 |
| `@图1`, `@图2` | 中文格式引用 |
| `@image1`, `@image2` | 英文格式引用 |

**示例：**

```
prompt: "让@1中的人物缓慢走向@2中的场景，电影感转场"
files: [portrait.png, background.jpg]
```

> **无占位符时：** 如果 prompt 中没有 `@` 占位符，系统会自动构建默认引用，将所有素材以 `"使用...素材，{prompt}"` 形式组合。

---

## 8. 参数参考

### 8.1 创建任务参数

| 参数 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| `prompt` | `string` | ✅ | - | 视频描述文本 |
| `model` | `string` | 否 | `seedance-2.0` | 模型名称 |
| `duration` | `int` | 否 | `4` | 视频时长（秒），支持 4、5、10 |
| `ratio` | `string` | 否 | `9:16` | 画面比例 |
| `files` | `file[]` | 否 | - | Multipart 素材文件 |

### 8.2 支持的比例与分辨率

所有模型固定使用 720p 分辨率，以下为各比例对应的像素尺寸：

| 比例 | 分辨率 (宽×高) | 典型用途 |
|------|-------------|---------|
| `1:1` | 720×720 | 方形，社交媒体 |
| `4:3` | 960×720 | 传统屏幕 |
| `3:4` | 720×960 | 竖屏适中 |
| `16:9` | 1280×720 | 横屏，YouTube |
| `9:16` | 720×1280 | 竖屏，抖音/TikTok |

---

## 9. 内容审核机制

> ⚠️ **重要**：上游（jimeng.jianying.com）实施三层服务端审核，Gateway 无法绕过。

### 9.1 三层审核架构

```
用户提交 prompt + 素材
        ↓
  ┌─────────────┐
  │  L1: 输入文本  │ ← 关键词/人名/IP 匹配，即时拦截
  └──────┬──────┘
         ↓ (通过)
  ┌─────────────┐
  │  L2: 输入图片  │ ← AI 人脸/版权内容识别
  └──────┬──────┘
         ↓ (通过)
    [视频生成中 2-5 分钟]
         ↓
  ┌─────────────┐
  │  L3: 输出视频  │ ← AI 视觉识别，扫描生成结果
  └──────┬──────┘
         ↓
    成功 / 拦截
```

### 9.2 各层审核详情

#### L1: 输入文本审核

- **检查时机**：任务提交时，几乎即时
- **检查内容**：prompt 文本中的敏感词、真人姓名、IP 角色名
- **触发示例**：
  - `"Taylor Swift singing on stage"` → ❌ 立即拦截
  - `"Spider-Man swinging through New York"` → ❌ 立即拦截
  - `"成龙在功夫电影中飞踢"` → ❌ 拦截（中文名检测略慢于英文）
- **错误标识**：`web_fail2generate_input_retry`、`web_text_violates_community_guidelines_toast`、`character_toast_sensitive_text`

#### L2: 输入图片审核

- **检查时机**：素材上传后
- **检查内容**：上传图片中的人脸、版权标识
- **错误标识**：`web_image_violates_community_guidelines_toast`、`inputimagerisk`

#### L3: 输出视频审核

- **检查时机**：视频生成完成后（2-5 分钟等待后）
- **检查内容**：AI 视觉识别，扫描生成的视频是否包含受保护的人物/角色形象
- **关键发现**：
  - 间接/委婉描述**能通过 L1**，但如果生成结果视觉上类似已知真人或 IP 角色，**L3 仍会拦截**
  - 例如 `"A blonde female pop singer in a sparkly dress performing on stage"` → L1 通过 → 生成视频 → **L3 拦截**
  - 例如 `"A superhero in red and blue suit with web patterns swinging between buildings"` → L1 通过 → 生成视频 → **L3 拦截**
- **写实风格均会触发 L3**：只要生成结果看起来像真实的受保护人物或角色，无论 prompt 多间接
- **错误标识**：`ErrMessage_APP_OutputVideoRisk`、`web_video_violates_community_guidelines_toast`、`outputvideorisk`

### 9.3 安全内容建议

✅ **安全**：
- 原创角色、风景、动物、抽象概念
- 通用场景描述（无指向特定真人/IP 的暗示）
- 使用自己拍摄的原始素材

❌ **会被拦截**：
- 任何真人名字（明星、政治人物等）
- 任何 IP 角色名（漫威、迪士尼等）
- 间接描述但视觉上仍能辨识的受保护形象
- 写实风格的名人/角色模仿

> **注意**：`safe_check: 0`、`skip_check: 1` 等参数对服务端审核**无效**，审核完全在上游服务端执行。

---

## 10. 错误处理

### 10.1 HTTP 层错误

| HTTP 状态码 | 含义 | 处理建议 |
|------------|------|---------|
| `401` | 认证失败 | 检查 API Key 和 Authorization 头格式 |
| `403` | 权限不足 / Key 过期/禁用 | 联系平台方 |
| `404` | 任务不存在 | 检查 task_id |
| `429` | 速率限制 | 读取 `X-RateLimit-Reset` 头，等待后重试 |
| `500` | 服务器内部错误 | 稍后重试 |

### 10.2 任务级错误分类 (`error_kind`)

任务失败时 `error_kind` 字段标识错误类别：

| error_kind | 含义 | 典型触发 | 建议操作 |
|-----------|------|---------|---------|
| `content_risk` | 内容审核不通过 | prompt/素材/输出触发审核 | 修改 prompt 后重试 |
| `account_blocked` | 上游账号被封禁 | 频繁触发审核或异常行为 | 等待平台方更换 session |
| `auth` | 上游认证失败 | session token 过期 | 等待平台方更新 session |
| `quota` | 上游配额耗尽 | 每日使用达到上游限额 | 次日重试 |
| `timeout` | 轮询超时 | 上游排队过长 | 稍后重试 |
| `generation_failed` | 生成失败（通用） | 上游内部错误 | 可直接重试 |
| `network` | 网络错误 | 连接上游失败 | 自动恢复，稍后重试 |
| `unknown` | 未分类错误 | - | 检查 `error_message` 详情 |

### 10.3 常见错误 `error_message` 示例

| 错误消息 | error_kind | 说明 |
|---------|-----------|------|
| `web_fail2generate_input_retry: ...` | `content_risk` | L1 文本审核拦截 |
| `web_text_violates_community_guidelines_toast: ...` | `content_risk` | L1 文本内容违规 |
| `ErrMessage_APP_OutputVideoRisk: 视频未通过审核` | `content_risk` | L3 输出视频审核拦截 |
| `web_notice_reach_daily_usage_limit: ...` | `quota` | 每日使用配额已满 |
| `Polling timed out after 600s` | `timeout` | 轮询等待超时 |
| `100402: ...` | `generation_failed` | 上游生成失败 |

### 10.4 可重试错误

以下 `error_kind` 值得重试：

| error_kind | 重试建议 |
|-----------|---------|
| `timeout` | ✅ 直接重试 |
| `generation_failed` | ✅ 直接重试（可换 seed） |
| `network` | ✅ 等待 10 秒后重试 |
| `content_risk` | ⚠️ 修改 prompt 后重试 |
| `quota` | ⏰ 次日重试 |
| `auth` / `account_blocked` | ❌ 不要重试，等平台方处理 |

---

## 11. 速率限制与配额

### 11.1 速率限制

API Key 可配置每分钟请求上限。超限时返回 `HTTP 429`。

**响应头：**

| Header | 说明 |
|--------|------|
| `X-RateLimit-Limit` | 每分钟允许的请求数 |
| `X-RateLimit-Remaining` | 剩余可用请求数 |
| `X-RateLimit-Reset` | 限制重置倒计时（秒） |

### 11.2 每日配额

API Key 可配置每日任务创建上限。超额时返回 `HTTP 429`：

```json
{
  "error": "Daily quota exceeded",
  "daily_quota": 100,
  "used": 100
}
```

---

## 12. 系统架构

### 12.1 请求处理流程

```
客户端 → POST /v1/videos/generations
           ↓
      [API Key 认证 + 速率限制检查]
           ↓
      [解析 prompt/model/duration/ratio + 保存素材]
           ↓
      [入队: status=queued]
           ↓ (Worker 取出)
      [上传素材到 ImageX/VOD]
           ↓
      [通过 headless Chromium 提交到 jimeng (a_bogus 签名)]
           ↓
      [轮询 get_history_by_ids 获取生成状态]
           ↓
      [获取 HQ 视频 URL]
           ↓
      [status=succeeded, video_url 可用]
```

### 12.2 并发与队列

- 系统使用异步任务队列，多 Worker 并发处理
- 并发数由 `CONCURRENCY` 环境变量控制
- 无可用上游 Session 时，任务自动重新入队等待

### 12.3 a_bogus 签名机制

上游 Seedance API 使用字节跳动 `bdms` 反爬虫 SDK，要求所有提交请求携带 `a_bogus` 签名参数。Gateway 通过内置的 headless Chromium 浏览器实例代理请求，由浏览器内的 `bdms` SDK 自动注入签名。

> 仅**任务提交**走浏览器代理；状态轮询和文件上传使用直接 HTTP 请求。

---

## 13. 最佳实践

### 13.1 轮询策略

```python
import time
import requests

def wait_for_video(base_url, task_id, api_key, timeout=600):
    """轮询任务状态直到完成或超时。"""
    headers = {"Authorization": f"Bearer {api_key}"}
    deadline = time.time() + timeout
    
    time.sleep(3)  # 初始等待
    
    while time.time() < deadline:
        resp = requests.get(f"{base_url}/api/v1/tasks/{task_id}", headers=headers)
        task = resp.json()["task"]
        
        if task["status"] == "succeeded":
            return task["video_url"]
        elif task["status"] == "failed":
            raise Exception(f"[{task['error_kind']}] {task['error_message']}")
        elif task["status"] == "cancelled":
            raise Exception("Task was cancelled")
        
        # 显示进度
        if task.get("queue_eta"):
            print(f"排队中... 位置 {task['queue_position']}/{task['queue_total']}, ETA: {task['queue_eta']}")
        
        time.sleep(5)
    
    raise TimeoutError("Polling timed out")
```

### 13.2 错误处理建议

1. **区分可重试与不可重试错误**：根据 `error_kind` 判断
2. **内容审核失败不要盲目重试**：修改 prompt 后再提交
3. **auth/account_blocked 不要重试**：联系平台方
4. **实现指数退避**：对 `network` 和 `timeout` 错误使用递增等待

### 13.3 Prompt 编写建议

- 使用具体、详细的描述：镜头运动、光影、情绪、速度
- 避免任何真人姓名或受版权保护的角色名
- 避免试图间接描述已知公众人物外貌特征
- 多用抽象/原创角色描述

---

## 14. 完整调用示例

### 14.1 纯文本生成（最简）

```bash
# 1. 创建任务
TASK_ID=$(curl -s -X POST 'http://185.200.65.233:5100/v1/videos/generations' \
  -H 'Authorization: Bearer <API_KEY>' \
  -H 'Content-Type: application/json' \
  -d '{"prompt":"一只橘猫在阳光下慵懒地打哈欠，浅景深特写","model":"seedance-2.0-fast","duration":5,"ratio":"16:9"}' \
  | jq -r '.task.id')

echo "Task ID: $TASK_ID"

# 2. 轮询状态
while true; do
  RESULT=$(curl -s "http://185.200.65.233:5100/api/v1/tasks/$TASK_ID" \
    -H 'Authorization: Bearer <API_KEY>')
  STATUS=$(echo $RESULT | jq -r '.task.status')
  echo "Status: $STATUS"
  
  if [ "$STATUS" = "succeeded" ]; then
    VIDEO_URL=$(echo $RESULT | jq -r '.task.video_url')
    echo "Video URL: $VIDEO_URL"
    break
  elif [ "$STATUS" = "failed" ]; then
    echo "Error: $(echo $RESULT | jq -r '.task.error_message')"
    break
  fi
  
  sleep 5
done
```

### 14.2 图片驱动视频

```bash
curl -X POST 'http://185.200.65.233:5100/v1/videos/generations' \
  -H 'Authorization: Bearer <API_KEY>' \
  -F 'prompt=让@1中的人物缓慢眨眼并微笑，镜头轻微推进' \
  -F 'model=seedance-2.0-pro' \
  -F 'duration=5' \
  -F 'ratio=9:16' \
  -F 'files=@portrait.png'
```

### 14.3 多素材组合

```bash
curl -X POST 'http://185.200.65.233:5100/v1/videos/generations' \
  -H 'Authorization: Bearer <API_KEY>' \
  -F 'prompt=将@1的人物放入@2的场景中，人物缓缓走向镜头' \
  -F 'model=seedance-2.0' \
  -F 'duration=5' \
  -F 'ratio=16:9' \
  -F 'files=@person.png' \
  -F 'files=@background.jpg'
```

### 14.4 重试失败任务

```bash
# 用原始参数创建新任务
curl -X POST "http://185.200.65.233:5100/api/v1/tasks/<failed_task_id>/retry" \
  -H 'Authorization: Bearer <API_KEY>'
```

---

## API 端点汇总

| 方法 | 端点 | 说明 | 认证 |
|------|------|------|------|
| `GET` | `/ping` | 健康检查 | ❌ |
| `GET` | `/v1/models` | 模型列表 | ✅ |
| `POST` | `/v1/videos/generations` | 创建视频生成任务 | ✅ (scope: `video:create`) |
| `GET` | `/api/v1/tasks` | 列出任务 | ✅ |
| `GET` | `/api/v1/tasks/{id}` | 查询单个任务 | ✅ |
| `POST` | `/api/v1/tasks/{id}/cancel` | 取消任务 | ✅ |
| `POST` | `/api/v1/tasks/{id}/retry` | 重试任务 | ✅ |
| `GET` | `/api/v1/stats` | 任务统计 | ✅ |
