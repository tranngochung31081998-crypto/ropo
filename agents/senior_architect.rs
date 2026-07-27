use anyhow::Result;
use async_trait::async_trait;
use tracing::info;

use super::{AgentRole, AgentConfig, AgentAnalysis};

/// Senior Architect Agent - 15+ năm kinh nghiệm
/// Focus: Architecture, scalability, maintainability, technical debt
pub struct SeniorArchitect {
    config: AgentConfig,
}

impl SeniorArchitect {
    pub fn new() -> Self {
        Self {
            config: AgentConfig {
                name: "senior_architect".into(),
                model: "gpt-4o".into(),
                temperature: 0.7,
                max_tokens: 4096,
                context_window: 128000,
            },
        }
    }

    /// Analyze architecture of existing code/project
    pub fn analyze_architecture(&self, _code_context: &str) -> AgentAnalysis {
        info!("Senior Architect analyzing architecture...");
        
        AgentAnalysis {
            agent_name: self.name().into(),
            perspective: "Senior perspective: focus on architecture, scalability, maintainability".into(),
            findings: vec![
                "Đánh giá tổng quan kiến trúc hệ thống".into(),
                "Xác định các design patterns đang sử dụng".into(),
                "Phát hiện technical debt tiềm ẩn".into(),
            ],
            recommendations: vec![
                "Đề xuất cải tiến kiến trúc nếu cần".into(),
                "SOLID principles review".into(),
                "Scalability assessment".into(),
            ],
            risks: vec![
                "Technical debt có thể ảnh hưởng đến maintainability".into(),
                "Cần cân nhắc trade-offs giữa performance và code clarity".into(),
            ],
            confidence: 0.85,
        }
    }
}

#[async_trait]
impl AgentRole for SeniorArchitect {
    fn name(&self) -> &str {
        "senior_architect"
    }

    fn description(&self) -> &str {
        "Senior Software Architect với 15+ năm kinh nghiệm. \
        Chuyên về kiến trúc phần mềm, scalability, design patterns, \
        và đánh giá technical debt."
    }

    fn system_prompt(&self) -> String {
        format!(
            r#"Bạn là Senior Software Architect với 15+ năm kinh nghiệm.

Perspective: {}
Config: Model={}, Temperature={}

Rules:
1. Luôn nhìn tổng quan hệ thống trước khi đi vào chi tiết
2. Đặt câu hỏi về assumptions và requirements
3. Xác định technical debt và rủi ro tiềm ẩn
4. Đề xuất giải pháp scalable và maintainable
5. Áp dụng SOLID principles, design patterns phù hợp
6. Cân nhắc trade-offs và ưu tiên
7. Code phải clean, testable, và dễ maintain"#,
            self.perspective(),
            self.config.model,
            self.config.temperature,
        )
    }

    fn perspective(&self) -> String {
        "Senior perspective: focus on architecture, scalability, \
        maintainability, and long-term code health. \
        Tôi luôn nhìn xa hơn requirement trước mắt.".into()
    }

    fn allowed_tools(&self) -> Vec<String> {
        vec![
            "read_file".into(),
            "write_file".into(),
            "search_files".into(),
            "list_files".into(),
            "terminal".into(),
            "web_search".into(),
            "web_fetch".into(),
        ]
    }

    fn config(&self) -> AgentConfig {
        self.config.clone()
    }

    async fn process(&self, task: &str) -> Result<String> {
        info!("Senior Architect processing task: {}", task);
        let analysis = self.analyze_architecture(task);
        Ok(analysis.summary())
    }

    async fn analyze_task(&self, task: &str) -> Result<AgentAnalysis> {
        info!("senior_architect analyzing task: {}", task);
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
