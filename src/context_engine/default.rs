//! Default ContextEngine that delegates to existing memory and compaction logic.

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use mormos_plugin_registry::{
    Context, ContextEngine, ExecutionPolicy, Session, SpawnRequest, SubagentSpawner,
};

use crate::agent::loop_::context::{build_context, build_hardware_context};
use crate::memory::Memory;
use crate::rag::HardwareRag;

/// Default context engine: delegates to existing `build_context`, `build_hardware_context`.
/// No-op for compact, after_turn, subagent hooks — existing loop handles those.
pub struct DefaultContextEngine {
    min_relevance_score: f64,
    rag_limit: usize,
}

impl DefaultContextEngine {
    pub fn new(min_relevance_score: f64, compact_context: bool) -> Self {
        let rag_limit = if compact_context { 2 } else { 5 };
        Self {
            min_relevance_score,
            rag_limit,
        }
    }

    /// Build from config-like params. Caller provides memory and optional RAG.
    pub async fn assemble_impl(
        mem: &dyn Memory,
        hardware_rag: Option<&HardwareRag>,
        boards: &[String],
        user_msg: &str,
        session_id: Option<&str>,
        min_relevance: f64,
        rag_limit: usize,
    ) -> Context {
        let mem_context = build_context(mem, user_msg, min_relevance, session_id).await;
        let hw_context = hardware_rag
            .map(|r| build_hardware_context(r, user_msg, boards, rag_limit))
            .unwrap_or_default();
        let full = format!("{mem_context}{hw_context}");
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S %Z");
        let enriched_prompt = if full.is_empty() {
            format!("[{now}] {user_msg}")
        } else {
            format!("{full}[{now}] {user_msg}")
        };
        Context {
            mem_context,
            hw_context,
            enriched_prompt,
            compact_transcript: None,
            compact_summary: None,
        }
    }
}

#[async_trait]
impl ContextEngine for DefaultContextEngine {
    async fn assemble(&self, session: &Session, context: &mut Context) -> anyhow::Result<()> {
        // DefaultContextEngine cannot assemble without Memory/RAG — those are
        // provided at call site. This impl is a no-op; the loop calls
        // assemble_impl directly with concrete mem/rag.
        let _ = (session, context);
        Ok(())
    }
}

/// Default sub-agent spawner: allows all spawn requests (no-op policy gate).
pub struct DefaultSubagentSpawner;

#[async_trait]
impl SubagentSpawner for DefaultSubagentSpawner {
    async fn can_spawn(&self, _request: &SpawnRequest) -> anyhow::Result<bool> {
        Ok(true)
    }
}

/// Default execution policy: allows all tool calls (no-op policy gate).
pub struct DefaultExecutionPolicy;

#[async_trait]
impl ExecutionPolicy for DefaultExecutionPolicy {
    async fn can_execute_tool(
        &self,
        _tool_name: &str,
        _args: &serde_json::Value,
    ) -> anyhow::Result<bool> {
        Ok(true)
    }
}

/// Allowlist/denylist execution policy: enforces allowed_tools and denied_tools per call.
pub struct AllowlistExecutionPolicy {
    allowed: Vec<String>,
    denied: Vec<String>,
}

impl AllowlistExecutionPolicy {
    pub fn new(allowed: Vec<String>, denied: Vec<String>) -> Self {
        Self { allowed, denied }
    }
}

#[async_trait]
impl ExecutionPolicy for AllowlistExecutionPolicy {
    async fn can_execute_tool(
        &self,
        tool_name: &str,
        _args: &serde_json::Value,
    ) -> anyhow::Result<bool> {
        if self.denied.iter().any(|t| t == tool_name) {
            return Ok(false);
        }
        if self.allowed.is_empty() {
            return Ok(true);
        }
        Ok(self.allowed.iter().any(|t| t == tool_name))
    }
}

/// Execution policy config for allowlist/denylist. Use `agent.allowed_tools` and `agent.denied_tools` when available.
#[derive(Clone, Default)]
pub struct ExecutionPolicyConfig {
    pub allowed_tools: Vec<String>,
    pub denied_tools: Vec<String>,
}

/// Allowlist/denylist sub-agent spawner: enforces agent.subagents.allowed_agents and denied_agents.
pub struct AllowlistSubagentSpawner {
    allowed: Vec<String>,
    denied: Vec<String>,
}

impl AllowlistSubagentSpawner {
    pub fn new(allowed: Vec<String>, denied: Vec<String>) -> Self {
        Self { allowed, denied }
    }
}

#[async_trait]
impl SubagentSpawner for AllowlistSubagentSpawner {
    async fn can_spawn(&self, request: &SpawnRequest) -> anyhow::Result<bool> {
        if self.denied.iter().any(|a| a == &request.agent_id) {
            return Ok(false);
        }
        if self.allowed.is_empty() {
            return Ok(true);
        }
        Ok(self.allowed.iter().any(|a| a == &request.agent_id))
    }
}

/// Factory: create registry with default context engine for given config.
pub fn create_default_registry(
    min_relevance_score: f64,
    compact_context: bool,
) -> mormos_plugin_registry::PluginRegistry {
    create_registry_from_config(
        min_relevance_score,
        compact_context,
        None,
        None,
        None,
        None,
        None,
        None,
    )
}

/// Subagents config for allowlist/denylist spawner. Use `agent.subagents` when available.
#[derive(Clone, Default)]
pub struct SubagentsPolicyConfig {
    pub allowed_agents: Vec<String>,
    pub denied_agents: Vec<String>,
}

