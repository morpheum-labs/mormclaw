//! Default ContextEngine that delegates to existing memory and compaction logic.

use std::sync::Arc;

use async_trait::async_trait;
use mormos_plugin_registry::{Context, ContextEngine, Session};

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

/// Factory: create registry with default context engine for given config.
pub fn create_default_registry(
    min_relevance_score: f64,
    compact_context: bool,
) -> mormos_plugin_registry::PluginRegistry {
    create_registry_from_config(min_relevance_score, compact_context, None)
}

/// Factory: create registry from config. Uses `plugins.slots.contextEngine` when set.
/// Currently only "mormos-legacy" is supported; unknown IDs fall back to legacy.
pub fn create_registry_from_config(
    min_relevance_score: f64,
    compact_context: bool,
    engine_id: Option<&str>,
) -> mormos_plugin_registry::PluginRegistry {
    let id = engine_id.unwrap_or("mormos-legacy");
    let engine: Arc<dyn mormos_plugin_registry::ContextEngine> = match id {
        "mormos-legacy" => Arc::new(DefaultContextEngine::new(min_relevance_score, compact_context)),
        other => {
            tracing::warn!(
                engine = %other,
                "Unknown context engine; falling back to mormos-legacy"
            );
            Arc::new(DefaultContextEngine::new(min_relevance_score, compact_context))
        }
    };
    let mut registry = mormos_plugin_registry::PluginRegistry::new();
    registry.register_context_engine(id, engine);
    registry
}
