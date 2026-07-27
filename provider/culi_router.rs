//! CuliRouterProvider — forwards all LLM calls to CulirouterAPI at :4000
//!
//! CulirouterAPI is an OpenAI-compatible gateway that provides:
//! - Auto failover: Blackbox → Sixth AI → Qveris
//! - 30+ models (deepseek-v4-flash, claude-fable-5, gpt-4.1-mini, etc.)
//! - SSE streaming
//! - Zero API keys needed for free-tier models
//!
//! Architecture:
//!   CULI Agent → CuliRouterProvider → CulirouterAPI(:4000) → Blackbox/Sixth/Qveris

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, debug};

use super::{LLMProvider, LLMResponse, Message, ToolCall, ToolDefinition, TokenUsage, FunctionCall};

/// Default model to use when none is specified.
/// "deepseek-v4-flash" = free tier via Blackbox, fastest option.
pub const DEFAULT_MODEL: &str = "deepseek-v4-flash";

/// Provider that delegates all requests to CulirouterAPI
pub struct CuliRouterProvider {
    /// Base URL of CulirouterAPI (default: http://localhost:4000)
    base_url: String,
    /// HTTP client for API calls
    client: reqwest::Client,
    /// Default model when caller doesn't specify
    default_model: String,
}

// ─── OpenAI-compatible request/response types ─────────────────────────────

#[derive(Debug, Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [OaiMessage],
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<&'a [OaiTool]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<&'a str>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct OaiMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OaiToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct OaiToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: OaiFunctionCall,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct OaiFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Debug, Serialize)]
struct OaiTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OaiFunction,
}

#[derive(Debug, Serialize)]
struct OaiFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ChatResponse {
    model: Option<String>,
    choices: Vec<Choice>,
    usage: Option<OaiUsage>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Choice {
    message: Option<ChoiceMessage>,
    delta: Option<ChoiceMessage>,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChoiceMessage {
    content: Option<String>,
    tool_calls: Option<Vec<OaiToolCall>>,
}

#[derive(Debug, Deserialize)]
struct OaiUsage {
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
    total_tokens: Option<u32>,
}

// ─── Implementation ────────────────────────────────────────────────────────

impl CuliRouterProvider {
    /// Create with default settings (localhost:4000)
    pub fn new() -> Self {
        let base_url = std::env::var("CULI_ROUTER_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:4000".to_string());
        Self::with_url(base_url)
    }

    /// Create with explicit base URL
    pub fn with_url(base_url: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client,
            default_model: DEFAULT_MODEL.to_string(),
        }
    }

    /// Set default model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    /// Check if CulirouterAPI is reachable
    pub async fn is_available(&self) -> bool {
        match self.client
            .get(format!("{}/health", self.base_url))
            .timeout(std::time::Duration::from_secs(3))
            .send()
            .await
        {
            Ok(r) => r.status().is_success(),
            Err(_) => false,
        }
    }

    /// Convert CULI Message to OpenAI format
    fn convert_messages(messages: &[Message]) -> Vec<OaiMessage> {
        messages.iter().map(|m| OaiMessage {
            role: m.role.clone(),
            content: if m.content.is_empty() { None } else { Some(m.content.clone()) },
            tool_calls: m.tool_calls.as_ref().map(|tcs| {
                tcs.iter().map(|tc| OaiToolCall {
                    id: tc.id.clone(),
                    call_type: tc.call_type.clone(),
                    function: OaiFunctionCall {
                        name: tc.function.name.clone(),
                        arguments: tc.function.arguments.clone(),
                    },
                }).collect()
            }),
            tool_call_id: m.tool_call_id.clone(),
            name: m.name.clone(),
        }).collect()
    }

    /// Convert ToolDefinitions to OpenAI format
    fn convert_tools(tools: &[ToolDefinition]) -> Vec<OaiTool> {
        tools.iter().map(|t| OaiTool {
            tool_type: "function".to_string(),
            function: OaiFunction {
                name: t.name.clone(),
                description: t.description.clone(),
                parameters: t.parameters.clone(),
            },
        }).collect()
    }

    /// Non-streaming chat (stream=false)
    async fn chat_no_stream(
        &self,
        model: &str,
        oai_messages: &[OaiMessage],
        oai_tools: &[OaiTool],
    ) -> Result<LLMResponse> {
        let payload = ChatRequest {
            model,
            messages: oai_messages,
            tools: if oai_tools.is_empty() { None } else { Some(oai_tools) },
            tool_choice: if oai_tools.is_empty() { None } else { Some("auto") },
            stream: false,
            temperature: None,
            max_tokens: None,
        };

        let url = format!("{}/v1/chat/completions", self.base_url);
        debug!("CuliRouter non-stream → {} model={}", url, model);

        let res = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| anyhow!("CulirouterAPI request failed: {}", e))?;

        // Read provider/model from response headers
        let resp_provider = res.headers()
            .get("x-culi-provider")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("culi-router")
            .to_string();
        let resp_model = res.headers()
            .get("x-culi-model")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(anyhow!("CulirouterAPI {} — {}", status, &body[..body.len().min(200)]));
        }

