//! ContextEngine integration for mormOS.
//!
//! Wires the slot-based registry into the agent loop. `DefaultContextEngine`
//! delegates to existing memory/compaction logic for zero-breaking migration.

mod default;
mod wasm_engine;
mod wasm_execution_policy;
mod wasm_spawner;

#[allow(unused_imports)]
pub use default::{
    create_default_registry, create_registry_from_config, DefaultContextEngine,
    DefaultExecutionPolicy, DefaultSubagentSpawner, ExecutionPolicyConfig, SubagentsPolicyConfig,
};
#[allow(unused_imports)]
pub use mormos_plugin_registry::{
    Context, ContextEngine, PluginRegistry, Session, Slot, SpawnRequest, SubagentResult,
    SubagentSpawner, Turn,
};
