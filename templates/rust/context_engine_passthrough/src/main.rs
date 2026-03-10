#![forbid(unsafe_code)]

//! Context Engine Passthrough — minimal WASM ContextEngine plugin.
//!
//! Implements the WASI stdio protocol for mormOS ContextEngine:
//! - Host reads JSON from stdin: { "hook": "...", "session": {...}, ... }
//! - Host writes JSON to stdout: { "ok": true, "error": null, "session": null, ... }
//!
//! WIT: wit/zeroclaw/context-engine/v1/context-engine.wit
//!
//! This plugin is a no-op: all hooks succeed without modifying any data.
//! Use as a template for custom context engines.
//!
//! Build:  cargo build --target wasm32-wasip1 --release
//!         cp target/wasm32-wasip1/release/context_engine_passthrough.wasm context-engine.wasm
//!
//! Config: [plugins.slots]
//!         contextEngine = "wasm:./plugins/context-engine.wasm"

use std::io::{self, Read, Write};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct HookRequest {
    hook: String,
    #[allow(dead_code)]
    session: Option<serde_json::Value>,
    #[allow(dead_code)]
    turn: Option<serde_json::Value>,
    #[allow(dead_code)]
    context: Option<serde_json::Value>,
    #[allow(dead_code)]
    spawn_request: Option<serde_json::Value>,
    #[allow(dead_code)]
    subagent_result: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct HookResponse {
    ok: bool,
    error: Option<String>,
    session: Option<serde_json::Value>,
    turn: Option<serde_json::Value>,
    context: Option<serde_json::Value>,
}

fn write_response(r: &HookResponse) {
    let out = serde_json::to_string(r)
        .unwrap_or_else(|_| r#"{"ok":false,"error":"serialization error"}"#.to_string());
    let _ = io::stdout().write_all(out.as_bytes());
}

fn main() {
    let mut buf = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut buf) {
        write_response(&HookResponse {
            ok: false,
            error: Some(format!("failed to read stdin: {e}")),
            session: None,
            turn: None,
            context: None,
        });
        return;
    }

    let _req: HookRequest = match serde_json::from_str(&buf) {
        Ok(r) => r,
        Err(e) => {
            write_response(&HookResponse {
                ok: false,
                error: Some(format!("invalid input: {e}")),
                session: None,
                turn: None,
                context: None,
            });
            return;
        }
    };

    // No-op: always succeed with no modifications
    write_response(&HookResponse {
        ok: true,
        error: None,
        session: None,
        turn: None,
        context: None,
    });
}
