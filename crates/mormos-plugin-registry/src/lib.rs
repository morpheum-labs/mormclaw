//! Slot-based plugin registry and ContextEngine lifecycle for mormOS.
//!
//! Mirrors OpenClaw's pluginized Context Engine pattern:
//! - `Slot` enum defines pluggable extension points
//! - `PluginRegistry` holds one plugin per slot (e.g. `contextEngine = "mormos-lossless"`)
//! - `ContextEngine` trait exposes 7 lifecycle hooks for full agent controllability

pub mod lifecycle;
pub mod registry;

pub use lifecycle::{
    Context, ContextEngine, ExecutionPolicy, Session, SpawnRequest, SubagentResult,
    SubagentSpawner, Turn,
};
pub use registry::{Plugin, PluginRegistry, Slot};
