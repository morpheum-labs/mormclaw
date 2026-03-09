# TOTP Gateway Login — Design Document

**Status:** Proposal  
**Date:** 2025-03-10  
**Scope:** Web dashboard authentication via TOTP (optional alternative to pairing code)

---

## 1. Summary

Add an optional TOTP-based login flow for the web dashboard. When enabled and configured:

1. **First-time setup**: User scans QR code, confirms with first OTP code.
2. **Subsequent logins**: User enters 6-digit TOTP code instead of pairing code.
3. **Post-auth**: Same bearer-token flow as pairing — no change to authenticated experience.

---

## 2. Design Principles (KISS, DRY, SRP)

| Principle | Application |
|-----------|-------------|
| **KISS** | Reuse existing `OtpValidator` and `otp-secret` storage. No new crypto or secret management. |
| **DRY** | Single TOTP secret for both (a) agent action gating and (b) gateway login when both enabled. |
| **SRP** | Gateway auth layer stays separate from agent OTP gating; shared validator, distinct use cases. |
| **Fail fast** | Explicit errors when TOTP login enabled but secret missing; no silent fallback. |
| **Secure by default** | TOTP login opt-in; pairing remains available as fallback/parallel path. |

---

## 3. Config Schema

### 3.1 Extend `[gateway]`

```toml
[gateway]
require_pairing = true
# New: when true, TOTP login is offered (enrollment or OTP input per auth mode matrix)
totp_login_enabled = false   # default: false (opt-in)
```

### 3.2 Reuse `[security.otp]`

- **Secret**: Same `otp-secret` file, same `OtpValidator`.
- **Independent**: `security.otp.enabled` gates agent actions; `gateway.totp_login_enabled` gates web login. Do **not** require `security.otp.enabled` for gateway TOTP (see §11.5).
- **OtpConfig**: Use `config.security.otp` for `token_ttl_secs`, etc. Always present with defaults.

### 3.3 Auth Mode Matrix

| `require_pairing` | `totp_login_enabled` | Secret exists | Auth mode |
|-------------------|---------------------|---------------|-----------|
| false | * | * | No auth (current) |
| true | false | * | Pairing only (current) |
| true | true | no | **Enrollment** — show QR, user confirms |
| true | true | yes | **TOTP login** — show OTP input |

---

## 4. API Contract

### 4.1 Extend `GET /health` (public)

Add fields for frontend to choose which screen to show:

```json
{
  "status": "ok",
  "require_pairing": true,
  "paired": true,
  "auth_mode": "totp"
}
```

| Field | Type | When present |
|-------|------|--------------|
| `auth_mode` | `"pairing"` \| `"totp"` \| `"totp_enrollment"` | When `require_pairing` is true |

**Note:** Do **not** return `totp_enrollment_uri` from `/health`. The enrollment URI is obtained by calling `GET /auth/totp/enroll` (which creates the secret on first call). See §11.1.

### 4.2 New: `GET /auth/totp/enroll` (no auth, localhost only)

Returns enrollment data for QR code. **Localhost only** (same as `/pairing/regenerate`).

```json
{
  "otpauth_uri": "otpauth://totp/ZeroClaw:zeroclaw?secret=...&issuer=ZeroClaw&period=30",
  "qr_data_url": "data:image/png;base64,..."
}
```

- If secret does not exist: `OtpValidator::from_config` creates it, returns URI.
- If secret exists: loads validator, returns `validator.otpauth_uri()`. Supports "Set up authenticator" flow on TotpLoginScreen (see §11.10).
- `qr_data_url`: optional; frontend can generate QR from `otpauth_uri` if preferred.

### 4.3 New: `POST /auth/totp` (public)

Exchange TOTP code for bearer token. Same rate limiting as `/pair`.

**Request:**
- Header: `X-TOTP-Code: 123456`
- Or body: `{"code": "123456"}` (Content-Type: application/json)

**Response (success):**
```json
{
  "token": "zc_...",
  "message": "Use Authorization: Bearer <token>"
}
```

**Response (failure):**
- 403: `{"error": "Invalid TOTP code"}`
- 429 (rate limited): `{"error": "...", "retry_after": N}` — same as `/pair`

