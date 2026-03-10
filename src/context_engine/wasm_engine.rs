//! WASM ContextEngine — executes a `.wasm` binary as a ContextEngine plugin.
//!
//! # Feature gate
//! Compiled when `--features wasm-tools` is active on supported targets
//! (Linux, macOS, Windows). Unsupported targets use the stub implementation.
//!
//! # Protocol (WASI stdio)
//!
//! The WASM module communicates via standard WASI stdin / stdout:
//!
//! ```text
//! Host → stdin  : UTF-8 JSON of the hook request
//! Host ← stdout : UTF-8 JSON of the hook result
//! ```
//!
//! Request shape:
//! ```json
//! {
//!   "hook": "bootstrap" | "ingest" | "assemble" | "compact" | "after_turn" | "prepare_subagent_spawn" | "on_subagent_ended",
//!   "session": { "id": "...", "channel": "...", "session_id": "..." },
//!   "turn": { "input": "...", "output": "..." } | null,
//!   "context": { ... } | null,
//!   "spawn_request": { "agent_id": "...", "command": "..." } | null,
//!   "subagent_result": { "session_id": "...", "success": true, "output": "..." } | null
//! }
//! ```
//!
//! Response shape:
//! ```json
//! {
//!   "ok": true,
//!   "error": null,
//!   "session": { ... } | null,
//!   "turn": { ... } | null,
//!   "context": { ... } | null
//! }
//! ```
//!
//! Mutated fields (session, turn, context) are returned when the guest modifies them.
//!
//! # Security
//! Same as WASM tools: epoch interruption, no filesystem preopened, no network.

use std::path::Path;

#[allow(unused_imports)]
use mormos_plugin_registry::{Context, ContextEngine, Session, SpawnRequest, SubagentResult, Turn};

/// Maximum hook result size (1 MiB).
const MAX_OUTPUT_BYTES: usize = 1_048_576;

/// Wall-clock timeout for a single WASM invocation.
const WASM_TIMEOUT_SECS: u64 = 30;

// ─── Feature-gated implementation ─────────────────────────────────────────────

#[cfg(all(
    feature = "wasm-tools",
    any(target_os = "linux", target_os = "macos", target_os = "windows")
))]
mod inner {
    use super::*;
    use anyhow::{bail, Context};
    use async_trait::async_trait;
    use mormos_plugin_registry::Context as Ctx;
    use serde::{Deserialize, Serialize};
    use std::sync::Arc;
    use wasmtime::{Config as WtConfig, Engine, Linker, Module, Store};
    use wasmtime_wasi::{
        p2::pipe::{MemoryInputPipe, MemoryOutputPipe},
        preview1::{self, WasiP1Ctx},
        WasiCtxBuilder,
    };

    #[derive(Debug, Serialize, Deserialize)]
    struct HookRequest {
        hook: String,
        session: Option<serde_json::Value>,
        turn: Option<serde_json::Value>,
        context: Option<serde_json::Value>,
        spawn_request: Option<serde_json::Value>,
        subagent_result: Option<serde_json::Value>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct HookResponse {
        ok: bool,
        error: Option<String>,
        session: Option<Session>,
        turn: Option<Turn>,
        context: Option<Ctx>,
    }

    pub struct WasmContextEngine {
        engine: Engine,
        module: Module,
    }

    impl WasmContextEngine {
        pub fn load(path: &Path) -> anyhow::Result<Self> {
            let mut cfg = WtConfig::new();
            cfg.epoch_interruption(true);

            let engine = Engine::new(&cfg).context("failed to create WASM engine")?;

            let bytes = std::fs::read(path)
                .with_context(|| format!("cannot read WASM file: {}", path.display()))?;
            let module = Module::new(&engine, &bytes)
                .with_context(|| format!("cannot compile WASM module: {}", path.display()))?;

            Ok(Self { engine, module })
        }

        fn invoke_sync(&self, req: &HookRequest) -> anyhow::Result<HookResponse> {
            let input_bytes = serde_json::to_vec(req)?;

            let stdout_pipe = MemoryOutputPipe::new(MAX_OUTPUT_BYTES);
            let stdout_for_read = stdout_pipe.clone();

            let wasi_ctx: WasiP1Ctx = WasiCtxBuilder::new()
                .stdin(MemoryInputPipe::new(input_bytes))
                .stdout(stdout_pipe)
                .build_p1();

            let mut store = Store::new(&self.engine, wasi_ctx);
            store.set_epoch_deadline(WASM_TIMEOUT_SECS);

            let mut linker: Linker<WasiP1Ctx> = Linker::new(&self.engine);
            preview1::add_to_linker_sync(&mut linker, |ctx| ctx)
                .context("failed to add WASI to linker")?;

            let instance = linker.instantiate(&mut store, &self.module)?;

            let engine_for_ticker = self.engine.clone();
            let (stop_tx, stop_rx) = std::sync::mpsc::channel::<()>();
            let ticker = std::thread::spawn(move || {
                while stop_rx
                    .recv_timeout(std::time::Duration::from_secs(1))
                    .is_err()
                {
                    engine_for_ticker.increment_epoch();
                }
            });

            let call_result = instance
                .get_typed_func::<(), ()>(&mut store, "_start")
                .context("WASM module must export '_start' (compile as a WASI binary)")
                .and_then(|start| {
                    start
                        .call(&mut store, ())
                        .context("WASM execution failed or timed out")
                });

            let _ = stop_tx.send(());
            let _ = ticker.join();

            call_result?;

            let raw = stdout_for_read.contents().to_vec();
            if raw.is_empty() {
                bail!("WASM context engine wrote nothing to stdout");
            }

            serde_json::from_slice::<HookResponse>(&raw)
                .context("WASM context engine stdout is not valid HookResponse JSON")
        }
    }

