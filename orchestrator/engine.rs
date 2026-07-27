use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};
use chrono::Utc;

use crate::config::Config;
use crate::provider::{ProviderRouter, Message, RouterConfig};
use crate::orchestrator::{
    agent_router::AgentRouter,
    task_decomposer::TaskDecomposer,
    AgentOutput, AgentResponse, Complexity, ContextManager, ExecutedToolCall,
    FocusPlan, TaskAnalysis, TaskPlan, TaskType,
};
use crate::agents::{AgentType};
use crate::tools::ToolResult;
use crate::subagent::{SubAgent, SubAgentRequest};
use crate::memory::{MemoryPipeline, MemoryEntry, MemoryType, ConsolidationReport, MemoryStats};

/// Orchestrator Engine - trái tim của CULI Agent
/// Điều phối agent loop, multi-agent collaboration, context management
pub struct Orchestrator {
    pub config: Config,
    provider_router: Arc<Mutex<ProviderRouter>>,
    pub tool_registry: Arc<crate::tools::ToolRegistry>,
    pub context: ContextManager,
    pub metrics: OrchestratorMetrics,
    pub sub_agent: Option<SubAgent>,
    pub session_id: String,
    pub memory: MemoryPipeline,
    task_decomposer: TaskDecomposer,
    /// Multi-agent router (inspired by OpenClaw + Hermes)
    agent_router: AgentRouter,
    last_consolidation: Option<ConsolidationReport>,
}

#[derive(Debug, Clone, Default)]
pub struct OrchestratorMetrics {
    pub total_conversations: u64,
    pub total_tool_calls: u64,
    pub total_tokens: u64,
    pub total_agent_calls: u64,
    pub avg_iterations_per_task: f64,
    pub start_time: chrono::DateTime<chrono::Utc>,
}

impl Orchestrator {
    pub async fn new(config: Config) -> Result<Self> {
        let router_config = RouterConfig {
            max_retries: config.provider.max_retries,
            timeout_seconds: config.provider.timeout_seconds,
            enable_fallback: true,
            enable_token_tracking: true,
            use_composite_tiers: false, // Default to traditional routing
        };

        let provider_router = ProviderRouter::new(router_config);
        let session_id = uuid::Uuid::new_v4().to_string();
        
        Ok(Self {
            config,
            provider_router: Arc::new(Mutex::new(provider_router)),
            tool_registry: Arc::new(crate::tools::ToolRegistry::new()),
            context: ContextManager::new(),
            metrics: OrchestratorMetrics {
                start_time: Utc::now(),
                ..Default::default()
            },
            sub_agent: None,
            session_id,
            memory: MemoryPipeline::new(),
            task_decomposer: TaskDecomposer::new(),
            agent_router: AgentRouter::new(),
            last_consolidation: None,
        })
    }

    pub async fn set_sub_agent(&mut self, sub_agent: SubAgent) {
        let _ = sub_agent.session_manager().start_session(
            &self.session_id,
            &serde_json::json!({"config": format!("{:?}", self.config.agent)}).to_string(),
        );
        self.sub_agent = Some(sub_agent);
    }

    pub async fn query_sub_agent(&mut self, request: SubAgentRequest) -> Result<crate::subagent::SubAgentResponse> {
        match &mut self.sub_agent {
            Some(ref mut agent) => agent.handle_request(request).await,
            None => Err(anyhow::anyhow!("Sub-agent not initialized")),
        }
    }

    pub fn session_id(&self) -> &str { &self.session_id }

    pub async fn log_event(&self, event_type: &str, content: &str) {
        if let Some(ref agent) = self.sub_agent {
            if let Ok(sessions) = agent.session_manager().get_active_sessions() {
                if sessions.contains(&self.session_id) {
                    let _ = agent.session_manager().log_event(&self.session_id, event_type, content);
                }
            }
        }
    }

    pub fn memory_stats(&self) -> MemoryStats { self.memory.stats() }
    pub fn last_consolidation(&self) -> Option<&ConsolidationReport> { self.last_consolidation.as_ref() }

