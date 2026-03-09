# TOTP Gateway Login — UI Design Proposal

**Status:** Proposal  
**Date:** 2025-03-10  
**Depends on:** [totp-gateway-login-design.md](totp-gateway-login-design.md) (backend)  
**Scope:** Web dashboard (webmormos) authentication UI for TOTP login flow

---

## 1. Summary

Extend the web dashboard auth UI to support TOTP login as defined in the backend design. When the gateway returns `auth_mode = "totp"` or `"totp_enrollment"`, show the corresponding screen instead of the pairing dialog. Post-auth flow is unchanged (bearer token, normal dashboard).

---

## 2. Design Principles

| Principle | Application |
|-----------|-------------|
| **Consistency** | Reuse existing `pairing-shell`, `pairing-card`, `electric-button` styles. Same visual language as PairingDialog. |
| **Progressive disclosure** | Enrollment: QR first, then code. TotpLogin: code only; "Set up authenticator" reveals QR on demand. |
| **Fallback** | Always offer "Use pairing code instead" when TOTP is primary. |
| **Backward compatible** | When `auth_mode` is undefined (older gateway), fall back to PairingDialog. |

---

## 3. Auth Mode Selection

### 3.1 Logic (AppContent / useAuth)

```ts
// After getPublicHealth(), before rendering auth screens:
if (!health.require_pairing)               → auto-authenticate (no screen)
if (forcePairing)                          → <PairingDialog />  // user chose fallback
if (health.auth_mode === 'totp_enrollment') → <EnrollmentScreen />
if (health.auth_mode === 'totp')           → <TotpLoginScreen />
else                                       → <PairingDialog />  // pairing or !auth_mode
```

### 3.2 useAuth Changes

- Store `authMode` from `getPublicHealth` response: `auth_mode ?? 'pairing'`.
- Expose `authMode` from context so AppContent can route to the correct screen.
- Add `loginWithTotp(code)` to AuthState. Implementation: call `api.loginWithTotp(code)` (which stores token via `setToken`), then update React state: `setTokenState(data.token)`, `setAuthenticated(true)` — same pattern as `pair`.

---

## 4. Screens

### 4.1 EnrollmentScreen

**When:** `auth_mode = "totp_enrollment"`

**Layout:** Same shell as PairingDialog (`pairing-shell`, `pairing-card`).

**Content:**
1. Title: "MormOS"
2. Subtitle: "Scan the QR code with your authenticator app, then enter the 6-digit code to confirm."
3. QR code area: Fetch `GET /auth/totp/enroll`, render QR from `otpauth_uri` (or `qr_data_url` if provided). Use a QR library (e.g. `qrcode.react`) or `<img src={qr_data_url} />` if backend returns it.
4. Manual entry fallback: **Do not** display the raw secret in plain text (security risk). If needed, offer "Copy setup link" to clipboard (otpauth_uri) for apps that accept it. QR is the primary path.
5. 6-digit input (same styling as PairingDialog).
6. Primary button: "Confirm" — submits code via `loginWithTotp(code)`.
7. Secondary link: "Use pairing code instead" — switches to PairingDialog (local state or re-fetch with pairing preference).

**States:**
- Loading QR: Show spinner while fetching enroll.
- Error (enroll failed, e.g. remote user): "Enrollment must be done from this machine. Use pairing code instead, or run the server locally."
- Error (invalid code): "Invalid code. Try again."

**Remote user:** When `GET /auth/totp/enroll` returns 403 (localhost only), show the remote-user message and emphasize "Use pairing code instead."

---

### 4.2 TotpLoginScreen

**When:** `auth_mode = "totp"`

**Layout:** Same shell as PairingDialog.

**Content:**
1. Title: "MormOS"
2. Subtitle: "Enter the 6-digit code from your authenticator app."
3. 6-digit input (same styling).
4. Primary button: "Sign in" — submits via `loginWithTotp(code)`.
5. Secondary links:
   - "Use pairing code instead" — switch to PairingDialog.
   - "Set up authenticator" — fetch `GET /auth/totp/enroll`, show QR in a modal or expandable section. User scans, then enters code.

