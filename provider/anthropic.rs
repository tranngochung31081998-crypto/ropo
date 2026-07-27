use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

use super::{LLMProvider, Message, LLMResponse, ToolDefinition, TokenUsage, StreamItem, ToolCall, FunctionCall};

/// Anthropic Claude API Provider
pub struct AnthropicProvider {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl AnthropicProvider {
    pub fn new() -> Self {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .unwrap_or_else(|_| String::new());
        
        Self {
            api_key,
            base_url: "https://api.anthropic.com/v1".into(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap(),
        }
    }

    pub fn with_key(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            base_url: "https://api.anthropic.com/v1".into(),
            client: reqwest::Client::new(),
        }
    }

    /// Convert OpenAI-format messages to Anthropic format
    fn to_anthropic_messages(messages: &[Message]) -> Vec<serde_json::Value> {
        let mut anthropic_messages = Vec::new();
        let mut system_content = String::new();

        for msg in messages {
            match msg.role.as_str() {
                "system" => system_content.push_str(&msg.content),
                "user" => {
                    anthropic_messages.push(json!({
                        "role": "user",
                        "content": msg.content
                    }));
                }
                "assistant" => {
                    let mut assistant_msg = json!({
                        "role": "assistant",
                        "content": msg.content
                    });
                    if let Some(_calls) = &msg.tool_calls {
                        assistant_msg["content"] = json!(msg.content);
                        // Anthropic tool_calls format khác
                    }
                    anthropic_messages.push(assistant_msg);
                }
                "tool" => {
                    anthropic_messages.push(json!({
                        "role": "user",
                        "content": [
                            {
                                "type": "tool_result",
                                "tool_use_id": msg.tool_call_id,
                                "content": msg.content
                            }
                        ]
                    }));
                }
                _ => {}
            }
        }

        anthropic_messages
    }
}

#[async_trait]
impl LLMProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    async fn chat(
        &self,
        messages: &[Message],
        model: &str,
        tools: &[ToolDefinition],
    ) -> Result<LLMResponse> {
        let url = format!("{}/messages", self.base_url);
        
        let anthropic_messages = Self::to_anthropic_messages(messages);
        
        let mut body = json!({
            "model": model,
            "messages": anthropic_messages,
            "max_tokens": 4096,
        });

        if !tools.is_empty() {
            let anthropic_tools: Vec<serde_json::Value> = tools.iter().map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "input_schema": t.parameters,
                })
            }).collect();
            body["tools"] = json!(anthropic_tools);
        }

        let response = self.client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Anthropic API error {}: {}", status, error_text));
        }

        let data: serde_json::Value = response.json().await?;
        
        let content = data["content"][0]["text"]
            .as_str()
            .map(|s| s.to_string());

        let tool_calls = data["content"].as_array()
            .map(|blocks| {
                blocks.iter()
                    .filter(|b| b["type"] == "tool_use")
                    .map(|block| {
                        ToolCall {
                            id: block["id"].as_str().unwrap_or("").to_string(),
                            call_type: "function".into(),
                            function: FunctionCall {
                                name: block["name"].as_str().unwrap_or("").to_string(),
                                arguments: block["input"].to_string(),
                            },
                        }
                    }).collect::<Vec<_>>()
            })
            .filter(|v: &Vec<ToolCall>| !v.is_empty());

        let usage = TokenUsage {
            prompt_tokens: data["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: data["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: 0,
        };

        Ok(LLMResponse {
            content,
            tool_calls,
            usage,
            model: model.to_string(),
            provider: "anthropic".into(),
        })
    }

    async fn chat_stream(
        &self,
        _messages: &[Message],
        _model: &str,
        _tools: &[ToolDefinition],
    ) -> Result<Box<dyn StreamItem>> {
        Err(anyhow::anyhow!("Streaming not yet implemented for Anthropic"))
    }
}
