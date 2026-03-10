#![forbid(unsafe_code)]

//! Subagent Spawner Passthrough — minimal WASM SubagentSpawner plugin.
//!
//! Protocol: read JSON from stdin, write JSON to stdout.
//! Host sends: { "hook": "can_spawn", "spawn_request": { "agent_id": "...", "command": "..." } }
//! Host expects: { "ok": true, "allowed": true } or { "ok": false, "allowed": false, "error": "..." }
//!
//! WIT: wit/zeroclaw/subagent-spawner/v1/subagent-spawner.wit
//!
//! Build:  cargo build --target wasm32-wasip1 --release
//!         cp target/wasm32-wasip1/release/subagent_spawner_passthrough.wasm spawner.wasm
//!
//! Config: [plugins.slots]
//!         subagentSpawner = "wasm:./plugins/spawner.wasm"

use std::io::{self, Read, Write};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct CanSpawnRequest {
    hook: String,
    spawn_request: SpawnRequest,
}

#[derive(Debug, Deserialize)]
struct SpawnRequest {
    agent_id: String,
    command: String,
}

#[derive(Debug, Serialize)]
struct CanSpawnResponse {
    ok: bool,
    allowed: bool,
    error: Option<String>,
}

fn main() {
    let mut buf = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut buf) {
        let out = serde_json::to_string(&CanSpawnResponse {
            ok: false,
            allowed: false,
            error: Some(format!("failed to read stdin: {e}")),
        })
        .unwrap_or(r#"{"ok":false,"allowed":false,"error":"serialization error"}"#.to_string());
        let _ = io::stdout().write_all(out.as_bytes());
        return;
    }

    let _req: CanSpawnRequest = match serde_json::from_str(&buf) {
        Ok(r) => r,
        Err(e) => {
            let out = serde_json::to_string(&CanSpawnResponse {
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
    let out = serde_json::to_string(&CanSpawnResponse {
        ok: true,
        allowed: true,
        error: None,
    })
    .unwrap_or(r#"{"ok":false,"allowed":false,"error":"serialization error"}"#.to_string());
    let _ = io::stdout().write_all(out.as_bytes());
}