**States:**
- Error (invalid code): "Invalid code. Try again."
- "Set up authenticator" expanded: Show QR (same as EnrollmentScreen). User scans, closes, enters code on main form.

---

### 4.3 PairingDialog (existing)

**When:** `auth_mode = "pairing"` or `!auth_mode`

**Changes:** None. Keep current behavior (pairing code input, Regenerate code, Pair).

---

## 5. API Layer

### 5.1 Extend `getPublicHealth`

```ts
export async function getPublicHealth(): Promise<{
  require_pairing: boolean;
  paired: boolean;
  auth_mode?: 'pairing' | 'totp' | 'totp_enrollment';
}> {
  const response = await fetch('/health');
  if (!response.ok) throw new Error(`Health check failed (${response.status})`);
  return response.json();
}
```

### 5.2 New: `getTotpEnrollment`

```ts
export async function getTotpEnrollment(): Promise<{
  otpauth_uri: string;
  qr_data_url?: string;
}> {
  const response = await fetch('/auth/totp/enroll');
  if (!response.ok) {
    const data = await response.json().catch(() => ({}));
    throw new Error(data.error ?? `Enrollment failed (${response.status})`);
  }
  return response.json();
}
```

### 5.3 New: `loginWithTotp`

```ts
export async function loginWithTotp(code: string): Promise<{ token: string }> {
  const response = await fetch('/auth/totp', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ code: code.trim() }),
  });
  if (!response.ok) {
    const data = await response.json().catch(() => ({}));
    throw new Error(data.error ?? `TOTP login failed (${response.status})`);
  }
  const data = (await response.json()) as { token: string };
  setToken(data.token);
  return data;
}
```

**Note:** Backend accepts `X-TOTP-Code` header or JSON body `{ code }`. Prefer body for consistency with other API calls.

---

## 6. Component Structure

### 6.1 Suggested File Layout

```
webmormos/src/
├── components/
│   └── auth/
│       ├── PairingDialog.tsx      (extract from App.tsx)
│       ├── EnrollmentScreen.tsx   (new)
│       ├── TotpLoginScreen.tsx    (new)
│       └── AuthShell.tsx          (optional: shared layout wrapper)
├── hooks/
│   └── useAuth.ts                 (extend)
├── lib/
│   └── api.ts                     (extend)
└── App.tsx                        (import, route by auth_mode)
```

### 6.2 AuthShell (optional)

Shared wrapper for all auth screens: `pairing-shell`, `pairing-card`, consistent spacing. Each screen provides title, subtitle, form content, and secondary links.

---

## 7. QR Code Rendering

**Options:**
- **qrcode.react**: `<QRCodeSVG value={otpauth_uri} size={200} />` — add dependency.
- **Backend qr_data_url**: If backend returns `qr_data_url`, use `<img src={qr_data_url} alt="QR code" />` — no extra dependency.
- **qrcode** (node): Server-side only; not suitable for SPA.

**Recommendation:** Use `qrcode.react` or similar lightweight lib. Backend `qr_data_url` is optional; frontend can generate from `otpauth_uri` to avoid adding a dependency if backend provides it.

---

## 8. "Use pairing code instead" Behavior

**Implementation:** `const [forcePairing, setForcePairing] = useState(false)` in AppContent. When `forcePairing` is true, render PairingDialog regardless of `auth_mode`. Link calls `setForcePairing(true)`. State resets on unmount (e.g. after logout and revisit).

---

## 9. Styling

- Reuse `pairing-shell`, `pairing-card`, `electric-button`, `electric-loader` from `index.css`.
- Input: Same as PairingDialog (`rounded-xl`, `border-[#29509c]`, `bg-[#071228]/90`, etc.).
- Links: `text-xs text-[#9bb8e8]/80 hover:text-[#9bb8e8]` (match Regenerate code button).
- QR container: Centered, padded, optional border. Size ~200×200px.

---

## 10. i18n

