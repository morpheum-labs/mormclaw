# Gateway API Paths Reference

Canonical list of HTTP endpoints exposed by the ZeroClaw gateway. Source: `src/gateway/mod.rs`.

Last refreshed: **March 9, 2026**.

---

## Health & Metrics

| Method | Path | Handler | Auth | Description |
|--------|------|---------|------|-------------|
| GET | `/health` | `handle_health` | No | Basic liveness check (includes `auth_mode` when `require_pairing`) |
| GET | `/metrics` | `handle_metrics` | No | Prometheus metrics |

---

## Pairing

| Method | Path | Handler | Auth | Description |
|--------|------|---------|------|-------------|
| POST | `/pair` | `handle_pair` | No | Pair device with `X-Pairing-Code` header |
| POST | `/pairing/regenerate` | `handle_pairing_regenerate` | No | Regenerate pairing code (localhost only) |

---

## TOTP Auth (when `gateway.totp_login_enabled = true`)

| Method | Path | Handler | Auth | Description |
|--------|------|---------|------|-------------|
| GET | `/auth/totp/enroll` | `handle_auth_totp_enroll` | No | Get otpauth URI for QR (localhost only) |
| POST | `/auth/totp` | `handle_auth_totp` | No | Exchange TOTP code for bearer token |

---

## Webhooks (Channel Inbound)

| Method | Path | Handler | Auth | Description |
|--------|------|---------|------|-------------|
| GET | `/webhook` | `handle_webhook_usage` | No | Webhook usage info |
| POST | `/webhook` | `handle_webhook` | Yes | Generic webhook |
| GET | `/whatsapp` | `handle_whatsapp_verify` | No | WhatsApp webhook verification |
| POST | `/whatsapp` | `handle_whatsapp_message` | No | WhatsApp inbound messages |
| POST | `/linq` | `handle_linq_webhook` | No | Linq webhook |
| POST | `/github` | `handle_github_webhook` | No | GitHub webhook |
| POST | `/bluebubbles` | `handle_bluebubbles_webhook` | No | BlueBubbles (iMessage) webhook |
| GET | `/wati` | `handle_wati_verify` | No | WATI webhook verification |
| POST | `/wati` | `handle_wati_webhook` | No | WATI inbound messages |
| POST | `/nextcloud-talk` | `handle_nextcloud_talk_webhook` | No | Nextcloud Talk webhook |
| POST | `/qq` | `handle_qq_webhook` | No | QQ webhook |

---

## Chat & Completions

| Method | Path | Handler | Auth | Description |
|--------|------|---------|------|-------------|
| POST | `/api/chat` | `handle_api_chat` | Bearer | ZeroClaw-native chat (tools + memory) |
| POST | `/v1/chat/completions` | `handle_v1_chat_completions_with_tools` | Bearer | OpenAI-compatible chat (tools + memory) |
| GET | `/v1/models` | `handle_v1_models` | Bearer | List available models |

---

## Web Dashboard API (Bearer token required)

| Method | Path | Handler | Description |
|--------|------|---------|-------------|
| GET | `/api/status` | `handle_api_status` | Runtime status |
| GET | `/api/config` | `handle_api_config_get` | Read config |
| PUT | `/api/config` | `handle_api_config_put` | Write config (1MB body limit) |
| GET | `/api/tools` | `handle_api_tools` | List registered tools |
| GET | `/api/cron` | `handle_api_cron_list` | List cron jobs |
| POST | `/api/cron` | `handle_api_cron_add` | Add cron job |
| DELETE | `/api/cron/{id}` | `handle_api_cron_delete` | Delete cron job |
| GET | `/api/integrations` | `handle_api_integrations` | List integrations |
| GET | `/api/integrations/settings` | `handle_api_integrations_settings` | Integration settings |
| PUT | `/api/integrations/{id}/credentials` | `handle_api_integrations_credentials_put` | Update integration credentials |
| GET | `/api/doctor` | `handle_api_doctor` | Health diagnostics |
| POST | `/api/doctor` | `handle_api_doctor` | Run health diagnostics |
| GET | `/api/memory` | `handle_api_memory_list` | List memory entries |
| POST | `/api/memory` | `handle_api_memory_store` | Store memory |
| DELETE | `/api/memory/{key}` | `handle_api_memory_delete` | Delete memory by key |
| GET | `/api/pairing/devices` | `handle_api_pairing_devices` | List paired devices |
| DELETE | `/api/pairing/devices/{id}` | `handle_api_pairing_device_revoke` | Revoke paired device |
| GET | `/api/cost` | `handle_api_cost` | Cost tracking |
| GET | `/api/cli-tools` | `handle_api_cli_tools` | CLI tools info |
| GET | `/api/health` | `handle_api_health` | Detailed health |
| POST | `/api/node-control` | `handle_node_control` | Experimental node-control RPC |

---

## Streaming & WebSocket

| Method | Path | Handler | Auth | Description |
|--------|------|---------|------|-------------|
| GET | `/api/events` | `handle_sse_events` | Bearer | SSE event stream |
| GET | `/ws/chat` | `handle_ws_chat` | Bearer | WebSocket agent chat |

---

## Static Assets

| Method | Path | Handler | Auth | Description |
|--------|------|---------|------|-------------|
| GET | `/_app/{*path}` | `handle_static` | No | Web dashboard static files |
| GET | `/*` (fallback) | `handle_spa_fallback` | No | SPA index.html for non-API GET |

---

## Auth Summary

- **No auth**: `/health`, `/metrics`, `/pair`, `/pairing/regenerate`, webhook verification (GET), channel webhooks (POST), `/_app/*`, SPA fallback
- **Bearer token**: All `/api/*` and `/v1/*` endpoints
- **Webhook auth**: Channel-specific (e.g. signature verification for GitHub, WhatsApp)

---

## Related Docs

- [OpenClaw Migration Guide](migration/openclaw-migration-guide.md) — `/api/chat` and `/v1/chat/completions` request/response formats
- [Channels Reference](channels-reference.md) — Webhook endpoint usage per channel
- [Config Reference](config-reference.md) — `gateway.port`, `gateway.node_control.enabled`, etc.
