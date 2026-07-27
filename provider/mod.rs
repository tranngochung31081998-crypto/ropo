pub mod router;
pub mod openai;
pub mod anthropic;
pub mod ollama;
pub mod groq;
pub mod fallback;
pub mod token_tracker;
pub mod upgrade;
pub mod composite_tiers;
pub mod sixth;
pub mod culi_router;
pub mod blackbox;
pub mod culi_models;
pub mod qveris;

#[cfg(test)]

mod composite_tiers_test;

#[cfg(test)]
mod integration_test;

pub use router::ProviderRouter;
pub use router::RouterConfig;
pub use router::TokenTracker;
pub use openai::OpenAIProvider;
pub use anthropic::AnthropicProvider;
pub use ollama::OllamaProvider;
pub use groq::GroqProvider;
pub use fallback::FallbackConfig;
pub use fallback::FallbackReason;
pub use fallback::FallbackEvent;
pub use fallback::FallbackMetrics;
pub use token_tracker::TokenTracker as TokenTrackerImpl;
pub use composite_tiers::{CompositeTiers, TierConfig};
pub use sixth::SixthProvider;
pub use culi_router::CuliRouterProvider;
pub use culi_router::CollectedStreamItem;
pub use blackbox::BlackboxProvider;
pub use culi_models::{culi_model_catalog, resolve_culi_model, CuliModel};
pub use qveris::QverisProvider;

use anyhow::Result;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Message trong conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Tool call từ LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// Response từ LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub usage: TokenUsage,
    pub model: String,
    pub provider: String,
}

/// Token usage tracking
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Tool definition cho LLM function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// LLM Provider trait - tất cả provider phải implement
#[async_trait]
pub trait LLMProvider: Send + Sync {
    fn name(&self) -> &str;
    
    async fn chat(
        &self,
        messages: &[Message],
        model: &str,
        tools: &[ToolDefinition],
    ) -> Result<LLMResponse>;
    
    async fn chat_stream(
        &self,
        messages: &[Message],
        model: &str,
        tools: &[ToolDefinition],
    ) -> Result<Box<dyn StreamItem>>;
}

/// Stream item cho streaming responses
#[async_trait]
pub trait StreamItem: Send {
    async fn next(&mut self) -> Option<Result<StreamChunk>>;
}

#[derive(Debug, Clone)]
pub enum StreamChunk {
    Content(String),
    ToolCall(ToolCall),
    Done(TokenUsage),
    Error(String),
}