**Flow:**
1. Validate code via `OtpValidator::validate`.
2. On success: generate bearer token (same as pairing), add to `PairingGuard.paired_tokens`, persist, return token.
3. On failure: return 403, no lockout (TOTP codes expire naturally).

---

## 5. Gateway Implementation

### 5.1 AppState Additions

```rust
// Optional: only Some when totp_login_enabled and secret already exists
pub totp_validator: Option<Arc<OtpValidator>>,
```

- Construct during gateway startup **only when** `totp_login_enabled` and secret file exists. Use `secret_file_path(config_dir).exists()` to check. Do not create secret at startup.
- `config_dir`: `config.config_path.parent()` (must match agent; see §11.3).
- `SecretStore`: `SecretStore::new(config_dir, config.secrets.encrypt)`.
- When in enrollment mode (secret does not exist), `totp_validator` is `None`. Handlers create `OtpValidator` on demand for `GET /auth/totp/enroll` and `POST /auth/totp` (see §11.2).

### 5.2 Handler Responsibilities

| Handler | Responsibility |
|---------|----------------|
| `handle_health` | Set `auth_mode` from state (no `totp_enrollment_uri`). |
| `handle_auth_totp_enroll` | Create OtpValidator (creates secret if missing), return `otpauth_uri`. Localhost only. |
| `handle_auth_totp` | Validate code via OtpValidator, issue token via PairingGuard, persist. |

### 5.3 Token Issuance (DRY)

Extract shared logic: "generate token, add to PairingGuard, persist". Both `handle_pair` and `handle_auth_totp` use it.

Add `PairingGuard::issue_token(source: &str) -> String` that:
1. Generates bearer token (reuse existing `generate_token()`)
2. Hashes and inserts into `paired_tokens` and `paired_device_meta` with `paired_by: Some(source)`
3. Returns the plaintext token

- `handle_pair`: continues to use `try_pair` (which does its own generation). No change.
- `handle_auth_totp`: on valid TOTP, calls `pairing.issue_token("totp")`, persists, returns token.

Alternatively, refactor `handle_pair` to use a shared helper that both paths call. Prefer minimal change: add `issue_token` for TOTP; leave `try_pair` as-is for pairing.

---

## 6. Frontend Changes

> **See also:** [totp-gateway-login-ui-design.md](totp-gateway-login-ui-design.md) for the full UI proposal.

### 6.1 Auth Mode Selection (useAuth)

```ts
// getPublicHealth returns auth_mode (undefined for older gateways)
if (!health.require_pairing)               → auto-authenticate (current)
if (health.auth_mode === 'totp_enrollment') → show EnrollmentScreen
if (health.auth_mode === 'totp')           → show TotpLoginScreen
if (health.auth_mode === 'pairing' || !health.auth_mode) → show PairingDialog (current, fallback)
```

### 6.2 New Screens

| Screen | When | Actions |
|--------|------|---------|
| **EnrollmentScreen** | `auth_mode = "totp_enrollment"` | Call `GET /auth/totp/enroll` for URI, display QR, input for first code, POST to `/auth/totp` to confirm. On success → store token, proceed. Include "Use pairing code instead" link. |
| **TotpLoginScreen** | `auth_mode = "totp"` | 6-digit input, POST to `/auth/totp`. On success → store token, proceed. Include "Use pairing code instead" and "Set up authenticator" (fetches QR via `GET /auth/totp/enroll`) links. |
| **PairingDialog** | `auth_mode = "pairing"` | Current behavior. |

### 6.3 API Functions

```ts
getPublicHealth(): Promise<{
  require_pairing: boolean;
  paired: boolean;
  auth_mode?: 'pairing' | 'totp' | 'totp_enrollment';
}>

getTotpEnrollment(): Promise<{ otpauth_uri: string; qr_data_url?: string }>
// GET /auth/totp/enroll — localhost only

loginWithTotp(code: string): Promise<{ token: string }>
// POST /auth/totp with X-TOTP-Code or body
```

---

## 7. Security Considerations