/// Factory: create registry from config. Uses `plugins.slots.contextEngine`,
/// `plugins.slots.subagentSpawner`, and `plugins.slots.executionPolicy` when set.
/// Context engine: "mormos-legacy" or "wasm:path/to/plugin.wasm".
/// Subagent spawner: "mormos-default", "mormos-allowlist", or "wasm:path/to/plugin.wasm".
/// Execution policy: "mormos-default", "mormos-allowlist", or "wasm:path/to/plugin.wasm".
/// When `execution_policy_id` is "mormos-allowlist", `execution_policy_config` provides allow/deny lists.
pub fn create_registry_from_config(
    min_relevance_score: f64,
    compact_context: bool,
    engine_id: Option<&str>,
    base_dir: Option<&Path>,
    subagent_spawner_id: Option<&str>,
    subagents_policy: Option<&SubagentsPolicyConfig>,
    execution_policy_id: Option<&str>,
    execution_policy_config: Option<&ExecutionPolicyConfig>,
) -> mormos_plugin_registry::PluginRegistry {
    let id = engine_id.unwrap_or("mormos-legacy");
    let engine: Arc<dyn mormos_plugin_registry::ContextEngine> =
        if let Some(wasm_path) = id.strip_prefix("wasm:") {
            let path = if Path::new(wasm_path).is_absolute() {
                Path::new(wasm_path).to_path_buf()
            } else {
                base_dir.unwrap_or_else(|| Path::new(".")).join(wasm_path)
            };
            match super::wasm_engine::load_wasm_context_engine(&path) {
                Ok(e) => {
                    tracing::info!(path = %path.display(), "Loaded WASM context engine");
                    e
                }
                Err(e) => {
                    tracing::warn!(
                        path = %path.display(),
                        error = %e,
                        "Failed to load WASM context engine; falling back to mormos-legacy"
                    );
                    Arc::new(DefaultContextEngine::new(
                        min_relevance_score,
                        compact_context,
                    ))
                }
            }
        } else {
            match id {
                "mormos-legacy" => Arc::new(DefaultContextEngine::new(
                    min_relevance_score,
                    compact_context,
                )),
                other => {
                    tracing::warn!(
                        engine = %other,
                        "Unknown context engine; falling back to mormos-legacy"
                    );
                    Arc::new(DefaultContextEngine::new(
                        min_relevance_score,
                        compact_context,
                    ))
                }
            }
        };
    let mut registry = mormos_plugin_registry::PluginRegistry::new();
    registry.register_context_engine(id, engine);

    let spawner_id = subagent_spawner_id.unwrap_or("mormos-default");
    let spawner: Arc<dyn mormos_plugin_registry::SubagentSpawner> =
        if let Some(wasm_path) = spawner_id.strip_prefix("wasm:") {
            let path = if Path::new(wasm_path).is_absolute() {
                Path::new(wasm_path).to_path_buf()
            } else {
                base_dir.unwrap_or_else(|| Path::new(".")).join(wasm_path)
            };
            match super::wasm_spawner::load_wasm_subagent_spawner(&path) {
                Ok(s) => {
                    tracing::info!(path = %path.display(), "Loaded WASM subagent spawner");
                    s
                }
                Err(e) => {
                    tracing::warn!(
                        path = %path.display(),
                        error = %e,
                        "Failed to load WASM subagent spawner; falling back to mormos-default"
                    );
                    Arc::new(DefaultSubagentSpawner)
                }
            }
        } else {
            match spawner_id {
                "mormos-default" => Arc::new(DefaultSubagentSpawner),
                "mormos-allowlist" => {
                    let policy = subagents_policy.cloned().unwrap_or_default();
                    Arc::new(AllowlistSubagentSpawner::new(
                        policy.allowed_agents,
                        policy.denied_agents,
                    ))
                }
                other => {
                    tracing::warn!(
                        spawner = %other,
                        "Unknown subagent spawner; falling back to mormos-default"
                    );
                    Arc::new(DefaultSubagentSpawner)
                }
            }
        };
    registry.register_subagent_spawner(spawner_id, spawner);

    let policy_id = execution_policy_id.unwrap_or("mormos-default");
    let policy: Arc<dyn mormos_plugin_registry::ExecutionPolicy> =
        if let Some(wasm_path) = policy_id.strip_prefix("wasm:") {
            let path = if Path::new(wasm_path).is_absolute() {
                Path::new(wasm_path).to_path_buf()
            } else {
                base_dir.unwrap_or_else(|| Path::new(".")).join(wasm_path)
            };
            match super::wasm_execution_policy::load_wasm_execution_policy(&path) {
                Ok(p) => {
                    tracing::info!(path = %path.display(), "Loaded WASM execution policy");
                    p
                }
                Err(e) => {
                    tracing::warn!(
                        path = %path.display(),
                        error = %e,
                        "Failed to load WASM execution policy; falling back to mormos-default"
                    );
                    Arc::new(DefaultExecutionPolicy)
                }
            }
        } else {
            match policy_id {
                "mormos-default" => Arc::new(DefaultExecutionPolicy),
                "mormos-allowlist" => {
                    let config = execution_policy_config.cloned().unwrap_or_default();
                    Arc::new(AllowlistExecutionPolicy::new(
                        config.allowed_tools,
                        config.denied_tools,
                    ))
                }
                other => {
                    tracing::warn!(
                        policy = %other,
                        "Unknown execution policy; falling back to mormos-default"
                    );
                    Arc::new(DefaultExecutionPolicy)
                }
            }
        };
    registry.register_execution_policy(policy_id, policy);

    registry
}
