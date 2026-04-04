# Grok2API Go Rewrite — Master Plan

## 项目概述

将 grok2api（Python/FastAPI）重写为 Go，保持 100% API 兼容。
原项目 ~20K LOC Python，预计 Go 版本 ~15K LOC。

## 架构决策

| 层 | Python 原版 | Go 选型 | 理由 |
|---|---|---|---|
| HTTP 框架 | FastAPI + Granian | net/http (stdlib) | 足够强大，零依赖 |
| JSON | orjson | encoding/json + jsoniter | 性能+兼容 |
| TLS 指纹 | curl_cffi (chrome136) | utls + net/http roundtripper | 原生 Go TLS fingerprint |
| WebSocket | aiohttp | gorilla/websocket | 成熟稳定 |
| 配置 | TOML (tomllib) | BurntSushi/toml | Go 标准 TOML |
| 日志 | 自定义 logger | slog (stdlib) | Go 1.21+ 内置 |
| 代理 | SOCKS5 via curl_cffi | golang.org/x/net/proxy | 标准库扩展 |
| 存储 | Local JSON / Redis / MySQL / PgSQL | 同（但 Phase 1 仅 Local JSON） |
| 并发 | asyncio.Semaphore | channel-based semaphore | goroutine 原生 |

## 目录结构

```
grok2api-go/
├── cmd/
│   └── server/
│       └── main.go                 # 入口
├── internal/
│   ├── config/
│   │   ├── config.go               # TOML 配置加载
│   │   └── defaults.go             # 默认配置值
│   ├── auth/
│   │   └── auth.go                 # API Key / App Key 验证中间件
│   ├── model/
│   │   └── model.go                # ModelInfo, ModelService (模型路由)
│   ├── token/
│   │   ├── models.go               # TokenInfo, TokenStatus, EffortType
│   │   ├── pool.go                 # TokenPool (选择/管理)
│   │   ├── manager.go              # TokenManager (单例，持久化，刷新)
│   │   └── scheduler.go            # 定时刷新调度
│   ├── reverse/
│   │   ├── client.go               # TLS fingerprint HTTP client (utls)
│   │   ├── headers.go              # Header 构建 (SSO Cookie, Client Hints, Statsig)
│   │   ├── statsig.go              # Statsig ID 生成器
│   │   ├── session.go              # ResettableSession (auto-reset on 403)
│   │   ├── retry.go                # RetryContext, retry_on_status
│   │   ├── appchat.go              # /rest/app-chat/conversations/new
│   │   ├── mediapost.go            # /rest/media/post/create
│   │   ├── mediapostlink.go        # Media post link (公开资产)
│   │   ├── ratelimits.go           # Rate limits 查询
│   │   ├── upload.go               # assets.grok.com 上传
│   │   ├── download.go             # 资产下载
│   │   ├── websocket.go            # WebSocket 客户端
│   │   ├── wsimagine.go            # Imagine WebSocket (图片)
│   │   ├── videoupscale.go         # 视频超分
│   │   ├── nsfw.go                 # NSFW 管理
│   │   ├── accepttos.go            # 接受 ToS
│   │   └── setbirth.go             # 设置生日
│   ├── service/
│   │   ├── chat.go                 # ChatService + SSE 流处理
│   │   ├── image.go                # ImageGenerationService
│   │   ├── imageedit.go            # ImageEditService
│   │   ├── video.go                # VideoService (多轮生成+扩展)
│   │   ├── voice.go                # VoiceService (LiveKit)
│   │   ├── responses.go            # Responses API
│   │   └── stream.go               # 流式通用工具
│   ├── proxy/
│   │   └── pool.go                 # 代理池 (sticky + failover)
│   ├── storage/
│   │   ├── storage.go              # Storage 接口
│   │   └── local.go                # Local JSON 存储
│   └── middleware/
│       ├── cors.go                 # CORS
│       ├── logger.go               # 请求日志
│       └── requestid.go            # Request ID
├── api/
│   ├── router.go                   # 路由注册
│   ├── chat.go                     # POST /v1/chat/completions
│   ├── image.go                    # POST /v1/images/generations
│   ├── video.go                    # POST /v1/video/*
│   ├── models.go                   # GET /v1/models
│   ├── files.go                    # 文件服务
│   ├── response.go                 # Responses API
│   └── admin/
│       ├── token.go                # Token 管理
│       ├── config.go               # 配置管理
│       └── cache.go                # 缓存管理
├── pkg/
│   └── sse/
│       └── writer.go               # SSE 流式写入工具
├── config.defaults.toml            # 复用原版默认配置
├── Dockerfile
├── docker-compose.yml
├── go.mod
├── go.sum
└── PLAN.md
```

## 分阶段实施

### Phase 1: 骨架 + Chat Completions（可独立测试）

**目标**：能跑通一个 streaming chat completions 请求

