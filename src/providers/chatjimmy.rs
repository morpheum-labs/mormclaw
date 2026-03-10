//! ChatJimmy provider — chatjimmy.ai (Taalas HC1 inference, no auth).
//!
//! API reference: <https://github.com/kichichifightclubx/chatjimmy-cli/blob/main/docs/02-api-reference.md>

use crate::providers::traits::{ChatMessage, Provider, ProviderCapabilities};
use async_trait::async_trait;
use reqwest::Client;
use serde::Serialize;

const DEFAULT_BASE_URL: &str = "https://chatjimmy.ai";
const DEFAULT_MODEL: &str = "llama3.1-8B";

// ─── Request Structures ───────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChatJimmyRequest {
    messages: Vec<ChatJimmyMessage>,
    chat_options: ChatOptions,
    attachment: Option<()>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChatJimmyMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChatOptions {
    selected_model: String,
    system_prompt: String,
    top_k: u32,
}

// ─── Implementation ───────────────────────────────────────────────────────────

pub struct ChatJimmyProvider {
    base_url: String,
}

impl ChatJimmyProvider {
    fn normalize_base_url(raw: Option<&str>) -> String {
        let raw = raw
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(DEFAULT_BASE_URL);
        raw.trim_end_matches('/').to_string()
    }

    pub fn new(base_url: Option<&str>, _api_key: Option<&str>) -> Self {
        Self {
            base_url: Self::normalize_base_url(base_url),
        }
    }

    fn http_client(&self) -> Client {
        crate::config::build_runtime_proxy_client_with_timeouts("provider.chatjimmy", 60, 10)
    }

    /// Strip <|stats|>{...}<|/stats|> suffix from streaming response.
    fn strip_stats_suffix(text: &str) -> &str {
        const STATS_START: &str = "<|stats|>";
        const STATS_END: &str = "<|/stats|>";
        if let Some(start) = text.rfind(STATS_START) {
            if text[start..].find(STATS_END).is_some() {
                return text[..start].trim_end();
            }
        }
        text.trim_end()
    }

    async fn send_chat_request(
        &self,
        messages: Vec<ChatJimmyMessage>,
        model: &str,
        system_prompt: &str,
    ) -> anyhow::Result<String> {
        let url = format!("{}/api/chat", self.base_url);

        let body = ChatJimmyRequest {
            messages,
            chat_options: ChatOptions {
                selected_model: model.to_string(),
                system_prompt: system_prompt.to_string(),
                top_k: 8,
            },
            attachment: None,
        };

        tracing::debug!(
            "ChatJimmy request: url={} model={} message_count={}",
            url,
            model,
            body.messages.len(),
        );

        let response = self
            .http_client()
            .post(&url)
            .json(&body)
            .header("User-Agent", "zeroclaw/1.0")
            .send()
            .await?;

        let status = response.status();
        let body_bytes = response.bytes().await?;

        if !status.is_success() {
            let raw = String::from_utf8_lossy(&body_bytes);
            let sanitized = super::sanitize_api_error(&raw);
            tracing::error!(
                "ChatJimmy error: status={} body_excerpt={}",
                status,
                sanitized
            );
            anyhow::bail!("ChatJimmy API error ({}): {}", status, sanitized);
        }

        let text = String::from_utf8_lossy(&body_bytes).to_string();
        let content = Self::strip_stats_suffix(&text);

        if content.is_empty() {
            tracing::warn!("ChatJimmy returned empty content (input may exceed ~6k token limit)");
            anyhow::bail!(
                "ChatJimmy returned empty response. Input may exceed prefill limit (~6,064 tokens)."
            );
        }

        Ok(content.to_string())
    }
}

#[async_trait]
impl Provider for ChatJimmyProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            native_tool_calling: false,
            vision: false,
        }
    }

    async fn chat_with_system(
        &self,
        system_prompt: Option<&str>,
        message: &str,
        model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        let model = if model.is_empty() {
            DEFAULT_MODEL
        } else {
            model
        };

        let messages = vec![ChatJimmyMessage {
            role: "user".to_string(),
            content: message.to_string(),
        }];

        self.send_chat_request(messages, model, system_prompt.unwrap_or(""))
            .await
    }

    async fn chat_with_history(
        &self,
        messages: &[ChatMessage],
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        let model = if model.is_empty() {
            DEFAULT_MODEL
        } else {
            model
        };

        let system = messages
            .iter()
            .find(|m| m.role == "system")
            .map(|m| m.content.as_str())
            .unwrap_or("");

        let chat_messages: Vec<ChatJimmyMessage> = messages
            .iter()
            .filter(|m| m.role == "user" || m.role == "assistant")
            .map(|m| ChatJimmyMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        if chat_messages.is_empty() {
            return self
                .chat_with_system(
                    if system.is_empty() {
                        None
                    } else {
                        Some(system)
                    },
                    "",
                    model,
                    temperature,
                )
                .await;
        }

        self.send_chat_request(chat_messages, model, system).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_stats_suffix_removes_trailing_stats_block() {
        let text = "Hello world<|stats|>{\"decode_tokens\":10}<|/stats|>";
        assert_eq!(ChatJimmyProvider::strip_stats_suffix(text), "Hello world");
    }

    #[test]
    fn strip_stats_suffix_returns_unchanged_when_no_stats() {
        let text = "Hello world";
        assert_eq!(ChatJimmyProvider::strip_stats_suffix(text), "Hello world");
    }

    #[test]
    fn normalize_base_url_default() {
        assert_eq!(
            ChatJimmyProvider::normalize_base_url(None),
            "https://chatjimmy.ai"
        );
    }

    #[test]
    fn normalize_base_url_strips_trailing_slash() {
        assert_eq!(
            ChatJimmyProvider::normalize_base_url(Some("https://chatjimmy.ai/")),
            "https://chatjimmy.ai"
        );
    }
}
