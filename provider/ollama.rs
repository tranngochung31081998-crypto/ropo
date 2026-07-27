use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

use super::{LLMProvider, Message, LLMResponse, ToolDefinition, TokenUsage, StreamItem};

/// Ollama Provider - chạy local models
pub struct OllamaProvider {
    base_url: String,
    client: reqwest::Client,
}

impl OllamaProvider {
    pub fn new() -> Self {
        Self {
            base_url: std::env::var("OLLAMA_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:11434".into()),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .unwrap(),
        }
    }
}

#[async_trait]
impl LLMProvider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    async fn chat(
        &self,
        messages: &[Message],
        model: &str,
        _tools: &[ToolDefinition],
    ) -> Result<LLMResponse> {
        let url = format!("{}/api/chat", self.base_url);
        
        let body = json!({
            "model": model,
            "messages": messages,
            "stream": false,
            "options": {
                "temperature": 0.7,
                "num_predict": 4096
            }
        });

        let response = self.client
            .post(&url)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Ollama error: {}", response.status()));
        }

        let data: serde_json::Value = response.json().await?;
        
        let content = data["message"]["content"]
            .as_str()
            .map(|s| s.to_string());

        Ok(LLMResponse {
            content,
            tool_calls: None,
            usage: TokenUsage {
                prompt_tokens: data["prompt_eval_count"].as_u64().unwrap_or(0) as u32,
                completion_tokens: data["eval_count"].as_u64().unwrap_or(0) as u32,
                total_tokens: 0,
            },
            model: model.to_string(),
            provider: "ollama".into(),
        })
    }

    async fn chat_stream(
        &self,
        _messages: &[Message],
        _model: &str,
        _tools: &[ToolDefinition],
    ) -> Result<Box<dyn StreamItem>> {
        Err(anyhow::anyhow!("Streaming not yet implemented for Ollama"))
    }
}