包含模块：
1. **P1-A: 项目骨架 + 配置 + 模型**
   - `cmd/server/main.go` — HTTP server 启动
   - `internal/config/` — TOML 配置加载，默认值
   - `internal/model/` — ModelInfo + ModelService
   - `internal/auth/` — Bearer token 验证中间件
   - `internal/middleware/` — CORS, RequestID, Logger
   - `api/router.go` — 路由注册骨架
   - `api/models.go` — GET /v1/models
   - `go.mod` + `config.defaults.toml` + `Dockerfile`

2. **P1-B: Token 池系统**
   - `internal/token/models.go` — TokenInfo, TokenStatus, EffortType, 消耗逻辑
   - `internal/token/pool.go` — TokenPool (select: quota 模式 + consumed 模式)
   - `internal/token/manager.go` — TokenManager 单例 (加载/保存/消耗/刷新)
   - `internal/storage/` — Storage 接口 + Local JSON 实现

3. **P1-C: Reverse 层 (TLS + Headers + AppChat)**
   - `internal/reverse/client.go` — utls TLS fingerprint HTTP client
   - `internal/reverse/headers.go` — build_headers, build_sso_cookie, client hints
   - `internal/reverse/statsig.go` — Statsig ID 生成
   - `internal/reverse/session.go` — ResettableSession
   - `internal/reverse/retry.go` — RetryContext, retry_on_status
   - `internal/reverse/appchat.go` — AppChatReverse.Request()
   - `internal/proxy/pool.go` — 代理池

4. **P1-D: Chat Service + API**
   - `internal/service/chat.go` — GrokChatService + ChatStreamProcessor
   - `internal/service/stream.go` — wrap_stream_with_usage
   - `pkg/sse/writer.go` — SSE 编码器
   - `api/chat.go` — POST /v1/chat/completions (stream + non-stream)

### Phase 2: Image + Video

5. **P2-A: Image 服务**
   - `internal/reverse/websocket.go` — WebSocket 客户端
   - `internal/reverse/wsimagine.go` — ImagineWebSocketReverse
   - `internal/reverse/upload.go` — AssetsUploadReverse
   - `internal/reverse/download.go` — 资产下载
   - `internal/service/image.go` — ImageGenerationService
   - `internal/service/imageedit.go` — ImageEditService
   - `api/image.go` — POST /v1/images/generations
   - `api/files.go` — 文件服务

6. **P2-B: Video 服务**
   - `internal/reverse/mediapost.go` — MediaPostReverse
   - `internal/reverse/mediapostlink.go` — 公开资产
   - `internal/reverse/videoupscale.go` — 视频超分
   - `internal/service/video.go` — VideoService (多轮+扩展+超分)
   - `api/video.go` — Video API

### Phase 3: 管理 + 完善

7. **P3-A: Admin + 调度**
   - `api/admin/` — Token 管理 API, 配置 API, 缓存 API
   - `internal/token/scheduler.go` — 定时刷新调度
   - `internal/reverse/ratelimits.go` — Rate limits
   - `internal/reverse/nsfw.go` — NSFW 批量管理
   - `internal/reverse/accepttos.go` + `setbirth.go`

8. **P3-B: Responses API + Voice + Function**
   - `internal/service/responses.go`
   - `internal/service/voice.go`
   - Responses API 路由

## 关键技术要点

### TLS 指纹 (最关键)

```go
import (
    tls "github.com/refraction-networking/utls"
    "net/http"
    "golang.org/x/net/http2"
)

// Chrome 136 指纹
spec := tls.ClientHelloID{
    Client:  "Chrome",
    Version: "136",
}
```

需要自定义 `http.RoundTripper` 通过 utls 握手后桥接到标准 `net/http`。

### SSE 流式输出

```go
func streamSSE(w http.ResponseWriter, ch <-chan string) {
    flusher, _ := w.(http.Flusher)
    w.Header().Set("Content-Type", "text/event-stream")
    w.Header().Set("Cache-Control", "no-cache")
    w.Header().Set("Connection", "keep-alive")
    for chunk := range ch {
        fmt.Fprintf(w, "data: %s\n\n", chunk)
        flusher.Flush()
    }
}
```

### Token 池选择 (保持与 Python 版完全一致)

- 默认模式：选 quota 最高的 active token
- Consumed 模式：选 consumed 最低的 active token
- 排除已试过的 token，支持 tag 偏好

### Grok App-Chat 流解析

逐行读取 ndjson → 解析 `result.response` → 提取 token/isThinking/modelResponse/streamingImageGenerationResponse → 转换为 OpenAI SSE 格式

## 并行实施分配

| Thread | 任务 | 依赖 | 预计 LOC |
|--------|------|------|---------|
| Thread 1 | P1-A: 骨架+配置+模型+中间件+路由 | 无 | ~800 |
| Thread 2 | P1-B: Token 池系统+存储 | 无 | ~1200 |
| Thread 3 | P1-C: Reverse 层 (TLS+Headers+AppChat) | 无 | ~1000 |
| Thread 4 | P1-D: Chat Service + API（等 1,2,3 完成后）| P1-A,B,C | ~1500 |

Phase 1 完成后可以本地 `go run` 测试 chat completions。

Phase 2/3 在 Phase 1 验证后启动。
