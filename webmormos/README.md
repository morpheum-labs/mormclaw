# MormOS Webmormos

Bun-only build of the MormOS web UI. Same app as `web/` but uses **Bun** for install, scripts, and runtime.

## Setup

```bash
bun install
```

## Scripts

| Command | Description |
|---------|-------------|
| `bun run dev` | Start Vite dev server |
| `bun run build` | Type-check + production build |
| `bun run preview` | Preview production build |
| `bun run test` | Run Vitest unit tests |
| `bun run test:watch` | Vitest watch mode |
| `bun run test:mobile-smoke` | Mobile smoke tests (Vitest + optional Playwright) |

## Differences from `web/`

- **Package manager**: `bun install` only (no npm)
- **Lockfile**: `bun.lock` only
- **Script runner**: All scripts run via Bun (`bun --bun x` for binaries)
- **Playwright**: Uses `bun run dev` for web server

## Requirements

- [Bun](https://bun.sh) v1.0+
