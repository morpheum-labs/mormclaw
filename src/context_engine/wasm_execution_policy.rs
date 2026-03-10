//! WASM ExecutionPolicy — executes a `.wasm` binary as a tool execution policy plugin.
//!
//! # Feature gate
//! Compiled when `--features wasm-tools` is active on supported targets.
//!
//! # Protocol (WASI stdio)
//!
//! Host → stdin:  `{ "hook": "can_execute_tool", "tool_name": "...", "args": {...} }`
//! Host ← stdout: `{ "ok": true, "allowed": true }` or `{ "ok": false, "allowed": false, "error": "..." }`

use std::path::Path;

#[allow(unused_imports)]
use mormos_plugin_registry::ExecutionPolicy;

const MAX_OUTPUT_BYTES: usize = 65_536;
const WASM_TIMEOUT_SECS: u64 = 10;

#[cfg(all(
    feature = "wasm-tools",
    any(target_os = "linux", target_os = "macos", target_os = "windows")
))]
mod inner {
    use super::*;
    use anyhow::{bail, Context};
    use async_trait::async_trait;
    use serde::{Deserialize, Serialize};
    use std::sync::Arc;
    use wasmtime::{Config as WtConfig, Engine, Linker, Module, Store};
    use wasmtime_wasi::{
        p2::pipe::{MemoryInputPipe, MemoryOutputPipe},
        preview1::{self, WasiP1Ctx},
        WasiCtxBuilder,
    };

    #[derive(Debug, Serialize, Deserialize)]
    struct CanExecuteToolRequest {
        hook: String,
        tool_name: String,
        args: serde_json::Value,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct CanExecuteToolResponse {
        ok: bool,
        allowed: bool,
        error: Option<String>,
    }

    pub struct WasmExecutionPolicy {
        engine: Engine,
        module: Module,
    }

    impl WasmExecutionPolicy {
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

        fn invoke_sync(
            &self,
            req: &CanExecuteToolRequest,
        ) -> anyhow::Result<CanExecuteToolResponse> {
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
                .context("WASM module must export '_start'")
                .and_then(|start| start.call(&mut store, ()).context("WASM execution failed"));
            let _ = stop_tx.send(());
            let _ = ticker.join();
            call_result?;
            let raw = stdout_for_read.contents().to_vec();
            if raw.is_empty() {
                bail!("WASM execution policy wrote nothing to stdout");
            }
            serde_json::from_slice::<CanExecuteToolResponse>(&raw)
                .context("WASM execution policy stdout is not valid JSON")
        }
    }

    #[async_trait]
    impl ExecutionPolicy for WasmExecutionPolicy {
        async fn can_execute_tool(
            &self,
            tool_name: &str,
            args: &serde_json::Value,
        ) -> anyhow::Result<bool> {
            let req = CanExecuteToolRequest {
                hook: "can_execute_tool".into(),
                tool_name: tool_name.to_string(),
                args: args.clone(),
            };
            let res = self.invoke_sync(&req)?;
            if !res.ok {
                bail!(
                    "{}",
                    res.error
                        .unwrap_or_else(|| "WASM can_execute_tool failed".into())
                );
            }
            Ok(res.allowed)
        }
    }

    pub fn load_wasm_execution_policy(path: &Path) -> anyhow::Result<Arc<dyn ExecutionPolicy>> {
        Ok(Arc::new(WasmExecutionPolicy::load(path)?))
    }
}

#[cfg(any(
    not(feature = "wasm-tools"),
    not(any(target_os = "linux", target_os = "macos", target_os = "windows"))
))]
mod inner {
    use super::*;
    use anyhow::bail;
    use std::sync::Arc;

    pub fn load_wasm_execution_policy(_path: &Path) -> anyhow::Result<Arc<dyn ExecutionPolicy>> {
        bail!("WASM execution policy requires --features wasm-tools on Linux/macOS/Windows");
    }
}

pub use inner::load_wasm_execution_policy;
