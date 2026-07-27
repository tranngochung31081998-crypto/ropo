use anyhow::Result;
use async_trait::async_trait;
use tracing::info;

use super::{AgentRole, AgentConfig, AgentAnalysis};

/// Planner Agent - lên kế hoạch chi tiết
/// Focus: Task decomposition, timelines, dependencies, risk assessment
pub struct Planner {
    config: AgentConfig,
}

impl Planner {
    pub fn new() -> Self {
        Self {
            config: AgentConfig {
                name: "planner".into(),
                model: "gpt-4o".into(),
                temperature: 0.5,
                max_tokens: 4096,
                context_window: 128000,
            },
        }
    }
}

#[async_trait]
impl AgentRole for Planner {
    fn name(&self) -> &str {
        "planner"
    }

    fn description(&self) -> &str {
        "Planning Analyst - Creates detailed execution plans, \
        breaks down complex tasks, identifies dependencies, \
        and assesses risks and timelines."
    }

    fn system_prompt(&self) -> String {
        format!(
            r#"Bạn là Planning Analyst chuyên nghiệp.

Config: Model={}, Temperature={}

Rules:
1. Phân tích task thành các bước nhỏ có thể thực thi
2. Xác định dependencies giữa các bước
3. Ước tính thời gian và độ phức tạp
4. Đánh giá rủi ro cho mỗi bước
5. Đề xuất thứ tự thực thi tối ưu
6. Xác định cần resources/tools gì
7. Luôn có Plan B cho các rủi ro chính"#,
            self.config.model,
            self.config.temperature,
        )
    }

    fn perspective(&self) -> String {
        "Planner perspective: structured thinking, task breakdown, \
        dependency tracking, and risk management. \
        Tôi luôn có kế hoạch chi tiết trước khi hành động.".into()
    }

    fn allowed_tools(&self) -> Vec<String> {
        vec![
            "search_files".into(),
            "list_files".into(),
            "read_file".into(),
            "web_search".into(),
        ]
    }

    fn config(&self) -> AgentConfig {
        self.config.clone()
    }

    async fn process(&self, task: &str) -> Result<String> {
        info!("Planner processing task: {}", task);
        
        let plan = format!(
            r#"## Execution Plan

### Task: {task}

### Steps:
1. **Phân tích yêu cầu** - Hiểu rõ requirements và constraints
2. **Thiết kế giải pháp** - Architecture và tech stack decisions
3. **Implement** - Code theo thiết kế
4. **Review & Test** - Quality assurance
5. **Deploy** - Triển khai và monitor

### Dependencies:
- Step 1 → Step 2 → Step 3 → Step 4 → Step 5

### Risks:
- Yêu cầu không rõ ràng cần clarification
- Technical complexity có thể underestimated

### Timeline Estimate:
- Total: ~2-4 hours depending on complexity
- Step 1: 15-30 min
- Step 2: 30-60 min
- Step 3: 60-120 min
- Step 4: 30-60 min
- Step 5: 15-30 min
"#);
        
        Ok(plan)
    }

    async fn analyze_task(&self, task: &str) -> Result<AgentAnalysis> {
        info!("planner analyzing task: {}", task);
        Ok(AgentAnalysis {
            agent_name: self.name().to_string(),
            perspective: self.perspective(),
            findings: vec!["Analyzed: ".to_string() + task.chars().take(100).collect::<String>().as_str()],
            recommendations: vec!["Review the full context for details".to_string()],
            risks: vec!["Incomplete analysis - full LLM integration needed".to_string()],
            confidence: 0.5,
        })
    }
}