Add keys for:
- `auth.totp_enrollment_title`, `auth.totp_enrollment_subtitle`
- `auth.totp_login_title`, `auth.totp_login_subtitle`
- `auth.use_pairing_instead`
- `auth.setup_authenticator`
- `auth.confirm`, `auth.sign_in`
- `auth.enrollment_remote_error`

Sync with existing `auth.*` keys in `lib/i18n.ts`. Follow `docs/i18n-guide.md` for locale coverage.

---

## 11. Dev Proxy

Ensure `vite.config.ts` proxies `/auth` and `/health` to the gateway:

```ts
server: {
  proxy: {
    "/pair": { target: "http://localhost:42617", changeOrigin: true },
    "/auth": { target: "http://localhost:42617", changeOrigin: true },
    "/health": { target: "http://localhost:42617", changeOrigin: true },
    "/api": { target: "http://localhost:42617", changeOrigin: true },
    "/ws": { target: "ws://localhost:42617", ws: true },
  },
},
```

---

## 12. Implementation Order

1. **API**: Extend `getPublicHealth` types, add `getTotpEnrollment`, `loginWithTotp`.
2. **useAuth**: Add `authMode` (from health), `loginWithTotp` to context.
3. **EnrollmentScreen**: New component, QR + code input, "Use pairing code instead."
4. **TotpLoginScreen**: New component, code input, "Use pairing code instead", "Set up authenticator."
5. **App.tsx**: Auth mode routing, `forcePairing` state, extract PairingDialog to component.
6. **Dev proxy**: Add `/auth`, `/health` if missing.
7. **i18n**: Add new keys, update locales per i18n-guide.

---

## 13. Out of Scope

- Separate route for TOTP (e.g. `/login/totp`); keep single entry, screen chosen by auth_mode.
- Remembering "force pairing" across sessions.
- Custom QR styling beyond size/padding.

---

## 14. Design Review — Gaps and Fixes

### 14.1 Secret Exposure (Security)

**Problem:** Displaying the raw TOTP secret for "manual entry" exposes it on screen (screenshots, shoulder surfing).

**Fix:** Do not show the secret in plain text. Use QR as primary. If manual entry is needed, offer "Copy setup link" (otpauth_uri to clipboard) for apps that accept it. See §4.1.

### 14.2 useAuth State Sync for loginWithTotp

**Problem:** `api.loginWithTotp` calls `setToken` (storage) but useAuth has React state (`token`, `authenticated`). Without updating state, the UI may not reflect authenticated status.

**Fix:** useAuth's `loginWithTotp` must call `api.loginWithTotp`, then `setTokenState(data.token)` and `setAuthenticated(true)` — same pattern as `pair`. See §3.2.

### 14.3 authMode Source and Loading

**Problem:** useAuth fetches health once on mount. We need `authMode` before rendering auth screens. If loading, we show spinner. When health returns, we have `auth_mode` (or undefined for older gateways).

**Fix:** Store `authMode` in useAuth state from health response. Set `authMode = health.auth_mode ?? 'pairing'` when `require_pairing` is true. Expose via context. AppContent reads it after loading completes.

### 14.4 forcePairing vs auth_mode

**Problem:** When `forcePairing` is true, we show PairingDialog. But PairingDialog needs a pairing code. The pairing code is printed to terminal when no tokens exist. If tokens exist, there is no pairing code — user must regenerate. So when we show PairingDialog via forcePairing, the "Regenerate code" button is essential. Current PairingDialog has it. Good.

### 14.5 getTotpEnrollment 403 Handling

**Problem:** Remote user gets 403 from `GET /auth/totp/enroll`. The error response body may contain a message. Frontend should show a user-friendly message, not raw "Enrollment failed (403)".

**Fix:** In getTotpEnrollment, when `response.status === 403`, prefer `data.error` (backend sends "Pairing regenerate is only allowed from localhost..."). EnrollmentScreen catches the error and shows `auth.enrollment_remote_error` message.

---

## 15. UI vs Backend Logic Flow Alignment Review

