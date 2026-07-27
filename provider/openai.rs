use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use super::{LLMProvider, Message, LLMResponse, ToolDefinition, TokenUsage, StreamItem, ToolCall, FunctionCall};

/// OpenAI API Provider
pub struct OpenAIProvider {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl OpenAIProvider {
    pub fn new() -> Self {
        let api_key = std::env::var("OPENAI_API_KEY")
            .unwrap_or_else(|_| String::new());
        
        Self {
            api_key,
            base_url: std::env::var("OPENAI_BASE_URL")
                .unwrap_or_else(|_| "https://api.openai.com/v1".into()),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap(),
        }
    }

    pub fn with_key(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            base_url: "https://api.openai.com/v1".into(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    fn name(&self) -> &str {
        "openai"
    }

    async fn chat(
        &self,
        messages: &[Message],
        model: &str,
        tools: &[ToolDefinition],
    ) -> Result<LLMResponse> {
        let url = format!("{}/chat/completions", self.base_url);
        
        let mut body = json!({
            "model": model,
            "messages": messages,
            "temperature": 0.7,
            "max_tokens": 4096,
        });

        if !tools.is_empty() {
            body["tools"] = json!(tools);
            body["tool_choice"] = json!("auto");
        }

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("OpenAI API error {}: {}", status, error_text));
        }

        let data: serde_json::Value = response.json().await?;
        let choice = &data["choices"][0];
        let message = &choice["message"];

        let content = message["content"].as_str().map(|s| s.to_string());
        
        let tool_calls = message["tool_calls"].as_array().map(|calls| {
            calls.iter().map(|call| {
                ToolCall {
                    id: call["id"].as_str().unwrap_or("").to_string(),
                    call_type: call["type"].as_str().unwrap_or("function").to_string(),
                    function: FunctionCall {
                        name: call["function"]["name"].as_str().unwrap_or("").to_string(),
                        arguments: call["function"]["arguments"].as_str().unwrap_or("{}").to_string(),
                    },
                }
            }).collect()
        });

        let usage = data["usage"].as_object().map(|u| TokenUsage {
            prompt_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: u["total_tokens"].as_u64().unwrap_or(0) as u32,
        }).unwrap_or_default();

        Ok(LLMResponse {
            content,
            tool_calls,
            usage,
            model: model.to_string(),
            provider: "openai".into(),
        })
    }

    async fn chat_stream(
        &self,
        _messages: &[Message],
        _model: &str,
        _tools: &[ToolDefinition],
    ) -> Result<Box<dyn StreamItem>> {
        Err(anyhow::anyhow!("Streaming not yet implemented for OpenAI"))
    }
}
