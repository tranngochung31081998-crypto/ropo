//! BlackboxProvider — calls Blackbox API directly
//! Endpoint: https://oi-vscode-server-985058387028.europe-west1.run.app/chat/completions
//! No real auth needed (Bearer xxx), rotate userIds on 401/403

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tracing::{debug, info, warn};

use super::{FunctionCall, LLMProvider, LLMResponse, Message, StreamChunk, StreamItem, ToolCall, ToolDefinition, TokenUsage};

const BASE_URL: &str = "https://oi-vscode-server-985058387028.europe-west1.run.app";
const MODEL: &str = "custom/blackbox-base";

/// Default userIds (from CulirouterAPI config). Can be overridden via env BLACKBOX_USERID
fn default_user_ids() -> Vec<String> {
    let env_ids = std::env::var("BLACKBOX_USERID").unwrap_or_default();
    if !env_ids.is_empty() {
        return env_ids.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    }
    vec![
        "892955990-8351072528-8400000445-9458952030".to_string(),
    ]
}

#[derive(Debug, Serialize)]
struct BlackboxRequest<'a> {
    model: &'static str,
    messages: &'a [OaiMsg],
    max_tokens: u32,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Serialize)]
struct OaiMsg {
    role: String,
    content: String,
}

pub struct BlackboxProvider {
    client:       reqwest::Client,
    user_ids:     Vec<String>,
    current_idx:  Arc<AtomicUsize>,
    total_reqs:   Arc<AtomicUsize>,
    total_errors: Arc<AtomicUsize>,
}

impl BlackboxProvider {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build Blackbox HTTP client");

        Self {
            client,
            user_ids: default_user_ids(),
            current_idx:  Arc::new(AtomicUsize::new(0)),
            total_reqs:   Arc::new(AtomicUsize::new(0)),
            total_errors: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn current_user_id(&self) -> &str {
        let idx = self.current_idx.load(Ordering::Relaxed) % self.user_ids.len();
        &self.user_ids[idx]
    }

    fn rotate_user_id(&self) {
        let next = (self.current_idx.load(Ordering::Relaxed) + 1) % self.user_ids.len();
        self.current_idx.store(next, Ordering::Relaxed);
        warn!("Blackbox: rotated to userId index {}", next);
    }

    fn build_headers(&self) -> reqwest::header::HeaderMap {
        use reqwest::header::*;
        let mut h = HeaderMap::new();
        h.insert(CONTENT_TYPE,  "application/json".parse().unwrap());
        h.insert(ACCEPT,        "application/json".parse().unwrap());
        h.insert("accept-encoding", "identity".parse().unwrap());
        h.insert(AUTHORIZATION, "Bearer xxx".parse().unwrap());
        h.insert("version",     "1.1".parse().unwrap());
        h.insert(USER_AGENT,    "Cs/JS 4.73.1".parse().unwrap());
        h.insert("x-stainless-arch",            "x64".parse().unwrap());
        h.insert("x-stainless-lang",            "js".parse().unwrap());
        h.insert("x-stainless-os",              "Windows".parse().unwrap());
        h.insert("x-stainless-package-version", "4.73.1".parse().unwrap());
        h.insert("x-stainless-runtime",         "node".parse().unwrap());
        h.insert("x-stainless-runtime-version", "v24.18.0".parse().unwrap());
        h.insert("userid", self.current_user_id().parse().unwrap());
        h
    }

    /// Send request, retry once with rotated userId on 401/403
    async fn send_stream(
        &self,
        messages: &[OaiMsg],
        temperature: Option<f32>,
    ) -> Result<reqwest::Response> {
        let url = format!("{BASE_URL}/chat/completions");
        let body = BlackboxRequest {
            model: MODEL,
            messages,
            max_tokens: 4096,
            stream: true,
            temperature,
        };

        for attempt in 0..2usize {
            self.total_reqs.fetch_add(1, Ordering::Relaxed);
            let resp = self.client
                .post(&url)
                .headers(self.build_headers())
                .json(&body)
                .send()
                .await
                .map_err(|e| anyhow!("Blackbox request failed: {}", e))?;

            match resp.status().as_u16() {
                401 | 403 => {
                    self.rotate_user_id();
                    self.total_errors.fetch_add(1, Ordering::Relaxed);
                    if attempt == 0 { continue; }
                    return Err(anyhow!("Blackbox: auth failed after rotation"));
                }
                200..=299 => {
                    info!("Blackbox: ✅ stream started");
                    return Ok(resp);
                }
                status => {
                    let text = resp.text().await.unwrap_or_default();
                    self.total_errors.fetch_add(1, Ordering::Relaxed);
                    return Err(anyhow!("Blackbox HTTP {}: {}", status, &text[..text.len().min(120)]));
                }
            }
        }
        Err(anyhow!("Blackbox: all attempts failed"))
    }
}

impl Default for BlackboxProvider {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl LLMProvider for BlackboxProvider {
    fn name(&self) -> &str { "blackbox" }

    async fn chat(
        &self,
        messages: &[Message],
        _model: &str,
        _tools: &[ToolDefinition],
    ) -> Result<LLMResponse> {
        debug!("Blackbox: chat request ({} messages)", messages.len());
        let oai: Vec<OaiMsg> = messages.iter().map(|m| OaiMsg {
            role:    m.role.clone(),
            content: m.content.clone(),
        }).collect();

        let resp = self.send_stream(&oai, None).await?;
        let content = collect_sse_stream(resp).await?;

        Ok(LLMResponse {
            content:    Some(content),
            tool_calls: None,
            usage:      TokenUsage::default(),
            model:      MODEL.to_string(),
            provider:   "blackbox".to_string(),
        })
    }

    async fn chat_stream(
        &self,
        messages: &[Message],
        _model: &str,
        _tools: &[ToolDefinition],
    ) -> Result<Box<dyn StreamItem>> {
        let oai: Vec<OaiMsg> = messages.iter().map(|m| OaiMsg {
            role: m.role.clone(),
            content: m.content.clone(),
        }).collect();

        let resp = self.send_stream(&oai, None).await?;
        let content = collect_sse_stream(resp).await?;
        Ok(Box::new(super::culi_router::CollectedStreamItem::new(
            LLMResponse {
                content: Some(content),
                tool_calls: None,
                usage: TokenUsage::default(),
                model: MODEL.to_string(),
                provider: "blackbox".to_string(),
            }
        )))
    }
}

/// Consume SSE stream, assemble full text content
async fn collect_sse_stream(resp: reqwest::Response) -> Result<String> {
    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    let mut content = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| anyhow!("Blackbox stream read error: {}", e))?;
        buf.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buf.find('\n') {
            let line = buf[..pos].trim().to_string();
            buf = buf[pos + 1..].to_string();

            if !line.starts_with("data: ") { continue; }
            let data = &line[6..];
            if data == "[DONE]" { break; }

            if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                if let Some(tok) = v["choices"][0]["delta"]["content"].as_str() {
                    content.push_str(tok);
                }
            }
        }
    }

    Ok(content)
}