    #[async_trait]
    impl ContextEngine for WasmContextEngine {
        async fn bootstrap(&self, session: &mut Session) -> anyhow::Result<()> {
            let req = HookRequest {
                hook: "bootstrap".into(),
                session: Some(serde_json::to_value(&*session)?),
                turn: None,
                context: None,
                spawn_request: None,
                subagent_result: None,
            };
            let res = self.invoke_sync(&req)?;
            if !res.ok {
                bail!(
                    "{}",
                    res.error.unwrap_or_else(|| "WASM bootstrap failed".into())
                );
            }
            if let Some(s) = res.session {
                *session = s;
            }
            Ok(())
        }

        async fn ingest(&self, _session: &Session, turn: &mut Turn) -> anyhow::Result<()> {
            let req = HookRequest {
                hook: "ingest".into(),
                session: Some(serde_json::to_value(_session)?),
                turn: Some(serde_json::to_value(&*turn)?),
                context: None,
                spawn_request: None,
                subagent_result: None,
            };
            let res = self.invoke_sync(&req)?;
            if !res.ok {
                bail!(
                    "{}",
                    res.error.unwrap_or_else(|| "WASM ingest failed".into())
                );
            }
            if let Some(t) = res.turn {
                *turn = t;
            }
            Ok(())
        }

        async fn assemble(&self, _session: &Session, context: &mut Ctx) -> anyhow::Result<()> {
            let req = HookRequest {
                hook: "assemble".into(),
                session: Some(serde_json::to_value(_session)?),
                turn: None,
                context: Some(serde_json::to_value(&*context)?),
                spawn_request: None,
                subagent_result: None,
            };
            let res = self.invoke_sync(&req)?;
            if !res.ok {
                bail!(
                    "{}",
                    res.error.unwrap_or_else(|| "WASM assemble failed".into())
                );
            }
            if let Some(c) = res.context {
                *context = c;
            }
            Ok(())
        }

        async fn compact(&self, _session: &Session, context: &mut Ctx) -> anyhow::Result<()> {
            let req = HookRequest {
                hook: "compact".into(),
                session: Some(serde_json::to_value(_session)?),
                turn: None,
                context: Some(serde_json::to_value(&*context)?),
                spawn_request: None,
                subagent_result: None,
            };
            let res = self.invoke_sync(&req)?;
            if !res.ok {
                bail!(
                    "{}",
                    res.error.unwrap_or_else(|| "WASM compact failed".into())
                );
            }
            if let Some(c) = res.context {
                *context = c;
            }
            Ok(())
        }

        async fn after_turn(&self, _session: &Session, _turn: &Turn) -> anyhow::Result<()> {
            let req = HookRequest {
                hook: "after_turn".into(),
                session: Some(serde_json::to_value(_session)?),
                turn: Some(serde_json::to_value(_turn)?),
                context: None,
                spawn_request: None,
                subagent_result: None,
            };
            let res = self.invoke_sync(&req)?;
            if !res.ok {
                bail!(
                    "{}",
                    res.error.unwrap_or_else(|| "WASM after_turn failed".into())
                );
            }
            Ok(())
        }

        async fn prepare_subagent_spawn(&self, request: &SpawnRequest) -> anyhow::Result<()> {
            let req = HookRequest {
                hook: "prepare_subagent_spawn".into(),
                session: None,
                turn: None,
                context: None,
                spawn_request: Some(serde_json::to_value(request)?),
                subagent_result: None,
            };
            let res = self.invoke_sync(&req)?;
            if !res.ok {
                bail!(
                    "{}",
                    res.error
                        .unwrap_or_else(|| "WASM prepare_subagent_spawn failed".into())
                );
            }
            Ok(())
        }

        async fn on_subagent_ended(&self, result: &SubagentResult) -> anyhow::Result<()> {
            let req = HookRequest {
                hook: "on_subagent_ended".into(),
                session: None,
                turn: None,
                context: None,
                spawn_request: None,
                subagent_result: Some(serde_json::to_value(result)?),
            };
            let res = self.invoke_sync(&req)?;
            if !res.ok {
                bail!(
                    "{}",
                    res.error
                        .unwrap_or_else(|| "WASM on_subagent_ended failed".into())
                );
            }
            Ok(())
        }
    }

    pub fn load_wasm_context_engine(path: &Path) -> anyhow::Result<Arc<dyn ContextEngine>> {
        Ok(Arc::new(WasmContextEngine::load(path)?))
    }
}

// ─── Feature-absent stub ──────────────────────────────────────────────────────

#[cfg(any(
    not(feature = "wasm-tools"),
    not(any(target_os = "linux", target_os = "macos", target_os = "windows"))
))]
mod inner {
    use super::*;
    use anyhow::bail;
    use std::sync::Arc;

    pub fn load_wasm_context_engine(_path: &Path) -> anyhow::Result<Arc<dyn ContextEngine>> {
        bail!(
            "WASM context engine requires --features wasm-tools on Linux/macOS/Windows; \
             recompile with --features wasm-tools"
        );
    }
}

pub use inner::load_wasm_context_engine;
