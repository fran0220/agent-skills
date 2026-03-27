# jimeng-gateway — Agent Instructions

## Project Overview

Video generation gateway for Jimeng/Seedance API. Rust backend (axum) + React frontend, deployed natively on jpdata server via systemd. Direct API integration with jimeng.jianying.com — no upstream Node.js dependency.

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Backend | Rust 2024 edition, axum 0.8, tokio, sqlx (SQLite) |
| Frontend | React 19, Vite, TailwindCSS |
| Auth | OIDC SSO (Rauthy) + API key auth |
| Browser | chromiumoxide (headless Chromium via CDP) for a_bogus signing |
| Runtime | systemd service, native binary on jpdata |
| Deployment | GitHub Actions CI → SSH → native build on server |
| Dependencies | rauthy (Docker for OIDC) |

## Project Structure

```
skill/                       # Agent 技能 (SKILL.md + knowledge/)
cli/                         # Rust 后端
├── src/
│   ├── main.rs              # Entry point, server setup
│   ├── config.rs            # Environment config parsing
│   ├── auth/                # OIDC SSO + API key middleware
│   ├── db/                  # SQLite schema, migrations, queries
│   ├── jimeng/              # Direct jimeng.jianying.com API integration
│   │   ├── auth.rs          # Cookie generation, Sign header, request headers
│   │   ├── browser.rs       # Headless Chromium for a_bogus signing (Seedance)
│   │   ├── models.rs        # Model mappings, resolution tables, material types
│   │   ├── poll.rs          # Poll /mweb/v1/get_history_by_ids for results
│   │   ├── submit.rs        # Submit via /mweb/v1/aigc_draft/generate (browser proxy)
│   │   └── upload.rs        # ImageX (images) + VOD (video/audio) upload with AWS4 signing
│   ├── pool/                # Session pool (LRU rotation, health check)
│   ├── queue/               # Async task queue (submit, poll, cancel, worker)
│   └── routes/              # Axum route handlers (admin, compat, public)
├── scripts/
│   ├── deploy.sh            # Server-side deploy script (git pull → build → restart)
│   └── jimeng-gateway.service  # systemd unit file
├── jimeng-free-api-fork/    # Original Node.js reference implementation (read-only reference)
├── docker-compose.yml       # Only for rauthy (NOT gateway)
├── Cargo.toml
└── Cargo.lock
web/                         # React 前端
├── src/                     # React dashboard
├── vite.config.js
└── tailwind.config.js
```

## Conventions

### Rust
- Use `anyhow::Result` for application errors, `thiserror` for library-style typed errors
- All async functions use tokio runtime
- Config via `dotenvy` + environment variables (see `.env.example`)
- Database queries use sqlx with compile-time checked SQL where possible
- Logging: `tracing` crate with `RUST_LOG` env filter

### Frontend
- React functional components with hooks
- TailwindCSS for styling, no CSS modules
- Vite dev server proxies `/api` to backend

## Jimeng API Integration

### Architecture
All video generation goes through direct API calls to jimeng.jianying.com:
1. **Submit**: POST `/mweb/v1/aigc_draft/generate` (via headless Chromium for a_bogus injection)
2. **Poll**: POST `/mweb/v1/get_history_by_ids` (direct HTTP with Sign header)
3. **HQ Download**: POST `/mweb/v1/get_local_item_list` (get high-quality video URL)
4. **Upload**: ImageX (`imagex.bytedanceapi.com`) for images, VOD (`vod.bytedanceapi.com`) for video/audio

### a_bogus Signing (Shark Anti-Crawl)
- Seedance `/mweb/v1/aigc_draft/generate` requires `a_bogus` signature from ByteDance `bdms` SDK
- `BrowserService` (chromiumoxide) launches headless Chromium, navigates to jimeng.jianying.com to load bdms SDK
- The SDK hooks `window.fetch` and auto-injects `a_bogus` into all requests
- Per-session browser pages with 10-minute idle timeout
- Only submit requests go through browser; poll/upload use direct HTTP

### Models
| User Model | Internal Key | Benefit Type |
|-----------|-------------|-------------|
| `seedance-2.0` / `seedance-2.0-pro` | `dreamina_seedance_40_pro` | `dreamina_video_seedance_20_pro` |
| `seedance-2.0-fast` | `dreamina_seedance_40` | `dreamina_seedance_20_fast` |

