# Grok2API Go

`grok2api-go` 是一个用 Go 重写的 Grok API 代理网关，目标是保持与原版 `grok2api` 的 API 兼容，并提供更清晰的部署与运行时边界。

## 项目概述

- 语言与运行时：Go 1.24
- 协议目标：兼容 OpenAI 风格接口，覆盖 chat、image、video、responses 等能力
- 核心技术：`net/http`、`utls`、WebSocket、TOML 配置、本地 JSON 存储
- 部署方式：手动 Docker 部署，对外服务域名为 `grok.xiaomao.chat`

## 当前结构

项目当前按独立 Go 服务工程组织，核心目录包括：

- `cmd/server/`：服务入口
- `internal/`：配置、鉴权、中间件、token 池、reverse 客户端与业务服务
- `api/`：OpenAI 兼容 API 与管理接口
- `pkg/sse/`：SSE 流式输出工具

## 配置

默认配置见 [`config.defaults.toml`](./config.defaults.toml)，主要覆盖：

- 应用访问与鉴权配置
- 代理与 Cloudflare 刷新配置
- 重试策略与 session 重建策略
- token 池刷新、保存与使用量同步
- chat、image、video、asset、nsfw、usage 等运行参数

## 部署

仓库内提供 [`Dockerfile`](./Dockerfile) 与 [`docker-compose.yml`](./docker-compose.yml)。当前服务采用手动 Docker 方式部署，不走本仓库其他 Rust 服务使用的 GitHub Actions 原生构建流程。

## 参考文档

- [`PLAN.md`](./PLAN.md)：重写范围、架构决策与分阶段实施计划
- [`config.defaults.toml`](./config.defaults.toml)：默认配置模板