| Concern | Mitigation |
|---------|------------|
| Enrollment URI exposure | `GET /auth/totp/enroll` localhost-only (like regenerate). |
| Brute force on TOTP | TOTP codes expire in 30s; 6 digits = 1M combos per window. Rate limit same as `/pair` (e.g. 10/min). |
| Replay | `OtpValidator` already caches used codes, rejects replay. |
| Secret storage | Reuse existing `otp-secret` + SecretStore; no new storage. |
| Pairing bypass | When `totp_login_enabled`, pairing remains valid; both paths issue same bearer tokens. |

---

## 8. Migration and Rollback

- **Config**: Add `totp_login_enabled = false` default. No migration for existing configs.
- **Rollback**: Set `totp_login_enabled = false`; frontend falls back to pairing. No data migration.

---

## 9. Implementation Order

1. **Config**: Add `gateway.totp_login_enabled`, schema + default.
2. **Backend**: Add `PairingGuard::issue_token(source)` for TOTP token issuance.
3. **Backend**: Wire `OtpValidator` into AppState when enabled.
4. **Backend**: Implement `GET /auth/totp/enroll`, `POST /auth/totp`.
5. **Backend**: Extend `GET /health` with `auth_mode`.
6. **Frontend**: Extend `getPublicHealth` types and useAuth logic.
7. **Frontend**: Add `EnrollmentScreen`, `TotpLoginScreen`.
8. **Docs**: Update `config-reference.md`, `api-paths-reference.md`.
9. **Dev proxy**: Add `/auth` and `/health` to `webmormos/vite.config.ts` proxy so dev server forwards to gateway (if not already covered).

---

## 10. Out of Scope (YAGNI)

- TOTP login for non-localhost (enrollment stays localhost-only).
- Multiple TOTP secrets (one per user).
- Backup codes.
- Changing `security.otp` to require TOTP for login when `login_enabled` — we use `gateway.totp_login_enabled` for clarity.

---

## 11. Design Review — Gaps and Fixes

### 11.1 Enrollment URI and Secret Creation (Critical)

**Problem:** `OtpValidator::from_config` **creates** the secret when the file does not exist. We cannot know `auth_mode = totp_enrollment` without checking secret existence, and we cannot return `totp_enrollment_uri` from `/health` without having an `OtpValidator` (which would create the secret).

**Fix:** Use `secret_file_path(zeroclaw_dir).exists()` to determine enrollment mode **without** calling `OtpValidator::from_config`. Do not create the secret at startup when in enrollment mode.

- `auth_mode = totp_enrollment` when `totp_login_enabled && !secret_exists`.
- **Do not** return `totp_enrollment_uri` from `/health`. The frontend must call `GET /auth/totp/enroll` to obtain it.
- Secret creation: `OtpValidator::from_config` creates the secret when the file is missing. Both `GET /auth/totp/enroll` and `POST /auth/totp` call it when needed, so either can create it on first use. The intended flow is enroll first.

### 11.2 OtpValidator Initialization (Lazy vs Eager)

**Problem:** When `auth_mode = totp_enrollment`, we must not create `OtpValidator` at startup (to avoid creating the secret). But `handle_auth_totp` needs it.

**Fix:** Create `OtpValidator` lazily in handlers when needed:
- `GET /auth/totp/enroll`: Create via `OtpValidator::from_config` (creates secret if missing). Return URI. Store validator in a one-time cache or recreate per request.
- `POST /auth/totp`: Create via `OtpValidator::from_config` (secret must exist by now). Validate code.

Alternatively: create `OtpValidator` at startup only when secret **already exists**. When in enrollment mode, handlers create it on demand (first enroll request creates secret; subsequent totp requests use it). AppState can hold `Option<Arc<OtpValidator>>` populated lazily or at startup when `secret_exists`.

### 11.3 Secret Path Consistency

**Problem:** Design said `zeroclaw_dir` from `config_path.parent()` or `workspace_dir`. Agent uses `config.config_path.parent()` (see `main.rs`).

**Fix:** Use `config.config_path.parent()` **only**, to match agent OTP. Secret path: `config_dir/otp-secret`.

