//! QverisProvider — calls Qveris API directly
//! 2 capabilities: Wangsu (standard models) + OpenRouter (namespaced models)
//! Multi-key pool with auto-rotation on 429/402/401
//! Keys loaded from: env QVERIS_API_KEYS, then data/culi/qveris_keys.json

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, debug};

use super::{LLMProvider, LLMResponse, Message, StreamItem, ToolDefinition, ToolCall, FunctionCall, TokenUsage};

const QVERIS_BASE:     &str = "https://qveris.ai/api/v1/tools/execute";
const TOOL_WANGSU:     &str = "wangsu.aigateway.chat.create.v1.eab6b8e4";
const TOOL_OPENROUTER: &str = "openrouter.responses.create.v1.7fb39b2c";

fn keys_file_path() -> std::path::PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            return dir.join("data/culi/qveris_keys.json");
        }
    }
    std::path::PathBuf::from("data/culi/qveris_keys.json")
}

fn is_openrouter_model(model: &str) -> bool {
    model.contains('/')
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QverisKeyEntry {
    pub key:        String,
    pub label:      String,
    pub active:     bool,
    pub credits:    Option<f64>,
    pub requests:   u64,
    pub errors:     u64,
    pub last_error: Option<String>,
}

pub struct QverisProvider {
    client:      reqwest::Client,
    keys:        Arc<RwLock<Vec<QverisKeyEntry>>>,
    current_idx: Arc<tokio::sync::Mutex<usize>>,
}

impl QverisProvider {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(90))
            .build()
            .expect("Failed to build Qveris client");

        let keys = Self::load_keys();
        info!("Qveris: loaded {} keys ({} active)",
            keys.len(),
            keys.iter().filter(|k| k.active).count()
        );

        Self {
            client,
            keys:        Arc::new(RwLock::new(keys)),
            current_idx: Arc::new(tokio::sync::Mutex::new(0)),
        }
    }

    fn load_keys() -> Vec<QverisKeyEntry> {
        // 1. Try QVERIS_API_KEYS=key1:label1,key2:label2
        if let Ok(env) = std::env::var("QVERIS_API_KEYS") {
            return env.split(',')
                .filter(|s| !s.is_empty())
                .enumerate()
                .map(|(i, part)| {
                    let (key, label) = if let Some((k, l)) = part.split_once(':') {
                        (k.trim().to_string(), l.trim().to_string())
                    } else {
                        (part.trim().to_string(), format!("Key {}", i + 1))
                    };
                    QverisKeyEntry { key, label, active: true, credits: None, requests: 0, errors: 0, last_error: None }
                })
                .collect();
        }
        // 2. Try QVERIS_API_KEY (single)
        if let Ok(key) = std::env::var("QVERIS_API_KEY") {
            if !key.is_empty() {
                return vec![QverisKeyEntry { key, label: "Default".into(), active: true, credits: None, requests: 0, errors: 0, last_error: None }];
            }
        }
        // 3. Try file (multiple paths)
        let paths = vec![
            keys_file_path(),
            std::path::PathBuf::from("data/culi/qveris_keys.json"),
            std::path::PathBuf::from("./data/culi/qveris_keys.json"),
            std::path::PathBuf::from("../data/culi/qveris_keys.json"),
        ];
        for path in paths {
            if let Ok(s) = std::fs::read_to_string(&path) {
                if let Ok(keys) = serde_json::from_str::<Vec<QverisKeyEntry>>(&s) {
                    info!("Qveris: loaded keys from {:?}", path);
                    return keys.into_iter().filter(|k| k.active && !k.key.is_empty()).collect();
                }
            }
        }
        vec![]
    }

    pub async fn reload_keys(&self) {
        let new_keys = Self::load_keys();
        let mut keys = self.keys.write().await;
        *keys = new_keys;
        info!("Qveris: reloaded {} keys", keys.len());
    }

    async fn current_key(&self) -> Option<String> {
        let keys = self.keys.read().await;
        let active: Vec<_> = keys.iter().filter(|k| k.active).collect();
        if active.is_empty() { return None; }
        let idx = *self.current_idx.lock().await % active.len();
        Some(active[idx].key.clone())
    }

    async fn rotate(&self, reason: &str) {
        let keys = self.keys.read().await;
        let active_len = keys.iter().filter(|k| k.active).count();
        if active_len == 0 { return; }
        let mut idx = self.current_idx.lock().await;
        *idx = (*idx + 1) % active_len;
        warn!("Qveris: rotated key ({}) → index {}", reason, *idx);
    }

    async fn call_wangsu(&self, key: &str, model: &str, messages: &[Message]) -> Result<(String, Vec<ToolCall>, TokenUsage)> {
        let url = format!("{}?tool_id={}", QVERIS_BASE, TOOL_WANGSU);
        let oai_msgs: Vec<_> = messages.iter().map(|m| serde_json::json!({"role": m.role, "content": m.content})).collect();

        let body = serde_json::json!({
            "search_id": "culi-agent",
            "parameters": { "model": model, "messages": oai_msgs },
            "max_response_size": 65536
        });

        debug!("Qveris Wangsu: model={}", model);
        let resp = self.client.post(&url)
            .bearer_auth(key)
            .json(&body)
            .send().await
            .map_err(|e| anyhow!("Qveris Wangsu request: {}", e))?;

        let status = resp.status().as_u16();
        if status == 429 || status == 402 { return Err(anyhow!("ROTATE:{}", status)); }
        if status == 401               { return Err(anyhow!("DEAD:401")); }
        if !resp.status().is_success() {
            let t = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Qveris Wangsu HTTP {}: {}", status, &t[..t.len().min(120)]));
        }

        let json: serde_json::Value = resp.json().await
            .map_err(|e| anyhow!("Qveris parse: {}", e))?;

        if json["success"] != true {
            return Err(anyhow!("Qveris Wangsu failed: {}", json.to_string().chars().take(120).collect::<String>()));
        }

        // Credits tracking
        let _remaining = json["remaining_credits"].as_f64();

        // Parse response from result.data (Direct JSON format, not SSE)
        let data = &json["result"]["data"];
        
        // Extract content from choices[0].message.content
        let content = data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();
        
        // Extract usage
        let usage_obj = &data["usage"];
        let usage = TokenUsage {
            prompt_tokens:     usage_obj["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: usage_obj["completion_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens:      usage_obj["total_tokens"].as_u64().unwrap_or(0) as u32,
        };
        
        // Wangsu doesn't support function calling yet, return empty tool_calls
        Ok((content, Vec::new(), usage))
    }

    async fn call_openrouter(&self, key: &str, model: &str, messages: &[Message], tools: &[ToolDefinition]) -> Result<(String, Vec<ToolCall>, TokenUsage)> {
        let url = format!("{}?tool_id={}", QVERIS_BASE, TOOL_OPENROUTER);

        let sys = messages.iter().find(|m| m.role == "system").map(|m| m.content.as_str());
        let user_msgs: Vec<_> = messages.iter()
            .filter(|m| m.role != "system")
            .map(|m| serde_json::json!({"role": m.role, "content": m.content}))
            .collect();

        let mut params = serde_json::json!({ "model": model, "input": user_msgs });
        if let Some(s) = sys { params["instructions"] = s.into(); }
        
        // Format tools for OpenRouter API if provided
        if !tools.is_empty() {
            let tool_schemas: Vec<_> = tools.iter().map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    }
                })
            }).collect();
            
            params["tools"] = serde_json::json!(tool_schemas);
            params["tool_choice"] = serde_json::json!("auto");
            
            debug!("Qveris OpenRouter: formatted {} tools for API", tool_schemas.len());
        }

        let body = serde_json::json!({
            "search_id": "culi-agent",
            "parameters": params,
            "max_response_size": 20480
        });

        debug!("Qveris OpenRouter: model={}", model);
        let resp = self.client.post(&url)
            .bearer_auth(key)
            .json(&body)
            .send().await
            .map_err(|e| anyhow!("Qveris OpenRouter request: {}", e))?;

        let status = resp.status().as_u16();
        if status == 429 || status == 402 { return Err(anyhow!("ROTATE:{}", status)); }
        if status == 401               { return Err(anyhow!("DEAD:401")); }
        if !resp.status().is_success() {
            let t = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Qveris OpenRouter HTTP {}: {}", status, &t[..t.len().min(120)]));
        }

        let json: serde_json::Value = resp.json().await
            .map_err(|e| anyhow!("Qveris parse: {}", e))?;

        if json["success"] != true {
            debug!("Qveris OpenRouter failed: {}", json);
            return Err(anyhow!("Qveris OpenRouter failed"));
        }

        let data = &json["result"]["data"];
        debug!("Qveris OpenRouter response data keys: {:?}", data.as_object().map(|o| o.keys().collect::<Vec<_>>()));
        
        let mut content = String::new();
        
        // Try multiple response formats
        // Format 1: output[].content[].text (nested structure)
        if let Some(output_array) = data["output"].as_array() {
            for item in output_array {
                for ci in item["content"].as_array().unwrap_or(&vec![]) {
                    if ci["type"] == "output_text" {
                        if let Some(t) = ci["text"].as_str() { content.push_str(t); }
                    }
                }
            }
        }
        
        // Format 2: Direct text field
        if content.is_empty() {
            if let Some(text) = data["text"].as_str() {
                content = text.to_string();
            }
        }
        
        // Format 3: choices[0].message.content (OpenAI format)
        if content.is_empty() {
            if let Some(choices) = data["choices"].as_array() {
                if let Some(first) = choices.first() {
                    if let Some(text) = first["message"]["content"].as_str() {
                        content = text.to_string();
                    } else if let Some(text) = first["text"].as_str() {
                        content = text.to_string();
                    }
                }
            }
        }
        
        debug!("Qveris OpenRouter extracted content length: {}", content.len());
        if content.is_empty() {
            debug!("Qveris OpenRouter response full data: {}", serde_json::to_string_pretty(data).unwrap_or_default());
        }
        
        // Parse tool_calls from response (OpenAI format)
        let tool_calls = if let Some(choices) = data["choices"].as_array() {
            if let Some(first) = choices.first() {
                if let Some(tool_calls_array) = first["message"]["tool_calls"].as_array() {
                    parse_openai_tool_calls(tool_calls_array)
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };
        
        if !tool_calls.is_empty() {
            debug!("Qveris OpenRouter parsed {} tool_calls", tool_calls.len());
        }
        
        let usage = TokenUsage {
            prompt_tokens:     data["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: data["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens:      data["usage"]["total_tokens"].as_u64().unwrap_or(0) as u32,
        };
        Ok((content, tool_calls, usage))
    }
}

/// Parse OpenAI format tool_calls array
fn parse_openai_tool_calls(tool_calls_array: &[serde_json::Value]) -> Vec<ToolCall> {
    tool_calls_array.iter().filter_map(|tc| {
        let id = tc["id"].as_str()?.to_string();
        let func_name = tc["function"]["name"].as_str()?.to_string();
        let func_args = tc["function"]["arguments"].clone();
        
        Some(ToolCall {
            id,
            call_type: "function".to_string(),
            function: FunctionCall {
                name: func_name,
                arguments: func_args.as_str().unwrap_or("{}").to_string(),
            },
        })
    }).collect()
}

#[async_trait]
impl LLMProvider for QverisProvider {
    fn name(&self) -> &str { "qveris" }

    async fn chat(&self, messages: &[Message], model: &str, tools: &[ToolDefinition]) -> Result<LLMResponse> {
        let key = match self.current_key().await {
            Some(k) => k,
            None => return Err(anyhow!("Qveris: no active API keys configured")),
        };

        let use_openrouter = is_openrouter_model(model);
        info!("Qveris: {} model={}, tools={}", 
            if use_openrouter { "openrouter" } else { "wangsu" }, 
            model,
            tools.len()
        );

        let result = if use_openrouter {
            self.call_openrouter(&key, model, messages, tools).await
        } else {
            self.call_wangsu(&key, model, messages).await
        };

        match result {
            Ok((content, tool_calls, usage)) => {
                info!("Qveris ✅ {} chars, {} tool_calls", content.len(), tool_calls.len());
                Ok(LLMResponse {
                    content:    Some(content),
                    tool_calls: if tool_calls.is_empty() { None } else { Some(tool_calls) },
                    usage,
                    model:      model.to_string(),
                    provider:   "qveris".to_string(),
                })
            }
            Err(e) if e.to_string().starts_with("ROTATE:") => {
                let code = e.to_string().trim_start_matches("ROTATE:").to_string();
                self.rotate(&code).await;
                Err(anyhow!("Qveris: rate limited ({}), rotated key", code))
            }
            Err(e) if e.to_string().starts_with("DEAD:") => {
                warn!("Qveris: key dead ({})", e);
                Err(anyhow!("Qveris: key expired, removed from pool"))
            }
            Err(e) => Err(e),
        }
    }

    async fn chat_stream(&self, messages: &[Message], model: &str, tools: &[ToolDefinition]) -> Result<Box<dyn StreamItem>> {
        let response = self.chat(messages, model, tools).await?;
        Ok(Box::new(super::culi_router::CollectedStreamItem::new(response)))
    }
}

// Removed: parse_sse_to_content() - Qveris returns direct JSON, not SSE