    pub async fn consolidate_memory(&mut self) -> Result<ConsolidationReport> {
        let report = self.memory.consolidate().await?;
        self.last_consolidation = Some(report.clone());
        Ok(report)
    }

    pub async fn observe(&mut self, hook_type: &str, tool_name: &str, input: &str, output: &str) {
        let entry = MemoryEntry::from_observation(hook_type, tool_name, input, output, &self.session_id);
        if let Err(e) = self.memory.observe(entry) {
            warn!("Failed to store observation: {}", e);
        }
    }

    // =========================================================================
    // Multi-Agent Collaboration Pipeline
    // =========================================================================
    // Flow:
    //   1. Decompose task with TaskDecomposer -> TaskAnalysis
    //   2. Route to suggested agents via AgentRouter
    //   3. Run each agent's perspective in sequence (collaboration debate)
    //   4. Synthesize results into unified plan
    //   5. Execute main agent loop with synthesized context
    // =========================================================================

    /// Run the full multi-agent collaboration pipeline for a task
    async fn run_multi_agent_collaboration(
        &mut self,
        user_input: &str,
        task_type: &TaskType,
        complexity: &Complexity,
    ) -> Result<MultiAgentResult> {
        info!("=== Multi-Agent Collaboration Start ===");
        let start = std::time::Instant::now();

        // Build analysis from task type + complexity
        let analysis = TaskAnalysis {
            task_type: task_type.clone(),
            complexity: *complexity,
            estimated_steps: match complexity {
                Complexity::Simple => 2,
                Complexity::Medium => 5,
                Complexity::Complex => 10,
                Complexity::VeryComplex => 15,
            },
            requires_multi_agent: matches!(complexity, Complexity::Complex | Complexity::VeryComplex),
            suggested_agents: self.agent_router.route_agents_for_task_type(task_type),
            risks: vec![],
        };

        // Route: get ordered list of agents for this task
        let agents = self.agent_router.route_with_analysis(user_input, &analysis)?;
        info!("Collaboration agents: {:?}", agents.iter().map(|a| a.to_string()).collect::<Vec<_>>());

        // Run each agent's perspective (sequential for traceability)
        let mut perspectives: Vec<AgentPerspective> = Vec::new();
        for agent_type in &agents {
            let agent_name = agent_type.to_string().to_string();
            info!("Running agent: {}", agent_name);

            let perspective = self.agent_router.run_agent_perspective(agent_type, user_input).await?;
            perspectives.push(AgentPerspective {
                agent_name: agent_name.clone(),
                perspective: perspective.perspective.clone(),
                findings: perspective.findings.clone(),
                recommendations: perspective.recommendations.clone(),
                risks: perspective.risks.clone(),
                confidence: perspective.confidence,
            });

            // Store each perspective in memory
            if self.config.agent.enable_memory {
                let entry = MemoryEntry::new(
                    MemoryType::Working,
                    &format!("[Agent: {}] Task: {} | Findings: {}", 
                        agent_name, user_input, perspective.findings.join("; ")),
                );
                let _ = self.memory.observe(entry);
            }

            self.metrics.total_agent_calls += 1;
        }

        // Synthesize results into a unified collaboration result
        let synthesis = self.synthesize_agent_perspectives(&perspectives, user_input);

        let duration = start.elapsed();
        info!("=== Multi-Agent Collaboration Complete ({:?}, {} agents) ===", duration, perspectives.len());

        Ok(MultiAgentResult {
            perspectives,
            synthesis,
            agents_used: agents,
            duration_ms: duration.as_millis() as u64,
        })
    }

