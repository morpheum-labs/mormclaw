//! ContextEngine lifecycle hooks.
//!
//! Seven explicit hooks mirror OpenClaw's `slots.contextEngine`:
//! bootstrap → ingest → assemble → [tool loop] → compact → after_turn
//! plus prepare_subagent_spawn and on_subagent_ended for sub-agent flows.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use anyhow::Result;

/// Session-scoped state. Passed to bootstrap and available across a conversation.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub channel: String,
    pub session_id: Option<String>,
}

impl Session {
    pub fn new(id: impl Into<String>, channel: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            channel: channel.into(),
            session_id: None,
        }
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }
}

/// Single turn: raw input and (after execution) output.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Turn {
    pub input: String,
    pub output: Option<String>,
}

impl Turn {
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            output: None,
        }
    }

    pub fn with_output(mut self, output: impl Into<String>) -> Self {
        self.output = Some(output.into());
        self
    }
}

/// Assembled context: memory preamble, hardware RAG, enriched prompt.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Context {
    pub mem_context: String,
    pub hw_context: String,
    pub enriched_prompt: String,
    /// When compacting history, the transcript being summarized. Engine can read.
    pub compact_transcript: Option<String>,
    /// When compacting history, the summary. Engine can modify before apply.
    pub compact_summary: Option<String>,
}

impl Context {
    pub fn full_context(&self) -> String {
        format!("{}{}", self.mem_context, self.hw_context)
    }

    /// Build a context for the compact hook (history compaction flow).
    pub fn for_compact(transcript: String, summary: String) -> Self {
        Self {
            mem_context: String::new(),
            hw_context: String::new(),
            enriched_prompt: String::new(),
            compact_transcript: Some(transcript),
            compact_summary: Some(summary),
        }
    }
}

/// Request to spawn a sub-agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnRequest {
    pub agent_id: String,
    pub command: String,
}

/// Result of a completed sub-agent run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentResult {
    pub session_id: String,
    pub success: bool,
    pub output: String,
}

/// Context engine trait — OpenClaw parity. All methods have default no-op implementations.
#[async_trait]
pub trait ContextEngine: Send + Sync {
    /// Called at session start. Initialize session-scoped state.
    async fn bootstrap(&self, _session: &mut Session) -> Result<()> {
        Ok(())
    }

    /// Ingest raw input (user message). Store, preprocess, enrich.
    async fn ingest(&self, _session: &Session, _turn: &mut Turn) -> Result<()> {
        Ok(())
    }

    /// Assemble context: memory retrieval, RAG, prompt building.
    async fn assemble(&self, _session: &Session, _context: &mut Context) -> Result<()> {
        Ok(())
    }

    /// Compact history: prune, summarize, flush durable facts to memory.
    async fn compact(&self, _session: &Session, _context: &mut Context) -> Result<()> {
        Ok(())
    }

    /// After turn completes. Persist, extract facts, update state.
    async fn after_turn(&self, _session: &Session, _turn: &Turn) -> Result<()> {
        Ok(())
    }

    /// Before spawning a sub-agent. Validate, inject context, set budget.
    async fn prepare_subagent_spawn(&self, _request: &SpawnRequest) -> Result<()> {
        Ok(())
    }

    /// When a sub-agent finishes. Merge result, update registry.
    async fn on_subagent_ended(&self, _result: &SubagentResult) -> Result<()> {
        Ok(())
    }
}

/// Sub-agent spawner trait — policy gate for sub-agent spawning (Phase 3).
/// Allows plugins to approve or reject spawn requests before execution.
#[async_trait]
pub trait SubagentSpawner: Send + Sync {
    /// Check whether a spawn request is allowed. Return `true` to allow, `false` to deny.
    async fn can_spawn(&self, request: &SpawnRequest) -> Result<bool> {
        let _ = request;
        Ok(true)
    }
}

/// Execution policy trait — policy gate for tool execution (Phase 4).
/// Allows plugins to approve or reject tool calls before execution.
#[async_trait]
pub trait ExecutionPolicy: Send + Sync {
    /// Check whether a tool call is allowed. Return `true` to allow, `false` to deny.
    async fn can_execute_tool(&self, tool_name: &str, args: &serde_json::Value) -> Result<bool> {
        let _ = (tool_name, args);
        Ok(true)
    }
}