### 11.4 SecretStore Availability

**Problem:** Gateway `AppState` does not currently hold `SecretStore`. `OtpValidator::from_config` requires it.

**Fix:** Create `SecretStore` in the handler or at gateway startup: `SecretStore::new(config_dir, config.secrets.encrypt)`. Add to AppState or create per-request. Config is in AppState; `config_dir` is derivable.

### 11.5 security.otp.enabled vs totp_login_enabled

**Problem:** Design required both. But `totp_login_enabled = true` with `security.otp.enabled = false` is valid (gateway-only TOTP, no agent gating).

**Fix:** Only require `totp_login_enabled` for gateway TOTP. Use `config.security.otp` (OtpConfig) for `token_ttl_secs`, etc. Do **not** require `security.otp.enabled`. This allows gateway TOTP login without agent OTP gating.

### 11.6 Pairing Fallback When TOTP Enabled

**Problem:** Auth matrix shows TOTP or enrollment only when `totp_login_enabled`. Users without a TOTP app cannot log in.

**Fix:** Add a "Use pairing code instead" link on `TotpLoginScreen` and `EnrollmentScreen` that switches to `PairingDialog`. Both flows remain valid; frontend offers both. Pairing code is still printed to terminal when no tokens exist.

### 11.7 Remote User Stuck in Enrollment

**Problem:** `GET /auth/totp/enroll` is localhost-only. A remote user seeing `EnrollmentScreen` cannot load the QR.

**Fix:** Document clearly. Frontend shows: "Enrollment must be done from this machine (localhost). If you're accessing remotely, run `zeroclaw` on the server and complete enrollment there first, or use the pairing code from the terminal."

### 11.8 PairedDeviceMeta for TOTP-Issued Tokens

**Problem:** `PairingGuard::add_token` must also update `paired_device_meta`. Current `try_pair` uses `PairedDeviceMeta::fresh(Some(client_id))`. For TOTP we have no client_id.

**Fix:** Use `paired_by: Some("totp".into())` when adding via TOTP. `PairingGuard::issue_token("totp")` inserts into both `paired_tokens` and `paired_device_meta`.

### 11.9 auth_mode Computation — When to Re-check Secret

**Problem:** After `GET /auth/totp/enroll` creates the secret, `auth_mode` should become `totp` on next page load. If we cache at startup only, we'd be wrong.

**Fix:** Re-check `secret_file_path(config_dir).exists()` on **every** `GET /health` request when `totp_login_enabled`. It's a cheap `Path::exists()` syscall. Avoid startup-only caching.

### 11.10 GET /auth/totp/enroll When Secret Already Exists

**Problem:** User in `totp` mode may need the QR (e.g. new device, lost authenticator). TotpLoginScreen should offer "Set up authenticator" for first-time setup.

**Fix:** `GET /auth/totp/enroll` must work when secret exists. Call `OtpValidator::from_config` (loads existing secret), return `validator.otpauth_uri()`. No creation. Document that this endpoint returns the URI in both enrollment and totp modes.

### 11.11 Token Generation Visibility

**Problem:** `generate_token()` and `hash_token()` in `pairing.rs` are private. Handler needs to generate a token for TOTP flow.

**Fix:** Add `PairingGuard::issue_token(source: &str) -> String` that internally calls `generate_token()`, hashes, inserts into `paired_tokens` and `paired_device_meta`, returns plaintext. Keeps token logic encapsulated in pairing.rs.

### 11.12 auth_mode When require_pairing Is False

**Problem:** Should `/health` include `auth_mode` when `require_pairing` is false?

**Fix:** Omit `auth_mode` when `require_pairing` is false. Frontend checks `!require_pairing` first and auto-authenticates; it never needs `auth_mode` in that case. Reduces payload and avoids ambiguity.

### 11.13 POST /auth/totp Body Parsing

**Problem:** Backend design says "Header: X-TOTP-Code or body: {\"code\": \"123456\"}". Handler must accept both.

**Fix:** Implement both: extract from `X-TOTP-Code` header first; if empty, parse JSON body for `code` field. Content-Type for body: `application/json`.