        let body: ChatResponse = res.json().await
            .map_err(|e| anyhow!("Failed to parse CulirouterAPI response: {}", e))?;

        let first = body.choices.into_iter().next()
            .ok_or_else(|| anyhow!("CulirouterAPI returned no choices"))?;

        let msg = first.message.unwrap_or(ChoiceMessage { content: None, tool_calls: None });

        let tool_calls = msg.tool_calls.map(|tcs| {
            tcs.into_iter().map(|tc| ToolCall {
                id: tc.id,
                call_type: tc.call_type,
                function: FunctionCall {
                    name: tc.function.name,
                    arguments: tc.function.arguments,
                },
            }).collect()
        });

        let usage = body.usage.unwrap_or(OaiUsage {
            prompt_tokens: Some(0),
            completion_tokens: Some(0),
            total_tokens: Some(0),
        });

        Ok(LLMResponse {
            content: msg.content,
            tool_calls,
            usage: TokenUsage {
                prompt_tokens:     usage.prompt_tokens.unwrap_or(0),
                completion_tokens: usage.completion_tokens.unwrap_or(0),
                total_tokens:      usage.total_tokens.unwrap_or(0),
            },
            model:    resp_model.unwrap_or_else(|| model.to_string()),
            provider: resp_provider,
        })
    }

    /// Streaming chat — collects the full SSE stream then returns assembled response
    /// (Used when tool calling requires the complete response)
    async fn chat_streaming(
        &self,
        model: &str,
        oai_messages: &[OaiMessage],
        oai_tools: &[OaiTool],
    ) -> Result<LLMResponse> {
        let payload = ChatRequest {
            model,
            messages: oai_messages,
            tools: if oai_tools.is_empty() { None } else { Some(oai_tools) },
            tool_choice: if oai_tools.is_empty() { None } else { Some("auto") },
            stream: true,
            temperature: None,
            max_tokens: None,
        };

        let url = format!("{}/v1/chat/completions", self.base_url);
        debug!("CuliRouter stream → {} model={}", url, model);

        let res = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| anyhow!("CulirouterAPI stream request failed: {}", e))?;

        let resp_provider = res.headers()
            .get("x-culi-provider")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("culi-router")
            .to_string();
        let resp_model = res.headers()
            .get("x-culi-model")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(anyhow!("CulirouterAPI stream {} — {}", status, &body[..body.len().min(200)]));
        }

        // Consume SSE stream and assemble full response
        use futures::StreamExt;
        let mut stream = res.bytes_stream();
        let mut content = String::new();
        let mut tool_calls_map: std::collections::HashMap<u32, OaiToolCall> = std::collections::HashMap::new();
        let mut buf = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| anyhow!("Stream read error: {}", e))?;
            buf.push_str(&String::from_utf8_lossy(&chunk));

            // Process complete SSE lines
            while let Some(pos) = buf.find('\n') {
                let line = buf[..pos].trim().to_string();
                buf = buf[pos + 1..].to_string();

                if !line.starts_with("data: ") { continue; }
                let data = &line[6..];
                if data == "[DONE]" { break; }

                if let Ok(chunk_json) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(choices) = chunk_json["choices"].as_array() {
                        for choice in choices {
                            let delta = &choice["delta"];

                            // Accumulate content
                            if let Some(tok) = delta["content"].as_str() {
                                content.push_str(tok);
                            }

                            // Accumulate tool calls
                            if let Some(tcs) = delta["tool_calls"].as_array() {
                                for tc in tcs {
                                    let idx = tc["index"].as_u64().unwrap_or(0) as u32;
                                    let entry = tool_calls_map.entry(idx).or_insert(OaiToolCall {
                                        id: String::new(),
                                        call_type: "function".to_string(),
                                        function: OaiFunctionCall { name: String::new(), arguments: String::new() },
                                    });
                                    if let Some(id) = tc["id"].as_str() { entry.id = id.to_string(); }
                                    if let Some(n) = tc["function"]["name"].as_str() { entry.function.name = n.to_string(); }
                                    if let Some(a) = tc["function"]["arguments"].as_str() { entry.function.arguments.push_str(a); }
                                }
                            }
                        }
                    }
                }
            }
        }

        let tool_calls = if tool_calls_map.is_empty() {
            None
        } else {
            let mut sorted: Vec<_> = tool_calls_map.into_iter().collect();
            sorted.sort_by_key(|(k, _)| *k);
            Some(sorted.into_iter().map(|(_, tc)| ToolCall {
                id: tc.id,
                call_type: tc.call_type,
                function: FunctionCall { name: tc.function.name, arguments: tc.function.arguments },
            }).collect())
        };

        Ok(LLMResponse {
            content: if content.is_empty() { None } else { Some(content) },
            tool_calls,
            usage: TokenUsage::default(),
            model: resp_model.unwrap_or_else(|| model.to_string()),
            provider: resp_provider,
        })
    }
}

