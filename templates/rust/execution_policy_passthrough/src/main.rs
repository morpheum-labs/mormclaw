#![forbid(unsafe_code)]

//! Execution Policy Passthrough — minimal WASM ExecutionPolicy plugin.
//!
//! Protocol: read JSON from stdin, write JSON to stdout.
//! Host sends: { "hook": "can_execute_tool", "tool_name": "...", "args": {...} }
//! Host expects: { "ok": true, "allowed": true } or { "ok": false, "allowed": false, "error": "..." }
//!
//! WIT: wit/zeroclaw/execution-policy/v1/execution-policy.wit
//!
//! Build:  cargo build --target wasm32-wasip1 --release
//!         cp target/wasm32-wasip1/release/execution_policy_passthrough.wasm policy.wasm
//!
//! Config: [plugins.slots]
//!         executionPolicy = "wasm:./plugins/policy.wasm"

use std::io::{self, Read, Write};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct CanExecuteToolRequest {
    hook: String,
    tool_name: String,
    args: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct CanExecuteToolResponse {
    ok: bool,
    allowed: bool,
    error: Option<String>,
}

fn main() {
    let mut buf = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut buf) {
        let out = serde_json::to_string(&CanExecuteToolResponse {
            ok: false,
            allowed: false,
            error: Some(format!("failed to read stdin: {e}")),
        })
        .unwrap_or(r#"{"ok":false,"allowed":false,"error":"serialization error"}"#.to_string());
        let _ = io::stdout().write_all(out.as_bytes());
        return;
    }

    let _req: CanExecuteToolRequest = match serde_json::from_str(&buf) {
        Ok(r) => r,
        Err(e) => {
            let out = serde_json::to_string(&CanExecuteToolResponse {
                ok: false,
                allowed: false,
                error: Some(format!("invalid input: {e}")),
            })
            .unwrap_or(r#"{"ok":false,"allowed":false,"error":"serialization error"}"#.to_string());
            let _ = io::stdout().write_all(out.as_bytes());
            return;
        }
    };

    // No-op: always allow
    let out = serde_json::to_string(&CanExecuteToolResponse {
        ok: true,
        allowed: true,
        error: None,
    })
    .unwrap_or(r#"{"ok":false,"allowed":false,"error":"serialization error"}"#.to_string());
    let _ = io::stdout().write_all(out.as_bytes());
}
