use super::traits::{AgentAnalysis, AgentConfig, AgentRole};
use anyhow::Result;
use async_trait::async_trait;

pub struct HarnessAgent {
    config: AgentConfig,
}

impl HarnessAgent {
    pub fn new() -> Self {
        Self {
            config: AgentConfig {
                name: "harness".to_string(),
                model: "gpt-4o-mini".to_string(), // Dùng model nhỏ/nhanh cho harness
                temperature: 0.2,
                max_tokens: 8192,
                context_window: 128000,
            },
        }
    }
}

#[async_trait]
impl AgentRole for HarnessAgent {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn description(&self) -> &str {
        "Harness Agent - Thực hiện các tác vụ lặt vặt: Đọc file theo chunk (Map-Reduce), Quét Blast Radius (Graphify), Patching Code, cập nhật trạng thái."
    }

    fn system_prompt(&self) -> String {
        r#"You are the Harness Agent in a Vibe Coding environment.
Your job is to support Core Agents (Architect, Dev) by handling context-heavy or repetitive tasks.
Rules:
1. Map-Reduce Chunking: When asked to read a large file, read it in chunks, and summarize it into an Index. Do NOT output the whole file.
2. Blast Radius: Before any code modification is done by the Dev Agent, use the Graphify tool to check dependencies and alert the Dev.
3. Stateful Memory: Update todo.md or CONTEXT.md with the progress.
4. Patching: When applying code changes, use SearchReplace instead of rewriting the whole file.
"#.to_string()
    }

    fn perspective(&self) -> String {
        "Focus on system state, context compression, blast radius analysis, and execution safety.".to_string()
    }

    fn allowed_tools(&self) -> Vec<String> {
        vec![
            "read_file".to_string(),
            "write_file".to_string(),
            "search_replace".to_string(),
            "graphify".to_string(),
            "chunk_reader".to_string(),
        ]
    }

    async fn process(&self, task: &str) -> Result<String> {
        // Implement Harness logic here
        // Call tools (Graphify, ChunkReader)
        Ok(format!("Harness Agent executed task: {}", task))
    }

    async fn analyze_task(&self, _task: &str) -> Result<AgentAnalysis> {
        Ok(AgentAnalysis {
            agent_name: self.name().to_string(),
            perspective: self.perspective(),
            findings: vec!["Task requires context compression or blast radius check.".to_string()],
            recommendations: vec!["Use chunk_reader for large files. Use graphify for dependencies.".to_string()],
            risks: vec!["Potential context bloat if file is not chunked.".to_string()],
            confidence: 0.9,
        })
    }

    fn config(&self) -> AgentConfig {
        self.config.clone()
    }
}
