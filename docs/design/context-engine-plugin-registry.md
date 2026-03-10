# Context Engine & Plugin Registry Design

**Status:** Implemented (Phases 1–5)  
**Last updated:** March 10, 2026

## Overview

mormOS adds a **slot-based plugin registry** and **ContextEngine** trait that mirrors OpenClaw's `slots.contextEngine` pattern. This gives production-grade controllability over agent behavior without forking core code.

## Architecture

### Slot enum (`mormos-plugin-registry`)

Pluggable extension points — one plugin per slot:

| Slot | Purpose | Status |
|------|---------|--------|
| `ContextEngine` | Context assembly, memory retrieval, compaction (the star slot) | Implemented |
| `SubagentSpawner` | Sub-agent spawning and lifecycle | Implemented |
| `ExecutionPolicy` | Budget, on-chain safety, human confirmation | Implemented |
| `MemoryManager` | Memory backend selection and policy | Placeholder |
| `ToolExecutor` | Tool execution sandboxing and routing | Placeholder |
| `ApprovalGuard` | Human approval, budget caps, on-chain confirmation | Placeholder |
| `Observability` | Metrics, tracing, cost tracking | Placeholder |

### ContextEngine trait (7 lifecycle hooks)

```rust
#[async_trait]
pub trait ContextEngine: Send + Sync {
    async fn bootstrap(&self, session: &mut Session) -> Result<()>;
    async fn ingest(&self, session: &Session, turn: &mut Turn) -> Result<()>;
    async fn assemble(&self, session: &Session, context: &mut Context) -> Result<()>;
    async fn compact(&self, session: &Session, context: &mut Context) -> Result<()>;
    async fn after_turn(&self, session: &Session, turn: &Turn) -> Result<()>;
    async fn prepare_subagent_spawn(&self, request: &SpawnRequest) -> Result<()>;
    async fn on_subagent_ended(&self, result: &SubagentResult) -> Result<()>;
}
```

All methods have default no-op implementations.

### Config support

```toml
[plugins]
enabled = true
slots = { contextEngine = "mormos-legacy", subagentSpawner = "mormos-default", executionPolicy = "mormos-default" }
```

- `plugins.enabled` — Master switch for the plugin system. When `false`, external plugin loading is disabled; the slot registry still reads `plugins.slots` and uses defaults for unset slots. See `docs/config-reference.md`.

Supported values for `contextEngine`:
- `mormos-legacy` (default) — built-in engine
- `wasm:path/to/plugin.wasm` — WASM plugin (requires `--features wasm-tools`). Path is relative to config file directory when not absolute.

Supported values for `subagentSpawner`:
- `mormos-default` (default) — allows all spawn requests
- `mormos-allowlist` — enforces `agent.subagents.allowed_agents` and `denied_agents`
- `wasm:path/to/plugin.wasm` — WASM plugin (requires `--features wasm-tools`)
- Example: `templates/rust/subagent_spawner_passthrough/`

Supported values for `executionPolicy`:
- `mormos-default` (default) — allows all tool calls (no-op policy gate)
- `mormos-allowlist` — enforces `agent.allowed_tools` and `agent.denied_tools` per tool call
- `wasm:path/to/plugin.wasm` — WASM plugin (requires `--features wasm-tools`)
- Example: `templates/rust/execution_policy_passthrough/`

When `contextEngine`, `subagentSpawner`, or `executionPolicy` is unset, defaults are used.

### WIT interfaces (implemented slots)

| Slot | WIT package | Path |
|------|-------------|------|
| ContextEngine | `zeroclaw:context-engine@1.0.0` | `wit/zeroclaw/context-engine/v1/context-engine.wit` |
| SubagentSpawner | `zeroclaw:subagent-spawner@1.0.0` | `wit/zeroclaw/subagent-spawner/v1/subagent-spawner.wit` |
| ExecutionPolicy | `zeroclaw:execution-policy@1.0.0` | `wit/zeroclaw/execution-policy/v1/execution-policy.wit` |

Each passthrough template (`context_engine_passthrough`, `subagent_spawner_passthrough`, `execution_policy_passthrough`) includes the WIT path in its doc comment for implementers.

## Files

- `crates/mormos-plugin-registry/` — Slot enum, PluginRegistry, ContextEngine, SubagentSpawner, ExecutionPolicy traits
- `src/context_engine/` — DefaultContextEngine, DefaultSubagentSpawner, DefaultExecutionPolicy, `create_registry_from_config`, `wasm_engine`, `wasm_spawner`, `wasm_execution_policy`
- `src/config/schema.rs` — `PluginSlotsConfig`, `plugins.slots.contextEngine`, `plugins.slots.subagentSpawner`, `plugins.slots.executionPolicy`

## Loop wiring (implemented)

**process_message_with_session:**
1. Creates `PluginRegistry` via `create_registry_from_config` (reads `plugins.slots.contextEngine`)
2. Builds `Session` and `Turn`
3. Calls `engine.bootstrap` and `engine.ingest` before context assembly
4. Builds context via `DefaultContextEngine::assemble_impl`, then `engine.assemble` for plugin modification
5. Calls `engine.after_turn` after the response

**run() interactive mode:**
- `auto_compact_history` receives `context_engine` and `session`; calls `engine.compact` before applying summary