    /// Synthesize multiple agent perspectives into unified guidance
    fn synthesize_agent_perspectives(
        &self,
        perspectives: &[AgentPerspective],
        task: &str,
    ) -> String {
        let mut output = String::from("=== Multi-Agent Synthesis ===\n\n");
        output.push_str(&format!("Task: {}\n\n", task));

        for p in perspectives {
            output.push_str(&format!("## {} (confidence: {:.0}%)\n", p.agent_name, p.confidence * 100.0));
            output.push_str(&format!("Perspective: {}\n", p.perspective));
            
            if !p.findings.is_empty() {
                output.push_str("\nFindings:\n");
                for f in &p.findings {
                    output.push_str(&format!("- {}\n", f));
                }
            }
            if !p.recommendations.is_empty() {
                output.push_str("\nRecommendations:\n");
                for r in &p.recommendations {
                    output.push_str(&format!("- {}\n", r));
                }
            }
            if !p.risks.is_empty() {
                output.push_str("\nRisks:\n");
                for r in &p.risks {
                    output.push_str(&format!("- {}\n", r));
                }
            }
            output.push('\n');
        }

        // Consensus recommendations
        output.push_str("### Consensus\n");
        let all_recs: Vec<&String> = perspectives.iter().flat_map(|p| p.recommendations.iter()).collect();
        if !all_recs.is_empty() {
            output.push_str("Top recommendations across agents:\n");
            let top = &all_recs[..all_recs.len().min(3)];
            for r in top {
                output.push_str(&format!("- {}\n", r));
            }
        }

        output
    }