This section verifies the UI design correctly follows the backend logic flow.

### 15.1 Auth Mode Selection Order ✓

Backend (§3.3, §11.12): `auth_mode` is omitted when `require_pairing` is false; when true, it is `pairing` | `totp` | `totp_enrollment`.

UI (§3.1) checks in order: `!require_pairing` → `forcePairing` → `totp_enrollment` → `totp` → else (pairing / !auth_mode). This matches the backend contract.

### 15.2 Health Re-fetch After Logout (Gap)

**Problem:** useAuth fetches health only on mount. After logout, the user is still mounted; health is not re-fetched. The UI would show the last known auth screen, but `auth_mode` was never stored in the current implementation. For TOTP, we need fresh `auth_mode` after logout to show the correct screen (Enrollment vs TotpLogin vs Pairing).

**Fix:** When `authenticated` becomes false (e.g. after logout), re-fetch health to obtain fresh `auth_mode`. useAuth should either: (a) re-fetch health when transitioning to unauthenticated, or (b) include `auth_mode` in the initial fetch and re-fetch when showing auth screens after logout. Prefer (a): add a useEffect that fetches health when `authenticated` becomes false and we need to show an auth screen.

### 15.3 TotpLoginScreen "Set up authenticator" 403 (Gap)

**Problem:** When a user on TotpLoginScreen clicks "Set up authenticator" while accessing remotely, `GET /auth/totp/enroll` returns 403 (localhost only). The UI design does not specify the error handling for this case.

**Fix:** When the "Set up authenticator" flow (modal or expandable) calls `getTotpEnrollment` and receives 403, show the same remote-user message as EnrollmentScreen: "Enrollment must be done from this machine. Use pairing code instead."

### 15.4 429 Rate Limit Handling (Minor)

**Problem:** Backend returns `{ error: "...", retry_after: N }` for 429. The UI design uses `data.error` for all non-ok responses, which is correct. Optionally surface `retry_after` for a friendlier message (e.g. "Too many attempts. Try again in 60 seconds").

**Fix:** Optional enhancement: when `response.status === 429` and `data.retry_after` exists, append it to the error message. Not required for MVP.

### 15.5 Enrollment → Totp Transition ✓

Backend (§11.9): `auth_mode` is re-computed on every GET /health. After enrollment (POST /auth/totp succeeds), the secret exists; the next health check returns `auth_mode = "totp"`. The UI does not need to re-fetch health immediately after successful login; the user is authenticated and proceeds to the dashboard. On a future visit (after logout), health returns `totp`. Aligned.

### 15.6 Page Refresh During Enrollment ✓

If the user refreshes while on EnrollmentScreen after the QR was fetched (secret created), the next health check returns `auth_mode = "totp"`. The UI shows TotpLoginScreen. The user can click "Set up authenticator" to fetch the QR again if needed, or enter the code if they already scanned. Aligned.

### 15.7 API Contract Alignment ✓

| Backend | UI |
|---------|-----|
| POST /auth/totp accepts `{ code }` or `X-TOTP-Code` | Sends JSON body `{ code }` ✓ |
| Returns `{ token, message }` | Uses `data.token`, stores via setToken ✓ |
| GET /auth/totp/enroll returns `{ otpauth_uri, qr_data_url? }` | getTotpEnrollment expects same ✓ |
| 403: `{ error: "..." }` | Uses `data.error` for user message ✓ |

### 15.8 forcePairing and Pairing Fallback ✓

Backend (§11.6): Pairing remains valid when TOTP is enabled. Both flows issue the same bearer tokens. UI offers "Use pairing code instead" and PairingDialog with Regenerate code. When `forcePairing` is true, PairingDialog is shown regardless of `auth_mode`. Aligned.

### 15.9 Token Storage and State Sync ✓

Backend issues the same token format as pairing. UI: `api.loginWithTotp` calls `setToken`; useAuth's `loginWithTotp` calls `setTokenState` and `setAuthenticated`. Same pattern as `pair`. Aligned.
