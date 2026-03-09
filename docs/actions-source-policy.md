# Actions Source Policy

This document defines the current GitHub Actions source-control policy for this repository.

## Current Policy

- Repository Actions permissions: enabled
- Allowed actions mode: selected

Selected allowlist (all actions currently used across CI, Beta Release, and Promote Release workflows):

| Action | Used In | Purpose |
|--------|---------|---------|
| `actions/checkout@v4` | All workflows | Repository checkout |
| `actions/upload-artifact@v4` | release, promote-release | Upload build artifacts |
| `actions/download-artifact@v4` | release, promote-release | Download build artifacts for packaging |
| `dtolnay/rust-toolchain@stable` | All workflows | Install Rust toolchain (1.92.0) |
| `Swatinem/rust-cache@v2` | All workflows | Cargo build/dependency caching |
| `softprops/action-gh-release@v2` | release, promote-release | Create GitHub Releases |
| `docker/setup-buildx-action@v3` | release, promote-release | Docker Buildx setup |
| `docker/login-action@v3` | release, promote-release | GHCR authentication |
| `docker/build-push-action@v6` | release, promote-release | Multi-platform Docker image build and push |

Equivalent allowlist patterns:

- `actions/*`
- `dtolnay/rust-toolchain@*`
- `Swatinem/rust-cache@*`
- `softprops/action-gh-release@*`
- `sigstore/cosign-installer@*`
- `Checkmarx/vorpal-reviewdog-github-action@*`
- `Swatinem/rust-cache@*`
- `docker/*`

## Workflows

| Workflow | File | Trigger |
|----------|------|---------|
| CI | `.github/workflows/ci.yml` | Pull requests to `master` |
| Beta Release | `.github/workflows/release.yml` | Push to `master` |
| Promote Release | `.github/workflows/promote-release.yml` | Manual `workflow_dispatch` |

## Change Control

Record each policy change with:

- change date/time (UTC)
- actor
- reason
- allowlist delta (added/removed patterns)
- rollback note

Use these commands to export the current effective policy:

```bash
gh api repos/zeroclaw-labs/zeroclaw/actions/permissions
gh api repos/zeroclaw-labs/zeroclaw/actions/permissions/selected-actions
```

## Guardrails

- Any PR that adds or changes `uses:` action sources must include an allowlist impact note.
- New third-party actions require explicit maintainer review before allowlisting.
- Expand allowlist only for verified missing actions; avoid broad wildcard exceptions.

## Change Log

After allowlist changes, validate:

1. `CI`
2. `Docker`
3. `Security Audit`
4. `Workflow Sanity`
5. `Release` (when safe to run)

Failure mode to watch for:

- `action is not allowed by policy`

If encountered, add only the specific trusted missing action, rerun, and document why.

Latest sweep notes:

- 2026-02-21: Added manual Vorpal reviewdog workflow for targeted secure-coding checks on supported file types
    - Added allowlist pattern: `Checkmarx/vorpal-reviewdog-github-action@*`
    - Workflow uses pinned source: `Checkmarx/vorpal-reviewdog-github-action@8cc292f337a2f1dea581b4f4bd73852e7becb50d` (v1.2.0)
- 2026-02-26: Standardized runner/action sources for cache and Docker build paths
    - Added allowlist pattern: `Swatinem/rust-cache@*`
    - Docker build jobs use `docker/setup-buildx-action` and `docker/build-push-action`
- 2026-02-16: Hidden dependency discovered in `release.yml`: `sigstore/cosign-installer@...`
    - Added allowlist pattern: `sigstore/cosign-installer@*`
- 2026-02-17: Security audit reproducibility/freshness balance update
    - Added allowlist pattern: `rustsec/audit-check@*`
    - Replaced inline `cargo install cargo-audit` execution with pinned `rustsec/audit-check@69366f33c96575abad1ee0dba8212993eecbe998` in `security.yml`
    - Supersedes floating-version proposal in #588 while keeping action source policy explicit

## Rollback

Emergency unblock path:

1. Temporarily set Actions policy back to `all`.
2. Restore selected allowlist after identifying missing entries.
3. Record incident and final allowlist delta.