    // =========================================================================
    // Main Agent Loop
    // =========================================================================
    pub async fn run(&mut self, user_input: &str) -> Result<AgentResponse> {
        info!("=== CULI Agent Loop Started ===");
        info!("Input: {}", user_input);
        let start_time = std::time::Instant::now();

        // ----- 1. Task Decomposition -----
        let decomposition = self.task_decomposer.decompose(user_input)?;
        let task_plan = decomposition.task_plan;
        let analysis = decomposition.analysis;
        let focus_plan = decomposition.focus_plan;
        info!("Task decomposed: {} subtasks, type={:?}, complexity={:?}",
            task_plan.subtasks.len(), analysis.task_type, analysis.complexity);

        // ----- 2. Multi-Agent Collaboration (for complex tasks) -----
        let collaboration_result = if analysis.requires_multi_agent && self.config.agent.enable_memory {
            info!("Launching multi-agent collaboration...");
            let result = self.run_multi_agent_collaboration(
                user_input, &analysis.task_type, &analysis.complexity
            ).await?;
            Some(result)
        } else {
            None
        };

        // ----- 3. Memory Context Retrieval -----
        let memory_context = if self.config.agent.enable_memory {
            let results = self.memory.search(user_input, 5);
            if !results.is_empty() {
                Some(results.iter()
                    .take(3)
                    .map(|r| format!("- [{}] {} (score: {:.2})", r.entry.memory_type_name(), r.entry.title, r.combined_score))
                    .collect::<Vec<_>>()
                    .join("\n"))
            } else { None }
        } else { None };

        // ----- 4. Store task in memory -----
        if self.config.agent.enable_memory {
            let task_entry = MemoryEntry::new(
                MemoryType::Working,
                &format!("Task: {} | Plan: {} | Agents: {}", 
                    user_input, focus_plan.action_sequence.join(" -> "),
                    collaboration_result.as_ref().map_or("none".into(), |c| 
                        c.agents_used.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(", ")))
            );
            let _ = self.memory.observe(task_entry);
        }

        // ----- 5. Build messages with multi-agent context -----
        let messages = self.build_initial_messages(
            user_input, &task_plan,
            memory_context.as_deref(),
            &focus_plan,
            collaboration_result.as_ref(),
        );

        // ----- 6. Main Agent Loop -----
        let mut iteration = 0u32;
        let max_iterations = self.config.agent.max_iterations;
        let mut executed_tools: Vec<ExecutedToolCall> = Vec::new();
        let mut current_messages = messages;
        let mut total_tokens = 0u32;

        loop {
            if iteration >= max_iterations {
                info!("Max iterations ({}) reached", max_iterations);
                break;
            }
            iteration += 1;
            info!("Iteration {}/{}", iteration, max_iterations);

            let tools = self.tool_registry.get_definitions();
            let response = {
                let mut router = self.provider_router.lock().await;
                router.chat(&current_messages, &self.config.provider.model, &tools).await?
            };

            total_tokens += response.usage.total_tokens;
            {
                let mut router = self.provider_router.lock().await;
                router.track_token_usage(&response.provider, &response.usage);
            }

            match response.tool_calls {
                Some(calls) if !calls.is_empty() => {
                    let call_count = calls.len();
                    for call in calls {
                        let tool_start = std::time::Instant::now();
                        let result = match self.tool_registry.execute(&call.function.name, &call.function.arguments).await {
                            Ok(r) => { info!("Tool '{}' succeeded", call.function.name); r }
                            Err(e) => { warn!("Tool '{}' failed: {}", call.function.name, e); ToolResult::error(&e.to_string()) }
                        };
                        let duration = tool_start.elapsed().as_millis() as u64;
                        let tool_success = result.success;
                        let result_value = serde_json::to_value(&result).unwrap_or(serde_json::json!({}));
                        let result_content = serde_json::to_string(&result.data).unwrap_or_default();

                        if self.config.agent.enable_memory {
                            self.observe(
                                if tool_success { "post_tool_use" } else { "post_tool_failure" },
                                &call.function.name, &call.function.arguments, &result_content,
                            ).await;
                        }

                        executed_tools.push(ExecutedToolCall {
                            name: call.function.name.clone(),
                            arguments: serde_json::from_str(&call.function.arguments).unwrap_or(serde_json::json!({})),
                            result: result_value,
                            duration_ms: duration,
                            success: tool_success,
                        });
                        current_messages.push(Message {
                            role: "tool".into(),
                            content: result_content,
                            tool_call_id: Some(call.id),
                            name: Some(call.function.name),
                            tool_calls: None,
                        });
                    }
                    self.metrics.total_tool_calls += call_count as u64;
                    continue;
                }
                _ => {
                    info!("Agent loop completed in {} iterations", iteration);
                    // Build final output with multi-agent synthesis if available
                    let mut content = response.content.unwrap_or_default();
                    if let Some(ref collab) = collaboration_result {
                        content = format!("{}\n\n{}", collab.synthesis, content);
                    }

                    let output = AgentOutput {
                        content,
                        tool_calls: executed_tools,
                        iterations: iteration,
                        tokens_used: total_tokens,
                        metadata: {
                            let mut m = std::collections::HashMap::new();
                            m.insert("provider".into(), response.provider);
                            m.insert("model".into(), response.model);
                            if let Some(focus) = focus_plan.action_sequence.first() {
                                m.insert("first_action".into(), focus.clone());
                            }
                            if let Some(ref collab) = collaboration_result {
                                m.insert("agents_used".into(), collab.agents_used.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(","));
                                m.insert("agent_count".into(), collab.agents_used.len().to_string());
                            }
                            m
                        },
                    };

                    self.metrics.total_conversations += 1;
                    self.metrics.total_tokens += total_tokens as u64;
                    let _ = self.log_event("task_completed", &format!("Completed in {} iterations", iteration)).await;
                    info!("=== CULI Agent Loop Completed ({:?}) ===", start_time.elapsed());

                    return Ok(AgentResponse::Complete(output));
                }
            }
        }

        Ok(AgentResponse::Partial(AgentOutput {
            content: "Maximum iterations reached. Task may be incomplete.".into(),
            tool_calls: executed_tools,
            iterations: iteration,
            tokens_used: total_tokens,
            metadata: std::collections::HashMap::new(),
        }))
    }

