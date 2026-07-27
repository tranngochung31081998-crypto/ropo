//! SSE (Server-Sent Events) streaming for real-time chat responses with tool execution
//!
//! Architecture (following Hermes pattern):
//! 1. Client: POST /api/chat/stream
//! 2. Server: Tool execution loop
//!    - LLM call with tools → Tool calls? → Execute → Add results → Loop
//!    - Stream events: thinking, content, tool_call, tool_result, done
//! 3. Client: Display thinking + answer + tool execution
//!
//! Key Features:
//! - Function calling with tool definitions
//! - Tool execution loop (max 10 iterations)
//! - Real-time SSE streaming
//! - Thinking/content separation

use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
    Json,
};
use futures::stream::{Stream, StreamExt};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, error, warn};

use crate::api::{AppState, models::ChatRequest};
use crate::provider::{Message, StreamChunk, ToolDefinition, ToolCall as ProviderToolCall};
use crate::tools::ToolResult;

/// SSE Event Types
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    /// Reasoning/thinking tokens
    #[serde(rename = "thinking")]
    Thinking { content: String },
    
    /// Answer content tokens
    #[serde(rename = "content")]
    Content { content: String },
    
    /// Tool execution started
    #[serde(rename = "tool_call")]
    ToolCall {
        id: String,
        name: String,
        arguments: serde_json::Value,
    },
    
    /// Tool execution completed
    #[serde(rename = "tool_result")]
    ToolResult {
        id: String,
        name: String,
        success: bool,
        data: serde_json::Value,
        duration_ms: u64,
    },
    
    /// Streaming completed
    #[serde(rename = "done")]
    Done { 
        tokens_used: u32,
        provider: String,
        model: String,
        iterations: u32,
    },
    
    /// Error occurred
    #[serde(rename = "error")]
    Error { message: String },
}

