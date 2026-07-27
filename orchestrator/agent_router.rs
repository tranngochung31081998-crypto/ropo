use std::collections::HashMap;
use anyhow::Result;
use tracing::info;

use crate::agents::{AgentType, AgentPool, AgentAnalysis};

/// Agent Router - routing tasks đến specialist agents
/// Inspired by OpenClaw's plugin routing + Hermes' agent delegation pattern
pub struct AgentRouter {
    agent_pool: AgentPool,
    routing_table: HashMap<String, AgentType>,
}

impl AgentRouter {
    pub fn new() -> Self {
        Self {
            agent_pool: AgentPool::new(),
            routing_table: Self::default_routing_table(),
        }
    }

    /// Default routing table dựa trên task type keywords
    fn default_routing_table() -> HashMap<String, AgentType> {
        let mut table = HashMap::new();
        
        // Architecture & Planning
        table.insert("architecture".into(), AgentType::SeniorArchitect);
        table.insert("design".into(), AgentType::SeniorArchitect);
        table.insert("plan".into(), AgentType::Planner);
        table.insert("strategy".into(), AgentType::Planner);
        
        // Security
        table.insert("security".into(), AgentType::SecurityAuditor);
        table.insert("vulnerability".into(), AgentType::SecurityAuditor);
        table.insert("audit".into(), AgentType::SecurityAuditor);
        
        // Frontend
        table.insert("frontend".into(), AgentType::FrontendDev);
        table.insert("ui".into(), AgentType::UiUxDesigner);
        table.insert("ux".into(), AgentType::UiUxDesigner);
        table.insert("design system".into(), AgentType::UiUxDesigner);
        
        // Backend
        table.insert("backend".into(), AgentType::BackendDev);
        table.insert("api".into(), AgentType::BackendDev);
        table.insert("database".into(), AgentType::BackendDev);
        
        // Testing & Debugging
        table.insert("test".into(), AgentType::Tester);
        table.insert("qa".into(), AgentType::Tester);
        table.insert("bug".into(), AgentType::BugFixer);
        table.insert("fix".into(), AgentType::BugFixer);
        table.insert("debug".into(), AgentType::BugFixer);
        
        // Harness / Context Management
        table.insert("harness".into(), AgentType::Harness);
        table.insert("chunk".into(), AgentType::Harness);
        table.insert("large file".into(), AgentType::Harness);
        table.insert("map-reduce".into(), AgentType::Harness);
        table.insert("blast radius".into(), AgentType::Harness);
        table.insert("dependencies".into(), AgentType::Harness);
        
        // General
        table.insert("review".into(), AgentType::SeniorArchitect);
        table.insert("document".into(), AgentType::Planner);
        table.insert("refactor".into(), AgentType::SeniorArchitect);
        
        table
    }

    /// Get suggested agents based on TaskType enum (for multi-agent collaboration)
    pub fn route_agents_for_task_type(&self, task_type: &crate::orchestrator::TaskType) -> Vec<String> {
        match task_type {
            crate::orchestrator::TaskType::CodeGeneration => vec![
                "senior_architect".into(),
                "backend_dev".into(),
                "frontend_dev".into(),
                "tester".into(),
            ],
            crate::orchestrator::TaskType::CodeReview => vec![
                "senior_architect".into(),
                "security_auditor".into(),
                "tester".into(),
            ],
            crate::orchestrator::TaskType::Debugging => vec![
                "bug_fixer".into(),
                "tester".into(),
                "security_auditor".into(),
            ],
            crate::orchestrator::TaskType::Refactoring => vec![
                "senior_architect".into(),
                "backend_dev".into(),
                "tester".into(),
            ],
            crate::orchestrator::TaskType::Architecture => vec![
                "senior_architect".into(),
                "planner".into(),
                "security_auditor".into(),
            ],
            crate::orchestrator::TaskType::Testing => vec![
                "tester".into(),
                "senior_architect".into(),
            ],
            crate::orchestrator::TaskType::Documentation => vec![
                "planner".into(),
                "senior_architect".into(),
            ],
            crate::orchestrator::TaskType::DevOps => vec![
                "backend_dev".into(),
                "security_auditor".into(),
            ],
            _ => vec![
                "planner".into(),
                "senior_architect".into(),
            ],
        }
    }

    /// Route task đến agent phù hợp
    pub fn route(&self, task_description: &str) -> Result<Vec<AgentType>> {
        let lower = task_description.to_lowercase();
        let mut agents = Vec::new();
        
        for (keyword, agent_type) in &self.routing_table {
            if lower.contains(keyword) {
                if !agents.contains(agent_type) {
                    agents.push(agent_type.clone());
                }
            }
        }
        
        if agents.is_empty() {
            agents.push(AgentType::SeniorArchitect);
            agents.push(AgentType::Planner);
        }
        
        info!("Routed task to agents: {:?}", agents.iter().map(|a| a.to_string()).collect::<Vec<_>>());
        Ok(agents)
    }

    /// Route với context analysis
    pub fn route_with_analysis(
        &self,
        task_description: &str,
        context: &crate::orchestrator::TaskAnalysis,
    ) -> Result<Vec<AgentType>> {
        let mut agents = self.route(task_description)?;
        
        match context.complexity {
            crate::orchestrator::Complexity::Complex | crate::orchestrator::Complexity::VeryComplex => {
                if !agents.contains(&AgentType::SeniorArchitect) {
                    agents.insert(0, AgentType::SeniorArchitect);
                }
                if !agents.contains(&AgentType::SecurityAuditor) {
                    agents.push(AgentType::SecurityAuditor);
                }
            }
            _ => {}
        }
        
        match context.task_type {
            crate::orchestrator::TaskType::CodeGeneration => {
                if !agents.contains(&AgentType::Tester) {
                    agents.push(AgentType::Tester);
                }
            }
            crate::orchestrator::TaskType::Refactoring => {
                if !agents.contains(&AgentType::Tester) {
                    agents.push(AgentType::Tester);
                }
                if !agents.contains(&AgentType::SecurityAuditor) {
                    agents.push(AgentType::SecurityAuditor);
                }
            }
            _ => {}
        }
        
        Ok(agents)
    }

    /// Run a single agent's perspective on a task (used by multi-agent collaboration)
    pub async fn run_agent_perspective(
        &self,
        agent_type: &AgentType,
        task: &str,
    ) -> Result<AgentAnalysis> {
        info!("Running agent '{}' perspective on task", agent_type.to_string());
        
        let agent = self.agent_pool.get_agent(agent_type);
        let analysis = agent.analyze_task(task).await?;
        Ok(analysis)
    }

    /// Execute task với multi-agent collaboration
    pub async fn execute_with_agents(
        &self,
        agents: &[AgentType],
        task: &str,
    ) -> Result<Vec<String>> {
        let mut results = Vec::new();
        
        for agent_type in agents {
            info!("Executing with agent: {}", agent_type.to_string());
            let result = self.agent_pool.get_agent(agent_type).process(task).await?;
            results.push(result);
        }
        
        Ok(results)
    }
}