### Poll Status Codes
| Code | Meaning |
|------|---------|
| 20 | Pending (queued/generating) |
| 30 | Failed |
| 50 | Succeeded |

### Error Classification (fail_starling_key from frontend i18n)
| Category | Keys / Patterns | Action |
|----------|----------------|--------|
| `content_risk` | `violates_community_guidelines`, `violate_guidelines`, `sensitive_text`, `inputtextrisk`, `inputimagerisk`, `outputimagerisk`, `outputvideorisk`, `content_violation`, fail_code 2038/2039/2040 | Normal failure |
| `account_blocked` | `account_block`, `risk_notification`, `risk_control` | Mark session unhealthy |
| `auth` | `authorization`, `unauthorized`, `login`, `token` | Mark session unhealthy |
| `quota` | `daily_usage_limit` | Normal failure |
| `timeout` | `timeout`, `timed out` | Normal failure |
| `generation_failed` | fail_code 100402, `generation_failed` | Normal failure |
| `network` | `network`, `econnrefused` | Normal failure |

### Content Moderation Stages
1. **Input text check** — prompt text scanned for sensitive/copyright/real person keywords
2. **Input image check** — uploaded images scanned for faces/copyright content
3. **Output video check** — generated video post-scanned for violations
- fail_starling_key values: `web_text_violates_community_guidelines_toast`, `web_image_violates_community_guidelines_toast`, `web_video_violates_community_guidelines_toast`, `web_prompt_violate_guidelines_tip`, `character_toast_sensitive_text`

### Upload Channels
- **ImageX** (scene=2): `get_upload_token` → `ApplyImageUpload` → binary upload → `CommitImageUpload`, returns URI
- **VOD** (scene=1): `get_upload_token` → `ApplyUploadInner` → binary upload → `CommitUploadInner`, returns vid + metadata
- Both use AWS4-HMAC-SHA256 signing, region `cn-north-1`

## Deployment

Gateway runs as a **native binary** managed by systemd. No Docker for gateway itself.

- **Target**: jpdata (185.200.65.233, Ubuntu 22.04, 4 cores, 8GB RAM)
- **Runtime**: systemd service (`jimeng-gateway.service`)
- **CI**: GitHub Actions → SSH → `scripts/deploy.sh` (git pull → cargo build --release → npm build → systemctl restart)
- **Secrets**: `JPDATA_SSH_KEY` (GitHub repo secret, already configured)
- **Server path**: `/opt/jimeng-gateway`
- **Rust**: 1.93.1 via rustup (`~/.cargo/env`)
- **Node.js**: v22 (system install)
- **Logs**: `journalctl -u jimeng-gateway -f`

### Manual deploy
```bash
ssh jpdata "bash /opt/jimeng-gateway/scripts/deploy.sh"
```

### Service management
```bash
ssh jpdata "systemctl status jimeng-gateway"
ssh jpdata "journalctl -u jimeng-gateway -n 100 -f"
ssh jpdata "systemctl restart jimeng-gateway"
```

## Key Environment Variables

| Variable | Description |
|----------|-------------|
| `PORT` | Gateway listen port (default: 5100) |
| `DATABASE_URL` | SQLite connection string |
| `CONCURRENCY` | Max concurrent video generation tasks |
| `AUTH_ENABLED` | Enable API key authentication |
| `OIDC_ISSUER_URL` | OIDC provider URL for SSO |
| `CHROMIUM_PATH` | Optional path to Chromium binary for BrowserService |
| `POLL_INTERVAL_SECS` | Polling interval for task status (default: 5) |
| `MAX_POLL_DURATION_SECS` | Max polling duration before timeout (default: 600) |

## Important Notes

- **Gateway 不使用 Docker**，以 systemd 原生服务运行
- rauthy 仍以 Docker 容器运行（`docker-compose.yml` 只管这个）
- SQLite DB 文件在 `/opt/jimeng-gateway/data/`，systemd 服务有 ReadWritePaths 权限
- `.env` 文件在服务器上手动管理，不进 git
- 服务器还运行 `frps` 容器（与本项目无关，勿动）
- Dockerfile 已移除，不再需要
- `jimeng-free-api-fork/` 是原始 Node.js 参考实现，仅供参考阅读，不再作为上游依赖