/// POST /api/chat/stream - Streaming chat endpoint with tool execution
pub async fn chat_stream_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChatRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let session_id = req.session_id.clone().unwrap_or_else(|| {
        uuid::Uuid::new_v4().to_string()
    });
    
    let model = req.model.clone().unwrap_or_else(|| state.config.provider.model.clone());
    
    info!("🌊 Stream request: session={}, model={}", session_id, model);
    
    // Load system prompt
    let skill_loader = crate::skills::SkillLoader::new();
    let culi_identity = std::fs::read_to_string("ABOUT_CULI.md")
        .unwrap_or_else(|_| include_str!("../../ABOUT_CULI.md").to_string());
    
    let brain_section = if skill_loader.has_role("orchestrator") {
        let brain = skill_loader.load_role("orchestrator");
        let arch = skill_loader.load_architecture_summary();
        format!("\n\n═══ AGENT BRAIN ═══\n{}\n\n═══ ARCHITECTURE ═══\n{}", brain, arch)
    } else {
        String::new()
    };
    
    let system_prompt = format!(
        r#"{}

{}

## Current Context
You are an AI agent that can execute actions using tools.
User is asking you to perform tasks or answer questions.

## Available Tools
You have access to these tools:
- filesystem: Read, write, list, delete files and directories
- terminal: Execute shell commands
- web_search: Search the web for information
- web_fetch: Fetch content from URLs
- graphify: Analyze code architecture and generate graphs
- chunk_reader: Read large files in chunks
- search_replace: Search and replace text in files

## Guidelines
- Use tools to accomplish tasks, don't just describe what to do
- Read files before modifying them
- Execute commands when user asks
- Always confirm actions by actually doing them
- When user says "create file X", use filesystem tool to create it
- When user says "run command Y", use terminal tool to execute it"#,
        culi_identity,
        brain_section
    );
    
    let messages = vec![
        Message {
            role: "system".to_string(),
            content: system_prompt,
            tool_calls: None,
            tool_call_id: None,
            name: None,
        },
        Message {
            role: "user".to_string(),
            content: req.message.clone(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        },
    ];
    
    // Get tool definitions for LLM function calling
    let tool_definitions = {
        let orchestrator = state.orchestrator.lock().await;
        orchestrator.tool_registry.get_definitions()
    };
    
    info!("🔧 Loaded {} tools: {:?}", 
        tool_definitions.len(),
        tool_definitions.iter().map(|t| &t.name).collect::<Vec<_>>()
    );
    
    // Create SSE stream with tool execution loop
    let stream = create_tool_execution_stream(
        state,
        messages,
        model,
        session_id,
        tool_definitions,
    );
    
    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Create SSE stream with tool execution loop (Hermes pattern)
///
/// Flow:
/// 1. Call LLM with messages + tools
/// 2. If tool_calls in response → execute tools → add results → go to 1
/// 3. If no tool_calls → stream final answer → done
fn create_tool_execution_stream(
    state: Arc<AppState>,
    mut messages: Vec<Message>,
    model: String,
    session_id: String,
    tool_definitions: Vec<ToolDefinition>,
) -> impl Stream<Item = Result<Event, Infallible>> {
    async_stream::stream! {
        let mut iteration = 0;
        let max_iterations = 10;
        let mut total_tokens = 0u32;
        
        // ═══ Tool Execution Loop ═══
        loop {
            iteration += 1;
            
            if iteration > max_iterations {
                warn!("⚠️ Max iterations ({}) reached for session {}", max_iterations, session_id);
                let event = StreamEvent::Error {
                    message: format!("Maximum tool execution iterations ({}) reached", max_iterations),
                };
                yield Ok(Event::default().json_data(event).unwrap());
                break;
            }
            
            info!("🔄 Iteration {}/{} - {} messages in context", iteration, max_iterations, messages.len());
            
            // ─── Step 1: Call LLM with tools ───
            let llm_response = match call_llm_with_tools(
                &state,
                &messages,
                &model,
                &tool_definitions,
            ).await {
                Ok(resp) => resp,
                Err(e) => {
                    error!("❌ LLM call failed: {}", e);
                    let event = StreamEvent::Error {
                        message: format!("LLM call failed: {}", e),
                    };
                    yield Ok(Event::default().json_data(event).unwrap());
                    break;
                }
            };
            
            total_tokens += llm_response.tokens_used;
            
            // ─── Step 2: Stream thinking/content ───
            if !llm_response.thinking.is_empty() {
                let event = StreamEvent::Thinking {
                    content: llm_response.thinking.clone(),
                };
                yield Ok(Event::default().json_data(event).unwrap());
            }
            
            if !llm_response.content.is_empty() {
                let event = StreamEvent::Content {
                    content: llm_response.content.clone(),
                };
                yield Ok(Event::default().json_data(event).unwrap());
            }
            
            // ─── Step 3: Check for tool calls ───
            if llm_response.tool_calls.is_empty() {
                // No tool calls → final answer reached
                info!("✅ No tool calls, conversation complete after {} iterations", iteration);
                
                // Add assistant message to history
                messages.push(Message {
                    role: "assistant".to_string(),
                    content: llm_response.content,
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                });
                
                let event = StreamEvent::Done {
                    tokens_used: total_tokens,
                    provider: llm_response.provider,
                    model: llm_response.model,
                    iterations: iteration,
                };
                yield Ok(Event::default().json_data(event).unwrap());
                break;
            }
            
            // ─── Step 4: Execute tools ───
            info!("🔧 Executing {} tool calls", llm_response.tool_calls.len());
            
            // Add assistant message with tool calls
            messages.push(Message {
                role: "assistant".to_string(),
                content: llm_response.content.clone(),
                tool_calls: Some(llm_response.tool_calls.clone()),
                tool_call_id: None,
                name: None,
            });
            
            for tool_call in llm_response.tool_calls {
                // Stream tool call event
                let arguments_value = serde_json::from_str(&tool_call.function.arguments)
                    .unwrap_or(serde_json::json!({}));
                let event = StreamEvent::ToolCall {
                    id: tool_call.id.clone(),
                    name: tool_call.function.name.clone(),
                    arguments: arguments_value,
                };
                yield Ok(Event::default().json_data(event).unwrap());
                
                // Execute tool
                let start = std::time::Instant::now();
                let tool_result = execute_tool(&state, &tool_call).await;
                let duration_ms = start.elapsed().as_millis() as u64;
                
                // Stream tool result event
                let event = StreamEvent::ToolResult {
                    id: tool_call.id.clone(),
                    name: tool_call.function.name.clone(),
                    success: tool_result.success,
                    data: tool_result.data.clone(),
                    duration_ms,
                };
                yield Ok(Event::default().json_data(event).unwrap());
                
                // Add tool result to messages
                messages.push(Message {
                    role: "tool".to_string(),
                    content: serde_json::to_string(&tool_result.data).unwrap_or_default(),
                    tool_calls: None,
                    tool_call_id: Some(tool_call.id.clone()),
                    name: Some(tool_call.function.name.clone()),
                });
                
                info!(
                    "🔧 Tool '{}' completed: success={}, duration={}ms",
                    tool_result.name(),
                    tool_result.success,
                    duration_ms
                );
            }
            
            // ─── Step 5: Continue loop with tool results ───
            // LLM will be called again with tool results in context
        }
    }
}

/// LLM Response with tool calls
#[derive(Debug)]
struct LLMResponse {
    thinking: String,
    content: String,
    tool_calls: Vec<ProviderToolCall>,
    tokens_used: u32,
    provider: String,
    model: String,
}

/// Call LLM with tools and parse response
async fn call_llm_with_tools(
    state: &Arc<AppState>,
    messages: &[Message],
    model: &str,
    tool_definitions: &[ToolDefinition],
) -> anyhow::Result<LLMResponse> {
    use crate::provider::resolve_culi_model;
    
    let resolved = resolve_culi_model(model);
    let effective = if resolved.is_empty() { model } else { &resolved };
    
    info!("📞 Calling LLM: model={}, messages={}, tools={}", 
        effective, messages.len(), tool_definitions.len());
    
    // Pass tools to chat_with_model
    let response = state.chat_service.chat_with_model(
        messages.to_vec(),
        effective,
        tool_definitions,  // NOW PASSED!
    ).await?;
    
    let content = response.content.unwrap_or_default();
    let thinking = String::new(); // TODO: Extract from content if present
    
    // TODO: Parse tool_calls from response (Phase 1.2)
    let tool_calls = response.tool_calls.unwrap_or_default();
    
    Ok(LLMResponse {
        thinking,
        content,
        tool_calls,
        tokens_used: response.usage.total_tokens,
        provider: response.provider,
        model: response.model,
    })
}

/// Execute a single tool call
async fn execute_tool(
    state: &Arc<AppState>,
    tool_call: &ProviderToolCall,
) -> ToolResult {
    let tool_name = &tool_call.function.name;
    let args_str = serde_json::to_string(&tool_call.function.arguments)
        .unwrap_or_else(|_| "{}".to_string());
    
    info!("🔧 Executing tool: {} with args: {}", tool_name, args_str);
    
    // Get tool registry from orchestrator (need to lock)
    let orchestrator = state.orchestrator.lock().await;
    match orchestrator.tool_registry.execute(tool_name, &args_str).await {
        Ok(result) => {
            if result.success {
                info!("✅ Tool '{}' succeeded", tool_name);
            } else {
                warn!("⚠️ Tool '{}' failed: {:?}", tool_name, result.error);
            }
            result
        }
        Err(e) => {
            error!("❌ Tool '{}' execution error: {}", tool_name, e);
            ToolResult::error(&format!("Tool execution failed: {}", e))
        }
    }
}

impl ToolResult {
    fn name(&self) -> &str {
        // Extract name from data if available
        "unknown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_stream_event_serialization() {
        let event = StreamEvent::Thinking {
            content: "Analyzing...".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("thinking"));
    }
    
    #[test]
    fn test_tool_call_event() {
        let event = StreamEvent::ToolCall {
            id: "call_1".to_string(),
            name: "filesystem".to_string(),
            arguments: serde_json::json!({"action": "read", "path": "test.txt"}),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("tool_call"));
        assert!(json.contains("filesystem"));
    }
}
