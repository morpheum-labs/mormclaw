//! Slot-based plugin registry.
//!
//! Mirrors OpenClaw's `plugins.slots.contextEngine` config: one plugin per slot,
//! selected by ID (e.g. "mormos-legacy", "mormos-onchain").

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use crate::lifecycle::ContextEngine;

/// Pluggable extension points. Each slot holds at most one active plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Slot {
    /// Context assembly, memory retrieval, compaction — the star slot (OpenClaw parity).
    ContextEngine,
    /// Memory backend selection and policy.
    MemoryManager,
    /// Tool execution sandboxing and routing.
    ToolExecutor,
    /// Human approval, budget caps, on-chain confirmation.
    ApprovalGuard,
    /// Execution policy: budget, on-chain safety, human confirmation.
    ExecutionPolicy,
    /// Sub-agent spawning and lifecycle.
    SubagentSpawner,
    /// Metrics, tracing, cost tracking.
    Observability,
}

impl Slot {
    /// Config key for this slot (e.g. `contextEngine`, `memoryManager`).
    pub fn config_key(&self) -> &'static str {
        match self {
            Slot::ContextEngine => "contextEngine",
            Slot::MemoryManager => "memoryManager",
            Slot::ToolExecutor => "toolExecutor",
            Slot::ApprovalGuard => "approvalGuard",
            Slot::ExecutionPolicy => "executionPolicy",
            Slot::SubagentSpawner => "subagentSpawner",
            Slot::Observability => "observability",
        }
    }
}

/// Base plugin trait. Slot-specific behavior is via `ContextEngine`, etc.
#[async_trait]
pub trait Plugin: Send + Sync {
    fn id(&self) -> &str;
    fn supports_slot(&self, slot: Slot) -> bool;
}

/// Registry of slot-bound plugins. One plugin per slot.
pub struct PluginRegistry {
    context_engine: Option<Arc<dyn ContextEngine>>,
    active_context_engine_id: Option<String>,
    /// Future: MemoryManager, ToolExecutor, ApprovalGuard, etc.
    _slots: HashMap<Slot, String>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            context_engine: None,
            active_context_engine_id: None,
            _slots: HashMap::new(),
        }
    }

    /// Register the ContextEngine for the given slot. Replaces any existing.
    pub fn register_context_engine(&mut self, id: impl Into<String>, engine: Arc<dyn ContextEngine>) {
        let id = id.into();
        self.active_context_engine_id = Some(id.clone());
        self.context_engine = Some(engine);
        self._slots.insert(Slot::ContextEngine, id);
    }

    /// Get the active ContextEngine, if any.
    pub fn get_context_engine(&self) -> Option<Arc<dyn ContextEngine>> {
        self.context_engine.clone()
    }

    /// ID of the active context engine (e.g. "mormos-legacy", "mormos-onchain").
    pub fn active_context_engine_id(&self) -> Option<&str> {
        self.active_context_engine_id.as_deref()
    }

    /// Check if a slot has an active plugin.
    pub fn has_slot(&self, slot: Slot) -> bool {
        match slot {
            Slot::ContextEngine => self.context_engine.is_some(),
            _ => self._slots.contains_key(&slot),
        }
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
