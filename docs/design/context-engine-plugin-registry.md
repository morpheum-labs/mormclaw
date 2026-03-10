# Context Engine & Plugin Registry Design

**Status:** Implemented (Phase 1)  
**Last updated:** March 10, 2026

## Overview

mormOS adds a **slot-based plugin registry** and **ContextEngine** trait that mirrors OpenClaw's `slots.contextEngine` pattern. This gives production-grade controllability over agent behavior without forking core code.

## Architecture

### Slot enum (`mormos-plugin-registry`)

Pluggable extension points — one plugin per slot:

| Slot | Purpose |
|------|---------|
| `ContextEngine` | Context assembly, memory retrieval, compaction (the star slot) |
| `MemoryManager` | Memory backend selection and policy |
| `ToolExecutor` | Tool execution sandboxing and routing |
| `ApprovalGuard` | Human approval, budget caps, on-chain confirmation |
| `ExecutionPolicy` | Budget, on-chain safety, human confirmation |
| `SubagentSpawner` | Sub-agent spawning and lifecycle |
| `Observability` | Metrics, tracing, cost tracking |

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
slots = { contextEngine = "mormos-legacy" }
```

When `contextEngine` is unset, the default legacy engine is used.

## Files

- `crates/mormos-plugin-registry/` — Slot enum, PluginRegistry, ContextEngine trait, Session/Turn/Context types
- `src/context_engine/` — DefaultContextEngine, `assemble_impl` helper, `create_default_registry` factory
- `src/config/schema.rs` — `PluginSlotsConfig`, `plugins.slots.contextEngine`

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
- `prepare_subagent_spawn` called before tokio::spawn
- `on_subagent_ended` called when sub-agent completes or fails (inside spawn closure)

## Migration path (zero breaking changes)

- Existing traits (Provider, Tool, Memory, Channel) unchanged
- `DefaultContextEngine` delegates to `build_context`, `build_hardware_context`
- When no engine is registered, behavior is unchanged (registry always has default engine)

## Phase 2 (future)

- Full WASM plugin support for ContextEngine
- MemoryManager and ExecutionPolicy slots
- Official plugins: lossless, semantic-compact, onchain-state

## Phase 3 (future)

- Sub-agent spawning with full lifecycle isolation
- Policy engine (runtime policy sync)
- Built-in on-chain wallet slot