impl Default for CuliRouterProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LLMProvider for CuliRouterProvider {
    fn name(&self) -> &str {
        "culi-router"
    }

    async fn chat(
        &self,
        messages: &[Message],
        model: &str,
        tools: &[ToolDefinition],
    ) -> Result<LLMResponse> {
        let effective_model = if model.is_empty() || model == "auto" {
            &self.default_model
        } else {
            model
        };

        info!("CuliRouter → model={} tools={} router={}", effective_model, tools.len(), self.base_url);

        let oai_messages = Self::convert_messages(messages);
        let oai_tools    = Self::convert_tools(tools);

        // Use non-streaming for tool calls (easier to parse), streaming for plain chat
        let result = if tools.is_empty() {
            self.chat_streaming(effective_model, &oai_messages, &oai_tools).await
        } else {
            self.chat_no_stream(effective_model, &oai_messages, &oai_tools).await
        };

        match &result {
            Ok(r) => info!("CuliRouter ✅ provider={} model={} tokens={}", r.provider, r.model, r.usage.total_tokens),
            Err(e) => warn!("CuliRouter ❌ {}", e),
        }

        result
    }

    async fn chat_stream(
        &self,
        messages: &[Message],
        model: &str,
        tools: &[ToolDefinition],
    ) -> Result<Box<dyn super::StreamItem>> {
        // For streaming, wrap in a simple collector
        let response = self.chat(messages, model, tools).await?;
        Ok(Box::new(CollectedStreamItem::new(response)))
    }
}

// ─── Collected stream adaptor ─────────────────────────────────────────────
/// Wraps an already-collected LLMResponse as a StreamItem (one-shot). Public for reuse by blackbox/sixth.
pub struct CollectedStreamItem {
    content:    Option<String>,
    tool_calls: Option<Vec<ToolCall>>,
    usage:      TokenUsage,
    done:       bool,
}

impl CollectedStreamItem {
    pub fn new(r: LLMResponse) -> Self {
        Self {
            content:    r.content,
            tool_calls: r.tool_calls,
            usage:      r.usage,
            done:       false,
        }
    }
}

#[async_trait]
impl super::StreamItem for CollectedStreamItem {
    async fn next(&mut self) -> Option<Result<super::StreamChunk>> {
        if self.done { return None; }
        self.done = true;
        if let Some(content) = self.content.take() {
            Some(Ok(super::StreamChunk::Content(content)))
        } else if let Some(tcs) = self.tool_calls.take() {
            let usage = std::mem::take(&mut self.usage);
            if let Some(tc) = tcs.into_iter().next() {
                Some(Ok(super::StreamChunk::ToolCall(tc)))
            } else {
                Some(Ok(super::StreamChunk::Done(usage)))
            }
        } else {
            let usage = std::mem::take(&mut self.usage);
            Some(Ok(super::StreamChunk::Done(usage)))
        }
    }
}