    fn build_initial_messages(
        &self,
        user_input: &str,
        task: &TaskPlan,
        memory_context: Option<&str>,
        focus_plan: &FocusPlan,
        collaboration: Option<&MultiAgentResult>,
    ) -> Vec<Message> {
        let memory_section = memory_context
            .map(|ctx| format!("\n\nRelevant Context from Memory:\n{}", ctx))
            .unwrap_or_default();

        let focus_section = format!(
            "\n\nFocus Plan:\n- Estimated time: {}min ({} focus blocks)\n- First action: {}\n- Steps: {}",
            focus_plan.estimated_focus_minutes,
            focus_plan.focus_blocks,
            focus_plan.first_action,
            focus_plan.action_sequence.join(" -> ")
        );

        let multi_agent_section = collaboration
            .map(|c| {
                let agents_str = c.agents_used.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(", ");
                format!(
                    "\n\nMulti-Agent Analysis:\n- Agents consulted: {}\n- Agent perspectives already evaluated\n- Synthesis: {}",
                    agents_str,
                    c.synthesis.chars().take(500).collect::<String>()
                )
            })
            .unwrap_or_default();

        // ──── Load agent brain from skills/ (Orchestrator role) ────
        let skill_loader = crate::skills::SkillLoader::new();
        let brain_section = if skill_loader.has_role("orchestrator") {
            info!("Loading brain for role: orchestrator");
            let brain = skill_loader.load_role("orchestrator");
            let arch  = skill_loader.load_architecture_summary();
            
            let mut section = String::with_capacity(brain.len() + arch.len() + 256);
            if !brain.is_empty() {
                section.push_str("\n\n═══ AGENT BRAIN (Orchestrator) ═══\n");
                section.push_str(&brain);
            }
            if !arch.is_empty() {
                section.push_str("\n\n═══ ARCHITECTURE CONTEXT ═══\n");
                section.push_str(&arch);
            }
            section
        } else {
            String::new()
        };

        let system_prompt = format!(
            r#"You are CULI — a multi-agent AI coding assistant.

## What is CULI?
CULI helps developers ship code faster without traditional coding skills. You solve complex software problems through:
- **Design-first architecture** (visual maps prevent hallucination)
- **Multi-agent collaboration** (specialized experts: coder, reviewer, security, architect)
- **Memory-aware context** (learn from past tasks, avoid repeating mistakes)
- **Process discipline** (anti-hallucination rules, Karpathy principles, surgical changes only)

## Your Capabilities
- Read architecture before touching code (never guess file locations)
- Decompose tasks into dependency graphs (implement bottom-up)
- Route work to specialist agents (right expert for each subtask)
- Validate against real codebase structure (no invented APIs)
- Stay within context budget (summarize large files via harness layer)

## Models You Use
User sees "CULI Models" (Flash/Pro/Coder/Ultra/Vision) — actually Qveris API.
Internal tasks use harness layer (Sixth AI + Blackbox) — free, hidden from user.
Current session: {} (via {})

System Config:
- Max iterations: {}
- Temperature: {}
- Memory: {}

Task:
- ID: {}
- Priority: {:?}
{}
{}
{}
{}

Core Rules:
1. Think before coding — state assumptions explicitly
2. Read architecture context before any structural change
3. Surgical changes only — touch minimum required files
4. Verify with cargo check / npm build before claiming done
5. Use specialist agents for their expertise (don't do everything yourself)
6. If uncertain → ask user, never hallucinate"#,
            self.config.provider.model,
            "provider-chain",  // Will show actual provider in logs
            self.config.agent.max_iterations,
            self.config.agent.temperature,
            if self.config.agent.enable_memory { "enabled" } else { "disabled" },
            task.id,
            task.priority,
            memory_section,
            focus_section,
            multi_agent_section,
            brain_section,
        );

        vec![
            Message { role: "system".into(), content: system_prompt, tool_calls: None, tool_call_id: None, name: None },
            Message { role: "user".into(), content: user_input.to_string(), tool_calls: None, tool_call_id: None, name: None },
        ]
    }
}

// =============================================================================
// Data structures for multi-agent collaboration
// =============================================================================

/// Single agent's perspective on a task
#[derive(Debug, Clone)]
pub struct AgentPerspective {
    pub agent_name: String,
    pub perspective: String,
    pub findings: Vec<String>,
    pub recommendations: Vec<String>,
    pub risks: Vec<String>,
    pub confidence: f32,
}

/// Result of running multi-agent collaboration
#[derive(Debug, Clone)]
pub struct MultiAgentResult {
    pub perspectives: Vec<AgentPerspective>,
    pub synthesis: String,
    pub agents_used: Vec<AgentType>,
    pub duration_ms: u64,
}