**SubAgentSpawnTool:**
- `SubagentSpawner::can_spawn` called first — if false, spawn is denied
- `prepare_subagent_spawn` called before tokio::spawn
- `on_subagent_ended` called when sub-agent completes or fails (inside spawn closure)

**Tool loop (run_tool_call_loop):**
- `ExecutionPolicy::can_execute_tool(tool_name, args)` called before approval — if false, tool is blocked
- Policy is set via `TOOL_LOOP_EXECUTION_POLICY` task-local when registry is available

## Migration path (zero breaking changes)

- Existing traits (Provider, Tool, Memory, Channel) unchanged
- `DefaultContextEngine` delegates to `build_context`, `build_hardware_context`
- When no engine is registered, behavior is unchanged (registry always has default engine)

## Phase 2 (implemented)

- **WASM ContextEngine** — `wasm:path/to/plugin.wasm` in config
- WASI stdio protocol: JSON over stdin/stdout (see `src/context_engine/wasm_engine.rs`)
  - Host → stdin: `{ "hook": "bootstrap"|"ingest"|"assemble"|"compact"|"after_turn"|"prepare_subagent_spawn"|"on_subagent_ended", "session": {...}, "turn": {...}|null, "context": {...}|null, "spawn_request": {...}|null, "subagent_result": {...}|null }`
  - Host ← stdout: `{ "ok": true, "error": null, "session": {...}|null, "turn": {...}|null, "context": {...}|null }`
- WIT interface: `wit/zeroclaw/context-engine/v1/context-engine.wit`
- Example: `templates/rust/context_engine_passthrough/`

## Phase 3 (implemented)

- **SubagentSpawner slot** — policy gate for sub-agent spawning
  - `SubagentSpawner::can_spawn(request)` — approve or deny spawn before execution
  - Config: `plugins.slots.subagentSpawner = "mormos-default"`
  - `DefaultSubagentSpawner` allows all spawns; custom plugins can enforce budget, allowlists, etc.
  - Wired in `SubAgentSpawnTool`: calls `can_spawn` before `prepare_subagent_spawn` and spawn
- **WASM SubagentSpawner** (implemented) — `wasm:path/to/plugin.wasm` in config
  - WASI stdio protocol: JSON over stdin/stdout (see `src/context_engine/wasm_spawner.rs`)
  - Host → stdin: `{ "hook": "can_spawn", "spawn_request": { "agent_id": "...", "command": "..." } }`
  - Host ← stdout: `{ "ok": true, "allowed": true }` or `{ "ok": false, "allowed": false, "error": "..." }`
  - WIT interface: `wit/zeroclaw/subagent-spawner/v1/subagent-spawner.wit`
  - Example: `templates/rust/subagent_spawner_passthrough/`

## Phase 4 (implemented)

- **ExecutionPolicy slot** — policy gate for tool execution
  - `ExecutionPolicy::can_execute_tool(tool_name, args)` — approve or deny each tool call before execution
  - Config: `plugins.slots.executionPolicy = "mormos-default"`
  - `DefaultExecutionPolicy` allows all tool calls; custom plugins can enforce budget, allowlists, etc.
  - Wired in `run_tool_call_loop`: calls `can_execute_tool` after excluded_tools check, before approval

## Phase 5 (implemented)

- **WASM ExecutionPolicy** — `wasm:path/to/plugin.wasm` in config
  - WASI stdio protocol: JSON over stdin/stdout (see `src/context_engine/wasm_execution_policy.rs`)
  - Host → stdin: `{ "hook": "can_execute_tool", "tool_name": "...", "args": {...} }`
  - Host ← stdout: `{ "ok": true, "allowed": true }` or `{ "ok": false, "allowed": false, "error": "..." }`
  - WIT interface: `wit/zeroclaw/execution-policy/v1/execution-policy.wit`
  - Example: `templates/rust/execution_policy_passthrough/`

## Phase 6 (future)

- Full lifecycle isolation (process isolation, resource limits)
- Policy engine (runtime policy sync)
- Built-in on-chain wallet slot

## Error handling and failure modes

- **WASM load failure** — If a `wasm:path/to/plugin.wasm` plugin fails to load or compile, the host falls back to the default built-in (e.g. `mormos-legacy`, `mormos-default`) and logs a warning.
- **WASM timeout** — ContextEngine: 30s; SubagentSpawner and ExecutionPolicy: 10s. On timeout (epoch interruption), the invocation fails and the host treats it as an error (spawn denied, tool blocked, or bootstrap/ingest/assemble/compact/after_turn fails).
- **Invalid JSON** — If the WASM guest writes invalid JSON to stdout, the host fails the invocation and applies the same fallback/deny behavior as above.
- **Policy errors** — When `can_spawn` or `can_execute_tool` returns `ok: false` with an error message, the host denies the operation and surfaces the error to the caller.

## See also

- `docs/config-reference.md` — `[plugins]` and `[plugins.slots]` config keys
- `wit/zeroclaw/context-engine/v1/context-engine.wit` — ContextEngine WIT interface
- `wit/zeroclaw/subagent-spawner/v1/subagent-spawner.wit` — SubagentSpawner WIT interface
- `wit/zeroclaw/execution-policy/v1/execution-policy.wit` — ExecutionPolicy WIT interface
